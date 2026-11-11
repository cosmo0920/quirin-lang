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

//! CT-PROOF-LEG-2 — the named runner behind CT37's proposed `proof`-stage
//! evidence, on the CT01 precedent (`compiler/tests/ct01_proof_leg.rs`).
//!
//! The Lean side is `proof/QCore/Ct37CompletionCapsule.lean`: a total decision
//! procedure `dischargesCompletionCapsule` over the F02 observation alphabet,
//! theorems that its acceptance implies CT37's scenario property (the child's
//! own program order is exactly allocate-then-finish, carrying the very
//! generation its only allocation created; the child's completion sits
//! strictly before the parent's join — the early finish; the moved generation
//! is released exactly once, by the parent, after the join and after the
//! allocation's own payload is read back out of the completion capsule), the
//! proof that CT37's reference trace is accepted, and the closure
//! `every_admitted_rendering_discharges_the_completion_capsule` over every
//! trace the F02 relation admits against it.
//!
//! This test binds those theorems to **CT37's own artifact**: it compiles
//! `compiler/tests/fixtures/ct/CT37/case.qr` through the real pipeline,
//! executes the checked task module's reference execution, derives the
//! admitted causal order with the production observer, and
//!
//! 1. checks the derived events are exactly the trace the Lean `referenceRun`
//!    term transcribes (host-unconditionally — no `cfg`, no Clang, no native
//!    half);
//! 2. recomputes the Lean-proved transcription pin
//!    `QCore.Ct37CompletionCapsule.ct37_trace_digest` from the artifact's own
//!    events with wrapping `u64` arithmetic (the F02 digest fold), so the
//!    Lean term and the artifact cannot drift apart silently;
//! 3. runs the same total decision procedure the Lean definition defines,
//!    transcribed below clause for clause, and requires acceptance;
//! 4. re-runs the procedure on the six named mutants the Lean module proves
//!    refused, and requires refusal — the transcription agrees on the
//!    refusing side too.
//!
//! What this does **not** establish: anything about the emitted C's native
//! execution (covered only by the `native_task_scenarios` harness's
//! observations), or a verified extraction between Lean and Rust — the
//! agreement is the checked-digest transcription pattern F02 established.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, ObservedOutcome, relate,
};

const CT37_SOURCE: &str = include_str!("fixtures/ct/CT37/case.qr");

/// `QCore.Ct37CompletionCapsule.ct37_trace_digest`, proved by the Lean kernel
/// on the `referenceRun` term this artifact must reproduce.
const CT37_LEAN_TRACE_DIGEST: u64 = 17239871955803798867;

/// `QCore.Ct37CompletionCapsule.movedGeneration`: the worker-local generation
/// of the child's own allocation, `(1 << 32) + 1`.
const MOVED_GENERATION: u64 = 4294967297;

/* --------------------------------------------------- the F02 digest fold */

const TRACE_SEED: u64 = 1469598103934665603;
const TRACE_MUL: u64 = 1000003;

fn natural(value: i32) -> u64 {
    u64::try_from(value).expect("CT37's trace carries no negative payload")
}

/// Mirrors `QCore.TaskTraceRelation.encodeEvent`, exactly as the F02 harness
/// and the CT01 runner do.
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
                    panic!("typed error outcomes are outside CT37's trace")
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
clause. `QCore.Ct37CompletionCapsule.dischargesCompletionCapsule` over the
production `ObservedEvent` alphabet; `projection` is the actor filter, the
four counted predicates are `isAllocationOf movedGeneration`,
`isReleaseOf movedGeneration`, `isCompletionOf 1` and `isJoinOf 1`, and
`returnPrecedesTheJoin` compares the first completion's and the first join's
positions. ----------------------------------------------------------------- */

fn creation() -> ObservedEvent {
    ObservedEvent::Spawned {
        child: 1,
        worker: 0,
    }
}

fn allocation() -> ObservedEvent {
    ObservedEvent::Allocated {
        task: 1,
        buffer: MOVED_GENERATION,
        payload: 42,
    }
}

fn early_return() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 1,
        outcome: ObservedOutcome::Buffer(MOVED_GENERATION),
    }
}

fn collection() -> ObservedEvent {
    ObservedEvent::Joined { child: 1 }
}

fn read_back() -> ObservedEvent {
    ObservedEvent::Read {
        task: 0,
        buffer: MOVED_GENERATION,
        value: 42,
    }
}

fn release() -> ObservedEvent {
    ObservedEvent::Dropped {
        task: 0,
        buffer: MOVED_GENERATION,
    }
}

fn root_result() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 0,
        outcome: ObservedOutcome::I32(42),
    }
}

/// `QCore.Ct37CompletionCapsule.referenceRun`.
fn reference_run() -> Vec<ObservedEvent> {
    vec![
        creation(),
        allocation(),
        early_return(),
        collection(),
        read_back(),
        release(),
        root_result(),
    ]
}

/// `QCore.Ct37CompletionCapsule.parentOrder`.
fn parent_order() -> Vec<ObservedEvent> {
    vec![
        creation(),
        collection(),
        read_back(),
        release(),
        root_result(),
    ]
}

/// `QCore.Ct37CompletionCapsule.childOrder`.
fn child_order() -> Vec<ObservedEvent> {
    vec![allocation(), early_return()]
}

