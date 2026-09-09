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

const OUTCOMES_CONTRACT: &str =
    include_str!("../../../contract/qcore/chunkio/filesystem_outcomes.qcore");
const DELETE_OUTCOMES_CONTRACT: &str =
    include_str!("../../../contract/qcore/chunkio/filesystem_delete.qcore");
const MEMORY_BACKEND_CONTRACT: &str =
    include_str!("../../../contract/qcore/chunkio/memory_backend.qcore");

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

/// The measured structural blocker for `file_outcomes` and `file_delete`.
///
/// The destructor family could transport its declared owned-field inventory
/// because the pinned contract *declares* it: `declared_owned_fields` extracts
/// exactly that set from `memory_backend.qcore`, and the core refuses a
/// projection naming any other set. The cleanup-outcome families have no such
/// referent: their `CleanupProfile` — the syscall name and signature, the
/// presence field, the field list and, decisively, the reset values that
/// assert what a successful syscall leaves behind (`fd = -1`, `map = NULL`) —
/// appears nowhere in either pinned contract. Transporting it would put
/// custody-bearing declared data on the wire with nothing on the contract side
/// to cross-check it against.
///
/// If a future contract revision declares this vocabulary, this test fails and
/// the blocker must be re-evaluated rather than inherited.
#[test]
fn the_cleanup_outcome_contracts_declare_no_field_or_reset_vocabulary() {
    // The destructor contract does name its owned fields, which is why that
    // family's inventory could travel under a contract cross-check.
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

    // The two dependent contracts name none of the profile vocabulary. The
    // comparison is over whole identifiers, not substrings, so
    // `native_unmap_outcomes` is not read as naming `munmap` and
    // `reset_field_count` is not read as naming a reset value.
    for (name, contract) in [
        ("filesystem_outcomes", OUTCOMES_CONTRACT),
        ("filesystem_delete", DELETE_OUTCOMES_CONTRACT),
    ] {
        let identifiers = identifiers(contract);
        for token in [
            // Field names the profile declares.
            "fd",
            "map",
            "alloc_size",
            "map_truncated_warned",
            "path",
            // Syscall and diagnostic callees the profile declares.
            "close",
            "munmap",
            "unlink",
            "__errno_location",
            "cio_errno_print",
            // Declared C types the profile fixes.
            "char",
            "size_t",
            // The declaration form that made the destructor inventory
            // contract-derived in the first place.
            "owned_or_null__",
        ] {
            assert!(
                !identifiers.contains(token),
                "`{name}` now declares `{token}`; the P04-NEXT-B blocker must be re-evaluated"
            );
        }
        // Whole-identifier matching is only meaningful if the contract really
        // does have identifiers, so the check is not vacuous.
        assert!(identifiers.contains("syscall_succeeded"), "{name}");
    }
}

/// Whole identifiers of a contract source, so a substring never passes for a
/// declaration.
///
/// Line comments are stripped first, and deliberately so: `filesystem_delete`
/// *mentions* `fd`, `map` and `path` in its header prose ("The fd/map fields
/// control eligibility; they are bookkeeping"). Prose is not a declaration —
/// `declared_owned_fields` reads `requires` clauses, and nothing can read a
/// comment — so a transported profile still has no machine-checkable referent.
/// That distinction is the finding, not an accident of this helper.
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
