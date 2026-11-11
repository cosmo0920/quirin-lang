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

//! Card `TRACE-ALPHABET-LEASE-BOUNDARY`: the arm of
//! [`qcore_core::task_trace_relation::reference_trace`] that refuses card
//! `WRITE-LEND-TASKS`'s two range-write-lease events, exercised.
//!
//! The residual `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` recorded that the
//! arm was written at wave-31 integration to resolve a semantic conflict
//! between `WRITE-LEND-TASKS` (which refused all three new task events) and
//! `CT03-PROOF-ALPHABET` (which admitted the Buffer write and deleted the
//! refusal's error variant), and that **no test in the workspace drove a
//! lease-carrying reference run into it** — so the green suites were evidence
//! about the paths they took, not about this arm.
//!
//! This file drives it, from a **real compiled program** rather than a
//! hand-built observation: `DISJOINT_WRITE_LEASES` is card `WRITE-LEND-TASKS`'s
//! own admitted flagship, byte for byte, and its reference run's third event is
//! a `BufferLeased`. The arm is therefore **reachable today** — the residual's
//! discharge (c), "show the arm unreachable", is measured **false** — and the
//! refusal is pinned here rather than assumed.
//!
//! What each test pins, and why the set is what it is:
//!
//! * that the refusal *happens* (an admitted program of today produces the
//!   refused shape);
//! * that it is a **refusal and not a silent drop** — `reference_trace` returns
//!   `Err`, and on the lease-free control the encoding is length-preserving, so
//!   "drop the event and encode the rest" is excluded in both directions;
//!   the arm is `reference_trace`'s only refusal on this program, `derive`
//!   never reaching its own relation checks;
//! * that the reported index is the **offending event's own** position, not the
//!   statement's, the task's, or the first event's;
//! * that the refusal is attributable to the *lease*, not to the program shape:
//!   the read-loan twin of the same program derives cleanly;
//! * that `BufferLeaseReturned` — the arm's second variant, which the program
//!   route can never reach because the lease that precedes it refuses first —
//!   refuses on its own;
//! * that the arm is the **only** route by which a lease could reach an
//!   `AdmittedCausalOrder`: the text route (`parse_observed_trace`) refuses the
//!   rendered lease line, and the `declare` route cannot express one at all,
//!   `ObservedEvent` having no lease constructor (a type-level fact, recorded
//!   in the card report rather than assertable here).
//!
//! The Lean twin of the boundary is `QCore.TaskTraceLeaseBoundary`, which is to
//! the *observation* alphabet what CT03's `QCore.TaskTraceWrites` is to the
//! observable one: `succeeds_iff_no_event_is_refusable` proves the crossing
//! exact in both directions, `encoding_is_positionwise` with
//! `encoding_preserves_length` proves "never silently dropped", and
//! `refusal_points_at_a_refused_event` with `refusal_is_the_first` proves the
//! index pinned below is the offending event's own rather than a coincidence.

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::scoped_tasks::TaskOutcome;
use qcore_core::task_execution::{
    TaskEvent, TaskExecutionObservation, TaskExecutionScript, TaskObservation, execute_task_module,
    render_task_observation,
};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ReferenceTraceError, TraceDeriveError, parse_observed_trace,
    reference_trace,
};

