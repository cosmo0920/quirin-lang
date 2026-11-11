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

//! PROOF-LEG-BATCH — the measurement behind two **withdrawn** proof rows, and
//! the tripwire that reopens them.
//!
//! Wave 32's card set out to bind CT01's already-proved decision procedure
//! (`QCore.Ct01OwnedTransfer.dischargesOwnedTransfer`,
//! `compiler/tests/ct01_proof_leg.rs`) to CT12's and CT14's own artifacts, on
//! the premise recorded by `docs/reports/ct-proof-leg-2/README.md` §1: that
//! their default reference traces are *event for event* CT01's `roundTrip`.
//!
//! The premise was measured rather than assumed, and it holds — for a stronger
//! reason than the survey recorded, which is why **both rows were withdrawn**:
//! `compiler/tests/fixtures/ct/{CT01,CT12,CT14}/case.qr` are one
//! **byte-identical file** (sha256 `131f635d…`, the only duplicate among the
//! corpus's forty case fixtures). The proof stage as all three landed proof
//! legs define it is a function of the fixture's bytes alone — compile,
//! reference-execute under the default script, derive the admitted causal
//! order, decide — so CT12's and CT14's proof rows could carry only CT01's
//! proof verbatim, with a different path in the `artifact` field and no new
//! proved content whatever. Constraint 23's gate ("a named test runs it on the
//! case's own artifact") would be satisfied *vacuously*, because here "the
//! case's own artifact" does not distinguish the cases from CT01. See
//! `docs/reports/proof-leg-batch/README.md`.
//!
//! What each case's *distinguishing* scenario needs is out of the relation's
//! reach, and that too is measured below: CT12's creation failure and CT14's
//! completed-but-unjoined arm are realised by the native harness, and the one
//! scripted arm that would put a creation failure into a reference run derives
//! to a `CreationFailed` event whose `actor()` is `None`, which
//! `AdmittedCausalOrder` refuses as `UnattributableEvent`. No declared order
//! can contain the event the case is about.
//!
//! **These tests are tripwires, in the sense constraint 24 gives the word: they
//! are meant to be flipped deliberately, not silently.** Two future changes
//! should fire them, and each is exactly the condition under which the
//! withdrawn rows genuinely reopen:
//!
//! 1. any of the three fixtures diverging — then CT12 or CT14 has an artifact
//!    of its own, and its proof leg is a real question again;
//! 2. the F02 alphabet being extended to attribute `CreationFailed` to a
//!    creator (the deliberate, digest-re-pinning shape card
//!    `CT03-PROOF-ALPHABET` established for Buffer writes) — then CT12's own
//!    scenario becomes expressible, and CT13/CT15 with it.
//!
//! Neither test asserts anything about the emitted C's native execution, and
//! neither adds proof-stage evidence for any case.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use qcore_core::task_execution::{TaskExecutionScript, execute_task_module};
use qcore_core::task_trace_relation::{
    AdmittedCausalOrder, ObservedEvent, TraceDeriveError, TraceRelationViolation,
    parse_observed_trace,
};

const CT01_SOURCE: &str = include_str!("fixtures/ct/CT01/case.qr");
const CT12_SOURCE: &str = include_str!("fixtures/ct/CT12/case.qr");
const CT14_SOURCE: &str = include_str!("fixtures/ct/CT14/case.qr");
const CT01_TRACE: &str =
    include_str!("../../crates/qcore-core/tests/fixtures/f02/round_trip.trace");

/// `QCore.Ct01OwnedTransfer.ct01_trace_digest`, the Lean-kernel-proved pin over
/// CT01's `roundTrip` term. Recomputed here from each case's own reference run
/// so the identity below is an identity of *trace terms*, not just of files.
const CT01_LEAN_TRACE_DIGEST: u64 = 13023433800416048247;

const TRACE_SEED: u64 = 1469598103934665603;
const TRACE_MUL: u64 = 1000003;

