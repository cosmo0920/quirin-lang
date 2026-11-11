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

//! CT03-PROOF-ALPHABET — the named runner behind CT03's proposed
//! `proof`-stage evidence, on the CT01/CT37 precedent
//! (`compiler/tests/ct01_proof_leg.rs`, `compiler/tests/ct37_proof_leg.rs`),
//! over the **extended** F02 alphabet (the T05 Buffer write).
//!
//! The Lean side is `proof/QCore/Ct03ReadShares.lean` over
//! `proof/QCore/TaskTraceWrites.lean`: a total decision procedure
//! `dischargesSharedReads`, theorems that its acceptance implies CT03's
//! scenario property (each borrower's own program order is exactly
//! read-then-complete; the parent's two writes sit on either side of the
//! lending window; each borrower's read sits strictly inside its own window;
//! every read reports the value the preceding write established rather than
//! the allocation's initial payload; the one shared generation is released
//! exactly once, by the parent, after the restored payload is read back), the
//! proof that CT03's reference trace is accepted, and the closure
//! `every_admitted_rendering_discharges_the_shared_reads` over every trace
//! the extended F02 relation admits against it.
//!
//! This test binds those theorems to **CT03's own artifact**: it compiles
//! `compiler/tests/fixtures/ct/CT03/case.qr` through the real pipeline,
//! executes the checked task module's reference execution, derives the
//! admitted causal order with the production observer — the derivation that
//! refused this very run before the alphabet grew
//! (`UnsupportedObservedEvent { index: 1 }`) — and
//!
//! 1. checks the derived events are exactly the trace the Lean `referenceRun`
//!    term transcribes (host-unconditionally — no `cfg`, no Clang, no native
//!    half);
//! 2. recomputes the Lean-proved transcription pin
//!    `QCore.Ct03ReadShares.ct03_trace_digest` from the artifact's own events
//!    with wrapping `u64` arithmetic (the extended F02 digest fold, in which
//!    the write carries the fresh code `11`);
//! 3. runs the same total decision procedure the Lean definition defines,
//!    transcribed below clause for clause, and requires acceptance;
//! 4. re-runs the procedure on the seven named mutants the Lean module proves
//!    refused, and requires refusal — the transcription agrees on the
//!    refusing side too.
//!
//! What this does **not** establish: anything about the emitted C's native
//! execution (covered only by the `native_heap_scenarios` harness's three
//! fixed-schedule sanitized observations), or a verified extraction between
//! Lean and Rust — the agreement is the checked-digest transcription pattern
//! F02 established.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, ObservedOutcome, relate,
};

const CT03_SOURCE: &str = include_str!("fixtures/ct/CT03/case.qr");

/// `QCore.Ct03ReadShares.ct03_trace_digest`, proved by the Lean kernel on the
/// `referenceRun` term this artifact must reproduce.
const CT03_LEAN_TRACE_DIGEST: u64 = 9638775376944872586;

/* --------------------------------------------------- the F02 digest fold */

const TRACE_SEED: u64 = 1469598103934665603;
const TRACE_MUL: u64 = 1000003;

fn natural(value: i32) -> u64 {
    u64::try_from(value).expect("CT03's trace carries no negative payload")
}

/// Mirrors `QCore.TaskTraceWrites.encodeEvent`: every pre-existing
/// constructor keeps its pinned code, and the write takes the fresh code
/// `11` — which is exactly why no pre-existing digest moves.
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
                    panic!("typed error outcomes are outside CT03's trace")
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

/// Mirrors `QCore.TaskTraceWrites.mixTrace`.
fn mix_trace(trace: &[ObservedEvent]) -> u64 {
    trace.iter().fold(TRACE_SEED, |accumulator, event| {
        accumulator
            .wrapping_mul(TRACE_MUL)
            .wrapping_add(encode_event(event))
            .wrapping_add(1)
    })
}

/* --------------------------------- the Lean decision procedure, clause for
clause. `QCore.Ct03ReadShares.dischargesSharedReads` over the production
`ObservedEvent` alphabet; `projection` is the actor filter, the six counted
predicates are `isAllocationOf 1`, `isReleaseOf 1`, `isCompletionOf 1/2` and
`isJoinOf 1/2`, and `readSitsInsideTheWindow` compares the first loan's, the
first shared read's and the first returned share's positions per borrower. - */

fn allocation() -> ObservedEvent {
    ObservedEvent::Allocated {
        task: 0,
        buffer: 1,
        payload: 1,
    }
}

fn prepared() -> ObservedEvent {
    ObservedEvent::Written {
        task: 0,
        buffer: 1,
        value: 42,
    }
}

