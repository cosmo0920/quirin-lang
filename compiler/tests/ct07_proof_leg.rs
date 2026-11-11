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

//! CT07-TWO-THREAD-PROOF-LEG — the named runner behind CT07's `proof`-stage
//! evidence, binding the disjointness theorem to the case's own artifact
//! **through the checked `ScopedTasks` module's lease set**, not through the
//! F02 trace relation.
//!
//! The Lean side is `proof/QCore/Ct07LeaseLedger.lean` over
//! `proof/QCore/WriteLendTasks.lean` (wave 31) and
//! `proof/QCore/WriteLendTasksExercise.lean` (wave 36). The theorem bound is
//! **`QCore.WriteLendTasks.no_byte_has_two_write_authorities`**: for every
//! ledger state a spawn/join interleaving can *reach* and every byte of the
//! payload, at most one party holds write authority. Its hypothesis is
//! `Reachable s`, a relation over lease lists; the Lean module adds one general
//! theorem, `replay_reaches` — whatever state a successful replay of admitted
//! opens and closes produces from the empty ledger is `Reachable` — and
//! instantiates the theorem on CT07's own two-thread ledger state
//! (`ct07_no_byte_has_two_write_authorities`), with wave 36's
//! `no_byte_is_written_by_two_parties` instantiated on the same state and the
//! artifact's two byte writes shown admitted against it.
//!
//! Why not the CT01/CT37/CT03 pattern: those legs bind through
//! `AdmittedCausalOrder::derive`, whose `reference_trace` **refuses** CT07's
//! reference run at its first lease event under constraint 15n as amended by
//! `WRITE-LEND-TASKS-EXERCISE` — a decided boundary
//! (`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`). This leg does not touch it and does
//! not grow the alphabet; it reads the observation-side events that
//! boundary refuses to encode and replays them through the ledger model.
//!
//! This test binds the theorem to **CT07's own artifact**: it compiles
//! `compiler/tests/fixtures/ct/CT07/case.qr` through the real pipeline,
//! reads the lease captures out of the checked task IR, executes the checked
//! module's reference execution, and
//!
//! 1. transcribes the reference run's lease opens and closes into the Lean
//!    `Step` vocabulary (holder = the child's own task identity) and checks
//!    they are exactly the `ct07Steps` term (host-unconditionally — no
//!    `cfg`, no Clang, no native half);
//! 2. runs the same total `replay` the Lean definition defines, transcribed
//!    below clause for clause (`admitsLease`, `openLease`, `closeLease`), and
//!    requires `Some` — which, by `replay_reaches`, is a `Reachable` witness
//!    for the state the artifact reaches, the theorem's hypothesis;
//! 3. recomputes the Lean-proved pin `ct07_ledger_digest` from that state with
//!    wrapping `u64` arithmetic;
//! 4. checks the scenario shape `dischargesDisjointHalves` — two leases, two
//!    distinct holders, every payload byte under **exactly one** write
//!    authority, computed by the theorem's own `writeAuthorities`;
//! 5. drives the production `ScopedTaskLedger` through the same steps and
//!    requires its lease set to agree with the transcribed state after every
//!    step — the `Reachable` term ↔ ledger state correspondence, and
//!    `LeaseRange::overlaps` (which calls `ownership::axes_overlap`) agrees
//!    with the transcribed `overlaps` on the artifact's ranges;
//! 6. re-runs the procedure on the three named mutations the Lean module
//!    proves decided — overlapping halves, an empty half, a single writer —
//!    and requires each to be decided **for the theorem's reason**, measured
//!    separately from where the checker catches the mutated fixture.
//!
//! What this does **not** establish: anything about the emitted C's native
//! execution (covered only by `run_native_heap_scenarios`' sanitized
//! observations, with no ThreadSanitizer evidence — `TASK-ROUTE-TSAN-SEGV`),
//! or a verified extraction between Lean and Rust — the agreement is the
//! checked-digest transcription pattern F02 established, here over the ledger
//! rather than the trace.

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};
use qcore_core::scoped_tasks::{ScopedTaskLedger, TaskOutcome};
use qcore_core::task_execution::{
    LeaseRange, ScalarOperand, TaskEvent, TaskExecutionObservation, TaskExecutionScript,
    TaskTerminatorKind, execute_task_module,
};
use qcore_core::task_trace_relation::AdmittedCausalOrder;

