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

//! Source-level acceptance for `source.mutex.guard.v1` (card T03).
//!
//! Every rejection below asserts the exact diagnostic code the core assigns,
//! and most carry an accepted control that differs by one statement. A test
//! that only asserted `is_err()` would not distinguish the intended rule from
//! any other refusal on the way.
use qcore_core::mutex_invariant::{
    AccessFootprint, AccessKind, ConcurrencyAssumption, ConcurrencyProfile,
    MutexInvariantDiagnostic, MutexInvariantEvent, check_mutex_invariant_program,
    recheck_mutex_invariant_certificate,
};
use qcore_frontend::frontend::{elaborate_source, parse_source_program};
use qcore_frontend::source_mutex::{LoweredMutexModule, has_source_mutex, lower_source_mutex};

fn lower(source: &str) -> Result<LoweredMutexModule, Vec<String>> {
    let parsed = parse_source_program(source).map_err(|e| vec![e.to_string()])?;
    assert!(
        has_source_mutex(&parsed),
        "route detection must select this module"
    );
    let typed = elaborate_source(&parsed)
        .map_err(|d| d.into_iter().map(|d| d.code.to_owned()).collect::<Vec<_>>())?;
    lower_source_mutex(&parsed, &typed, "main")
        .map_err(|d| d.into_iter().map(|d| d.code.to_owned()).collect())
}

/// Full route: parse, elaborate, lower, then check with the core. Returns the
/// diagnostic codes in the order the core reports them.
fn codes(source: &str) -> Result<LoweredMutexModule, Vec<String>> {
    let module = lower(source)?;
    match check_mutex_invariant_program(&module.program) {
        Ok(certificate) => {
            recheck_mutex_invariant_certificate(&module.program, &certificate).unwrap();
            Ok(module)
        }
        Err(diagnostics) => Err(diagnostics
            .iter()
            .map(|d| module.locate(d).code.to_owned())
            .collect()),
    }
}

fn entry(body: &str) -> String {
    format!("#import std.concurrency; fn main() -> unit {{ thread scope s {{ {body} }} }}")
}

const OPEN_WRITE_CLOSE: &str = "let buf = protected 4; let m = mutex guards buf; \
     with lock m as g { open g; write buf with g; close g; }";

// ---------------------------------------------------------------------------
// Positive: the profile checks, and names exactly the premises it retains.
// ---------------------------------------------------------------------------

#[test]
fn a_lexical_guard_over_one_fixed_footprint_checks_and_names_its_premises() {
    let module = codes(&entry(OPEN_WRITE_CLOSE)).unwrap();
    let certificate = check_mutex_invariant_program(&module.program).unwrap();
    assert_eq!(
        certificate.profile,
        ConcurrencyProfile::CheckedMutexInvariantV1
    );
    assert_eq!(
        certificate.assumption_names(),
        std::collections::BTreeSet::from([
            "InitialAuthorityAndUniqueLifetimeIds",
            "DeclaredFootprintShape",
            "InvariantPredicateDeclared",
        ]),
        "the lowering discharges guard-footprint and synchronized-handoff authority; \
         the declared shape and the declared predicate remain premises"
    );
    for forbidden in [
        ConcurrencyAssumption::GuardFootprintAuthority,
        ConcurrencyAssumption::SynchronizedCustodyEdges,
    ] {
        assert!(!certificate.assumptions.contains(&forbidden));
    }
    // Every event carries a source span, and the trace is exactly the lowering
    // of the six source statements plus the two scope-exit destructions.
    assert_eq!(module.event_spans.len(), module.program.events.len());
    assert!(matches!(
        module.program.events.first(),
        Some(MutexInvariantEvent::CreateMutex { protects, .. })
            if protects == &[AccessFootprint::new(1, 0, 4)]
    ));
    assert!(matches!(
        module.program.events.last(),
        Some(MutexInvariantEvent::Destroy { resource: 1, .. })
    ));
    assert_eq!(certificate.destroyed_mutexes, vec![1]);
    assert_eq!(certificate.destroyed_resources, vec![1]);
    assert_eq!(
        certificate.conservation_checkpoints,
        module.program.events.len() + 1
    );
}

#[test]
fn two_critical_sections_over_one_generation_mint_two_distinct_guards() {
    let module = codes(&entry(
        "let buf = protected 4; let m = mutex guards buf; \
         with lock m as g { open g; write buf with g; close g; } \
         with lock m as h { read buf with h; }",
    ))
    .unwrap();
    let guards = module
        .program
        .events
        .iter()
        .filter_map(|event| match event {
            MutexInvariantEvent::Lock { guard, .. } => Some(*guard),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        guards,
        vec![1, 2],
        "authority is keyed per critical section"
    );
}

#[test]
fn a_guard_derived_borrow_released_inside_its_section_is_accepted() {
    codes(&entry(
        "let buf = protected 4; let m = mutex guards buf; \
         with lock m as g { let b = borrow buf with g; read buf with g; drop b; }",
    ))
    .unwrap();
}

#[test]
fn an_unprotected_resource_is_reachable_without_a_guard() {
    let module = codes(&entry(
        "let buf = protected 4; let scratch = protected 2; let m = mutex guards buf; write scratch;",
    ))
    .unwrap();
    assert!(
        module.program.events.iter().any(|event| matches!(
            event,
            MutexInvariantEvent::Access {
                kind: AccessKind::Write,
                guard: None,
                ..
            }
        )),
        "an unparked footprint is decided by the pointwise share ledger"
    );
}

// ---------------------------------------------------------------------------
// Acceptance 1: a guard-derived borrow cannot escape, move to a child task, or
// survive unlock.
// ---------------------------------------------------------------------------

#[test]
fn a_borrow_that_outlives_its_critical_section_is_refused() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { let b = borrow buf with g; }",
        ))
        .unwrap_err(),
        // The full fail-closed cascade: the refused release leaves the guard
        // live, which blocks the scope-exit destruction of the generation
        // (Q619) and of the resource it parks, and surfaces in the end-of-trace
        // sweep. Pinning the whole vector proves the refusal did not silently
        // release the guard.
        vec!["Q615", "Q619", "Q600", "Q600", "Q600"],
        "GuardBorrowEscape leads the cascade; it is not a generic unlock complaint"
    );
}

#[test]
fn a_guard_name_does_not_exist_after_its_block() {
    // The binder is body-local (the W6 frontend-gate rule), so the escape is
    // refused during name resolution and never reaches the concurrency model.
    assert_eq!(
        lower(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { read buf with g; } read buf with g;",
        ))
        .unwrap_err(),
        vec!["Q013"]
    );
}

#[test]
fn task_creation_composed_with_a_checked_mutex_is_refused() {
    let source = "#import std.concurrency; fn worker(x: i32) -> i32 { return x; } \
         fn main() -> unit { thread scope s { let buf = protected 4; \
         let m = mutex guards buf; let h = spawn s worker(copy buf); } }";
    assert_eq!(
        lower(source).unwrap_err(),
        vec!["Q600"],
        "completion-capsule composition is refused, never silently admitted"
    );
}