fn creation_one() -> ObservedEvent {
    ObservedEvent::Spawned {
        child: 1,
        worker: 0,
    }
}

fn loan_one() -> ObservedEvent {
    ObservedEvent::Loaned {
        borrower: 1,
        buffer: 1,
    }
}

fn creation_two() -> ObservedEvent {
    ObservedEvent::Spawned {
        child: 2,
        worker: 0,
    }
}

fn loan_two() -> ObservedEvent {
    ObservedEvent::Loaned {
        borrower: 2,
        buffer: 1,
    }
}

fn parent_read() -> ObservedEvent {
    ObservedEvent::Read {
        task: 0,
        buffer: 1,
        value: 42,
    }
}

fn child_one_read() -> ObservedEvent {
    ObservedEvent::Read {
        task: 1,
        buffer: 1,
        value: 42,
    }
}

fn child_one_done() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 1,
        outcome: ObservedOutcome::I32(42),
    }
}

fn collection_one() -> ObservedEvent {
    ObservedEvent::Joined { child: 1 }
}

fn return_one() -> ObservedEvent {
    ObservedEvent::Returned {
        borrower: 1,
        buffer: 1,
    }
}

fn child_two_read() -> ObservedEvent {
    ObservedEvent::Read {
        task: 2,
        buffer: 1,
        value: 42,
    }
}

fn child_two_done() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 2,
        outcome: ObservedOutcome::I32(42),
    }
}

fn collection_two() -> ObservedEvent {
    ObservedEvent::Joined { child: 2 }
}

fn return_two() -> ObservedEvent {
    ObservedEvent::Returned {
        borrower: 2,
        buffer: 1,
    }
}

fn restored() -> ObservedEvent {
    ObservedEvent::Written {
        task: 0,
        buffer: 1,
        value: 7,
    }
}

fn after_read() -> ObservedEvent {
    ObservedEvent::Read {
        task: 0,
        buffer: 1,
        value: 7,
    }
}

fn release() -> ObservedEvent {
    ObservedEvent::Dropped { task: 0, buffer: 1 }
}

fn root_result() -> ObservedEvent {
    ObservedEvent::Completed {
        task: 0,
        outcome: ObservedOutcome::I32(7),
    }
}

/// `QCore.Ct03ReadShares.referenceRun`.
fn reference_run() -> Vec<ObservedEvent> {
    vec![
        allocation(),
        prepared(),
        creation_one(),
        loan_one(),
        creation_two(),
        loan_two(),
        parent_read(),
        child_one_read(),
        child_one_done(),
        collection_one(),
        return_one(),
        child_two_read(),
        child_two_done(),
        collection_two(),
        return_two(),
        restored(),
        after_read(),
        release(),
        root_result(),
    ]
}

/// `QCore.Ct03ReadShares.parentOrder`.
fn parent_order() -> Vec<ObservedEvent> {
    vec![
        allocation(),
        prepared(),
        creation_one(),
        loan_one(),
        creation_two(),
        loan_two(),
        parent_read(),
        collection_one(),
        return_one(),
        collection_two(),
        return_two(),
        restored(),
        after_read(),
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

/// `QCore.Ct03ReadShares.readSitsInsideTheWindow`, transcribed: the
/// borrower's first shared read sits strictly between its loan opening and
/// its share returning.
fn read_sits_inside_the_window(child: u64, trace: &[ObservedEvent]) -> bool {
    let loan = trace.iter().position(|event| {
        matches!(event, ObservedEvent::Loaned { borrower, buffer: 1 } if *borrower == child)
    });
    let read = trace.iter().position(
        |event| matches!(event, ObservedEvent::Read { task, buffer: 1, .. } if *task == child),
    );
    let returned = trace.iter().position(|event| {
        matches!(event, ObservedEvent::Returned { borrower, buffer: 1 } if *borrower == child)
    });
    match (loan, read, returned) {
        (Some(loan), Some(read), Some(returned)) => loan < read && read < returned,
        _ => false,
    }
}

/// `QCore.Ct03ReadShares.dischargesSharedReads`, transcribed.
fn discharges_shared_reads(trace: &[ObservedEvent]) -> bool {
    let allocations = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Allocated { buffer: 1, .. }))
        .count();
    let releases = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Dropped { buffer: 1, .. }))
        .count();
    let completions_one = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Completed { task: 1, .. }))
        .count();
    let completions_two = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Completed { task: 2, .. }))
        .count();
    let joins_one = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Joined { child: 1 }))
        .count();
    let joins_two = trace
        .iter()
        .filter(|event| matches!(event, ObservedEvent::Joined { child: 2 }))
        .count();
    trace.len() == 19
        && projection(trace, 0) == parent_order()
        && projection(trace, 1) == vec![child_one_read(), child_one_done()]
        && projection(trace, 2) == vec![child_two_read(), child_two_done()]
        && allocations == 1
        && releases == 1
        && completions_one == 1
        && completions_two == 1
        && joins_one == 1
        && joins_two == 1
        && read_sits_inside_the_window(1, trace)
        && read_sits_inside_the_window(2, trace)
}