const CT07_SOURCE: &str = include_str!("fixtures/ct/CT07/case.qr");

/// `QCore.Ct07LeaseLedger.ct07_ledger_digest`, proved by the Lean kernel on
/// the `ct07Ledger` term this artifact must reproduce.
const CT07_LEAN_LEDGER_DIGEST: u64 = 8876998389998880787;

/* -------------------------------------------- the Lean vocabulary, clause
for clause. `QCore.SlicePermissions.Range`/`overlaps`,
`QCore.WriteLendTasks.{Lease, MAX_WRITERS, admitsLease, openLease, closeLease,
leasesHolding, writeAuthorities}`, `QCore.WriteLendTasksExercise.admitsWrite`,
and `QCore.Ct07LeaseLedger.{Step, step, replayFrom, replay, PAYLOAD_BYTES,
distinctHolders, dischargesDisjointHalves, encodeLease, mixLedger}`. ------- */

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Range {
    low: u64,
    high: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Lease {
    holder: u64,
    range: Range,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// `Step.lease`: a lease opened at a child's spawn.
    Lease { holder: u64, range: Range },
    /// `Step.reclaim`: every lease of a child closed at its creator's join.
    Reclaim { holder: u64 },
}

/// `QCore.WriteLendTasks.MAX_WRITERS`, mirroring `scoped_tasks::MAX_WRITERS`.
const MAX_WRITERS: usize = 2;

/// `QCore.Ct07LeaseLedger.PAYLOAD_BYTES`: one `int32_t`.
const PAYLOAD_BYTES: u64 = 4;

/// `Range.contains`.
fn contains(range: Range, byte: u64) -> bool {
    range.low <= byte && byte < range.high
}

/// `QCore.SlicePermissions.overlaps`: `max low < min high`.
fn overlaps(a: Range, b: Range) -> bool {
    a.low.max(b.low) < a.high.min(b.high)
}

/// `admitsLease`: nonempty, within the holder bound, disjoint from every
/// outstanding lease.
fn admits_lease(leases: &[Lease], range: Range) -> bool {
    range.low < range.high
        && leases.len() < MAX_WRITERS
        && leases.iter().all(|lease| !overlaps(lease.range, range))
}

/// `openLease`: total-or-nothing, consing the new lease.
fn open_lease(leases: &[Lease], holder: u64, range: Range) -> Option<Vec<Lease>> {
    if admits_lease(leases, range) {
        let mut opened = vec![Lease { holder, range }];
        opened.extend_from_slice(leases);
        Some(opened)
    } else {
        None
    }
}

/// `closeLease`: every lease of one holder leaves.
fn close_lease(leases: &[Lease], holder: u64) -> Vec<Lease> {
    leases
        .iter()
        .copied()
        .filter(|lease| lease.holder != holder)
        .collect()
}

/// `leasesHolding`.
fn leases_holding(leases: &[Lease], byte: u64) -> Vec<Lease> {
    leases
        .iter()
        .copied()
        .filter(|lease| contains(lease.range, byte))
        .collect()
}

/// `writeAuthorities`: the leases holding the byte, plus the owner exactly
/// when no lease is outstanding.
fn write_authorities(leases: &[Lease], byte: u64) -> usize {
    leases_holding(leases, byte).len() + usize::from(leases.is_empty())
}

/// `QCore.WriteLendTasksExercise.admitsWrite`: some outstanding lease is the
/// writer's and contains the byte.
fn admits_write(leases: &[Lease], writer: u64, byte: u64) -> bool {
    leases
        .iter()
        .any(|lease| lease.holder == writer && contains(lease.range, byte))
}

/// `step`.
fn step(leases: &[Lease], step: Step) -> Option<Vec<Lease>> {
    match step {
        Step::Lease { holder, range } => open_lease(leases, holder, range),
        Step::Reclaim { holder } => Some(close_lease(leases, holder)),
    }
}

/// `replayFrom`.
fn replay_from(leases: &[Lease], steps: &[Step]) -> Option<Vec<Lease>> {
    match steps.split_first() {
        None => Some(leases.to_vec()),
        Some((first, rest)) => step(leases, *first).and_then(|next| replay_from(&next, rest)),
    }
}

/// `replay`: from the empty ledger.
fn replay(steps: &[Step]) -> Option<Vec<Lease>> {
    replay_from(&[], steps)
}

/// `distinctHolders`.
fn distinct_holders(leases: &[Lease]) -> bool {
    match leases.split_first() {
        None => true,
        Some((first, rest)) => {
            rest.iter().all(|other| other.holder != first.holder) && distinct_holders(rest)
        }
    }
}

/// `dischargesDisjointHalves`: exactly two leases, two distinct holders, and
/// every payload byte under exactly one write authority.
fn discharges_disjoint_halves(leases: &[Lease]) -> bool {
    leases.len() == 2
        && distinct_holders(leases)
        && (0..PAYLOAD_BYTES).all(|byte| write_authorities(leases, byte) == 1)
}

/// `encodeLease`.
fn encode_lease(lease: Lease) -> u64 {
    let bounds = lease
        .range
        .low
        .wrapping_add(64u64.wrapping_mul(lease.range.high));
    let identity = lease.holder.wrapping_add(8u64.wrapping_mul(bounds));
    1u64.wrapping_add(16u64.wrapping_mul(identity))
}

const LEDGER_SEED: u64 = 1469598103934665603;
const LEDGER_MUL: u64 = 1000003;

/// `mixLedger`: the F02 fold's seed and multiplier over the leases in ledger
/// order, `% 2^64` being wrapping `u64` arithmetic.
fn mix_ledger(leases: &[Lease]) -> u64 {
    leases.iter().fold(LEDGER_SEED, |accumulator, lease| {
        accumulator
            .wrapping_mul(LEDGER_MUL)
            .wrapping_add(encode_lease(*lease))
            .wrapping_add(1)
    })
}

/* ------------------------------------------------------- CT07's own terms */

/// `leftHalf`: `lend buffer 0..2`.
const LEFT_HALF: Range = Range { low: 0, high: 2 };
/// `rightHalf`: `lend buffer 2..4`.
const RIGHT_HALF: Range = Range { low: 2, high: 4 };

/// `ct07Steps`.
fn ct07_steps() -> Vec<Step> {
    vec![
        Step::Lease {
            holder: 1,
            range: LEFT_HALF,
        },
        Step::Lease {
            holder: 2,
            range: RIGHT_HALF,
        },
        Step::Reclaim { holder: 1 },
        Step::Reclaim { holder: 2 },
    ]
}

/// `ct07Ledger`: both leases out, the second child's at the head.
fn ct07_ledger() -> Vec<Lease> {
    vec![
        Lease {
            holder: 2,
            range: RIGHT_HALF,
        },
        Lease {
            holder: 1,
            range: LEFT_HALF,
        },
    ]
}

/// `ct07Writes`: `(writer, owner-side byte)`.
fn ct07_writes() -> Vec<(u64, u64)> {
    vec![(1, 0), (2, 2)]
}

/* ------------------------------------------------------------ the artifact */

/// One byte write of the reference run: who wrote which owner-side byte, and
/// the position of the event in the run (so the ledger state *at* the write
/// can be replayed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArtifactWrite {
    writer: u64,
    byte: u64,
    position: usize,
}

