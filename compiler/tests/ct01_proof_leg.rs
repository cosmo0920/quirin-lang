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

//! CT01-PROOF-LEG — the named runner behind CT01's first `proof`-stage
//! evidence.
//!
//! The Lean side is `proof/QCore/Ct01OwnedTransfer.lean`: a total decision
//! procedure `dischargesOwnedTransfer` over the F02 observation alphabet,
//! theorems that its acceptance implies CT01's scenario property (the parent
//! performs exactly allocate, spawn, join, read, drop, complete in that
//! order; the moved generation is released exactly once, by the parent,
//! after the join; the child's only completion returns the very generation
//! the parent's only allocation created), the proof that CT01's reference
//! trace `QCore.TaskTraceRelation.roundTrip` is accepted, and the closure
//! `every_admitted_rendering_discharges_the_owned_transfer` over every trace
//! the F02 relation admits against it.
//!
//! This test binds those theorems to **CT01's own artifact**: it compiles
//! `compiler/tests/fixtures/ct/CT01/case.qr` through the real pipeline,
//! executes the checked task module's reference execution, derives the
//! admitted causal order with the production observer, and
//!
//! 1. checks the derived events are exactly the recorded F02 fixture trace
//!    that `roundTrip` transcribes (host-unconditionally — the Linux-gated
//!    `f02_trace_relation.rs` twin additionally checks the native rendering,
//!    which this test deliberately does not claim);
//! 2. recomputes the Lean-proved transcription pin
//!    `QCore.Ct01OwnedTransfer.ct01_trace_digest` from the artifact's own
//!    events with wrapping `u64` arithmetic (the F02 digest fold), so the
//!    Lean term and the artifact cannot drift apart silently;
//! 3. runs the same total decision procedure the Lean definition defines,
//!    transcribed below clause for clause, and requires acceptance;
//! 4. re-runs the procedure on the five named mutants the Lean module proves
//!    refused, and requires refusal — the transcription agrees on the
//!    refusing side too.
//!
//! What this does **not** establish: anything about the emitted C's native
//! execution (covered only by `run_native_owned_transfer`'s byte comparison
//! and F02's relation checks), or a verified extraction between Lean and
//! Rust — the agreement is the checked-digest transcription pattern F02
//! established.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, ObservedOutcome, parse_observed_trace, relate,
};

const CT01_SOURCE: &str = include_str!("fixtures/ct/CT01/case.qr");
const CT01_TRACE: &str =
    include_str!("../../crates/qcore-core/tests/fixtures/f02/round_trip.trace");

/// `QCore.Ct01OwnedTransfer.ct01_trace_digest`, proved by the Lean kernel on
/// the `roundTrip` term this artifact must reproduce.
const CT01_LEAN_TRACE_DIGEST: u64 = 13023433800416048247;

/* --------------------------------------------------- the F02 digest fold */

const TRACE_SEED: u64 = 1469598103934665603;
const TRACE_MUL: u64 = 1000003;

fn natural(value: i32) -> u64 {
    u64::try_from(value).expect("CT01's trace carries no negative payload")
}

/// Mirrors `QCore.TaskTraceRelation.encodeEvent`, exactly as the F02 harness
/// in `crates/qcore-core/tests/task_trace_relation.rs` does.
fn encode_event(event: &ObservedEvent) -> u64 {
    match event {
        ObservedEvent::Allocated {
            task,
            buffer,
            payload,
        } => 1 + 16 * (task + 8 * (buffer + 64 * natural(*payload))),
        ObservedEvent::AllocationFailed { task } => 2 + 16 * task,
        ObservedEvent::Spawned { child, worker } => 3 + 16 * (child + 8 * worker),
        ObservedEvent::CreationFailed { worker } => 4 + 16 * worker,
        ObservedEvent::Read {
            task,
            buffer,
            value,
        } => 5 + 16 * (task + 8 * (buffer + 64 * natural(*value))),
        ObservedEvent::Written {
            task,
            buffer,
            value,
        } => 11 + 16 * (task + 8 * (buffer + 64 * natural(*value))),
        ObservedEvent::Dropped { task, buffer } => 6 + 16 * (task + 8 * buffer),
        ObservedEvent::Loaned { borrower, buffer } => 7 + 16 * (borrower + 8 * buffer),
        ObservedEvent::Returned { borrower, buffer } => 8 + 16 * (borrower + 8 * buffer),
        ObservedEvent::Completed { task, outcome } => {
            let encoded = match outcome {
                ObservedOutcome::Unit => 0,
                ObservedOutcome::I32(value) => 1 + 4 * natural(*value),
                ObservedOutcome::Buffer(buffer) => 2 + 4 * buffer,
                ObservedOutcome::Error(_) => {
                    panic!("typed error outcomes are outside CT01's trace")
                }
                // Card `W48-QR-SCALAR-TYPES-R6`: the grown alphabet's fixed-width
                // outcome, which no declared domain of this leg carries.
                ObservedOutcome::Fixed { .. } => {
                    panic!("fixed-width outcomes are outside the declared F02 domains")
                }
            };
            9 + 16 * (task + 8 * encoded)
        }
        ObservedEvent::Joined { child } => 10 + 16 * child,
    }
}

/// Mirrors `QCore.TaskTraceRelation.mixTrace`.
fn mix_trace(trace: &[ObservedEvent]) -> u64 {
    trace.iter().fold(TRACE_SEED, |accumulator, event| {
        accumulator
            .wrapping_mul(TRACE_MUL)
            .wrapping_add(encode_event(event))
            .wrapping_add(1)
    })
}

