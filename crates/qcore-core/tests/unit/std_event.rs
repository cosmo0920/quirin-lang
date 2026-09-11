//! `source.event.v1` — the checked model behind `std.event` (card `STD-EVENT`).
//!
//! These tests are about the two things this module decides: the **projection**
//! onto the existing reactor model, and the **determinism boundary**. What the
//! reactor concludes from the projection is `event_reactor`'s own test surface
//! and is not reproduced here; what is pinned here is that the projection is
//! the one the surface table claims, and that a program whose observation would
//! not be well-defined is refused by name.

use super::*;
use crate::event_reactor::{EventFlags, EventKey};

fn entry(statements: Vec<EventStatement>) -> EventSourceProgram {
    EventSourceProgram {
        entry: "main".to_owned(),
        declared_effects: EffectRow::new([Effect::Io]),
        statements,
    }
}

/// The canonical admitted program: one context, one endpoint pair, one read
/// registration, one induced readiness, one ready turn, one idle turn, and a
/// complete teardown.
fn canonical() -> EventSourceProgram {
    entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Watch {
            registration: "e".to_owned(),
            endpoint: "a".to_owned(),
            interest: Interest::Read,
        },
        EventStatement::Wake {
            endpoint: "a".to_owned(),
        },
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
        },
        EventStatement::DispatchIdle,
        EventStatement::Unwatch {
            registration: "e".to_owned(),
        },
        EventStatement::Close {
            endpoint: "a".to_owned(),
        },
        EventStatement::Close {
            endpoint: "b".to_owned(),
        },
        EventStatement::ContextClose,
        EventStatement::Return { status: 0 },
    ])
}

#[test]
fn the_canonical_program_is_admitted_and_rests_on_the_named_premise() {
    let certificate = check_event_source_program(&canonical()).expect("the canonical program");
    assert!(certificate.executable);
    assert_eq!(certificate.status, 0);
    assert_eq!(certificate.endpoints.len(), 2);
    assert_eq!(certificate.registrations.len(), 1);
    assert_eq!(certificate.turns.len(), 2);
    // The premise is named in the certificate rather than left implicit: a
    // reader sees which platform claim the verdict rests on.
    assert_eq!(certificate.premises, vec![LEVEL_TRIGGERED_READINESS]);
    recheck_event_certificate(&canonical(), &certificate).expect("the recheck");
}

#[test]
fn the_projection_is_the_reactor_vocabulary_the_surface_table_claims() {
    let projection = project(&canonical()).expect("the canonical program projects");
    let operations = &projection.program.operations;
    // The custody edge: a registration is an `Assign` naming the descriptor,
    // followed by an `Add`. This is the whole content of the claim that
    // `watch E D read;` *is* the existing model's registration.
    let watch = operations
        .iter()
        .find_map(|operation| match operation {
            EventOperation::Assign {
                event,
                descriptor,
                flags,
                priority,
                ..
            } => Some((*event, *descriptor, *flags, *priority)),
            _ => None,
        })
        .expect("the registration projects to an Assign");
    assert_eq!(watch.0, EventKey::new(0, 1));
    assert_eq!(watch.1, Some(0));
    assert_eq!(watch.2, EventFlags::new(EventFlags::READ));
    // Every registration has priority 0 — see the module header: an ordering
    // premise is exactly what v1 does not declare.
    assert_eq!(watch.3, 0);

    // A `wake` projects to nothing at all. The reactor models no byte flow and
    // this module does not invent one beside it.
    let without_wake = entry(
        canonical()
            .statements
            .into_iter()
            .filter(|statement| !matches!(statement, EventStatement::Wake { .. }))
            .collect(),
    );
    let awake = project(&canonical()).expect("with a wake");
    let asleep = project(&without_wake);
    // Removing the wake changes the *determinism* answer, not the projection's
    // custody content: the dispatch turn is then refused, so there is no
    // projection to compare. That refusal is the point.
    assert!(asleep.is_err());
    assert!(awake.program.operations.len() > 5);
}

#[test]
fn the_context_pair_brackets_every_event_statement() {
    let projection = project(&canonical()).expect("projects");
    assert!(matches!(
        projection.program.operations.first(),
        Some(EventOperation::CreateBase { .. })
    ));
    assert!(matches!(
        projection.program.operations.last(),
        Some(EventOperation::FreeBase { .. })
    ));
}