struct Ct07Artifact {
    /// The lease captures read out of the checked task IR's entry function:
    /// every `ScalarOperand::Lease` argument of every `TrySpawn`, in block
    /// order, with the worker it is handed to.
    ir_leases: Vec<(String, Range)>,
    /// The reference run's lease opens and closes, in the Lean `Step`
    /// vocabulary, in the order the run performed them.
    steps: Vec<Step>,
    /// The position in the event list of each step, parallel to `steps`.
    step_positions: Vec<usize>,
    writes: Vec<ArtifactWrite>,
    observation: TaskExecutionObservation,
}

fn compiled(name: &str, source: &str) -> Result<CompiledModule, Vec<(String, String)>> {
    compile_files(&[SourceFile::new(name, source)], &CompileOptions::default()).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| (error.code.to_string(), error.message))
            .collect()
    })
}

/// The fixture compiled through the real pipeline, its checked task module's
/// lease captures read from the IR, its reference execution run, and the
/// lease events transcribed into steps.
fn artifact_of(name: &str, source: &str) -> Ct07Artifact {
    let module = compiled(name, source).unwrap_or_else(|errors| {
        panic!("the checker refused the program before the binding could read it: {errors:?}")
    });
    let checked = module
        .checked
        .as_scoped_tasks()
        .expect("CT07 lowers to a checked task module");
    let ir = checked.module();
    let entry = &ir.functions[&ir.entry];
    let mut ir_leases = Vec::new();
    for block in entry.blocks.values() {
        if let TaskTerminatorKind::TrySpawn {
            worker, arguments, ..
        } = &block.terminator.kind
        {
            for argument in arguments {
                if let ScalarOperand::Lease { low, high, .. } = argument {
                    ir_leases.push((
                        ir.functions[worker].name.clone(),
                        Range {
                            low: *low,
                            high: *high,
                        },
                    ));
                }
            }
        }
    }
    let observation = execute_task_module(checked, &TaskExecutionScript::default())
        .expect("CT07's checked module has a reference execution");
    let mut steps = Vec::new();
    let mut step_positions = Vec::new();
    let mut writes = Vec::new();
    let mut buffers = Vec::new();
    for (position, event) in observation.events.iter().enumerate() {
        match event {
            TaskEvent::BufferLeased {
                borrower,
                buffer,
                low,
                high,
            } => {
                buffers.push(*buffer);
                steps.push(Step::Lease {
                    holder: *borrower,
                    range: Range {
                        low: *low,
                        high: *high,
                    },
                });
                step_positions.push(position);
            }
            TaskEvent::BufferLeaseReturned { borrower, buffer } => {
                buffers.push(*buffer);
                steps.push(Step::Reclaim { holder: *borrower });
                step_positions.push(position);
            }
            TaskEvent::BufferByteWritten {
                task,
                buffer,
                index,
                ..
            } => {
                buffers.push(*buffer);
                writes.push(ArtifactWrite {
                    writer: *task,
                    byte: *index,
                    position,
                });
            }
            _ => {}
        }
    }
    buffers.dedup();
    assert!(
        buffers.len() <= 1,
        "the ledger model is per resource, and CT07 leases one buffer: {buffers:?}"
    );
    Ct07Artifact {
        ir_leases,
        steps,
        step_positions,
        writes,
        observation,
    }
}