fn natural(value: i32) -> u64 {
    u64::try_from(value).expect("this trace carries no negative payload")
}

/// Mirrors `QCore.TaskTraceRelation.encodeEvent`, as the F02 harness and
/// `ct01_proof_leg.rs` both do.
fn encode_event(event: &ObservedEvent) -> u64 {
    use qcore_core::task_trace_relation::ObservedOutcome;
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
                    panic!("typed error outcomes are outside this trace")
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

/// One case's artifact: its own fixture through the real pipeline, the checked
/// task module's reference execution under the given script, the admitted
/// causal order derived by the production observer.
fn reference_order(
    source: &str,
    script: &TaskExecutionScript,
) -> Result<AdmittedCausalOrder, TraceDeriveError> {
    let module = compile_files(
        &[SourceFile::new("case.qr", source)],
        &CompileOptions::default(),
    )
    .expect("all three cases are mapped, accepted task modules");
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("the fixture lowers to a checked task module");
    let observation = execute_task_module(checked, script)
        .expect("the reference execution runs under this script");
    AdmittedCausalOrder::derive(&observation)
}

/// The measurement that withdrew both proposed proof rows: CT12 and CT14 are
/// CT01's artifact under two more names, at three levels of identity — the
/// fixture's bytes, the derived reference trace, and the Lean-proved digest
/// over that trace's term.
#[test]
fn ct12_and_ct14_are_ct01s_artifact_under_two_more_names() {
    // 1. The source bytes. Not "equivalent programs": the same file contents.
    assert_eq!(
        CT12_SOURCE, CT01_SOURCE,
        "CT12's fixture is byte-identical to CT01's"
    );
    assert_eq!(
        CT14_SOURCE, CT01_SOURCE,
        "CT14's fixture is byte-identical to CT01's"
    );

    // 2. The derived reference runs, event for event, against the recorded F02
    //    trace fixture that `QCore.TaskTraceRelation.roundTrip` transcribes.
    let recorded = parse_observed_trace(CT01_TRACE).expect("the recorded F02 trace parses");
    let script = TaskExecutionScript::default();
    for (case, source) in [("CT12", CT12_SOURCE), ("CT14", CT14_SOURCE)] {
        let order = reference_order(source, &script)
            .unwrap_or_else(|error| panic!("{case}'s default reference run derives: {error:?}"));
        assert_eq!(
            order.events(),
            recorded.as_slice(),
            "{case}'s reference run is CT01's `roundTrip`, event for event"
        );
        // 3. The Lean-proved transcription pin, recomputed from this case's own
        //    artifact. Equality here is what makes the point exact: there is
        //    one trace term, so there is one theorem, so a proof row for this
        //    case would publish CT01's proof a second time.
        assert_eq!(
            mix_trace(order.events()),
            CT01_LEAN_TRACE_DIGEST,
            "{case}'s reference run carries CT01's proved digest"
        );
    }
}

/// Why neither case can reach its *own* scenario through this relation today:
/// the one scripted arm that puts a creation failure into a reference run
/// derives a `CreationFailed` event, whose `actor()` is `None`, and the
/// relation refuses an unattributable line rather than guessing a creator.
/// CT12's scenario ("Create fails before any child starts") is therefore not
/// expressible as a declared order at all, and CT14's completed-but-unjoined
/// arm is a property of the native adapter rather than of any trace.
#[test]
fn the_creation_failure_arm_is_outside_the_relation() {
    let fail_create = TaskExecutionScript {
        spawn_outcomes: vec![false],
        allocation_outcomes: Vec::new(),
    };
    for (case, source) in [("CT12", CT12_SOURCE), ("CT14", CT14_SOURCE)] {
        match reference_order(source, &fail_create) {
            Err(TraceDeriveError::Violation(TraceRelationViolation::UnattributableEvent {
                ..
            })) => {}
            other => panic!(
                "{case}'s creation-failure arm must stay unattributable until the \
                 alphabet is deliberately extended; got {other:?}"
            ),
        }
    }
}
