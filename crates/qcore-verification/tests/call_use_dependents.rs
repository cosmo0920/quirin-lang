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

//! The dependent-capability boundary: what a prerequisite certificate may be
//! made of, and the measured reason `file_outcomes` and `file_delete` do not
//! follow `file_attempts` through the process boundary in this increment.
//!
//! Nothing here changes a checker. These are pinned measurements: if the
//! blockers they record ever stop holding, these tests fail and the decision
//! is revisited rather than silently inherited.
#![cfg(not(target_os = "windows"))]

// The fixture is shared with the replay and conformance suites; this file uses
// the native-close half of it.
#[allow(dead_code)]
#[path = "fixtures/call_use_source.rs"]
mod fixture;

use fixture::*;
use qcore_schemas::call_use_summary::PrerequisiteIdentity;

const OUTCOMES_CONTRACT: &str = include_str!(
    "../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_outcomes.qv"
);
const DELETE_OUTCOMES_CONTRACT: &str = include_str!(
    "../../../providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete.qv"
);
const MEMORY_BACKEND_CONTRACT: &str =
    include_str!("../../../providers/qcore-provider-chunkio/contracts/chunkio/memory_backend.qv");

/// A dependent capability may carry only the prerequisite's *identity*: the
/// core re-derives the prerequisite task from its own import and re-runs the
/// prerequisite's own evaluation, so nothing a producer sends is ever read as
/// a prerequisite conclusion. This pins that the identity carries no verdict,
/// no certificate and no reading — only three names the core recomputes.
#[test]
fn a_prerequisite_identity_carries_no_conclusion_only_recomputable_names() {
    let value = serde_json::to_value(PrerequisiteIdentity {
        semantic_profile: "qcore.c_call_result_use.v1".into(),
        task_digest: "a".repeat(64),
        obligation_id: "call_result_use_contract|f".into(),
    })
    .unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.len(), 3, "the identity grew a field");
    for field in ["semantic_profile", "task_digest", "obligation_id"] {
        assert!(object.contains_key(field));
    }
    let text = serde_json::to_string(&value).unwrap();
    for conclusion in [
        "verdict",
        "safe",
        "certificate",
        "witness",
        "cases",
        "calls",
        "paths",
        "reading",
    ] {
        assert!(
            !text.contains(conclusion),
            "a conclusion `{conclusion}` reached the prerequisite identity"
        );
    }
}

/// The prerequisite is re-established, never accepted. Two real imports of the
/// same function differing only in their source bytes produce different
/// identities, and neither validates against the other's context.
#[test]
fn the_core_re_establishes_a_prerequisite_rather_than_accepting_one() {
    if !clang_available() {
        return;
    }
    let first = import(NATIVE_SOURCE, "cio_file_native_close", &ATTEMPT_RECORDS);
    let first = task(&first, "cio_file_unix.c", ATTEMPT_CONTRACT);
    let second = import(
        &NATIVE_SOURCE.replace("struct cio_ctx {int x;};", "struct cio_ctx {int y;};"),
        "cio_file_native_close",
        &ATTEMPT_RECORDS,
    );
    let second = task(&second, "cio_file_unix.c", ATTEMPT_CONTRACT);
    assert_ne!(
        first.prerequisite_identity(),
        second.prerequisite_identity(),
        "changed source bytes must change the prerequisite identity"
    );
    assert!(
        first
            .revalidate_prerequisite(&second.prerequisite_identity())
            .is_err()
    );
    assert!(
        second
            .revalidate_prerequisite(&first.prerequisite_identity())
            .is_err()
    );
    // Re-establishing the prerequisite runs the prerequisite's own contract
    // evaluation, so a false prerequisite contract refuses here too.
    let false_contract = qcore_verification::call_use_source::FileAttemptSourceTask::derive(
        &import(NATIVE_SOURCE, "cio_file_native_close", &ATTEMPT_RECORDS),
        "cio_file_unix.c",
        &ATTEMPT_CONTRACT.replace(
            "ensures normal ignored_results == 0;",
            "ensures normal false;",
        ),
        &policy("cio_file_native_close"),
        target(&import(
            NATIVE_SOURCE,
            "cio_file_native_close",
            &ATTEMPT_RECORDS,
        )),
    )
    .unwrap();
    assert!(
        false_contract
            .revalidate_prerequisite(&false_contract.prerequisite_identity())
            .is_err(),
        "re-establishing a prerequisite must re-run its contract evaluation"
    );
}