/// Card `WRITE-LEND-TASKS`'s admitted flagship, verbatim: one allocation, two
/// children with disjoint write leases over the payload's two-byte halves,
/// both live at once, the owner's authority back after both joins.
const DISJOINT_WRITE_LEASES: &str = "#import std.concurrency; concurrency checked;
fn writer(half: lend<Buffer>) -> i32 {
  return 1;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s writer(lend buffer 0..2) {
      Result::Ok { value: first } => {
        match spawn s writer(lend buffer 2..4) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
            let after = qcore_buffer_read(buffer);
            drop buffer;
            return after;
          }
          Result::Err { error: refused } => {
            let a = join move first?;
            drop buffer;
            return a;
          }
        }
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

/// The same program with its two write leases replaced by read shares: the
/// control that makes the refusal above attributable to the lease rather than
/// to the program's spawn/join shape.
const DISJOINT_READ_SHARES: &str = "#import std.concurrency; concurrency checked;
fn reader(half: borrow<Buffer>) -> i32 {
  let seen = qcore_buffer_read(half);
  return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(7)?;
  thread scope s {
    match spawn s reader(read buffer share 512) {
      Result::Ok { value: first } => {
        match spawn s reader(read buffer share 512) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
            let after = qcore_buffer_read(buffer);
            drop buffer;
            return after;
          }
          Result::Err { error: refused } => {
            let a = join move first?;
            drop buffer;
            return a;
          }
        }
      }
      Result::Err { error: failure } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

fn compiled(source: &str) -> CompiledModule {
    compile_files(
        &[SourceFile::new("trace-alphabet-lease.qcore", source)],
        &CompileOptions::default(),
    )
    .unwrap()
}

/// The reference run of a compiled module under the both-spawns-succeed,
/// allocation-succeeds script — the same script `WRITE-LEND-TASKS` pins.
fn reference_run(source: &str) -> TaskExecutionObservation {
    execute_task_module(
        compiled(source).checked.as_scoped_tasks().unwrap(),
        &TaskExecutionScript {
            spawn_outcomes: vec![true, true],
            allocation_outcomes: vec![true],
        },
    )
    .unwrap()
}

/// **Why the refusal is the fail-closed answer, measured.** Had the arm
/// dropped the two lease events and encoded the rest — the plausible
/// alternative to refusing, and what a `filter` instead of a `return Err`
/// would do — the result is not merely a shorter trace: it is a trace the
/// relation **admits**. `declare` accepts the ten-event remainder as an
/// admitted causal order for a run in which two workers held outstanding write
/// authority over the payload that the order does not mention.
///
/// So the choice at this arm is not "refuse versus report less"; it is "refuse
/// versus admit an order that omits a transfer of write authority". Dropping
/// would have been a move in the **admitting** direction, which invariant 1
/// forbids. This test pins that, so a later reader cannot re-derive the
/// dropping variant as the harmless simplification it looks like.
///
/// The Lean twin is
/// `QCore.TaskTraceLeaseBoundary.the_dropped_encoding_would_have_been_admitted`.
#[test]
fn dropping_the_lease_events_would_have_produced_an_admitted_order() {
    let observation = reference_run(DISJOINT_WRITE_LEASES);
    let mut dropped = observation.clone();
    dropped.events.retain(|event| {
        !matches!(
            event,
            TaskEvent::BufferLeased { .. } | TaskEvent::BufferLeaseReturned { .. }
        )
    });
    assert_eq!(
        observation.events.len() - dropped.events.len(),
        4,
        "the flagship carries two leases and two lease returns"
    );

    let encoded = reference_trace(&dropped).expect("the lease-free remainder encodes");
    assert_eq!(encoded.len(), 10);
    let admitted = AdmittedCausalOrder::declare(encoded)
        .expect("and the relation ADMITS it — which is why dropping is not an option");
    assert_eq!(admitted.events().len(), 10);
}

/// The arm is **reachable today**: an admitted program of the current language
/// produces a reference run whose third event is a range write lease, and
/// driving that run into `reference_trace` refuses at exactly that event.
///
/// This is the measurement that decides against the residual's discharge (c).
#[test]
fn a_real_lease_program_reaches_the_refusal_at_the_offending_events_own_index() {
    let observation = reference_run(DISJOINT_WRITE_LEASES);

    // The offending event, located by its own shape rather than by the index
    // the refusal will report — so the index below is checked against the
    // observation, not against itself.
    let offending = observation
        .events
        .iter()
        .position(|event| matches!(event, TaskEvent::BufferLeased { .. }))
        .expect("card WRITE-LEND-TASKS' flagship reference run carries a lease event");
    assert_eq!(
        offending, 2,
        "the flagship's lease opens after the allocation and the first spawn"
    );
    assert!(
        matches!(
            observation.events[offending],
            TaskEvent::BufferLeased {
                borrower: 1,
                low: 0,
                high: 2,
                ..
            }
        ),
        "the refused event is the first worker's 0..2 lease, got {:?}",
        observation.events[offending]
    );

    assert_eq!(
        reference_trace(&observation),
        Err(ReferenceTraceError::UnsupportedObservedEvent { index: offending }),
        "a lease-carrying reference run is refused, at the lease's own index"
    );
    assert_eq!(
        AdmittedCausalOrder::derive(&observation),
        Err(TraceDeriveError::Unsupported(
            ReferenceTraceError::UnsupportedObservedEvent { index: offending }
        )),
        "`derive` carries the encoder's refusal out unchanged — it never reaches \
         its own relation checks, so no lease event can become an admitted order"
    );
}

/// Refused, **not silently dropped**. The lease program refuses outright; its
/// lease-free control encodes with the event count preserved, so neither
/// direction of the boundary elides an event: the encoder is total on the
/// alphabet it admits and refuses everything else.
#[test]
fn the_lease_boundary_refuses_rather_than_dropping_the_event() {
    let leased = reference_run(DISJOINT_WRITE_LEASES);
    assert!(
        reference_trace(&leased).is_err(),
        "a lease-carrying run must not encode at all"
    );

    let shared = reference_run(DISJOINT_READ_SHARES);
    assert!(
        !shared.events.iter().any(|event| matches!(
            event,
            TaskEvent::BufferLeased { .. } | TaskEvent::BufferLeaseReturned { .. }
        )),
        "the control program must carry no lease event"
    );
    let encoded = reference_trace(&shared).expect("a lease-free reference run encodes");
    assert_eq!(
        encoded.len(),
        shared.events.len(),
        "the encoding is length-preserving on the admitted alphabet: an event is \
         encoded or the whole observation is refused, never dropped"
    );

    // And the control's encoding is a *relation* order, not merely a list:
    // `derive` accepts it, which is what the lease program is being denied.
    AdmittedCausalOrder::derive(&shared)
        .expect("the read-share twin of the refused program derives cleanly");
}

/// The arm's **second variant**, which the program route cannot reach: a
/// `BufferLeaseReturned` is always preceded in a real run by the
/// `BufferLeased` that refuses first. Pinned on a minimal observation so the
/// variant is exercised rather than merely written.
#[test]
fn a_lease_return_refuses_on_its_own_at_its_own_index() {
    let observation = TaskExecutionObservation {
        root_outcome: TaskOutcome::I32(0),
        tasks: vec![TaskObservation {
            task: 1,
            function: 0,
            parent: Some(0),
            arguments: Vec::new(),
            outcome: TaskOutcome::I32(1),
            joined: true,
        }],
        events: vec![
            TaskEvent::BufferAllocated {
                task: 0,
                buffer: 1,
                initial: 7,
            },
            TaskEvent::Spawn { task: 1, worker: 0 },
            TaskEvent::Result {
                task: 1,
                outcome: TaskOutcome::I32(1),
            },
            TaskEvent::Join { task: 1 },
            TaskEvent::BufferLeaseReturned {
                borrower: 1,
                buffer: 1,
            },
            TaskEvent::BufferDropped { task: 0, buffer: 1 },
            TaskEvent::Result {
                task: 0,
                outcome: TaskOutcome::I32(0),
            },
        ],
    };
    assert_eq!(
        reference_trace(&observation),
        Err(ReferenceTraceError::UnsupportedObservedEvent { index: 4 }),
        "the lease-return variant refuses at its own index, not at the first event"
    );
}

/// The refusal is the **only** route a lease event could take toward an
/// admitted order through text: the rendered lease lines carry tags
/// (`QCORE_BUFFER_LEASE`, `QCORE_BUFFER_LEASE_RETURN`) the observable
/// alphabet's parser does not know, so `parse_observed_trace` refuses them at
/// their own line. The third route, `AdmittedCausalOrder::declare`, cannot
/// express a lease at all — `ObservedEvent` has no lease constructor — which is
/// a type-level fact, not an assertion this test can make.
#[test]
fn the_rendered_lease_lines_are_outside_the_parsers_alphabet_too() {
    let observation = reference_run(DISJOINT_WRITE_LEASES);
    let rendered = render_task_observation(&observation);
    assert!(
        rendered.contains("QCORE_BUFFER_LEASE 1 1 0 2\n"),
        "the renderer names the lease, so the parser is genuinely offered one:\n{rendered}"
    );
    assert!(
        rendered.contains("QCORE_BUFFER_LEASE_RETURN 1 1\n"),
        "and the return line too:\n{rendered}"
    );

    let refusal = parse_observed_trace(&rendered).expect_err("a lease line does not parse");
    assert_eq!(
        refusal.line, 3,
        "refused at the rendered lease's own line (1-based), the two lines before it \
         being the allocation and the first spawn"
    );
    assert_eq!(refusal.text, "QCORE_BUFFER_LEASE 1 1 0 2");

    // The return line alone is refused too, so the parser's coverage of the
    // boundary matches the encoder's: both variants, not just the first.
    let return_only = parse_observed_trace("QCORE_BUFFER_LEASE_RETURN 1 1\n")
        .expect_err("a lease-return line does not parse either");
    assert_eq!(return_only.line, 1);
    assert_eq!(return_only.text, "QCORE_BUFFER_LEASE_RETURN 1 1");
}