#[test]
fn a_child_task_cannot_act_through_its_parents_guard() {
    // No source form creates a task in this profile, so the escape is exercised
    // against the core directly, on the program this route lowers. The model
    // refuses it at Q612; the source refusal above is the reason it is
    // unreachable from the admitted grammar in the first place.
    let module = lower(&entry(OPEN_WRITE_CLOSE)).unwrap();
    let mut program = module.program.clone();
    let footprint = AccessFootprint::new(1, 0, 4);
    program.events.splice(
        2..2,
        [
            MutexInvariantEvent::Spawn {
                parent: 0,
                child: 1,
            },
            MutexInvariantEvent::Access {
                task: 1,
                footprint,
                kind: AccessKind::Read,
                guard: Some(1),
            },
        ],
    );
    let diagnostics = check_mutex_invariant_program(&program).unwrap_err();
    assert!(
        diagnostics.iter().any(|d| d.code() == "Q612"),
        "expected WrongGuardAffinity, got {diagnostics:?}"
    );
}

// ---------------------------------------------------------------------------
// Acceptance 2: typed-error cleanup restores the invariant before unlock, and
// unsupported cleanup is rejected.
// ---------------------------------------------------------------------------

const ERROR_TYPE: &str = "error Fault { Broken; }";

#[test]
fn a_typed_error_exit_after_restoring_the_invariant_is_accepted() {
    let module = codes(&format!(
        "{ERROR_TYPE} {}",
        entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { open g; write buf with g; close g; raise Fault::Broken; }",
        )
    ))
    .unwrap();
    assert_eq!(
        module.error_exits.get(&1).map(String::as_str),
        Some("Fault::Broken")
    );
    assert!(
        module
            .program
            .events
            .iter()
            .any(|event| matches!(event, MutexInvariantEvent::ErrorExit { exit: 1, .. }))
    );
}

#[test]
fn a_typed_error_exit_with_a_broken_invariant_is_refused() {
    assert_eq!(
        codes(&format!(
            "{ERROR_TYPE} {}",
            entry(
                "let buf = protected 4; let m = mutex guards buf; \
                 with lock m as g { open g; write buf with g; raise Fault::Broken; }",
            )
        ))
        .unwrap_err(),
        vec!["Q614", "Q619", "Q600", "Q600", "Q600"],
        "InvariantOpenAtErrorExit is its own code, distinct from the normal exit"
    );
}

#[test]
fn a_normal_exit_with_a_broken_invariant_is_refused() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { open g; write buf with g; }",
        ))
        .unwrap_err(),
        vec!["Q613", "Q619", "Q600", "Q600", "Q600"],
        "InvariantOpenAtUnlock, and the same cascade as the error exit"
    );
}

#[test]
fn deferred_cleanup_inside_a_critical_section_is_refused() {
    assert_eq!(
        lower(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { defer buf; open g; write buf with g; close g; }",
        ))
        .unwrap_err(),
        vec!["Q600"],
        "unsupported cleanup rejects; it is not silently dropped from the trace"
    );
}

#[test]
fn a_statement_after_a_typed_error_exit_is_refused() {
    assert_eq!(
        lower(&format!(
            "{ERROR_TYPE} {}",
            entry(
                "let buf = protected 4; let m = mutex guards buf; \
                 with lock m as g { raise Fault::Broken; read buf with g; }",
            )
        ))
        .unwrap_err(),
        vec!["Q600"]
    );
}

// ---------------------------------------------------------------------------
// The R03 repair, reached from source.
// ---------------------------------------------------------------------------

#[test]
fn an_unrelated_lock_grants_no_authority_over_a_protected_footprint() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let other = protected 4; \
             let m = mutex guards buf; let n = mutex guards other; \
             with lock n as g { read buf with g; }",
        ))
        .unwrap_err(),
        vec!["Q610"],
        "UnprotectedFootprint: holding an unrelated mutex creates no access authority"
    );
}

#[test]
fn an_unguarded_access_to_a_protected_footprint_has_no_fallback_authority() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; write buf;"
        ))
        .unwrap_err(),
        vec!["Q611"]
    );
}

#[test]
fn a_write_requires_the_invariant_to_be_declared_broken_first() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { write buf with g; }",
        ))
        .unwrap_err(),
        vec!["Q613"],
        "WriteWithClosedInvariant, in the invariant-phase family"
    );
    // The control differs by two statements only.
    codes(&entry(OPEN_WRITE_CLOSE)).unwrap();
}

#[test]
fn two_mutexes_cannot_protect_one_resource() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; let n = mutex guards buf;",
        ))
        .unwrap_err(),
        // The refused creation leaves no second generation, so the scope-exit
        // destruction of `n` names an unminted mutex (Q600 `UnknownMutex`).
        vec!["Q617", "Q600"],
        "FootprintAlreadyProtected falls out of authority conservation"
    );
}

#[test]
fn a_nested_critical_section_is_a_represented_refusal() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { with lock m as h { read buf with h; } }",
        ))
        .unwrap_err(),
        vec!["Q622"],
        "the construct is lowered to an explicit unsupported event, not dropped"
    );
}

#[test]
fn a_reopened_invariant_is_refused() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { open g; open g; write buf with g; close g; }",
        ))
        .unwrap_err(),
        vec!["Q613"]
    );
}

#[test]
fn a_close_without_an_open_is_refused() {
    assert_eq!(
        codes(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { close g; }",
        ))
        .unwrap_err(),
        vec!["Q613"],
        "InvalidInvariantWitness: nothing was declared broken"
    );
}

// ---------------------------------------------------------------------------
// Grammar boundary: everything outside the admitted surface refuses precisely.
// ---------------------------------------------------------------------------

#[test]
fn constructs_outside_the_admitted_grammar_refuse_with_a_located_diagnostic() {
    for body in [
        "let buf = protected 0; let m = mutex guards buf;",
        "let buf = protected 4; let m = mutex guards buf; while (1) { read buf; }",
        "let buf = protected 4; let m = mutex guards buf; with lock m as g { drop buf; }",
        "let buf = protected 4; let m = mutex guards buf; with lock m as g { open g; return; }",
    ] {
        let source = entry(body);
        let errors = lower(&source).unwrap_err();
        assert_eq!(errors, vec!["Q600"], "body: {body}");
    }
    // A name the profile cannot resolve is a name-resolution refusal, and stays
    // one: the concurrency route never invents an identity for it.
    for body in [
        "let buf = protected 4; let m = mutex guards missing;",
        "let buf = protected 4; let m = mutex guards buf; with lock absent as g { read buf with g; }",
    ] {
        assert_eq!(
            lower(&entry(body)).unwrap_err(),
            vec!["Q013"],
            "body: {body}"
        );
    }
}

#[test]
fn a_source_contract_and_a_nonunit_entry_stay_refused() {
    for source in [
        "#import std.concurrency; fn main() -> unit contract { requires x; } { thread scope s { let buf = protected 4; let m = mutex guards buf; } }",
        "#import std.concurrency; fn main() -> i32 { thread scope s { let buf = protected 4; let m = mutex guards buf; } }",
    ] {
        assert!(matches!(
            lower(source).unwrap_err().as_slice(),
            [code] if code == "Q600" || code == "Q021" || code == "Q020"
        ));
    }
}

// ---------------------------------------------------------------------------
// Card T03-F: the guard-profile completions.
//
// Three source forms land here, and each is tested in both directions: what it
// admits, and what it refuses. A form that only ever accepted would not show
// that the rule is the rule.
// ---------------------------------------------------------------------------

const DECLARATIONS: &str = "let buf = protected 4; let m = mutex guards buf;";

