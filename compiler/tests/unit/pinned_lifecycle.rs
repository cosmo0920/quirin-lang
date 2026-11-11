// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Source-level controls for `W43-CHUNK-SOURCE-PROFILE`.
//!
//! The positive controls run the rechecked ChunkIO importers over the vendored
//! verbatim copies of the pinned units. The negative controls mutate those
//! same pinned bytes — a close that skips a list unlink, the counter or the
//! backend release, a traversal that passes a nonzero `delete` or iterates
//! without a saved next pointer — and each must fail to derive any row. A
//! tampered vendored tree and a mutated unit presented as pinned are refused
//! before any importer runs.

use std::path::{Path, PathBuf};

use super::*;
use qcore_verification::chunk_lifecycle_coverage::{CompositionKind, LifecycleLeg, MODEL_LEGS};

fn vendored_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pinned/fluent-bit")
}

fn vendored() -> PinnedChunkUnits {
    load_vendored_units(&vendored_root()).expect("the vendored pinned units load")
}

/// The five model legs a null/memory close reaches; the two container legs
/// are retained by premise.
const CLOSE_LEGS: [LifecycleLeg; 5] = [
    LifecycleLeg::ChunkTeardown,
    LifecycleLeg::BackendTeardown,
    LifecycleLeg::PrimaryListUnlink,
    LifecycleLeg::ResidencyListUnlink,
    LifecycleLeg::ContextCounters,
];

#[test]
fn the_vendored_units_are_the_profile_pinned_bytes() {
    let units = vendored();
    assert_eq!(
        units.chunk().sha256(),
        profile_pinned_digest(CHUNK_UNIT).unwrap()
    );
    assert_eq!(units.digests().len(), 3);
}

#[test]
fn the_pinned_close_and_ring_close_derive_path_scoped_prefix_compositions() {
    if !crate::test_support::clang_available() {
        return;
    }
    let derived = derive_pinned_lifecycle(&vendored());
    if cfg!(windows) {
        assert!(
            derived.unwrap_err().contains("getpagesize"),
            "the Linux pinned unit must refuse under the Windows C target"
        );
        return;
    }
    let rows = derived.unwrap();
    let names: Vec<_> = rows.iter().map(|row| row.function.as_str()).collect();
    assert_eq!(names, ["cio_chunk_close", "cio_chunk_close_stream"]);
    for row in &rows {
        assert_eq!(row.role, LifecycleRole::DestructionDependent);
        assert!(!row.error_exits_modeled && !row.wrapper_obligations_discharged);
        let model = row.model.as_ref().unwrap();
        assert_eq!(model.kind(), CompositionKind::Prefix);
        for leg in CLOSE_LEGS {
            assert!(model.discharged().contains(&leg), "{}", leg.name());
        }
        assert!(!model.discharged().contains(&LifecycleLeg::ContextTeardown));
        assert!(!model.discharged().contains(&LifecycleLeg::StreamTeardown));
        assert_eq!(model.retained_live(), ["context 0", "stream 0"]);
        assert!(model.program_digest().starts_with("sha256:"));
        let derivation = row.derivation.as_ref().unwrap();
        assert_eq!(derivation.source.function, row.function);
        assert_eq!(derivation.source.file, "cio_chunk.c");
        assert_eq!(
            derivation.source.unit_sha256,
            profile_pinned_digest(CHUNK_UNIT).unwrap()
        );
        assert!(!derivation.checked_paths.is_empty());
        // The filesystem and unsupported-backend closes are not composed, and
        // each is published with its own reason rather than dropped.
        assert!(!derivation.uncovered_paths.is_empty());
        assert!(
            derivation
                .uncovered_paths
                .iter()
                .all(|path| path.reason == UNCOVERED_CLOSE_PATH)
        );
    }
    // The two rows' programs differ, so neither borrows the other's digest.
    assert_ne!(
        rows[0].model.as_ref().unwrap().program_digest(),
        rows[1].model.as_ref().unwrap().program_digest()
    );
    assert!(MODEL_LEGS.len() > CLOSE_LEGS.len());
}

