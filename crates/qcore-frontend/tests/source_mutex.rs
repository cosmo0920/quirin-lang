//! Source-level acceptance for `source.mutex.guard.v1` (card T03).
//!
//! Every rejection below asserts the exact diagnostic code the core assigns,
//! and most carry an accepted control that differs by one statement. A test
//! that only asserted `is_err()` would not distinguish the intended rule from
//! any other refusal on the way.
use qcore_core::mutex_invariant::{
    AccessFootprint, AccessKind, ConcurrencyAssumption, ConcurrencyProfile, MutexInvariantEvent,
    check_mutex_invariant_program, recheck_mutex_invariant_certificate,
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
    format!("fn main() -> unit {{ thread scope s {{ {body} }} }}")
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
    let source = "fn worker(x: i32) -> i32 { return x; } \
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
        "fn main() -> unit contract { requires x; } { thread scope s { let buf = protected 4; let m = mutex guards buf; } }",
        "fn main() -> i32 { thread scope s { let buf = protected 4; let m = mutex guards buf; } }",
    ] {
        assert!(matches!(
            lower(source).unwrap_err().as_slice(),
            [code] if code == "Q600" || code == "Q021" || code == "Q020"
        ));
    }
}
