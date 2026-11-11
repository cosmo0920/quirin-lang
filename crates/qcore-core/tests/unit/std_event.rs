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
            byte: DEFAULT_WAKE_BYTE,
        },
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
            binding: None,
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
            byte: DEFAULT_WAKE_BYTE,
        },
        EventStatement::Wake {
            endpoint: "c".to_owned(),
            byte: DEFAULT_WAKE_BYTE,
        },
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
            binding: None,
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
            binding: None,
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
            byte: DEFAULT_WAKE_BYTE,
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

// --- The composed line vocabulary (card `COMPOSE-EVENT-LINES`). ---

use crate::obs_reordering::Emitter;
use crate::std_event_c_emit::observe_event_program;
use crate::std_io::{Level, Rendering, StdIoEvent, StdIoOperand, Stream};

/// The canonical program with two declared payloads, a binding for each and a
/// piped line rendering each. This is `examples/events/wake_payload.qr`.
fn composed() -> EventSourceProgram {
    entry(vec![
        EventStatement::LogLevel {
            level: Level::Trace,
        },
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
            byte: 72,
        },
        EventStatement::Wake {
            endpoint: "a".to_owned(),
            byte: 105,
        },
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
            binding: Some("first".to_owned()),
        },
        EventStatement::Line(StdIoEvent::Log {
            level: Level::Info,
            rendering: Rendering::Dec,
            operand: StdIoOperand::Scalar("first".to_owned()),
        }),
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
            binding: Some("second".to_owned()),
        },
        EventStatement::Line(StdIoEvent::Log {
            level: Level::Info,
            rendering: Rendering::Hex,
            operand: StdIoOperand::Scalar("second".to_owned()),
        }),
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
fn a_drained_byte_is_the_byte_its_own_wake_delivered_in_order() {
    let certificate = check_event_source_program(&composed()).expect("the composed program");
    let bytes = certificate
        .payloads
        .iter()
        .map(|payload| (payload.byte, payload.binding.clone()))
        .collect::<Vec<_>>();
    // First in, first out: the ledger is a queue, not a count. Which is also
    // the whole content of the second named premise.
    assert_eq!(
        bytes,
        vec![
            (72, Some("first".to_owned())),
            (105, Some("second".to_owned()))
        ]
    );
    assert_eq!(certificate.scalars["first"], 72);
    assert_eq!(certificate.scalars["second"], 105);
    recheck_event_certificate(&composed(), &certificate).expect("the recheck");
}

#[test]
fn the_byte_order_premise_is_carried_only_where_a_byte_is_named() {
    let bound = check_event_source_program(&composed()).expect("the composed program");
    assert_eq!(
        bound.premises,
        vec![LEVEL_TRIGGERED_READINESS, STREAM_BYTE_ORDER]
    );
    // The canonical program drains a byte too — every ready turn does — but
    // names none, so it rests on nothing about the value.
    let unnamed = check_event_source_program(&canonical()).expect("the canonical program");
    assert_eq!(unnamed.premises, vec![LEVEL_TRIGGERED_READINESS]);
    assert_eq!(unnamed.payloads.len(), 1);
    assert_eq!(unnamed.payloads[0].byte, DEFAULT_WAKE_BYTE);
    assert!(unnamed.payloads[0].binding.is_none());
}

#[test]
fn the_lines_are_the_standard_io_routes_own_lines() {
    let certificate = check_event_source_program(&composed()).expect("the composed program");
    assert_eq!(certificate.lines.len(), 2);
    assert_eq!(certificate.lines[0].stream, Stream::Err);
    assert_eq!(certificate.lines[0].text, "[INFO] 72");
    assert_eq!(certificate.lines[1].text, "[INFO] 0x00000069");
    assert_eq!(
        certificate.renderings,
        [Rendering::Dec, Rendering::Hex].into_iter().collect()
    );
}

#[test]
fn erasing_every_line_leaves_the_reactor_projection_unchanged() {
    // The strong form of "composing lines decides no custody question", in the
    // shape card `STD-EVENT` used for `wake`: whatever the reactor concludes
    // about this program, it concludes about the same program with every line
    // removed.
    let with_lines = project(&composed()).expect("the composed program projects");
    let without = entry(
        composed()
            .statements
            .into_iter()
            .filter(|statement| !matches!(statement, EventStatement::Line(_)))
            .collect(),
    );
    let without = project(&without).expect("the same program without its lines");
    assert_eq!(with_lines.program, without.program);
}

#[test]
fn every_record_of_an_event_program_is_written_by_the_entrys_own_thread() {
    // The premise under the strict comparison, asserted rather than assumed.
    // v1's one profile is single-threaded and the reactor refuses a
    // cross-thread activation under it, so a callback runs on the dispatching
    // thread and there is no second emitter to order against.
    let certificate = check_event_source_program(&composed()).expect("the composed program");
    let observation =
        observe_event_program(&composed(), &certificate).expect("the composed observation");
    assert!(
        observation
            .records()
            .iter()
            .all(|record| record.emitter == Emitter::Parent)
    );
    assert!(observation.records().len() > 10);
}