fn projection(trace: &[ObservedEvent], actor: u64) -> Vec<ObservedEvent> {
    trace
        .iter()
        .filter(|event| event.actor() == Some(actor))
        .cloned()
        .collect()
}

/// `QCore.Ct37CompletionCapsule.returnPrecedesTheJoin`, transcribed: the
/// first completion of the child sits strictly before the first join of it.
fn return_precedes_the_join(trace: &[ObservedEvent]) -> bool {
    let completion = trace
        .iter()
        .position(|event| matches!(event, ObservedEvent::Completed { task: 1, .. }));
    let join = trace
        .iter()
        .position(|event| matches!(event, ObservedEvent::Joined { child: 1 }));
    match (completion, join) {
        (Some(completion), Some(join)) => completion < join,
        _ => false,
    }
}

/// `QCore.Ct37CompletionCapsule.dischargesCompletionCapsule`, transcribed.
fn discharges_completion_capsule(trace: &[ObservedEvent]) -> bool {
    let allocations = trace
        .iter()
        .filter(|event| {
            matches!(
                event,
                ObservedEvent::Allocated {
                    buffer: MOVED_GENERATION,
                    ..
                }
            )
        })
        .count();
    let releases = trace
        .iter()
        .filter(|event| {
            matches!(
                event,
                ObservedEvent::Dropped {
                    buffer: MOVED_GENERATION,
                    ..
                }
            )
        })
        .count();
    let completions = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Completed { task: 1, .. }))
        .count();
    let joins = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Joined { child: 1 }))
        .count();
    trace.len() == 7
        && projection(trace, 0) == parent_order()
        && projection(trace, 1) == child_order()
        && allocations == 1
        && releases == 1
        && completions == 1
        && joins == 1
        && return_precedes_the_join(trace)
}

/// CT37's artifact: the fixture compiled through the real pipeline, its
/// checked task module executed by the reference execution, the admitted
/// causal order derived by the production observer.
fn ct37_reference_events() -> Vec<ObservedEvent> {
    let module = compile_files(
        &[SourceFile::new("case.qr", CT37_SOURCE)],
        &CompileOptions::default(),
    )
    .expect("CT37 is the manifest's accepted completion-capsule case");
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("CT37 lowers to a checked task module");
    let observation = execute_task_module(checked, &TaskExecutionScript::default()).unwrap();
    let order = AdmittedCausalOrder::derive(&observation).unwrap();
    order.events().to_vec()
}

/// The proposed `proof`-stage executor: CT37's own artifact is the trace term
/// the Lean theorems are about, and the same total decision procedure the
/// Lean definition defines accepts it.
#[test]
fn ct37_artifact_is_the_lean_proved_completion_capsule_shape() {
    let events = ct37_reference_events();
    // The artifact's reference run is exactly the trace the Lean
    // `referenceRun` term transcribes, event for event.
    assert_eq!(events, reference_run());
    // The Lean-proved transcription pin, recomputed from the artifact.
    assert_eq!(mix_trace(&events), CT37_LEAN_TRACE_DIGEST);
    // The same total decision procedure the Lean definition defines.
    assert!(discharges_completion_capsule(&events));
    // The artifact's rendering is admitted by the production `relate` against
    // its own declared order, so the Lean closure theorem
    // `every_admitted_rendering_discharges_the_completion_capsule` covers it.
    let order = AdmittedCausalOrder::declare(events.clone()).unwrap();
    assert!(relate(&order, &events).is_ok());
}

/// The transcription agrees on the refusing side too: the six mutants the
/// Lean module proves refused are refused here, mutant for mutant.
#[test]
fn the_procedure_refuses_the_lean_named_mutations() {
    let join_before_the_return = vec![
        creation(),
        allocation(),
        collection(),
        early_return(),
        read_back(),
        release(),
        root_result(),
    ];
    let missing_release = vec![
        creation(),
        allocation(),
        early_return(),
        collection(),
        read_back(),
        root_result(),
    ];
    let release_by_the_child = vec![
        creation(),
        allocation(),
        ObservedEvent::Dropped {
            task: 1,
            buffer: MOVED_GENERATION,
        },
        early_return(),
        collection(),
        read_back(),
        root_result(),
    ];
    let double_release = vec![
        creation(),
        allocation(),
        early_return(),
        collection(),
        read_back(),
        release(),
        ObservedEvent::Dropped {
            task: 1,
            buffer: MOVED_GENERATION,
        },
        root_result(),
    ];
    let generation_changing_return = vec![
        creation(),
        allocation(),
        ObservedEvent::Completed {
            task: 1,
            outcome: ObservedOutcome::Buffer(MOVED_GENERATION + 1),
        },
        collection(),
        read_back(),
        release(),
        root_result(),
    ];
    let payload_losing_capsule = vec![
        creation(),
        allocation(),
        early_return(),
        collection(),
        ObservedEvent::Read {
            task: 0,
            buffer: MOVED_GENERATION,
            value: 7,
        },
        release(),
        root_result(),
    ];
    for (name, mutant) in [
        ("a join before the return", join_before_the_return),
        ("a missing release", missing_release),
        ("a release by the child", release_by_the_child),
        ("a double release", double_release),
        ("a generation-changing return", generation_changing_return),
        ("a payload-losing capsule", payload_losing_capsule),
    ] {
        assert!(
            !discharges_completion_capsule(&mutant),
            "{name} must be refused"
        );
    }
}