fn premises(module: &LoweredMutexModule) -> std::collections::BTreeSet<&'static str> {
    check_mutex_invariant_program(&module.program)
        .expect("this fixture checks")
        .assumption_names()
}

/// Sub-item 1. A section that only read while its invariant was open closes
/// with the **checked** witness, and its certificate drops
/// `InvariantPredicateDeclared`. The same section with one write keeps it.
#[test]
fn a_read_only_open_region_closes_with_the_checked_witness_and_drops_the_declared_premise() {
    let checked = codes(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ open g; read buf with g; close g unchanged; }}"
    )))
    .unwrap();
    assert_eq!(
        premises(&checked),
        std::collections::BTreeSet::from([
            "InitialAuthorityAndUniqueLifetimeIds",
            "DeclaredFootprintShape",
        ]),
        "a checked witness is the checker's own fact, so no predicate is declared"
    );
    assert!(matches!(
        checked
            .program
            .events
            .iter()
            .find(|event| matches!(event, MutexInvariantEvent::CloseInvariant { .. })),
        Some(MutexInvariantEvent::CloseInvariant {
            witness: qcore_core::mutex_invariant::InvariantWitness::NoWriteOccurred,
            ..
        })
    ));

    // The control differs by one word and keeps the premise.
    let declared = codes(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ open g; read buf with g; close g; }}"
    )))
    .unwrap();
    assert!(premises(&declared).contains("InvariantPredicateDeclared"));
}

/// The decidable shape is exactly "every statement since `open` was a read".
/// Anything else is refused here — never silently downgraded to the declared
/// witness, which would hide the difference the premise records.
#[test]
fn an_undecidable_open_region_refuses_the_checked_witness_rather_than_downgrading_it() {
    for body in [
        // a write while open
        "open g; write buf with g; close g unchanged;",
        // a guard-derived borrow: this profile models no write through it, and
        // does not claim to
        "open g; let b = borrow buf with g; drop b; close g unchanged;",
        // an announced acquisition
        "open g; wait m; close g unchanged;",
        // a child task's statements are statements too
        "open g; thread scope c { } close g unchanged;",
        // no open region at all
        "close g unchanged;",
    ] {
        let source = entry(&format!("{DECLARATIONS} with lock m as g {{ {body} }}"));
        assert_eq!(lower(&source).unwrap_err(), vec!["Q600"], "body: {body}");
    }
}

/// The region is tracked per guard and per region: a write in an earlier region
/// of the same section does not license the checked witness, and does not
/// forbid it either.
#[test]
fn the_checked_witness_is_tracked_per_guard_and_per_region() {
    let module = codes(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ open g; write buf with g; close g; \
         open g; read buf with g; close g unchanged; }}"
    )))
    .unwrap();
    assert!(
        premises(&module).contains("InvariantPredicateDeclared"),
        "the first region closed by declaration, so the section still carries the premise"
    );
    let witnesses = module
        .program
        .events
        .iter()
        .filter_map(|event| match event {
            MutexInvariantEvent::CloseInvariant { witness, .. } => Some(*witness),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        witnesses,
        vec![
            qcore_core::mutex_invariant::InvariantWitness::Declared { invariant: 1 },
            qcore_core::mutex_invariant::InvariantWitness::NoWriteOccurred,
        ]
    );
}

/// Sub-item 3. A fact established under one guard is usable under that guard.
#[test]
fn a_guard_scoped_fact_is_usable_inside_the_guard_that_established_it() {
    let module = codes(&entry(&format!(
        "{DECLARATIONS} let f = fact buf; \
         with lock m as g {{ assert f with g; read buf with g; use f; }}"
    )))
    .unwrap();
    assert!(
        module
            .program
            .events
            .iter()
            .any(|event| matches!(event, MutexInvariantEvent::AssertFact { fact: 2, .. }))
    );
    assert!(
        module
            .program
            .events
            .iter()
            .any(|event| matches!(event, MutexInvariantEvent::UseFact { fact: 2, .. }))
    );
}

/// …and unusable outside it. This is CT48's mechanism reached from source: the
/// release dropped the fact, so the next section's use is stale rather than
/// unresolvable.
#[test]
fn a_guard_scoped_fact_does_not_survive_the_release_that_dropped_it() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} let f = fact buf; \
             with lock m as g {{ assert f with g; }} \
             with lock m as h {{ use f; }}"
        )))
        .unwrap_err(),
        vec!["Q616"],
        "a fact from a released section is stale, not unknown and not silently live"
    );
    // Re-establishing it under the second guard is a *new* fact and checks.
    codes(&entry(&format!(
        "{DECLARATIONS} let f = fact buf; \
         with lock m as g {{ assert f with g; }} \
         with lock m as h {{ assert f with h; use f; }}"
    )))
    .unwrap();
}

#[test]
fn a_fact_is_refused_before_it_is_asserted_and_outside_every_guard() {
    // Declared but never established: the identity the declaration denotes is
    // one no assertion mints, so the core refuses it as unknown.
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} let f = fact buf; with lock m as g {{ use f; }}"
        )))
        .unwrap_err(),
        vec!["Q600"]
    );
    // Used with no guard held at all.
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} let f = fact buf; with lock m as g {{ assert f with g; }} use f;"
        )))
        .unwrap_err(),
        vec!["Q616"]
    );
    // An undeclared fact name is a name error, and is deliberately not
    // relabelled as a concurrency verdict — the same separation `Q013` already
    // has for an unknown guard or mutex.
    assert_eq!(
        lower(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ assert nothing with g; }}"
        )))
        .unwrap_err(),
        vec!["Q013"]
    );
}

/// A child task may name a fact its parent established, and is refused: the
/// fact is keyed to the guard's holder, not to the mutex.
#[test]
fn a_child_task_cannot_rely_on_its_parents_fact() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} let f = fact buf; \
             with lock m as g {{ assert f with g; thread scope c {{ use f; }} }}"
        )))
        .unwrap_err(),
        vec!["Q616", "Q600"],
        "the stale-fact refusal leads; the join inside the guard follows"
    );
}

/// Sub-item 2. The admitted composition: a child task takes the generation
/// itself, and the creator reads after joining it.
#[test]
fn a_child_task_may_take_the_generation_and_the_creator_reads_after_the_join() {
    let module = codes(&entry(&format!(
        "{DECLARATIONS} \
         thread scope c {{ with lock m as gc {{ open gc; write buf with gc; close gc; }} }} \
         with lock m as g {{ read buf with g; }}"
    )))
    .unwrap();
    let events = &module.program.events;
    assert!(events.iter().any(|event| matches!(
        event,
        MutexInvariantEvent::Spawn {
            parent: 0,
            child: 1
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        MutexInvariantEvent::Join {
            parent: 0,
            child: 1
        }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, MutexInvariantEvent::Lock { task: 1, .. }))
    );
    let certificate = check_mutex_invariant_program(&module.program).unwrap();
    assert_eq!(certificate.completed_tasks, vec![1]);
    assert_eq!(certificate.destroyed_resources, vec![1]);
}

/// The child holds no share, so every access it makes is mutex-mediated or
/// refused. Nothing here grants a child authority the creator did not park.
#[test]
fn a_child_task_reaches_a_protected_resource_only_through_its_own_acquisition() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} thread scope c {{ read buf; }}"
        )))
        .unwrap_err(),
        vec!["Q611"]
    );
}