/// The ledger state the replay has reached just before event `position`.
fn state_before(artifact: &Ct07Artifact, position: usize) -> Vec<Lease> {
    let applied = artifact
        .step_positions
        .iter()
        .take_while(|&&at| at < position)
        .count();
    replay(&artifact.steps[..applied]).expect("a prefix of an admitted replay is admitted")
}

/// The production ledger driven through the same steps, its lease set read
/// back after each one as `(holder, range)` pairs — the state the Lean
/// `Reachable` term is a term *of*.
fn production_ledger_states(steps: &[Step]) -> Vec<Vec<Lease>> {
    const OWNER: u64 = 0;
    const SCOPE: u64 = 10;
    const BUFFER: u64 = 7;
    let mut ledger = ScopedTaskLedger::default();
    ledger.enter_scope(OWNER, SCOPE).unwrap();
    ledger.acquire(OWNER, BUFFER).unwrap();
    let mut states = Vec::new();
    for step in steps {
        match *step {
            Step::Lease { holder, range } => {
                ledger
                    .reserve_with_transfers(
                        OWNER,
                        SCOPE,
                        20 + holder,
                        &[],
                        &[],
                        &[(
                            BUFFER,
                            LeaseRange {
                                low: range.low,
                                high: range.high,
                            },
                        )],
                    )
                    .unwrap_or_else(|error| {
                        panic!("the production ledger refused {step:?}: {error:?}")
                    });
                ledger
                    .commit(OWNER, 20 + holder, holder, 30 + holder)
                    .unwrap();
            }
            Step::Reclaim { holder } => {
                ledger.complete(holder, TaskOutcome::I32(1), &[]).unwrap();
                ledger.join(OWNER, 30 + holder).unwrap();
            }
        }
        let mut state: Vec<Lease> = ledger
            .lease_holders(BUFFER)
            .into_iter()
            .map(|holder| {
                let range = ledger
                    .held_lease(holder, BUFFER)
                    .expect("a committed holder holds a range");
                Lease {
                    holder,
                    range: Range {
                        low: range.low,
                        high: range.high,
                    },
                }
            })
            .collect();
        state.sort();
        states.push(state);
    }
    states
}

