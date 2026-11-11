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

use super::*;
use crate::chunk_source::{ChunkSourceResult, import_chunk_source_with_helpers};
use crate::external_integration::PINNED_FLUENT_BIT_REVISION;
use crate::legacy_c::C11TranslationUnit;

const BUSY: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/fluent_bit/busy_state.qv");
const LOCK: &str =
    include_str!("../../../../providers/qcore-provider-chunkio/contracts/chunkio/lock_state.qv");

fn summary(body: &str) -> ChunkSourceSummary {
    let source = format!(
        "struct flb_input_chunk {{ int busy; }};\n\
         struct cio_chunk {{ int lock; }};\n{body}"
    );
    let results = import_chunk_source_with_helpers(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &[],
    )
    .unwrap();
    let [ChunkSourceResult::Summary(summary)] = results.as_slice() else {
        panic!("{results:?}")
    };
    summary.as_ref().clone()
}

fn busy_summary() -> ChunkSourceSummary {
    summary(
        "int flb_input_chunk_release_lock(struct flb_input_chunk *ic) { if(ic->busy == 0) return -1; ic->busy = 0; return 0; }",
    )
}

#[test]
fn native_contract_files_are_checked_against_source_paths() {
    if !crate::test_support::clang_available() {
        return;
    }
    let busy = busy_summary();
    let checked = check_chunk_source_contract(&busy, BUSY).unwrap();
    assert_eq!(ChunkContractCertificate::checked_states(&checked), 4);
    assert_eq!(
        ChunkContractCertificate::function(&checked),
        checked.function()
    );
    assert_eq!(ChunkContractCertificate::contract_source(&checked), BUSY);
    assert_eq!(
        ChunkContractCertificate::dependency_sources(&checked),
        checked.dependency_sources()
    );
    assert_eq!(checked.contract_source(), BUSY);
    let lock = summary("int cio_chunk_is_locked(struct cio_chunk *ch) { return ch->lock; }");
    assert_eq!(
        check_chunk_source_contract(&lock, LOCK)
            .unwrap()
            .checked_states(),
        4
    );
}

#[test]
fn changing_qcore_postconditions_changes_the_verdict() {
    if !crate::test_support::clang_available() {
        return;
    }
    let busy = busy_summary();
    for contract in [
        BUSY.replace("#old_busy - 1", "#old_busy"),
        BUSY.replace("#busy == 0", "#busy == 1"),
        BUSY.replace("#lock == #old_lock", "#lock == 0"),
    ] {
        assert!(
            check_chunk_source_contract(&busy, &contract)
                .unwrap_err()
                .contains("contract mismatch")
        );
    }
    let wrong_source = summary(
        "int flb_input_chunk_release_lock(struct flb_input_chunk *ic) { ic->busy = 0; return 0; }",
    );
    assert!(
        check_chunk_source_contract(&wrong_source, BUSY)
            .unwrap_err()
            .contains("contract mismatch")
    );
}

#[test]
fn missing_assumptions_unknown_predicates_and_wrong_types_fail_closed() {
    if !crate::test_support::clang_available() {
        return;
    }
    let busy = busy_summary();
    for contract in [
        without_condition(BUSY, "exclusive"),
        BUSY.replace("#old_busy == 1", "#old_busy == 2"),
        BUSY.replace("#lock == #old_lock", "arbitrary_predicate"),
        without_condition(BUSY, "exit_3"),
        BUSY.replace("mut, InputChunk", "const, InputChunk"),
        BUSY.replace("mut, InputChunk", "mut, Chunk"),
        BUSY.replace("-> i32", "-> unit"),
        BUSY.replace("quant condition normal", "quant condition any"),
        BUSY.replace("flb_input_chunk_release_lock", "different_function"),
    ] {
        assert!(
            check_chunk_source_contract(&busy, &contract).is_err(),
            "accepted {contract}"
        );
    }
}

#[test]
fn qcore_trivia_does_not_change_predicate_meaning() {
    if !crate::test_support::clang_available() {
        return;
    }
    let contract = BUSY.replace("#old_busy - 1", "(#old_busy /* entry value */ - 1)");
    check_chunk_source_contract(&busy_summary(), &contract).unwrap();
}

#[test]
fn const_contract_rejects_even_an_idempotent_c_write() {
    if !crate::test_support::clang_available() {
        return;
    }
    let source = summary(
        "int cio_chunk_is_locked(struct cio_chunk *ch) { ch->lock = ch->lock; return ch->lock; }",
    );
    assert!(
        check_chunk_source_contract(&source, LOCK)
            .unwrap_err()
            .contains("const QCore borrow")
    );
}

fn without_condition(source: &str, id: &str) -> String {
    let marker = format!(" #{id}(");
    assert_eq!(
        source.lines().filter(|line| line.contains(&marker)).count(),
        1
    );
    source
        .lines()
        .filter(|line| !line.contains(&marker))
        .collect::<Vec<_>>()
        .join("\n")
}