#[test]
fn every_operation_is_attributed_to_the_statement_that_produced_it() {
    let projection = project(&canonical()).expect("projects");
    assert_eq!(
        projection.origins.len(),
        projection.program.operations.len(),
        "an operation with no origin could not be attributed to a source span"
    );
    assert!(projection.origins.windows(2).all(|pair| pair[0] <= pair[1]));
}

// --- The determinism boundary. ---

#[test]
fn two_ready_registrations_in_one_turn_are_refused_by_name() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Endpoints {
            first: "c".to_owned(),
            second: "d".to_owned(),
        },
        EventStatement::Watch {
            registration: "e".to_owned(),
            endpoint: "a".to_owned(),
            interest: Interest::Read,
        },
        EventStatement::Watch {
            registration: "f".to_owned(),
            endpoint: "c".to_owned(),
            interest: Interest::Read,
        },
        EventStatement::Wake {
            endpoint: "a".to_owned(),
        },
        EventStatement::Wake {
            endpoint: "c".to_owned(),
        },
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
        },
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("two ready registrations");
    let determinism = errors
        .iter()
        .find(|error| matches!(error, EventSourceError::Determinism { .. }))
        .expect("the determinism refusal");
    assert_eq!(determinism.code(), "Q061");
    let message = determinism.to_string();
    // The ground is in the message, not only in a design document: a reader of
    // the diagnostic learns why, and what it waits on.
    assert!(message.contains("`poll(2)` reports a set"), "{message}");
    assert!(message.contains("OBS-REORDERING"), "{message}");
}

#[test]
fn a_turn_declaring_readiness_the_program_did_not_induce_is_refused() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Watch {
            registration: "e".to_owned(),
            endpoint: "a".to_owned(),
            interest: Interest::Read,
        },
        // No `wake`. Waiting here would block on readiness from outside this
        // program, which has no well-defined observation.
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
        },
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("an uninduced wait");
    assert!(errors.iter().any(|error| error.code() == "Q061"));
}

#[test]
fn an_idle_turn_with_an_outstanding_wake_is_refused() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Watch {
            registration: "e".to_owned(),
            endpoint: "a".to_owned(),
            interest: Interest::Read,
        },
        EventStatement::Wake {
            endpoint: "a".to_owned(),
        },
        EventStatement::DispatchIdle,
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("an idle turn that is not idle");
    assert!(errors.iter().any(|error| error.code() == "Q061"));
}

#[test]
fn the_callback_drains_the_byte_so_the_next_turn_is_idle() {
    // This is the ledger's whole content: a ready turn consumes exactly one
    // induced readiness, so the *following* idle turn of the canonical program
    // is admitted. Without the drain it would be refused as above.
    let certificate = check_event_source_program(&canonical()).expect("the canonical program");
    assert_eq!(certificate.turns[0].ready, Some(0));
    assert_eq!(certificate.turns[1].ready, None);
}

#[test]
fn a_write_interest_is_accepted_and_checked_but_not_executable() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Watch {
            registration: "e".to_owned(),
            endpoint: "a".to_owned(),
            interest: Interest::Write,
        },
        EventStatement::DispatchIdle,
        EventStatement::Unwatch {
            registration: "e".to_owned(),
        },
        EventStatement::Close {
            endpoint: "a".to_owned(),
        },
        EventStatement::Close {
            endpoint: "b".to_owned(),
        },
        EventStatement::ContextClose,
        EventStatement::Return { status: 0 },
    ]);
    let certificate =
        check_event_source_program(&program).expect("a write registration is checked");
    // The checking is complete and identical; only the execution is out of
    // v1's boundary.
    assert!(!certificate.executable);
    assert_eq!(certificate.registrations.len(), 1);
    assert_eq!(certificate.registrations[0].interest, Interest::Write);
    // And it rests on no readiness premise, because it performs no wait this
    // route can say anything about.
    assert!(certificate.premises.is_empty());
}

// --- The reactor model's own refusals reach the source, unweakened. ---

#[test]
fn an_unclosed_endpoint_is_refused_by_the_reactor_model() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Close {
            endpoint: "a".to_owned(),
        },
        // `b` is never closed.
        EventStatement::ContextClose,
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("a leaked endpoint");
    let reactor = errors
        .iter()
        .find(|error| matches!(error, EventSourceError::Reactor { .. }))
        .expect("the reactor's own refusal");
    assert_eq!(reactor.code(), "Q060");
    assert!(
        reactor.to_string().contains("L03DescriptorCustody"),
        "{reactor}"
    );
}