/// CT28's mechanism from source: a child written inside a critical section can
/// *name* the creator's guard, and acting through it is refused.
#[test]
fn a_child_task_cannot_act_through_the_guard_it_lexically_sees() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ thread scope c {{ read buf with g; }} }}"
        )))
        .unwrap_err(),
        vec!["Q612", "Q600"]
    );
    for body in ["open g;", "close g;", "let b = borrow buf with g;"] {
        let source = entry(&format!(
            "{DECLARATIONS} with lock m as g {{ thread scope c {{ {body} }} }}"
        ));
        assert_eq!(
            codes(&source).unwrap_err().first().map(String::as_str),
            Some("Q612"),
            "body: {body}"
        );
    }
}

/// CT32's mechanism from source: the child's join lands inside the guard, and
/// the core refuses it. The control moves the same block out of the section.
#[test]
fn a_child_task_created_inside_a_critical_section_cannot_be_joined_there() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ read buf with g; thread scope c {{ }} }}"
        )))
        .unwrap_err(),
        vec!["Q600"],
        "JoinWhileGuardHeld, and nothing else: the child itself is well formed"
    );
    codes(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ read buf with g; }} thread scope c {{ }}"
    )))
    .unwrap();
}

/// A child may not release its parent's borrow.
#[test]
fn a_child_task_may_not_end_a_borrow_it_did_not_take() {
    // Deliberately a grammar refusal: the model's `EndBorrow` checks only that
    // the acting task is live, so lowering this would be *accepted* there.
    assert_eq!(
        lower(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ let b = borrow buf with g; \
             thread scope c {{ drop b; }} drop b; }}"
        )))
        .unwrap_err(),
        vec!["Q600"]
    );
}

/// Declarations stay with the creating task at thread-scope level.
#[test]
fn a_child_task_declares_no_resource_generation_or_fact() {
    for body in [
        "let other = protected 4;",
        "let n = mutex guards buf;",
        "let f = fact buf;",
    ] {
        assert_eq!(
            lower(&entry(&format!(
                "{DECLARATIONS} thread scope c {{ {body} }}"
            )))
            .unwrap_err(),
            vec!["Q600"],
            "body: {body}"
        );
    }
}

/// The scoped task profile's own expression forms remain refused: their
/// captures, owned values and completion capsules have no representation here.
#[test]
fn the_scoped_task_expression_forms_are_still_refused() {
    let source = "#import std.concurrency; \
         fn worker(value: i32 use one) -> unit { } \
         fn main() -> unit { thread scope s { \
         let buf = protected 4; let m = mutex guards buf; \
         let h = spawn s worker(copy buf); join move h; } }";
    assert_eq!(
        lower(source).unwrap_err(),
        vec!["Q600"],
        "the scoped task profile's expressions are refused before anything else is examined"
    );
}

/// CT31's mechanism from source: an announced acquisition outlives the
/// announcing task and blocks the generation's destruction.
#[test]
fn an_announced_acquisition_blocks_the_generations_destruction() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} thread scope c {{ wait m; }}"
        )))
        .unwrap_err(),
        vec!["Q619", "Q600", "Q600", "Q600"],
        "NonquiescentMutexDestruction leads; the refused destruction leaves the \
         generation, the resource and the waiter outstanding"
    );
    // The control: the announcing task performs the acquisition, which
    // withdraws the announcement.
    codes(&entry(&format!(
        "{DECLARATIONS} thread scope c {{ wait m; with lock m as gc {{ read buf with gc; }} }}"
    )))
    .unwrap();
}

/// The program printed in `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02c must
/// check, or the documented grammar is not the implemented one.
#[test]
fn the_documented_grammar_example_checks() {
    let source = r#"#import std.concurrency;
error Fault { Broken; }
fn main() -> unit {
    thread scope s {
        let buf = protected 4;
        let m = mutex guards buf;
        with lock m as g {
            open g;
            write buf with g;
            close g;
            let b = borrow buf with g;
            read buf with g;
            drop b;
        }
        with lock m as g2 { read buf with g2; }
        let f = fact buf;
        with lock m as g3 {
            assert f with g3;
            open g3;
            read buf with g3;
            use f;
            close g3 unchanged;
        }
        thread scope c {
            with lock m as gc { read buf with gc; }
        }
        thread scope d share buf under m {
            with lock m as gd { read buf with gd; }
        }
    }
}
"#;
    codes(source).unwrap();
}

// ---------------------------------------------------------------------------
// Guard-local children (card T03-GUARD-JOIN).
//
// `thread scope c under g { … }` opts a child into the core's guard-local join.
// Every refusal below is one statement away from the accepted control, and the
// plain `thread scope c { … }` form is untouched throughout.
// ---------------------------------------------------------------------------

/// The accepted control: one mutex-silent child created and joined strictly
/// inside its own critical section.
#[test]
fn a_guard_local_child_created_and_joined_inside_its_section_checks() {
    let module = codes(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ read buf with g; thread scope c under g {{ }} \
         read buf with g; }}"
    )))
    .unwrap();
    let certificate = check_mutex_invariant_program(&module.program).unwrap();
    assert_eq!(certificate.admitted_guard_local_joins.len(), 1);
    assert_eq!(certificate.completed_tasks, vec![1]);
    // The admission is checked: it names no premise of its own.
    assert_eq!(
        certificate.assumptions,
        std::collections::BTreeSet::from([
            ConcurrencyAssumption::InitialAuthorityAndUniqueLifetimeIds,
            ConcurrencyAssumption::DeclaredFootprintShape,
        ])
    );
    // The child's join lowered to the guard-local event, and to nothing else:
    // an ordinary `Join` here would be the refused direction.
    assert!(
        module
            .program
            .events
            .iter()
            .any(|event| matches!(event, MutexInvariantEvent::GuardLocalJoin { .. }))
    );
    assert!(
        !module
            .program
            .events
            .iter()
            .any(|event| matches!(event, MutexInvariantEvent::Join { .. }))
    );
}

/// The opt-in is real: the same block without `under g` keeps CT32's refusal.
#[test]
fn the_plain_child_form_inside_a_section_still_refuses() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ read buf with g; thread scope c {{ }} }}"
        )))
        .unwrap_err(),
        vec!["Q600"]
    );
}

/// A guard-local child that tries to take a generation is refused twice over:
/// the acquisition itself is this profile's represented `NestedProtectingMutex`
/// refusal (`Q622`, because the child inherits the enclosing section lexically),
/// and the guard-local join is refused (`Q600`) because the child named a mutex
/// at all. The self-deadlock is never representable.
#[test]
fn a_guard_local_child_that_locks_a_generation_is_refused() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ thread scope c under g \
             {{ with lock m as gc {{ read buf with gc; }} }} }}"
        )))
        .unwrap_err(),
        vec!["Q622", "Q600"]
    );
}

/// A guard-local child that acts through the guard it lexically sees is refused
/// at `Q612` before the join rule is even reached, and the join refuses too.
#[test]
fn a_guard_local_child_cannot_act_through_the_creators_guard() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ thread scope c under g {{ read buf with g; }} }}"
        )))
        .unwrap_err(),
        vec!["Q612", "Q600"]
    );
}