fn sorted(leases: &[Lease]) -> Vec<Lease> {
    let mut sorted = leases.to_vec();
    sorted.sort();
    sorted
}

/* ---------------------------------------------------------------- the leg */

/// The proposed `proof`-stage executor: CT07's own artifact reaches exactly
/// the ledger state the Lean `ct07Ledger` term names, by exactly the steps
/// `ct07Steps` transcribes; the transcribed `replay` admits them, which by
/// `replay_reaches` is a `Reachable` witness — the hypothesis of
/// `no_byte_has_two_write_authorities`, instantiated on this state as
/// `ct07_no_byte_has_two_write_authorities`; the Lean-proved digest is
/// recomputed from the artifact; the scenario shape holds; and the production
/// ledger reaches the same lease set step for step.
#[test]
fn ct07_artifact_is_the_lean_proved_disjoint_halves_shape() {
    let artifact = artifact_of("case.qr", CT07_SOURCE);

    // The lease captures the checked IR carries are the two halves, each
    // handed to the writer, and they are the ranges the reference run opens.
    let mut ir_ranges: Vec<Range> = artifact.ir_leases.iter().map(|(_, r)| *r).collect();
    ir_ranges.sort();
    assert_eq!(ir_ranges, [LEFT_HALF, RIGHT_HALF]);
    assert!(
        artifact
            .ir_leases
            .iter()
            .all(|(worker, _)| worker == "writer"),
        "{:?}",
        artifact.ir_leases
    );
    let mut opened: Vec<Range> = artifact
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Lease { range, .. } => Some(*range),
            Step::Reclaim { .. } => None,
        })
        .collect();
    opened.sort();
    assert_eq!(opened, ir_ranges, "the run opens exactly the IR's leases");

    // 1. The run's lease steps are exactly the Lean `ct07Steps` term, holders
    //    being the children's own identities, in the order performed.
    assert_eq!(artifact.steps, ct07_steps());

    // 2. The transcribed replay admits them: the two-thread state is
    //    `ct07Ledger` (Lean `ct07_both_leases_are_admitted`), and the whole
    //    run returns every lease (`ct07_the_run_returns_every_lease`). A
    //    `Some` here is, by `replay_reaches`, the `Reachable` witness.
    let two_threads = replay(&artifact.steps[..2]).expect("both opens are admitted");
    assert_eq!(two_threads, ct07_ledger());
    assert_eq!(replay(&artifact.steps), Some(Vec::new()));

    // 3. The Lean-proved transcription pin, recomputed from the artifact.
    assert_eq!(mix_ledger(&two_threads), CT07_LEAN_LEDGER_DIGEST);

    // 4. The scenario shape, and the theorem's bound met with equality on
    //    every byte of the payload.
    assert!(discharges_disjoint_halves(&two_threads));
    for byte in 0..PAYLOAD_BYTES {
        assert_eq!(write_authorities(&two_threads, byte), 1, "byte {byte}");
    }

    // 5. The production ledger, driven through the same steps, reaches the
    //    same lease set after every step — the `Reachable` term is a term of
    //    the ledger's state — and its overlap decision (`LeaseRange::overlaps`
    //    → `ownership::axes_overlap`) agrees with the transcribed `overlaps`
    //    on the artifact's ranges, both orders.
    let production = production_ledger_states(&artifact.steps);
    for (index, state) in production.iter().enumerate() {
        let transcribed = replay(&artifact.steps[..=index]).expect("admitted prefix");
        assert_eq!(*state, sorted(&transcribed), "after step {index}");
    }
    for a in &ir_ranges {
        for b in &ir_ranges {
            let production_a = LeaseRange {
                low: a.low,
                high: a.high,
            };
            let production_b = LeaseRange {
                low: b.low,
                high: b.high,
            };
            assert_eq!(production_a.overlaps(production_b), overlaps(*a, *b));
        }
    }

    // The second theorem, on the same state: the artifact's two byte writes
    // are the Lean `ct07Writes`, each admitted against the state the replay
    // has reached at the write (and against the two-thread state,
    // `ct07_writes_are_admitted`), so `ct07_no_byte_is_written_by_two_parties`
    // covers them: two admitted writes to one byte would be one writer's.
    let writes: Vec<(u64, u64)> = artifact.writes.iter().map(|w| (w.writer, w.byte)).collect();
    assert_eq!(writes, ct07_writes());
    for write in &artifact.writes {
        let at = state_before(&artifact, write.position);
        assert!(
            admits_write(&at, write.writer, write.byte),
            "{write:?} against {at:?}"
        );
        assert!(admits_write(&two_threads, write.writer, write.byte));
    }
    for a in &artifact.writes {
        for b in &artifact.writes {
            if a.byte == b.byte {
                assert_eq!(a.writer, b.writer);
            }
        }
    }

    // The premise this leg rests on, re-measured rather than assumed: the F02
    // derivation refuses this very run (constraint 15n as amended), which is
    // why the binding goes through the ledger and not the trace.
    assert!(
        AdmittedCausalOrder::derive(&artifact.observation).is_err(),
        "CT07-PROOF-LEG-PATTERN-UNAVAILABLE: reference_trace no longer refuses the lease \
         run, so this leg's premise has moved — re-examine the binding route"
    );
}