#[test]
fn tearing_down_a_context_with_a_live_registration_is_refused_by_the_reactor_model() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Watch {
            registration: "e".to_owned(),
            endpoint: "a".to_owned(),
            interest: Interest::Read,
        },
        // No `unwatch`: the registration is still live when the context is
        // torn down.
        EventStatement::Close {
            endpoint: "a".to_owned(),
        },
        EventStatement::Close {
            endpoint: "b".to_owned(),
        },
        EventStatement::ContextClose,
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("a live registration at teardown");
    assert!(errors.iter().any(|error| error.code() == "Q060"));
}

#[test]
fn a_registration_on_a_closed_endpoint_is_refused_by_the_reactor_model() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Close {
            endpoint: "a".to_owned(),
        },
        EventStatement::Watch {
            registration: "e".to_owned(),
            endpoint: "a".to_owned(),
            interest: Interest::Read,
        },
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("a closed endpoint");
    // The *source* rule catches this one first — `a` is no longer live — and
    // that is deliberate: the message names the source, and the reactor's own
    // `L03` stands behind it for anything the source rule does not see.
    assert!(errors.iter().any(|error| error.code() == "Q059"));
}

// --- Shape. ---

#[test]
fn a_second_context_is_refused() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::ContextOpen,
        EventStatement::ContextClose,
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("two contexts");
    assert!(errors.iter().any(|error| error.code() == "Q058"));
}

#[test]
fn an_event_statement_before_the_context_is_refused() {
    let program = entry(vec![
        EventStatement::Endpoints {
            first: "a".to_owned(),
            second: "b".to_owned(),
        },
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("no context");
    assert!(errors.iter().any(|error| error.code() == "Q058"));
}

#[test]
fn an_entry_that_never_returns_is_refused() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::ContextClose,
    ]);
    let errors = check_event_source_program(&program).expect_err("no return");
    assert!(errors.iter().any(|error| error.code() == "Q058"));
}

#[test]
fn an_unclosed_context_is_refused_by_the_source_rule() {
    let program = entry(vec![
        EventStatement::ContextOpen,
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("an open context");
    assert!(errors.iter().any(|error| error.code() == "Q058"));
}

// --- Effects. ---

#[test]
fn the_derived_row_is_exactly_io_and_never_alloc_or_mut() {
    let derived = derived_effects(&canonical());
    assert!(derived.contains(&Effect::Io));
    assert!(!derived.contains(&Effect::Alloc));
    assert!(!derived.contains(&Effect::Mut));
}

#[test]
fn an_undeclared_io_effect_is_refused() {
    let mut program = canonical();
    program.declared_effects = EffectRow::default();
    let errors = check_event_source_program(&program).expect_err("an undeclared effect");
    assert!(errors.iter().any(|error| error.code() == "Q062"));
}

#[test]
fn a_program_that_only_returns_derives_nothing() {
    let program = entry(vec![EventStatement::Return { status: 3 }]);
    let mut bare = program.clone();
    bare.declared_effects = EffectRow::default();
    assert_eq!(derived_effects(&bare), EffectRow::default());
    let certificate = check_event_source_program(&bare).expect("a bare return");
    assert_eq!(certificate.status, 3);
    // A program that opens no context performs no wait and therefore rests on
    // no readiness premise.
    assert!(certificate.premises.is_empty());
}

// --- The recheck. ---

#[test]
fn a_certificate_from_another_program_does_not_survive_the_recheck() {
    let certificate = check_event_source_program(&canonical()).expect("the canonical program");
    let other = entry(vec![EventStatement::Return { status: 0 }]);
    let mut declared = other.clone();
    declared.declared_effects = EffectRow::default();
    assert!(recheck_event_certificate(&declared, &certificate).is_err());
}

#[test]
fn only_the_one_accepted_profile_is_admitted() {
    // The accepted-profile policy is exercised the way an adapter exercises it:
    // v1 names exactly one, and it is the single-threaded one.
    assert_eq!(ACCEPTED_PROFILES.len(), 1);
    assert!(!ACCEPTED_PROFILES[0].threading_enabled());
}