/// `under` names *this* section. Outside one, and naming another guard, are
/// grammar refusals here rather than silent downgrades to a plain join.
#[test]
fn the_guard_local_header_must_name_its_own_enclosing_section() {
    assert_eq!(
        lower(&entry(&format!(
            "{DECLARATIONS} thread scope c under g {{ }}"
        )))
        .unwrap_err(),
        vec!["Q013"],
        "no section is open, so the guard name does not even resolve"
    );
    assert_eq!(
        lower(&entry(
            "let buf = protected 4; let m = mutex guards buf; \
             with lock m as g { with lock m as g2 { } thread scope c under g2 { } }"
        ))
        .unwrap_err(),
        vec!["Q013"],
        "a guard binder is body-local, so a sibling section's guard is unresolvable"
    );
}

/// The guard-local admission does not survive leaving the section: a child
/// written after the block still uses the plain form, and `under` there is a
/// name-resolution refusal because guard binders are body-local.
#[test]
fn a_guard_local_header_cannot_escape_its_section() {
    codes(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ read buf with g; }} thread scope c {{ }}"
    )))
    .unwrap();
    assert_eq!(
        lower(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ read buf with g; }} thread scope c under g {{ }}"
        )))
        .unwrap_err(),
        vec!["Q013"]
    );
}

/// A guard-local child may itself create a quiet grandchild; a grandchild that
/// names a mutex refuses the whole admission.
#[test]
fn the_quiet_subtree_rule_reaches_a_grandchild() {
    codes(&entry(&format!(
        "{DECLARATIONS} with lock m as g {{ thread scope c under g {{ thread scope d {{ }} }} }}"
    )))
    .unwrap();
    // The grandchild's announced acquisition refuses the join (`Q600`) and, as
    // the pinned cascade shows, keeps the waiter outstanding so the generation
    // cannot be destroyed either (`Q619`). Nothing here silently released it.
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ thread scope c under g \
             {{ thread scope d {{ wait m; }} }} }}"
        )))
        .unwrap_err(),
        vec!["Q619", "Q600", "Q600", "Q600", "Q600"]
    );
}

/// A guard-local join with the invariant open is refused: protected memory is
/// in a declared-broken state where the join lands.
#[test]
fn a_guard_local_join_with_an_open_invariant_is_refused() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ open g; thread scope c under g {{ }} \
             write buf with g; close g; }}"
        )))
        .unwrap_err(),
        vec!["Q600"]
    );
}

/// A guard-local join with a live guard-derived borrow is refused.
#[test]
fn a_guard_local_join_with_a_live_borrow_is_refused() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ let b = borrow buf with g; \
             thread scope c under g {{ }} drop b; }}"
        )))
        .unwrap_err(),
        vec!["Q600"]
    );
}

/// A guard-local child that announces an acquisition of the enclosing
/// generation is exactly the deadlock the blanket refusal covered.
#[test]
fn a_guard_local_child_that_waits_for_the_held_generation_is_refused() {
    assert_eq!(
        codes(&entry(&format!(
            "{DECLARATIONS} with lock m as g {{ thread scope c under g {{ wait m; }} }}"
        )))
        .unwrap_err(),
        vec!["Q619", "Q600", "Q600", "Q600", "Q600"]
    );
}

// ---------------------------------------------------------------------------
// Card CT33-RANKS: the opt-in lock-rank discipline, reached from source.
//
// The point of these cases is that a lock-order cycle is now *writable*. Before
// this card no source form declared a rank, so the shape CT33 describes could
// not be expressed at all and the profile's refusal was untestable from source.
// ---------------------------------------------------------------------------

/// Two protected resources and two ranked mutexes over them. `a` sits strictly
/// below `b`, so nesting `b` inside `a` is the admitted order.
const RANKED: &str = "let x = protected 4; let y = protected 4; \
     let a = mutex rank 1 guards x; let b = mutex rank 2 guards y;";

/// The accepted control: acquisitions in strictly increasing rank order. This
/// is one statement away from every refusal below.
#[test]
fn nested_guards_in_increasing_rank_order_check() {
    let module = codes(&entry(&format!(
        "{RANKED} with lock a as ga {{ with lock b as gb {{ open gb; write y with gb; \
         close gb; }} }}"
    )))
    .unwrap();
    let certificate = check_mutex_invariant_program(&module.program).unwrap();
    assert_eq!(
        certificate.declared_lock_ranks,
        std::collections::BTreeMap::from([(1, 1), (2, 2)]),
        "the certificate records the declared order verbatim"
    );
    // The discipline adds no premise: a rank is a declared position, not a
    // claim about the world, and a passing check is not a deadlock proof.
    assert_eq!(
        certificate.assumption_names(),
        std::collections::BTreeSet::from([
            "InitialAuthorityAndUniqueLifetimeIds",
            "DeclaredFootprintShape",
            "InvariantPredicateDeclared",
        ])
    );
    recheck_mutex_invariant_certificate(&module.program, &certificate).unwrap();
}

/// CT33 itself: a lock-order cycle. One section nests `b` inside `a` and the
/// next nests `a` inside `b`. The second is refused at `Q624`.
#[test]
fn a_lock_order_cycle_is_refused() {
    assert_eq!(
        codes(&entry(&format!(
            "{RANKED} with lock a as ga {{ with lock b as gb {{ }} }} \
             with lock b as gc {{ with lock a as gd {{ }} }}"
        )))
        .unwrap_err(),
        vec!["Q624", "Q612"],
        "the descending acquisition is the refusal; the ascending one checks. `Q612` is \
         its fallout: the refused `Lock` minted no guard, so the section's own `Unlock` \
         names a dead one"
    );
}

/// A three-mutex cycle: `a < b < c` individually, closed by acquiring `a` while
/// holding `c`. Only the closing edge is refused.
#[test]
fn a_three_mutex_cycle_is_refused_at_its_closing_edge() {
    let declarations = "let x = protected 4; let y = protected 4; let z = protected 4; \
         let a = mutex rank 1 guards x; let b = mutex rank 2 guards y; \
         let c = mutex rank 3 guards z;";
    assert_eq!(
        codes(&entry(&format!(
            "{declarations} with lock a as g1 {{ with lock b as g2 {{ }} }} \
             with lock b as g3 {{ with lock c as g4 {{ }} }} \
             with lock c as g5 {{ with lock a as g6 {{ }} }}"
        )))
        .unwrap_err(),
        vec!["Q624", "Q612"],
        "only the closing edge is refused; the two ascending edges check"
    );
    // Every edge in increasing order, same three sections, checks.
    codes(&entry(&format!(
        "{declarations} with lock a as g1 {{ with lock b as g2 {{ }} }} \
         with lock b as g3 {{ with lock c as g4 {{ }} }} \
         with lock a as g5 {{ with lock c as g6 {{ }} }}"
    )))
    .unwrap();
}

/// Equal ranks are refused, not admitted: the order is a strict one, so two
/// generations at one rank may never be nested in either direction.
#[test]
fn equal_ranks_may_not_be_nested() {
    let declarations = "let x = protected 4; let y = protected 4; \
         let a = mutex rank 7 guards x; let b = mutex rank 7 guards y;";
    assert_eq!(
        codes(&entry(&format!(
            "{declarations} with lock a as ga {{ with lock b as gb {{ }} }}"
        )))
        .unwrap_err(),
        vec!["Q624", "Q612"]
    );
    assert_eq!(
        codes(&entry(&format!(
            "{declarations} with lock b as gb {{ with lock a as ga {{ }} }}"
        )))
        .unwrap_err(),
        vec!["Q624", "Q612"],
        "neither direction is the admitted one"
    );
    // Both sections separately, with no nesting, check under the same ranks.
    codes(&entry(&format!(
        "{declarations} with lock a as ga {{ }} with lock b as gb {{ }}"
    )))
    .unwrap();
}