/// The transcription agrees on the refusing side too: the three mutations the
/// Lean module proves decided are decided here, each at the clause the
/// theorem names — the same total `replay`, fed the mutant's would-be steps
/// directly, with no checker in between.
#[test]
fn the_procedure_decides_the_lean_named_mutations_for_the_theorems_reason() {
    // `lend buffer 0..3` beside `lend buffer 2..4`: refused at the
    // disjointness clause (`overlapping_halves_are_refused`,
    // `the_overlap_is_the_disjointness_clause`).
    let wide = Range { low: 0, high: 3 };
    let overlapping = [
        Step::Lease {
            holder: 1,
            range: wide,
        },
        Step::Lease {
            holder: 2,
            range: RIGHT_HALF,
        },
    ];
    assert_eq!(replay(&overlapping), None);
    let after_first = replay(&overlapping[..1]).unwrap();
    assert!(RIGHT_HALF.low < RIGHT_HALF.high && after_first.len() < MAX_WRITERS);
    assert!(overlaps(wide, RIGHT_HALF), "byte 2 is shared");
    assert!(!admits_lease(&after_first, RIGHT_HALF));

    // `lend buffer 2..2`: refused at the nonemptiness clause
    // (`an_empty_half_is_refused`, `the_empty_half_is_the_nonemptiness_clause`).
    let empty = Range { low: 2, high: 2 };
    let with_empty = [
        Step::Lease {
            holder: 1,
            range: LEFT_HALF,
        },
        Step::Lease {
            holder: 2,
            range: empty,
        },
    ];
    assert_eq!(replay(&with_empty), None);
    let after_first = replay(&with_empty[..1]).unwrap();
    assert!(!overlaps(LEFT_HALF, empty) && after_first.len() < MAX_WRITERS);
    assert!(!(empty.low < empty.high));
    assert!(!admits_lease(&after_first, empty));

    // One writer: a reachable state the theorem holds of, that is not the
    // scenario (`a_single_writer_is_reachable_but_not_the_scenario`,
    // `a_single_writer_leaves_the_other_half_unheld`).
    let single = [Step::Lease {
        holder: 1,
        range: LEFT_HALF,
    }];
    let state = replay(&single).expect("one lease is admitted");
    assert_eq!(
        state,
        [Lease {
            holder: 1,
            range: LEFT_HALF
        }]
    );
    assert!(!discharges_disjoint_halves(&state));
    assert_eq!(write_authorities(&state, 2), 0);
    for byte in 0..PAYLOAD_BYTES {
        assert!(write_authorities(&state, byte) <= 1, "the theorem's bound");
    }
}

/* ------------------------- where each fixture mutation is caught, measured */

/// A mutant of CT07's fixture, built from the fixture itself so it cannot
/// drift from it: `old` must occur exactly once.
fn mutated(old: &str, new: &str) -> String {
    let source = CT07_SOURCE.replace("\r\n", "\n");
    assert_eq!(
        source.matches(old).count(),
        1,
        "the mutation site must occur exactly once in CT07's fixture: {old:?}"
    );
    source.replacen(old, new, 1)
}