/* --------------------------------- the Lean decision procedure, clause for
clause. `QCore.Ct01OwnedTransfer.dischargesOwnedTransfer` over the production
`ObservedEvent` alphabet; `projection` is the actor filter, and the three
counted predicates are `isAllocationOf 1`, `isReleaseOf 1`,
`isCompletionOf 1`. -------------------------------------------------------- */

fn allocation() -> ObservedEvent {
    ObservedEvent::Allocated {
        task: 0,
        buffer: 1,
        payload: 42,
    }
}

fn creation() -> ObservedEvent {
    ObservedEvent::Spawned {
        child: 1,
        worker: 0,
    }
}

fn child_return() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 1,
        outcome: ObservedOutcome::Buffer(1),
    }
}

fn collection() -> ObservedEvent {
    ObservedEvent::Joined { child: 1 }
}

fn read_back() -> ObservedEvent {
    ObservedEvent::Read {
        task: 0,
        buffer: 1,
        value: 42,
    }
}

fn release() -> ObservedEvent {
    ObservedEvent::Dropped { task: 0, buffer: 1 }
}

fn root_result() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 0,
        outcome: ObservedOutcome::I32(42),
    }
}

/// `QCore.Ct01OwnedTransfer.parentOrder`.
fn parent_order() -> Vec<ObservedEvent> {
    vec![
        allocation(),
        creation(),
        collection(),
        read_back(),
        release(),
        root_result(),
    ]
}

fn projection(trace: &[ObservedEvent], actor: u64) -> Vec<ObservedEvent> {
    trace
        .iter()
        .filter(|event| event.actor() == Some(actor))
        .cloned()
        .collect()
}

/// `QCore.Ct01OwnedTransfer.dischargesOwnedTransfer`, transcribed.
fn discharges_owned_transfer(trace: &[ObservedEvent]) -> bool {
    let allocations = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Allocated { buffer: 1, .. }))
        .count();
    let releases = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Dropped { buffer: 1, .. }))
        .count();
    let completions = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Completed { task: 1, .. }))
        .count();
    trace.len() == 7
        && projection(trace, 0) == parent_order()
        && projection(trace, 1) == vec![child_return()]
        && allocations == 1
        && releases == 1
        && completions == 1
}

/// CT01's artifact: the fixture compiled through the real pipeline, its
/// checked task module executed by the reference execution, the admitted
/// causal order derived by the production observer.
fn ct01_reference_events() -> Vec<ObservedEvent> {
    let module = compile_files(
        &[SourceFile::new("case.qr", CT01_SOURCE)],
        &CompileOptions::default(),
    )
    .expect("CT01 is the manifest's accepted owned-transfer case");
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("CT01 lowers to a checked task module");
    let observation = execute_task_module(checked, &TaskExecutionScript::default()).unwrap();
    let order = AdmittedCausalOrder::derive(&observation).unwrap();
    order.events().to_vec()
}

/// The proposed `proof`-stage executor: CT01's own artifact is the trace term
/// the Lean theorems are about, and the same total decision procedure the
/// Lean definition defines accepts it.
#[test]
fn ct01_artifact_is_the_lean_proved_owned_transfer_shape() {
    let events = ct01_reference_events();
    // The recorded fixture trace `roundTrip` transcribes is this artifact's
    // own reference run, event for event.
    assert_eq!(events, parse_observed_trace(CT01_TRACE).unwrap());
    // The Lean-proved transcription pin, recomputed from the artifact.
    assert_eq!(mix_trace(&events), CT01_LEAN_TRACE_DIGEST);
    // The same total decision procedure the Lean definition defines.
    assert!(discharges_owned_transfer(&events));
    // The artifact's rendering is admitted by the F02 relation against its
    // own admitted causal order, so the Lean closure theorem
    // `every_admitted_rendering_discharges_the_owned_transfer` covers it.
    let order = AdmittedCausalOrder::declare(events.clone()).unwrap();
    assert!(relate(&order, &events).is_ok());
}

/// The transcription agrees on the refusing side too: the five mutants the
/// Lean module proves refused are refused here, mutant for mutant.
#[test]
fn the_procedure_refuses_the_lean_named_mutations() {
    let release_before_join = vec![
        allocation(),
        creation(),
        child_return(),
        release(),
        collection(),
        read_back(),
        root_result(),
    ];
    let missing_release = vec![
        allocation(),
        creation(),
        child_return(),
        collection(),
        read_back(),
        root_result(),
    ];
    let child_release = vec![
        allocation(),
        creation(),
        child_return(),
        collection(),
        read_back(),
        ObservedEvent::Dropped { task: 1, buffer: 1 },
        root_result(),
    ];
    let double_release = vec![
        allocation(),
        creation(),
        child_return(),
        collection(),
        read_back(),
        release(),
        ObservedEvent::Dropped { task: 1, buffer: 1 },
        root_result(),
    ];
    let generation_changing_return = vec![
        allocation(),
        creation(),
        ObservedEvent::Completed {
            task: 1,
            outcome: ObservedOutcome::Buffer(2),
        },
        collection(),
        read_back(),
        release(),
        root_result(),
    ];
    for (name, mutant) in [
        ("a release before the join", release_before_join),
        ("a missing release", missing_release),
        ("a release by the child", child_release),
        ("a double release", double_release),
        ("a generation-changing return", generation_changing_return),
    ] {
        assert!(
            !discharges_owned_transfer(&mutant),
            "{name} must be refused"
        );
    }
}