/// Re-locking a generation the task already holds is refused by the existing
/// `DuplicateLockCapability` (`Q600`), **not** by the new `Q624`, even though
/// its equal rank also violates R2. The discipline is additive: it is the last
/// precondition of `Lock` and never displaces a refusal the profile already
/// issues.
#[test]
fn a_self_relock_keeps_its_existing_refusal_in_rank_mode() {
    let ranked = codes(&entry(
        "let x = protected 4; let a = mutex rank 1 guards x; \
         with lock a as g1 { with lock a as g2 { } }",
    ))
    .unwrap_err();
    assert!(
        ranked.contains(&"Q600".to_owned()) && !ranked.contains(&"Q624".to_owned()),
        "the held generation is the stronger, rank-free fact: {ranked:?}"
    );
    // Unranked, the same source is not even expressible: nesting without a
    // declared order keeps its `Q622` represented refusal, unchanged.
    assert_eq!(
        codes(&entry(
            "let x = protected 4; let a = mutex guards x; \
             with lock a as g1 { with lock a as g2 { } }",
        ))
        .unwrap_err(),
        vec!["Q622"]
    );
}

/// R1: a module that ranks some of its mutexes and not others has no lock
/// order, so it is refused rather than checked over its ranked part.
#[test]
fn a_module_that_mixes_ranked_and_unranked_mutexes_is_refused() {
    let mixed = codes(&entry(
        "let x = protected 4; let y = protected 4; \
         let a = mutex rank 1 guards x; let b = mutex guards y; \
         with lock a as ga { } with lock b as gb { }",
    ))
    .unwrap_err();
    assert_eq!(
        mixed[0], "Q625",
        "the unranked generation is the refusal; the rest is fallout from its \
         creation never being admitted, {mixed:?}"
    );
    assert!(mixed.iter().any(|code| code == "Q600"));
    // Ranking the second mutex too is the one-declaration repair.
    codes(&entry(
        "let x = protected 4; let y = protected 4; \
         let a = mutex rank 1 guards x; let b = mutex rank 2 guards y; \
         with lock a as ga { } with lock b as gb { }",
    ))
    .unwrap();
}

/// Declaring the same mutex name twice is refused before any rank question is
/// reached, ranked spelling or not: a redeclaration is a name-binding failure.
#[test]
fn a_redeclared_mutex_name_is_refused_with_either_spelling() {
    for declarations in [
        "let x = protected 4; let m = mutex rank 1 guards x; let m = mutex rank 2 guards x;",
        "let x = protected 4; let m = mutex rank 1 guards x; let m = mutex guards x;",
    ] {
        assert_eq!(
            codes(&entry(declarations)).unwrap_err(),
            vec!["Q012"],
            "a duplicate binding is a name-resolution refusal, reached before any \
             rank question: `{declarations}`"
        );
    }
}

/// A malformed rank is refused by the grammar, never defaulted. `integer` is
/// `u32`-valued, so a sign, an overflow and a non-literal all land here.
#[test]
fn a_malformed_lock_rank_is_refused() {
    for declaration in [
        "let x = protected 4; let m = mutex rank guards x;",
        "let x = protected 4; let m = mutex rank x guards x;",
        "let x = protected 4; let m = mutex rank -1 guards x;",
        "let x = protected 4; let m = mutex rank 4294967296 guards x;",
        "let x = protected 4; let m = mutex rank 1 x;",
        "let x = protected 4; let m = mutex rank 1 guards;",
    ] {
        assert_eq!(
            codes(&entry(declaration)).unwrap_err(),
            vec!["Q600"],
            "`{declaration}`"
        );
    }
}

/// The ranked spelling selects this route and needs the concurrency import,
/// exactly as the unranked one does. Without both, the ranked declaration
/// would be the one mutex form usable with no import.
#[test]
fn the_ranked_spelling_selects_the_route_and_requires_the_import() {
    let parsed = parse_source_program(
        "fn main() -> unit { thread scope s { let x = protected 4; \
         let m = mutex rank 1 guards x; } }",
    )
    .unwrap();
    assert!(has_source_mutex(&parsed));
    assert_eq!(
        elaborate_source(&parsed)
            .unwrap_err()
            .into_iter()
            .map(|d| d.code.to_owned())
            .collect::<Vec<_>>(),
        vec!["Q015"],
        "the missing `#import std.concurrency;` is refused before checking"
    );
}

/// `rank` joins the profile keywords in the reserved-word list, and that
/// reservation is load-bearing rather than decorative: without it the ranked
/// declaration collects `rank` as a referenced name and refuses at `Q013`.
/// Measured alongside it: a resource still *may* be named `rank`, exactly as
/// one may be named `guards` today, so the reservation costs nothing already
/// written. Both halves are asserted so a future edit cannot quietly drop
/// either.
#[test]
fn reserving_rank_is_load_bearing_and_costs_no_existing_spelling() {
    // Load-bearing half: the ranked declaration parses and checks.
    codes(&entry(
        "let x = protected 4; let a = mutex rank 1 guards x; with lock a as g { }",
    ))
    .unwrap();
    // No-cost half: `rank` and the already-reserved `guards` are both still
    // usable as ordinary resource names.
    for word in ["rank", "guards"] {
        codes(&entry(&format!(
            "let {word} = protected 4; let m = mutex guards {word}; \
             with lock m as g {{ open g; write {word} with g; close g; }}"
        )))
        .unwrap_or_else(|codes| panic!("`{word}` should still name a resource: {codes:?}"));
    }
}

/// An unranked module is unchanged: the certificate records an empty rank map,
/// and a reader must not read that as "ranks were satisfied".
#[test]
fn an_unranked_module_records_no_ranks() {
    let module = codes(&entry(OPEN_WRITE_CLOSE)).unwrap();
    let certificate = check_mutex_invariant_program(&module.program).unwrap();
    assert!(certificate.declared_lock_ranks.is_empty());
    assert!(module.program.lock_ranks.is_empty());
}

/// Admitting a nested section does not widen guard authority: the inner body's
/// frame carries only the inner section, so naming the enclosing guard from
/// inside the nested one is refused. A nested acquisition buys the right to
/// hold two guards, not the right to conflate them.
#[test]
fn a_nested_section_may_not_name_the_enclosing_guard() {
    assert_eq!(
        codes(&entry(&format!(
            "{RANKED} with lock a as ga {{ with lock b as gb {{ open ga; }} }}"
        )))
        .unwrap_err(),
        vec!["Q600"]
    );
}