#[test]
fn the_declared_relation_is_strict_on_both_streams() {
    // Card `OBS-REORDERING`'s §1.2 live-set walk, over this model's own
    // regions: every record is a barrier, so the relation admits exactly one
    // stream — its own — and comparing a native run against it **is** byte
    // equality (`strict_iff_byte_equality`). No reordering is declared, because
    // this route needs none.
    let certificate = check_event_source_program(&composed()).expect("the composed program");
    let observation =
        observe_event_program(&composed(), &certificate).expect("the composed observation");
    for stream in [Stream::Out, Stream::Err] {
        let relation = observation
            .admitted_reordering(stream)
            .expect("the declaration");
        assert!(relation.is_strict(), "{stream:?}");
        assert_eq!(relation.barriers().len(), relation.records().len());
        assert_eq!(relation.declared_text(), observation.projection(stream));
        // The degenerate case in both directions: the declared stream is
        // admitted, and a transposition of two of its records is not.
        let mut lines = relation
            .records()
            .iter()
            .map(|record| record.text.clone())
            .collect::<Vec<_>>();
        assert!(
            relation
                .compare(&lines.iter().map(String::as_str).collect::<Vec<_>>())
                .is_ok()
        );
        lines.swap(0, 1);
        assert!(
            relation
                .compare(&lines.iter().map(String::as_str).collect::<Vec<_>>())
                .is_err()
        );
    }
}

#[test]
fn a_quiet_program_observes_its_lines_and_not_its_retained_trace() {
    // The quiet-emission boundary as a property of the observation: the trace
    // events are retained either way — they are in the reactor certificate and
    // in the emitted program's typed logger calls — but they are stream bytes
    // only where `log_level` installs the adapter.
    let quiet = entry(
        composed()
            .statements
            .into_iter()
            .filter(|statement| !matches!(statement, EventStatement::LogLevel { .. }))
            .collect(),
    );
    let certificate = check_event_source_program(&quiet).expect("the quiet program");
    let observation = observe_event_program(&quiet, &certificate).expect("the observation");
    assert_eq!(
        observation.labelled(),
        vec![
            "err: [INFO] 72".to_owned(),
            "err: [INFO] 0x00000069".to_owned()
        ]
    );
    assert!(
        observation
            .records()
            .iter()
            .all(|record| !record.system_trace)
    );
}

#[test]
fn a_rendering_operand_this_entry_bound_no_value_for_refuses() {
    let program = entry(vec![
        EventStatement::Line(StdIoEvent::Print {
            rendering: Rendering::Dec,
            operand: StdIoOperand::Scalar("absent".to_owned()),
        }),
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("an unbound operand");
    assert!(errors.iter().any(|error| error.code() == "Q074"));
}

#[test]
fn a_borrowed_render_refuses_because_this_route_publishes_no_identity() {
    let program = entry(vec![
        EventStatement::Line(StdIoEvent::Print {
            rendering: Rendering::Dec,
            operand: StdIoOperand::Borrowed("a".to_owned()),
        }),
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("a borrowed render");
    assert!(errors.iter().any(|error| error.code() == "Q074"));
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("STD-FMT-DESCRIPTOR"))
    );
}

#[test]
fn a_binding_that_collides_with_a_custody_name_refuses_at_the_model() {
    // Unreachable from source — name resolution refuses the duplicate binding
    // at `Q012` first, which `compiler/tests/compose_event_lines.rs` pins — and
    // kept for the reason `EXTERN-F1`'s `resource_stored` gate is kept: this is
    // a public model API, and a composing route could drive it directly.
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
            byte: DEFAULT_WAKE_BYTE,
        },
        EventStatement::DispatchReady {
            registration: "e".to_owned(),
            binding: Some("a".to_owned()),
        },
        EventStatement::Return { status: 0 },
    ]);
    let errors = check_event_source_program(&program).expect_err("a colliding binding");
    assert!(errors.iter().any(|error| error.code() == "Q074"));
    assert!(
        errors
            .iter()
            .any(|error| error.to_string().contains("no rebinding form"))
    );
}

#[test]
fn the_determinism_boundary_is_unmoved_by_a_line() {
    // Two ready registrations admit no turn, with a line in the program or
    // without one. `Q061` is about the ledger's state and nothing else.
    let statements = |lines: bool| {
        let mut statements = vec![
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
                byte: 1,
            },
            EventStatement::Wake {
                endpoint: "c".to_owned(),
                byte: 2,
            },
        ];
        if lines {
            statements.push(EventStatement::Line(StdIoEvent::Print {
                rendering: Rendering::Dec,
                operand: StdIoOperand::Literal(0),
            }));
        }
        statements.push(EventStatement::DispatchReady {
            registration: "e".to_owned(),
            binding: Some("value".to_owned()),
        });
        statements.push(EventStatement::Return { status: 0 });
        entry(statements)
    };
    for lines in [false, true] {
        let errors = check_event_source_program(&statements(lines)).expect_err("two ready");
        assert!(errors.iter().any(|error| error.code() == "Q061"), "{lines}");
    }
}