/// CT03's artifact: the fixture compiled through the real pipeline, its
/// checked task module executed by the reference execution, the admitted
/// causal order derived by the production observer.
fn ct03_reference_events() -> Vec<ObservedEvent> {
    let module = compile_files(
        &[SourceFile::new("case.qr", CT03_SOURCE)],
        &CompileOptions::default(),
    )
    .expect("CT03 is the manifest's accepted shared-reads case");
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("CT03 lowers to a checked task module");
    let observation = execute_task_module(checked, &TaskExecutionScript::default()).unwrap();
    let order = AdmittedCausalOrder::derive(&observation)
        .expect("the extended alphabet admits CT03's reference run");
    order.events().to_vec()
}

/// The proposed `proof`-stage executor: CT03's own artifact is the trace term
/// the Lean theorems are about, and the same total decision procedure the
/// Lean definition defines accepts it.
#[test]
fn ct03_artifact_is_the_lean_proved_shared_reads_shape() {
    let events = ct03_reference_events();
    // The artifact's reference run is exactly the trace the Lean
    // `referenceRun` term transcribes, event for event.
    assert_eq!(events, reference_run());
    // The Lean-proved transcription pin, recomputed from the artifact.
    assert_eq!(mix_trace(&events), CT03_LEAN_TRACE_DIGEST);
    // The same total decision procedure the Lean definition defines.
    assert!(discharges_shared_reads(&events));
    // The artifact's rendering is admitted by the production `relate` against
    // its own declared order, so the Lean closure theorem
    // `every_admitted_rendering_discharges_the_shared_reads` covers it.
    let order = AdmittedCausalOrder::declare(events.clone()).unwrap();
    assert!(relate(&order, &events).is_ok());
}

/// The transcription agrees on the refusing side too: the seven mutants the
/// Lean module proves refused are refused here, mutant for mutant.
#[test]
fn the_procedure_refuses_the_lean_named_mutations() {
    let read_before_the_loan = vec![
        allocation(),
        prepared(),
        creation_one(),
        child_one_read(),
        loan_one(),
        creation_two(),
        loan_two(),
        parent_read(),
        child_one_done(),
        collection_one(),
        return_one(),
        child_two_read(),
        child_two_done(),
        collection_two(),
        return_two(),
        restored(),
        after_read(),
        release(),
        root_result(),
    ];
    let read_after_the_return = vec![
        allocation(),
        prepared(),
        creation_one(),
        loan_one(),
        creation_two(),
        loan_two(),
        parent_read(),
        child_one_done(),
        collection_one(),
        return_one(),
        child_one_read(),
        child_two_read(),
        child_two_done(),
        collection_two(),
        return_two(),
        restored(),
        after_read(),
        release(),
        root_result(),
    ];
    let write_inside_the_window = {
        let mut trace = reference_run();
        trace[6] = ObservedEvent::Written {
            task: 0,
            buffer: 1,
            value: 9,
        };
        trace
    };
    let stale_after_read = {
        let mut trace = reference_run();
        trace[16] = ObservedEvent::Read {
            task: 0,
            buffer: 1,
            value: 42,
        };
        trace
    };
    let missing_release = {
        let mut trace = reference_run();
        trace.remove(17);
        trace
    };
    let double_release = {
        let mut trace = reference_run();
        trace.insert(18, ObservedEvent::Dropped { task: 1, buffer: 1 });
        trace
    };
    let borrower_that_writes = {
        let mut trace = reference_run();
        trace[7] = ObservedEvent::Written {
            task: 1,
            buffer: 1,
            value: 5,
        };
        trace
    };
    for (name, mutant) in [
        ("a read before the loan", read_before_the_loan),
        ("a read after the return", read_after_the_return),
        ("a write inside the window", write_inside_the_window),
        ("a stale after-read", stale_after_read),
        ("a missing release", missing_release),
        ("a double release", double_release),
        ("a borrower that writes", borrower_that_writes),
    ] {
        assert!(!discharges_shared_reads(&mutant), "{name} must be refused");
    }
}