/// Two interactions measured and pinned rather than assumed. Both are admitted,
/// and both are sound for the same reason: the enclosing guard is still held
/// throughout, and everything derived from it stays keyed to it. The inner
/// section reaches a disjoint footprint that its own mutex protects.
#[test]
fn an_enclosing_open_invariant_and_a_live_borrow_survive_a_nested_section() {
    // The outer invariant is open across the nesting. Nothing in the inner
    // section can reach the outer footprint, and the outer `close` is still
    // required before its `unlock`.
    codes(&entry(&format!(
        "{RANKED} with lock a as ga {{ open ga; with lock b as gb {{ }} \
         write x with ga; close ga; }}"
    )))
    .unwrap();
    // An outer guard-derived borrow is still live across the nesting, and
    // still has to end before the outer section does.
    codes(&entry(&format!(
        "{RANKED} with lock a as ga {{ let bo = borrow x with ga; \
         with lock b as gb {{ }} drop bo; }}"
    )))
    .unwrap();
    // Leaving that borrow live at the outer section's end is still `Q615`.
    assert_eq!(
        codes(&entry(&format!(
            "{RANKED} with lock a as ga {{ let bo = borrow x with ga; \
             with lock b as gb {{ }} }}"
        )))
        .unwrap_err(),
        vec!["Q615", "Q619", "Q600", "Q600", "Q600"],
        "the same fail-closed cascade an unnested escaping borrow produces"
    );
}

/// A nested acquisition is a statement like any other, so it invalidates the
/// decidable read-only region behind `close g unchanged;`. The checked
/// `NoWriteOccurred` witness is never silently downgraded to the declared one.
#[test]
fn a_nested_section_invalidates_the_checked_no_write_witness() {
    assert_eq!(
        codes(&entry(&format!(
            "{RANKED} with lock a as ga {{ open ga; with lock b as gb {{ }} \
             close ga unchanged; }}"
        )))
        .unwrap_err(),
        vec!["Q600"]
    );
}

/// Nesting composes with card T03-GUARD-JOIN in the conservative direction:
/// its condition G3 requires the joining guard to be the *only* one the parent
/// holds, so a guard-local join written inside a nested section is refused
/// while the same join in an unnested section is admitted. The nesting
/// admission does not quietly widen a rule that was argued for one held guard.
#[test]
fn a_guard_local_join_inside_a_nested_section_is_refused() {
    // Admitted: one held guard, the guard-local join's own section.
    codes(&entry(&format!(
        "{RANKED} with lock b as gb {{ thread scope c under gb {{ }} }}"
    )))
    .unwrap();
    // Refused: the same join with the outer section also held.
    assert_eq!(
        codes(&entry(&format!(
            "{RANKED} with lock a as ga {{ with lock b as gb {{ \
             thread scope c under gb {{ }} }} }}"
        )))
        .unwrap_err(),
        vec!["Q600"],
        "G3 wants the section to be the only one held; two guards keep the \
         existing JoinWhileGuardHeld refusal"
    );
}

// ---------------------------------------------------------------------------
// Card CT34-GROWTH: `grow GUARD guards RESOURCE;`
//
// Every refusal below has an accepted control differing by one statement, so
// the refusal is attributable to the rule under test rather than to growth
// being written down at all.
// ---------------------------------------------------------------------------

/// The declarations CT34's shape starts from: one protected resource with a
/// generation over it, and a second protected resource with no protector.
const GROWABLE: &str = "let x = protected 4; let y = protected 4; \
     let m = mutex guards x;";

#[test]
fn a_guarded_growth_checks_and_records_the_generation_it_widened() {
    let module = codes(&entry(&format!(
        "{GROWABLE} with lock m as g {{ open g; grow g guards y; close g; }} \
         with lock m as g2 {{ read y with g2; }}"
    )))
    .unwrap();
    let certificate = check_mutex_invariant_program(&module.program).unwrap();
    let [growth] = certificate.admitted_protection_growths.as_slice() else {
        panic!("exactly one growth was admitted");
    };
    assert_eq!(growth.mutex, 1);
    assert_eq!(growth.task, 0);
    assert_eq!(growth.footprint, AccessFootprint::new(2, 0, 4));
    assert_ne!(
        growth.from_invariant, growth.to_invariant,
        "a wider region is a different proposition, so the predicate identity moves"
    );
    assert_eq!(
        certificate.protected_shape[&1],
        vec![AccessFootprint::new(1, 0, 4), AccessFootprint::new(2, 0, 4)],
        "protected_shape records the effective shape the checker authorised against"
    );
}

/// The point of growing: memory that was reachable without a guard before the
/// growth is reachable only through the generation afterwards, and the growth
/// is permanent — the original footprint is still protected too.
#[test]
fn growth_moves_the_resource_behind_the_guard_permanently() {
    // Before the growth, `y` has no protector, so an unguarded read is fine.
    codes(&entry(&format!("{GROWABLE} read y;"))).unwrap();
    // After it, the same statement is `Q611 UnguardedProtectedAccess`.
    assert_eq!(
        codes(&entry(&format!(
            "{GROWABLE} with lock m as g {{ open g; grow g guards y; close g; }} read y;"
        )))
        .unwrap_err(),
        vec!["Q611"]
    );
    // And the originally declared footprint is still protected: both resources
    // are now reachable under one guard, in a later section.
    codes(&entry(&format!(
        "{GROWABLE} with lock m as g {{ open g; grow g guards y; close g; }} \
         with lock m as g2 {{ read x with g2; read y with g2; }}"
    )))
    .unwrap();
}

/// W2. Growth belongs in the window where the invariant need not hold. Under a
/// closed invariant it would silently assert the declared predicate of memory
/// nothing established it for.
#[test]
fn growing_outside_an_open_invariant_is_refused() {
    assert_eq!(
        codes(&entry(&format!(
            "{GROWABLE} with lock m as g {{ grow g guards y; }}"
        )))
        .unwrap_err(),
        vec!["Q626"],
        "a growth under a closed invariant is refused, and refusing it leaves \
         the section otherwise well formed"
    );
    // One statement apart: open first, and it checks.
    codes(&entry(&format!(
        "{GROWABLE} with lock m as g {{ open g; grow g guards y; close g; }}"
    )))
    .unwrap();
}

/// W3, from the source side. A section that grew may not close with the checked
/// `NoWriteOccurred` witness: growth changes what the region *is*, so "the
/// invariant is unchanged" is not available to it.
#[test]
fn a_section_that_grew_cannot_use_the_checked_witness() {
    // The control: an open region with only reads decides `unchanged`.
    codes(&entry(&format!(
        "{GROWABLE} with lock m as g {{ open g; read x with g; close g unchanged; }}"
    )))
    .unwrap();
    assert_eq!(
        codes(&entry(&format!(
            "{GROWABLE} with lock m as g {{ open g; grow g guards y; close g unchanged; }}"
        )))
        .unwrap_err(),
        vec!["Q600"],
        "the checked witness is refused at the lowering, never downgraded to \
         the declared one"
    );
}

/// W3, from the model side: the core refuses a growth whose supplied identity
/// this generation has already carried. Reached by handing the core a program
/// whose growth re-uses the creation identity — a shape the lowering never
/// mints, which is exactly why the core checks it rather than trusting the
/// lowering.
#[test]
fn a_growth_that_does_not_move_the_predicate_identity_is_refused() {
    let module = lower(&entry(&format!(
        "{GROWABLE} with lock m as g {{ open g; grow g guards y; close g; }}"
    )))
    .unwrap();
    let mut program = module.program.clone();
    let created = program
        .events
        .iter()
        .find_map(|event| match event {
            MutexInvariantEvent::CreateMutex { invariant, .. } => Some(*invariant),
            _ => None,
        })
        .unwrap();
    for event in &mut program.events {
        if let MutexInvariantEvent::GrowProtection { invariant, .. } = event {
            *invariant = created;
        }
    }
    let reported = check_mutex_invariant_program(&program)
        .unwrap_err()
        .iter()
        .map(MutexInvariantDiagnostic::code)
        .collect::<Vec<_>>();
    assert!(
        reported.contains(&"Q627"),
        "re-using the pre-growth declaration is refused: {reported:?}"
    );
}