/// Overlapping halves `[0, 3)`/`[2, 4)`: caught by the **checker** — the
/// ledger's own `OverlappingLeases` at the second spawn site, before any
/// artifact exists for the binding to read — so the binding never runs on it.
/// The procedure, fed the would-be steps directly, refuses it too, at the
/// disjointness clause: the two catch points are separated, and both are the
/// same decision (`ownership::axes_overlap` in the ledger, `overlaps` in the
/// model).
#[test]
fn an_overlapping_mutant_is_caught_by_the_checker_and_refused_by_the_procedure_at_disjointness() {
    let source = mutated("lend buffer 0..2", "lend buffer 0..3");
    let Err(errors) = compiled("overlapping.qcore", &source) else {
        panic!("the checker refuses overlapping leases before anything is emitted");
    };
    assert!(
        errors
            .iter()
            .any(|(code, message)| code == "Q600" && message.contains("OverlappingLeases")),
        "{errors:?}"
    );
    let steps = [
        Step::Lease {
            holder: 1,
            range: Range { low: 0, high: 3 },
        },
        Step::Lease {
            holder: 2,
            range: RIGHT_HALF,
        },
    ];
    assert_eq!(replay(&steps), None);
    assert!(overlaps(Range { low: 0, high: 3 }, RIGHT_HALF));
}

/// An empty half `[2, 2)`: caught by the **checker** — the task checker's own
/// "must cover at least one byte" at the capture, before the ledger is even
/// asked — so again the binding never runs on it. The procedure refuses the
/// would-be step at the nonemptiness clause (`an_empty_range_is_not_a_lease`):
/// the model's answer is the same refusal, one layer down.
#[test]
fn an_empty_lease_mutant_is_caught_by_the_checker_and_refused_by_the_procedure_at_nonemptiness() {
    let source = mutated("lend buffer 2..4", "lend buffer 2..2");
    let Err(errors) = compiled("empty.qcore", &source) else {
        panic!("the checker refuses an empty lease");
    };
    assert!(
        errors
            .iter()
            .any(|(code, message)| code == "Q600"
                && message.contains("must cover at least one byte")),
        "{errors:?}"
    );
    let steps = [
        Step::Lease {
            holder: 1,
            range: LEFT_HALF,
        },
        Step::Lease {
            holder: 2,
            range: Range { low: 2, high: 2 },
        },
    ];
    assert_eq!(replay(&steps), None);
    assert!(!admits_lease(
        &replay(&steps[..1]).unwrap(),
        Range { low: 2, high: 2 }
    ));
}

/// A single writer: **admitted by the checker** — it is a perfectly good
/// program, one lease, reachable — so this is the mutation the **binding**
/// must catch, and does, for the theorem's own reason: the state is
/// reachable (the theorem holds of it) but is not CT07's — one lease, not
/// two; bytes 2 and 3 under no authority; the digest is not the pin.
#[test]
fn a_single_writer_mutant_is_admitted_by_the_checker_and_refused_by_the_binding_for_the_shape() {
    let source = mutated(
        "        match spawn s writer(lend buffer 2..4) {
          Result::Ok { value: second } => {
            let a = join move first?;
            let b = join move second?;
            let byte0 = qcore_buffer_read(buffer, 0);
            let byte2 = qcore_buffer_read(buffer, 2);
            drop buffer;
            return byte2;
          }
          Result::Err { error: refused } => {
            let a = join move first?;
            drop buffer;
            return a;
          }
        }
",
        "        let a = join move first?;
        let byte0 = qcore_buffer_read(buffer, 0);
        drop buffer;
        return byte0;
",
    );
    // The checker admits it and the artifact exists.
    let artifact = artifact_of("single.qcore", &source);
    assert_eq!(
        artifact.steps,
        [
            Step::Lease {
                holder: 1,
                range: LEFT_HALF
            },
            Step::Reclaim { holder: 1 }
        ]
    );
    // Reachable: the replay admits it, so the theorem holds of this state.
    let state = replay(&artifact.steps[..1]).expect("one lease is admitted");
    for byte in 0..PAYLOAD_BYTES {
        assert!(write_authorities(&state, byte) <= 1);
    }
    // Not CT07: the binding refuses it at the shape, and the pin.
    assert_ne!(artifact.steps, ct07_steps());
    assert_ne!(state, ct07_ledger());
    assert!(!discharges_disjoint_halves(&state));
    assert_eq!(write_authorities(&state, 2), 0);
    assert_eq!(write_authorities(&state, 3), 0);
    assert_ne!(mix_ledger(&state), CT07_LEAN_LEDGER_DIGEST);
    assert_eq!(artifact.writes.len(), 1);
}