/// **Deliberately flipped by card P04-B1, per this test's own design.**
///
/// This test previously pinned the *absence* of a contract-side referent for
/// `file_outcomes` and `file_delete` — the measured blocker P04-NEXT-AB §6.3(a)
/// recorded — and said in as many words that a future contract revision
/// declaring the vocabulary must fail it and force a re-evaluation rather than
/// let the blocker be inherited. P04-B1 is that revision, so the assertion is
/// inverted rather than deleted: both contracts now declare the field
/// inventory, the reset values, the syscall/helper names and the C types, in
/// the same clause idiom that made the destructor family's inventory
/// contract-derived, and `qcore_verification::cleanup_vocabulary` refuses a
/// profile that is not exactly that declaration.
///
/// Blocker (a) is therefore closed. Blocker (b) — that a producer would have
/// to reimplement a symbolic replay evaluator over a fourth transported
/// vocabulary, which is a separate capability definition and not a widening of
/// the third — is untouched by this card and is still what gates P04-NEXT-B2.
#[test]
fn the_cleanup_outcome_contracts_declare_their_field_and_reset_vocabulary() {
    // The destructor contract still names its owned fields; that family is the
    // precedent this declaration follows.
    assert!(
        MEMORY_BACKEND_CONTRACT.contains("owned_or_null__"),
        "the destructor contract must still declare its inventory"
    );
    let declared = qcore_verification::field_destructor::declared_owned_fields(
        MEMORY_BACKEND_CONTRACT,
        "cio_memfs_close",
    )
    .unwrap();
    assert!(!declared.is_empty());

    // Every profile member the earlier census reported missing is now a
    // declared premise, matched over whole identifiers so a substring never
    // passes for a declaration.
    for (policy, contract, expected) in [
        (
            "native_close_outcomes",
            OUTCOMES_CONTRACT,
            &[
                "cleanup_field__0__fd__int__integer_m1",
                "cleanup_syscall__close",
            ][..],
        ),
        (
            "native_unmap_outcomes",
            OUTCOMES_CONTRACT,
            &[
                "cleanup_field__1__alloc_size__unsigned_long__integer_0",
                "cleanup_syscall__munmap",
            ][..],
        ),
        (
            "native_delete_outcomes",
            DELETE_OUTCOMES_CONTRACT,
            &["cleanup_mapping_field__map", "cleanup_syscall__unlink"][..],
        ),
    ] {
        let identifiers = identifiers(contract);
        assert!(
            identifiers.contains("cleanup_vocabulary_v1"),
            "{policy} declares no vocabulary selector"
        );
        for token in expected {
            assert!(
                identifiers.contains(*token),
                "{policy} does not declare {token}"
            );
        }
        // The declaration is machine-readable, not prose: the reader returns
        // the profile the checker binds, reset values included.
        let vocabulary =
            qcore_verification::cleanup_vocabulary::declared_cleanup_vocabulary(contract, policy)
                .unwrap();
        assert!(!vocabulary.shape().fields.is_empty(), "{policy}");
        assert!(!vocabulary.shape().argument_fields.is_empty(), "{policy}");
        assert_eq!(
            vocabulary.shape().root_type,
            "struct cio_file *",
            "{policy}"
        );
        assert_eq!(
            vocabulary.shape().errno_function,
            "__errno_location",
            "{policy}"
        );
        // Still not vacuous: these are the same contracts, with their existing
        // postcondition vocabulary intact.
        assert!(identifiers.contains("syscall_succeeded"), "{policy}");
    }
}

/// Whole identifiers of a contract source, so a substring never passes for a
/// declaration.
///
/// Line comments are stripped first, and deliberately so: `filesystem_delete`
/// *mentions* `fd`, `map` and `path` in its header prose ("The fd/map fields
/// control eligibility; they are bookkeeping"). Prose is not a declaration —
/// nothing can read a comment — so what this helper sees is exactly the
/// declared surface. Before P04-B1 that surface named none of the profile;
/// it now names all of it, and the distinction between the two is why the
/// helper strips comments.
fn identifiers(source: &str) -> std::collections::BTreeSet<String> {
    let stripped: String = source
        .lines()
        .map(|line| match line.find("//") {
            Some(at) => &line[..at],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = std::collections::BTreeSet::new();
    let mut current = String::new();
    for ch in stripped.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
        } else if !current.is_empty() {
            out.insert(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.insert(current);
    }
    out
}