fn mutated(old: &str, new: &str) -> PinnedChunkUnits {
    let mut units = vendored();
    let source = units.chunk.source.replace(old, new);
    assert_ne!(source, units.chunk.source, "inert mutation: {old}");
    units.chunk.source = source;
    units
}

#[test]
fn a_mutated_pinned_unit_derives_no_row() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        // One of the two embedded list memberships is left linked.
        (
            "    mk_list_del(&ch->_head);\n    mk_list_del(&ch->_state_head);\n",
            "    mk_list_del(&ch->_head);\n",
        ),
        // The context counter is never decremented.
        (
            "    /* Adjust counter */\n    cio_chunk_counter_total_sub(ctx);\n",
            "    /* Adjust counter */\n",
        ),
        // The memory backend is never released.
        (
            "    if (type == CIO_STORE_MEM) {\n        cio_memfs_close(ch);\n    }",
            "    if (type == CIO_STORE_MEM) {\n    }",
        ),
        // The chunk allocation itself survives the close.
        (
            "    free(ch->name);\n    free(ch);\n\n    /* Adjust counter */",
            "    free(ch->name);\n\n    /* Adjust counter */",
        ),
        // The ring traversal asks each member's backend to delete its content.
        (
            "        cio_chunk_close(ch, CIO_FALSE);",
            "        cio_chunk_close(ch, CIO_TRUE);",
        ),
        // The ring traversal advances through the member it just closed.
        (
            "    mk_list_foreach_safe(head, tmp, &st->chunks) {\n        ch = mk_list_entry(head, struct cio_chunk, _head);\n        cio_chunk_close(ch, CIO_FALSE);",
            "    mk_list_foreach(head, &st->chunks) {\n        ch = mk_list_entry(head, struct cio_chunk, _head);\n        cio_chunk_close(ch, CIO_FALSE);",
        ),
    ] {
        let units = mutated(old, new);
        let error = derive_pinned_lifecycle(&units)
            .err()
            .unwrap_or_else(|| panic!("a pinned unit mutated at `{old}` must derive no row"));
        eprintln!("mutation `{}` refused: {error}", old.trim());
    }
}

#[test]
fn a_mutated_unit_is_not_accepted_as_the_pinned_unit() {
    let units = vendored();
    let mut chunk = units.chunk.clone();
    chunk.source.push('\n');
    let error = PinnedChunkUnits::from_options(chunk, units.memfs.clone(), units.error.clone())
        .unwrap_err();
    assert!(error.contains("not the profile-pinned unit"), "{error}");
    let mut swapped = units.chunk.clone();
    swapped.path = MEMFS_UNIT.into();
    assert!(
        PinnedChunkUnits::from_options(swapped, units.memfs.clone(), units.error.clone()).is_err()
    );
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::write(&target, std::fs::read(entry.path()).unwrap()).unwrap();
        }
    }
}

#[test]
fn a_tampered_vendored_tree_is_refused() {
    let base = std::env::temp_dir().join(format!(
        "qcore-w43-vendored-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    // A header byte changes.
    let header = base.join("header");
    copy_tree(&vendored_root(), &header);
    let path = header.join("source/lib/monkey/include/monkey/mk_core/mk_list.h");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.push(b'\n');
    std::fs::write(&path, bytes).unwrap();
    let error = load_vendored_units(&header).unwrap_err();
    assert!(error.contains("differs from its pinned digest"), "{error}");
    // An unlisted file appears, which could shadow a listed header.
    let extra = base.join("extra");
    copy_tree(&vendored_root(), &extra);
    std::fs::write(extra.join("source/lib/chunkio/src/cio_utils.h"), b"").unwrap();
    let error = load_vendored_units(&extra).unwrap_err();
    assert!(error.contains("unlisted file"), "{error}");
    std::fs::remove_dir_all(&base).unwrap();
}