/// W4. The new interval may not be one a live generation already protects —
/// another generation's, or this generation's own, declared or already grown.
///
/// Each refusal carries the same measured fallout, pinned rather than filtered
/// because every entry says something: the refused growth did not install the
/// new declared predicate identity the lowering minted, so the section's
/// `close` names one the generation never took (`Q613 InvalidInvariantWitness`)
/// and therefore unlocks with its invariant still open
/// (`Q613 InvariantOpenAtUnlock`), which in turn leaves the generation and its
/// guard outstanding at scope exit. A change that started mutating a
/// generation on a refused growth would break this vector.
#[test]
fn growing_over_protected_memory_is_refused() {
    const FALLOUT: [&str; 5] = ["Q619", "Q613", "Q600", "Q600", "Q600"];
    let cascade = |own: &[&str]| {
        own.iter()
            .chain(FALLOUT.iter())
            .map(|code| (*code).to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        codes(&entry(&format!(
            "{GROWABLE} let n = mutex guards y; \
             with lock m as g {{ open g; grow g guards y; close g; }}"
        )))
        .unwrap_err(),
        cascade(&["Q613", "Q617"]),
        "another live generation already protects `y`"
    );
    assert_eq!(
        codes(&entry(&format!(
            "{GROWABLE} with lock m as g {{ open g; grow g guards y; grow g guards y; close g; }}"
        )))
        .unwrap_err(),
        // One extra `Q600`: the first growth succeeded, so `y`'s own
        // destruction at scope exit is nonquiescent too.
        [cascade(&["Q613", "Q617"]), vec!["Q600".to_owned()]].concat(),
        "the second growth finds the interval parked by this same generation"
    );
    // And growing over the generation's own declared footprint is refused too.
    assert_eq!(
        codes(&entry(&format!(
            "{GROWABLE} with lock m as g {{ open g; grow g guards x; close g; }}"
        )))
        .unwrap_err(),
        cascade(&["Q613", "Q617"])
    );
}

/// W1. A growth names this section's own guard. A child task written inside its
/// parent's section can name that guard — that is how the frame works — and the
/// core refuses the act, because the child does not hold it.
#[test]
fn a_child_task_may_not_grow_its_parents_section() {
    let refused = codes(&entry(&format!(
        "{GROWABLE} with lock m as g {{ open g; thread scope c {{ grow g guards y; }} close g; }}"
    )))
    .unwrap_err();
    assert!(
        refused.contains(&"Q612".to_owned()),
        "the child does not hold the section: {refused:?}"
    );
    // The parent's own growth in the same shape checks.
    codes(&entry(&format!(
        "{GROWABLE} with lock m as g {{ open g; grow g guards y; close g; }}"
    )))
    .unwrap();
}

/// The grammar refuses the shapes it cannot represent, one spelling each — and
/// the two spellings that name something undeclared are refused earlier still,
/// by name resolution. Both halves are asserted so a later change cannot move a
/// refusal from one to the other unnoticed.
#[test]
fn malformed_growth_spellings_are_refused() {
    for body in [
        // outside any critical section. A guard name never resolves out there —
        // guard binders are body-local — so this spelling uses a name that does
        // resolve, and the grammar refusal is the one measured.
        format!("{GROWABLE} grow x guards y;"),
        // missing the `guards` clause word
        format!("{GROWABLE} with lock m as g {{ open g; grow g y; close g; }}"),
        // no resource
        format!("{GROWABLE} with lock m as g {{ open g; grow g guards; close g; }}"),
    ] {
        assert_eq!(
            codes(&entry(&body))
                .unwrap_err()
                .first()
                .map(String::as_str),
            Some("Q600"),
            "{body}"
        );
    }
    for body in [
        // naming a guard this section did not bind
        format!("{GROWABLE} with lock m as g {{ open g; grow other guards y; close g; }}"),
        // an undeclared resource
        format!("{GROWABLE} with lock m as g {{ open g; grow g guards z; close g; }}"),
    ] {
        assert_eq!(
            codes(&entry(&body)).unwrap_err(),
            vec!["Q013"],
            "an undeclared name is a name-resolution refusal, reached before any \
             growth question: {body}"
        );
    }
}

/// The growth form needs the concurrency import like every other form here.
#[test]
fn a_growth_without_the_concurrency_import_is_refused() {
    let source = format!(
        "fn main() -> unit {{ thread scope s {{ {GROWABLE} \
         with lock m as g {{ open g; grow g guards y; close g; }} }} }}"
    );
    assert_eq!(lower(&source).unwrap_err(), vec!["Q015"]);
}

/// The reservation costs no existing spelling: `grow` is still usable as a
/// resource name, exactly as the already-reserved `guards` and `rank` are.
#[test]
fn reserving_grow_costs_no_existing_spelling() {
    codes(&entry(
        "let grow = protected 4; let m = mutex guards grow; \
         with lock m as g { open g; write grow with g; close g; }",
    ))
    .unwrap();
}

/// `ExtendProtection` — the unconditional refusal growth sits beside — has no
/// source form at all and is untouched: a module that never writes `grow`
/// lowers exactly as it did, declared predicate identities included.
#[test]
fn the_fixed_shape_refusal_is_untouched_by_the_growth_form() {
    let module = lower(&entry(OPEN_WRITE_CLOSE)).unwrap();
    assert!(
        module
            .program
            .events
            .iter()
            .all(|event| !matches!(event, MutexInvariantEvent::GrowProtection { .. })),
        "no growth event without the source form"
    );
    assert!(
        module.program.events.iter().any(|event| matches!(
            event,
            MutexInvariantEvent::CreateMutex {
                invariant: 1,
                mutex: 1,
                ..
            }
        )),
        "an ungrown module keeps the historical `invariant == mutex` numbering"
    );
    let certificate = check_mutex_invariant_program(&module.program).unwrap();
    assert!(certificate.admitted_protection_growths.is_empty());
}

/// Growth composes with the opt-in rank discipline without touching it: a
/// ranked module may grow, and growing does not make a descending acquisition
/// admissible.
#[test]
fn growth_and_the_rank_discipline_do_not_interact() {
    codes(&entry(
        "let x = protected 4; let y = protected 4; let z = protected 4; \
         let a = mutex rank 1 guards x; let b = mutex rank 2 guards y; \
         with lock a as ga { open ga; grow ga guards z; close ga; } \
         with lock a as g1 { with lock b as g2 { } }",
    ))
    .unwrap();
    assert_eq!(
        codes(&entry(
            "let x = protected 4; let y = protected 4; let z = protected 4; \
             let a = mutex rank 1 guards x; let b = mutex rank 2 guards y; \
             with lock a as ga { open ga; grow ga guards z; close ga; } \
             with lock b as g1 { with lock a as g2 { } }"
        ))
        .unwrap_err(),
        vec!["Q624", "Q612"],
        "a grown generation keeps its declared rank; growth orders nothing"
    );
}
