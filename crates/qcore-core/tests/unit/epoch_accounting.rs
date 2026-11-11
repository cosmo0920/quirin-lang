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

//! `FlatEpochBoundV1` — card T08.
//!
//! Positive, negative and mutation coverage for the three accounting rules: the
//! shared entry union is counted once, overlapping private peaks are summed,
//! and a completed child's result is live until an actual transfer or release.
//! Credits never exceed the allocation, and no work total becomes a wall clock.

use super::*;

const MEMORY: &str = "memory";
const CPU: &str = "cpu";

fn quantity(axes: &[(&str, u64)]) -> Quantity {
    Quantity::from_axes(
        axes.iter()
            .map(|(axis, amount)| ((*axis).to_owned(), *amount)),
    )
    .expect("test quantity")
}

fn memory(amount: u64) -> Quantity {
    Quantity::single(MEMORY, amount)
}

fn cell(cell: SharedCellId, footprint: u64) -> SharedStorageCell {
    SharedStorageCell {
        cell,
        footprint: memory(footprint),
    }
}

/// One task that reads `shared`, peaks at `peak` memory, performs `work` CPU
/// and returns nothing.
fn worker(task: EpochTaskId, shared: &[SharedCellId], peak: u64, work: u64) -> EpochTask {
    EpochTask {
        task,
        shared_entry: shared.to_vec(),
        private_peak: memory(peak),
        work: Quantity::single(CPU, work),
        result: TaskResult::ReleasedInWorker,
        credit: quantity(&[(MEMORY, peak + 3), (CPU, work)]),
        joined: true,
    }
}

fn retaining(mut task: EpochTask, footprint: u64, disposal: ResultDisposal) -> EpochTask {
    task.result = TaskResult::Retained {
        footprint: memory(footprint),
        disposal,
    };
    task
}

/// Three independent tasks over two shared cells. Cell 1 is read by all three;
/// cell 2 by one. Per-task overhead 2, per-epoch overhead 5.
fn epoch(tasks: Vec<EpochTask>) -> FlatEpochProgram {
    FlatEpochProgram {
        declared_task_count: tasks.len() as u64,
        shared_storage: vec![cell(1, 100), cell(2, 40)],
        overhead: RuntimeOverhead {
            per_task: memory(2),
            per_epoch: memory(5),
            basis: OverheadBasis::Declared,
        },
        allocation: quantity(&[(MEMORY, 1_000), (CPU, 1_000)]),
        tasks,
        unsupported: Vec::new(),
    }
}

fn admitted() -> FlatEpochProgram {
    epoch(vec![
        worker(1, &[1], 10, 7),
        worker(2, &[1, 2], 20, 11),
        worker(3, &[1], 30, 13),
    ])
}

fn checked(program: &FlatEpochProgram) -> EpochBoundCertificate {
    check_flat_epoch(program).expect("admitted epoch")
}

fn refusals(program: &FlatEpochProgram) -> Vec<EpochDiagnostic> {
    check_flat_epoch(program).expect_err("refused epoch")
}

fn codes(diagnostics: &[EpochDiagnostic]) -> Vec<&'static str> {
    diagnostics.iter().map(EpochDiagnostic::code).collect()
}

/// Every assignment of a phase to every task: `4^n` configurations.
fn every_configuration(tasks: usize) -> Vec<Vec<TaskPhase>> {
    let phases = [
        TaskPhase::Ready,
        TaskPhase::Running,
        TaskPhase::Completed,
        TaskPhase::Joined,
    ];
    let mut configurations = vec![Vec::new()];
    for _ in 0..tasks {
        configurations = configurations
            .into_iter()
            .flat_map(|prefix| {
                phases.iter().map(move |phase| {
                    let mut next = prefix.clone();
                    next.push(*phase);
                    next
                })
            })
            .collect();
    }
    configurations
}

/// Acceptance 1 and the profile's soundness claim in one place: the derived
/// bound is exact for the declared shape, no admitted configuration exceeds it,
/// and the all-running configuration attains it — which is why the overlapping
/// peaks had to be summed.
#[test]
fn the_epoch_bound_covers_every_configuration_and_the_all_running_one_attains_it() {
    let program = admitted();
    let certificate = checked(&program);

    assert_eq!(certificate.profile, ConcurrencyProfile::FlatEpochBoundV1);
    assert_eq!(certificate.task_count, 3);
    // Cell 1 is read by three tasks and counted once; cell 2 once.
    assert_eq!(certificate.shared_entry_union, memory(140));
    assert_eq!(certificate.concurrent_private_peak, memory(60));
    // 5 + 3 * 2.
    assert_eq!(certificate.runtime_overhead, memory(11));
    assert_eq!(certificate.epoch_peak, memory(211));
    // No task retains a result, so completion holds only shared plus overhead.
    assert_eq!(certificate.completed_residency, memory(151));
    assert_eq!(certificate.exit_residency, memory(145));
    assert_eq!(certificate.work_total, Quantity::single(CPU, 31));
    assert_eq!(
        certificate.granted_credits,
        quantity(&[(MEMORY, 69), (CPU, 31)])
    );
    assert_eq!(
        certificate.assumption_names(),
        [
            "DeclaredRuntimeOverhead",
            "DeclaredTaskFootprints",
            "DisjointSharedStorageCells",
            "NoScheduleModel"
        ]
        .into_iter()
        .collect()
    );

    let configurations = every_configuration(program.tasks.len());
    assert_eq!(configurations.len(), 64);
    for phases in &configurations {
        let residency = epoch_residency(&program, phases).expect("residency");
        assert!(
            residency.componentwise_le(&certificate.epoch_peak),
            "configuration {phases:?} exceeds the bound: {residency:?}"
        );
    }
    assert_eq!(
        epoch_residency(&program, &[TaskPhase::Running; 3]).unwrap(),
        certificate.epoch_peak
    );
    assert_eq!(
        epoch_residency(&program, &[TaskPhase::Completed; 3]).unwrap(),
        certificate.completed_residency
    );
    assert_eq!(
        epoch_residency(&program, &[TaskPhase::Joined; 3]).unwrap(),
        certificate.exit_residency
    );
    // Before anything starts, only shared storage and the epoch's own overhead
    // are live: per-task overhead is not paid for a task that does not exist.
    assert_eq!(
        epoch_residency(&program, &[TaskPhase::Ready; 3]).unwrap(),
        memory(145)
    );
    assert_eq!(
        epoch_residency(&program, &[TaskPhase::Ready; 2]).unwrap_err(),
        EpochDiagnostic::PhaseVectorMismatch {
            expected: 3,
            supplied: 2
        }
    );

    recheck_epoch_bound(&program, &certificate).expect("independent recheck");
}

/// Acceptance 1, first clause. Three tasks reading one cell allocate it once.
/// Counting it per task is a different number and is named as such.
#[test]
fn the_shared_entry_allocation_union_is_counted_once_not_once_per_task() {
    let program = admitted();
    let certificate = checked(&program);
    assert_eq!(certificate.shared_entry_union, memory(140));

    // Adding a fourth task that reads only already-referenced cells does not
    // grow the union, only the private and overhead terms.
    let mut wider = program.clone();
    wider.tasks.push(worker(4, &[1, 2], 0, 0));
    wider.declared_task_count = 4;
    let widened = checked(&wider);
    assert_eq!(widened.shared_entry_union, certificate.shared_entry_union);
    assert_eq!(widened.runtime_overhead, memory(13));

    // A cell no task reads is not part of the epoch's entry allocation.
    let mut unread = program.clone();
    unread.shared_storage.push(cell(3, 500));
    assert_eq!(
        checked(&unread).shared_entry_union,
        certificate.shared_entry_union
    );

    // The double count: 100 + 140 + 100 = 340 instead of 140.
    let mut forged = certificate.clone();
    forged.shared_entry_union = memory(340);
    forged.epoch_peak = memory(411);
    forged.completed_residency = memory(351);
    forged.exit_residency = memory(345);
    let errors = recheck_epoch_bound(&program, &forged).expect_err("double count");
    assert!(
        errors.contains(&EpochRecheckError::SharedUnionDoubleCounted {
            summed: memory(340),
            union: memory(140),
        })
    );
    assert!(errors.contains(&EpochRecheckError::PeakOvercounted {
        axis: MEMORY.to_owned(),
        claimed: 411,
        derived: 211,
    }));

    // One task naming the same cell twice is refused outright, not deduplicated.
    let mut repeated = program.clone();
    repeated.tasks[0].shared_entry = vec![1, 1];
    assert_eq!(
        refusals(&repeated),
        [EpochDiagnostic::DuplicateSharedEntry { task: 1, cell: 1 }]
    );
    assert_eq!(codes(&refusals(&repeated)), ["Q630"]);
}

/// Acceptance 1, second clause. Independent tasks overlap, so the componentwise
/// maximum is an undercount that a real configuration exceeds.
#[test]
fn overlapping_incremental_peaks_are_summed_and_a_maximum_is_refused() {
    let program = admitted();
    let certificate = checked(&program);
    assert_eq!(certificate.concurrent_private_peak, memory(60));

    let mut maxed = certificate.clone();
    maxed.concurrent_private_peak = memory(30);
    maxed.epoch_peak = memory(181);
    let errors = recheck_epoch_bound(&program, &maxed).expect_err("maximized peaks");
    assert!(
        errors.contains(&EpochRecheckError::OverlappingPeaksMaxedNotSummed {
            maximum: memory(30),
            sum: memory(60),
        })
    );
    assert!(errors.contains(&EpochRecheckError::PeakUndercounted {
        axis: MEMORY.to_owned(),
        claimed: 181,
        derived: 211,
    }));

    // The undercount is not academic: the all-running configuration exceeds it.
    let residency = epoch_residency(&program, &[TaskPhase::Running; 3]).unwrap();
    assert!(!residency.componentwise_le(&maxed.epoch_peak));
    assert_eq!(residency, memory(211));
}

/// Acceptance 1, third clause. Overhead scales with the fixed task count, is a
/// recorded premise, and a native measurement may not stand in for it.
#[test]
fn declared_runtime_overhead_scales_with_the_task_count_and_is_never_a_measurement() {
    let program = admitted();
    assert_eq!(checked(&program).runtime_overhead, memory(11));

    let mut single = epoch(vec![worker(1, &[1], 10, 7)]);
    single.declared_task_count = 1;
    assert_eq!(checked(&single).runtime_overhead, memory(7));

    let mut measured = program.clone();
    measured.overhead.basis = OverheadBasis::MeasuredNativeRun { runs: 6 };
    assert_eq!(
        refusals(&measured),
        [EpochDiagnostic::MeasuredOverheadIsNotADeclaredBound { runs: 6 }]
    );
    assert_eq!(codes(&refusals(&measured)), ["Q640"]);

    let mut forged = checked(&program);
    forged.runtime_overhead = memory(5);
    assert!(
        recheck_epoch_bound(&program, &forged)
            .expect_err("overhead mutation")
            .contains(&EpochRecheckError::FieldMismatch {
                field: "runtime_overhead"
            })
    );

    // The premise cannot be dropped from the certificate.
    let mut silent = checked(&program);
    silent
        .assumptions
        .remove(&EpochAssumption::DeclaredRuntimeOverhead);
    let errors = recheck_epoch_bound(&program, &silent).expect_err("dropped premise");
    assert!(
        errors.contains(&EpochRecheckError::DischargedPremiseNotEstablished {
            assumption: EpochAssumption::DeclaredRuntimeOverhead
        })
    );
}

/// Acceptance 2. A completed child's result is live from completion until the
/// creator's join actually transfers or releases it.
#[test]
fn a_completed_child_result_stays_live_until_transfer_not_at_completion() {
    let program = epoch(vec![
        retaining(
            worker(1, &[1], 10, 7),
            8,
            ResultDisposal::TransferredToCreator,
        ),
        retaining(
            worker(2, &[1, 2], 20, 11),
            6,
            ResultDisposal::ReleasedByCreator,
        ),
        worker(3, &[1], 30, 13),
    ]);
    let certificate = checked(&program);

    // Completed but unjoined: both results and all three overheads are live.
    assert_eq!(certificate.completed_residency, memory(165));
    assert_eq!(
        epoch_residency(&program, &[TaskPhase::Completed; 3]).unwrap(),
        certificate.completed_residency
    );
    // The transferred result survives the join; the released one does not.
    assert_eq!(certificate.exit_residency, memory(153));
    assert_eq!(
        epoch_residency(&program, &[TaskPhase::Joined; 3]).unwrap(),
        certificate.exit_residency
    );
    // One child completed while its siblings still run: the completed child's
    // result and overhead are counted, not reclaimed.
    assert_eq!(
        epoch_residency(
            &program,
            &[TaskPhase::Completed, TaskPhase::Running, TaskPhase::Running]
        )
        .unwrap(),
        memory(209)
    );

    // A certificate that reclaims the results at completion is named.
    let mut discarded = certificate.clone();
    discarded.completed_residency = memory(151);
    let errors = recheck_epoch_bound(&program, &discarded).expect_err("discarded results");
    assert_eq!(
        errors,
        [EpochRecheckError::ResultsDroppedFromResidency {
            claimed: memory(151),
            derived: memory(165),
        }]
    );
}

/// Acceptance 2, negative side. A plan whose result stops being live at
/// completion, and one whose result is never joined, are both refused.
#[test]
fn a_result_that_ends_at_completion_or_is_never_transferred_is_refused() {
    let dropped = epoch(vec![
        retaining(
            worker(1, &[1], 10, 7),
            8,
            ResultDisposal::DroppedAtCompletion,
        ),
        worker(2, &[1], 20, 11),
    ]);
    assert_eq!(
        refusals(&dropped),
        [EpochDiagnostic::ResultDiscardedAtCompletion { task: 1 }]
    );
    assert_eq!(codes(&refusals(&dropped)), ["Q634"]);

    let mut unjoined = epoch(vec![
        retaining(
            worker(1, &[1], 10, 7),
            8,
            ResultDisposal::TransferredToCreator,
        ),
        worker(2, &[1], 20, 11),
    ]);
    unjoined.tasks[0].joined = false;
    unjoined.tasks[1].joined = false;
    assert_eq!(
        refusals(&unjoined),
        [
            EpochDiagnostic::UntransferredResultAtEpochExit { task: 1 },
            EpochDiagnostic::UnjoinedTask { task: 2 },
        ]
    );
    assert_eq!(codes(&refusals(&unjoined)), ["Q635", "Q635"]);

    // A result larger than the peak its own task was accounted for was never
    // allocated in this accounting.
    let oversized = epoch(vec![
        retaining(
            worker(1, &[1], 10, 7),
            11,
            ResultDisposal::TransferredToCreator,
        ),
        worker(2, &[1], 20, 11),
    ]);
    assert_eq!(
        refusals(&oversized),
        [EpochDiagnostic::RetainedResultExceedsTaskPeak {
            task: 1,
            axis: MEMORY.to_owned(),
            result: 11,
            task_peak: 10,
        }]
    );
    assert_eq!(codes(&refusals(&oversized)), ["Q633"]);

    // A retained result with no footprint is not a retained result.
    let empty = epoch(vec![
        retaining(
            worker(1, &[1], 10, 7),
            0,
            ResultDisposal::TransferredToCreator,
        ),
        worker(2, &[1], 20, 11),
    ]);
    assert_eq!(
        refusals(&empty),
        [EpochDiagnostic::EmptyRetainedResult { task: 1 }]
    );
}

/// Acceptance 3, first clause. Credits partition the allocation: their sum
/// cannot exceed it, and no task may demand more than its own credit.
#[test]
fn credit_sums_cannot_exceed_the_epoch_allocation() {
    let mut program = admitted();
    program.allocation = quantity(&[(MEMORY, 68), (CPU, 1_000)]);
    let errors = refusals(&program);
    assert_eq!(
        errors,
        [
            EpochDiagnostic::CreditsExceedAllocation {
                axis: MEMORY.to_owned(),
                granted: 69,
                allocation: 68,
            },
            EpochDiagnostic::EpochExceedsAllocation {
                axis: MEMORY.to_owned(),
                required: 211,
                allocation: 68,
            },
        ]
    );
    assert_eq!(codes(&errors), ["Q637", "Q638"]);

    // A task demanding more than its own credit, with the sum still fitting.
    let mut tight = admitted();
    tight.tasks[1].credit = quantity(&[(MEMORY, 20), (CPU, 11)]);
    assert_eq!(
        refusals(&tight),
        [EpochDiagnostic::TaskExceedsItsCredit {
            task: 2,
            axis: MEMORY.to_owned(),
            demand: 22,
            credit: 20,
        }]
    );
    assert_eq!(codes(&refusals(&tight)), ["Q636"]);

    // Credits that fit are not, by themselves, an epoch that fits: shared
    // storage and per-epoch overhead belong to no task's credit.
    let mut shared_heavy = admitted();
    shared_heavy.allocation = quantity(&[(MEMORY, 200), (CPU, 1_000)]);
    assert_eq!(
        refusals(&shared_heavy),
        [EpochDiagnostic::EpochExceedsAllocation {
            axis: MEMORY.to_owned(),
            required: 211,
            allocation: 200,
        }]
    );

    // The work axis is checked against the allocation exactly like memory: a
    // work total beyond the allocation is refused on the work axis, and the
    // credits granting that work are refused with it.
    let mut cpu_bound = admitted();
    cpu_bound.allocation = quantity(&[(MEMORY, 1_000), (CPU, 30)]);
    assert_eq!(
        refusals(&cpu_bound),
        [
            EpochDiagnostic::CreditsExceedAllocation {
                axis: CPU.to_owned(),
                granted: 31,
                allocation: 30,
            },
            EpochDiagnostic::EpochExceedsAllocation {
                axis: CPU.to_owned(),
                required: 31,
                allocation: 30,
            },
        ]
    );
}

/// Acceptance 3, second clause. The work total is a work total. Nothing here
/// converts it into elapsed time, and the premise saying so cannot be dropped.
#[test]
fn work_totals_never_become_wall_clock_bounds() {
    let program = admitted();
    let certificate = checked(&program);
    assert_eq!(certificate.work_total, Quantity::single(CPU, 31));
    assert_eq!(
        epoch_wall_clock_bound(&certificate).unwrap_err(),
        EpochDiagnostic::WallClockUnavailable
    );
    assert_eq!(
        epoch_wall_clock_bound(&certificate).unwrap_err().code(),
        "Q639"
    );

    let mut timed = certificate.clone();
    timed.assumptions.remove(&EpochAssumption::NoScheduleModel);
    let errors = recheck_epoch_bound(&program, &timed).expect_err("dropped schedule premise");
    assert!(
        errors.contains(&EpochRecheckError::DischargedPremiseNotEstablished {
            assumption: EpochAssumption::NoScheduleModel
        })
    );

    // A declared latency claim is a represented refusal, not an omission.
    let mut latency = program.clone();
    latency.unsupported = vec![UnsupportedEpochShape::SchedulerLatencyClaim];
    assert_eq!(
        refusals(&latency),
        [EpochDiagnostic::UnsupportedEpochShape {
            construct: UnsupportedEpochShape::SchedulerLatencyClaim
        }]
    );

    // Two epochs with identical work totals differ in how many tasks are
    // outstanding at once: work does not determine residency either.
    let mut serial = admitted();
    serial.tasks[0].private_peak = memory(60);
    serial.tasks[0].credit = quantity(&[(MEMORY, 62), (CPU, 7)]);
    serial.tasks[1].private_peak = Quantity::zero();
    serial.tasks[2].private_peak = Quantity::zero();
    let unbalanced = checked(&serial);
    assert_eq!(unbalanced.work_total, certificate.work_total);
    assert_eq!(unbalanced.epoch_peak, certificate.epoch_peak);
    assert_ne!(
        epoch_residency(
            &serial,
            &[TaskPhase::Running, TaskPhase::Ready, TaskPhase::Ready]
        )
        .unwrap(),
        epoch_residency(
            &program,
            &[TaskPhase::Running, TaskPhase::Ready, TaskPhase::Ready]
        )
        .unwrap()
    );
}

/// Every shape outside the profile is a named refusal, and they are reported
/// together rather than one at a time.
#[test]
fn unsupported_epoch_shapes_are_represented_refusals() {
    let constructs = [
        UnsupportedEpochShape::NestedTask,
        UnsupportedEpochShape::DynamicTaskCount,
        UnsupportedEpochShape::DynamicSharedStorage,
        UnsupportedEpochShape::SharedMutation,
        UnsupportedEpochShape::InterTaskCommunication,
        UnsupportedEpochShape::RecursiveWorker,
        UnsupportedEpochShape::SchedulerLatencyClaim,
        UnsupportedEpochShape::OsStackReservation,
        UnsupportedEpochShape::CommunicatingDynamicHeap,
    ];
    for construct in constructs {
        let mut program = admitted();
        program.unsupported = vec![construct];
        assert_eq!(
            refusals(&program),
            [EpochDiagnostic::UnsupportedEpochShape { construct }]
        );
        assert_eq!(codes(&refusals(&program)), ["Q632"]);
    }

    let mut all = admitted();
    all.unsupported = constructs.to_vec();
    assert_eq!(refusals(&all).len(), constructs.len());
}

/// Structural refusals precede accounting, so a malformed epoch never receives
/// a bound at all.
#[test]
fn a_malformed_epoch_shape_is_refused_before_any_accounting() {
    let mut empty = admitted();
    empty.tasks.clear();
    empty.declared_task_count = 0;
    assert_eq!(refusals(&empty), [EpochDiagnostic::EmptyEpoch]);

    let mut miscounted = admitted();
    miscounted.declared_task_count = 4;
    assert_eq!(
        refusals(&miscounted),
        [EpochDiagnostic::TaskCountMismatch {
            declared: 4,
            actual: 3
        }]
    );

    let mut duplicated = admitted();
    duplicated.tasks[2].task = 1;
    assert_eq!(
        refusals(&duplicated),
        [EpochDiagnostic::DuplicateTask { task: 1 }]
    );

    let mut twice = admitted();
    twice.shared_storage.push(cell(1, 100));
    assert_eq!(
        refusals(&twice),
        [EpochDiagnostic::DuplicateSharedCell { cell: 1 }]
    );

    let mut unknown = admitted();
    unknown.tasks[0].shared_entry = vec![9];
    assert_eq!(
        refusals(&unknown),
        [EpochDiagnostic::UnknownSharedCell { task: 1, cell: 9 }]
    );
    assert_eq!(codes(&refusals(&unknown)), ["Q631"]);

    // A structural refusal suppresses the accounting refusals of the same
    // program: it is refused, never partially bounded.
    let mut both = admitted();
    both.declared_task_count = 9;
    both.allocation = Quantity::zero();
    assert_eq!(
        refusals(&both),
        [EpochDiagnostic::TaskCountMismatch {
            declared: 9,
            actual: 3
        }]
    );
}

/// Overflow is refused, never wrapped or saturated.
#[test]
fn quantitative_overflow_is_refused() {
    let mut program = admitted();
    program.tasks[0].private_peak = memory(u64::MAX);
    program.tasks[0].credit = memory(u64::MAX);
    let errors = refusals(&program);
    assert!(errors.contains(&EpochDiagnostic::Arithmetic {
        detail: FlowError::Overflow
    }));
    assert_eq!(
        EpochDiagnostic::Arithmetic {
            detail: FlowError::Overflow
        }
        .code(),
        "Q641"
    );

    let mut overhead = admitted();
    overhead.overhead.per_task = memory(u64::MAX);
    assert!(refusals(&overhead).contains(&EpochDiagnostic::Arithmetic {
        detail: FlowError::Overflow
    }));
}

/// One field-level forgery of a supplied certificate.
type CertificateMutation = Box<dyn Fn(&mut EpochBoundCertificate)>;

/// The recheck rederives every field: no single mutation survives it.
#[test]
fn the_recheck_rederives_every_certificate_field() {
    let program = admitted();
    let certificate = checked(&program);
    recheck_epoch_bound(&program, &certificate).expect("unmutated certificate");

    let mutations: Vec<(&'static str, CertificateMutation)> = vec![
        (
            "profile",
            Box::new(|c: &mut EpochBoundCertificate| {
                c.profile = ConcurrencyProfile::TrustedExplicitTrace;
            }),
        ),
        (
            "task_count",
            Box::new(|c: &mut EpochBoundCertificate| c.task_count = 2),
        ),
        (
            "shared_entry_union",
            Box::new(|c: &mut EpochBoundCertificate| c.shared_entry_union = memory(139)),
        ),
        (
            "concurrent_private_peak",
            Box::new(|c: &mut EpochBoundCertificate| c.concurrent_private_peak = memory(59)),
        ),
        (
            "runtime_overhead",
            Box::new(|c: &mut EpochBoundCertificate| c.runtime_overhead = memory(10)),
        ),
        (
            "exit_residency",
            Box::new(|c: &mut EpochBoundCertificate| c.exit_residency = memory(1)),
        ),
        (
            "work_total",
            Box::new(|c: &mut EpochBoundCertificate| c.work_total = Quantity::single(CPU, 30)),
        ),
        (
            "granted_credits",
            Box::new(|c: &mut EpochBoundCertificate| c.granted_credits = Quantity::zero()),
        ),
        (
            "allocation",
            Box::new(|c: &mut EpochBoundCertificate| c.allocation = memory(1)),
        ),
    ];
    for (field, mutate) in mutations {
        let mut forged = certificate.clone();
        mutate(&mut forged);
        let errors = recheck_epoch_bound(&program, &forged).expect_err(field);
        assert!(
            errors.contains(&EpochRecheckError::FieldMismatch { field })
                || matches!(field, "shared_entry_union" | "concurrent_private_peak"),
            "{field} survived the recheck: {errors:?}"
        );
    }

    // The two accounting fields have sharper readings when the mutation is a
    // near miss rather than one of the two named accounting mistakes.
    let mut near = certificate.clone();
    near.shared_entry_union = memory(139);
    assert!(
        recheck_epoch_bound(&program, &near)
            .expect_err("near miss")
            .contains(&EpochRecheckError::FieldMismatch {
                field: "shared_entry_union"
            })
    );
    let mut peak = certificate.clone();
    peak.concurrent_private_peak = memory(59);
    assert!(
        recheck_epoch_bound(&program, &peak)
            .expect_err("near miss")
            .contains(&EpochRecheckError::FieldMismatch {
                field: "concurrent_private_peak"
            })
    );

    // An extra premise is a mismatch too: the set is exactly the four.
    let mut extra = certificate.clone();
    extra.assumptions.remove(&EpochAssumption::NoScheduleModel);
    extra
        .assumptions
        .insert(EpochAssumption::DisjointSharedStorageCells);
    assert!(
        recheck_epoch_bound(&program, &extra)
            .expect_err("premise set")
            .contains(&EpochRecheckError::DischargedPremiseNotEstablished {
                assumption: EpochAssumption::NoScheduleModel
            })
    );
}

/// A certificate for a program the checker refuses is not rechecked field by
/// field: the whole program is refused first.
#[test]
fn a_refused_program_cannot_be_rechecked() {
    let program = admitted();
    let certificate = checked(&program);
    let mut refused = program.clone();
    refused.unsupported = vec![UnsupportedEpochShape::NestedTask];
    assert_eq!(
        recheck_epoch_bound(&refused, &certificate).expect_err("refused program"),
        [EpochRecheckError::ProgramRejected(vec![
            EpochDiagnostic::UnsupportedEpochShape {
                construct: UnsupportedEpochShape::NestedTask
            }
        ])]
    );
}

/// A one-task epoch with no shared storage still pays its declared overhead,
/// and an epoch whose tasks share nothing sums exactly their private peaks.
#[test]
fn the_degenerate_epochs_are_accounted_exactly() {
    let mut solo = FlatEpochProgram {
        declared_task_count: 1,
        shared_storage: Vec::new(),
        overhead: RuntimeOverhead {
            per_task: memory(3),
            per_epoch: memory(4),
            basis: OverheadBasis::Declared,
        },
        allocation: quantity(&[(MEMORY, 100), (CPU, 100)]),
        tasks: vec![worker(1, &[], 5, 2)],
        unsupported: Vec::new(),
    };
    let certificate = checked(&solo);
    assert_eq!(certificate.shared_entry_union, Quantity::zero());
    assert_eq!(certificate.epoch_peak, memory(12));
    assert_eq!(certificate.exit_residency, memory(4));
    recheck_epoch_bound(&solo, &certificate).expect("recheck");

    solo.tasks.push(worker(2, &[], 5, 2));
    solo.declared_task_count = 2;
    // Two disjoint tasks: 4 + 2*3 + 5 + 5.
    assert_eq!(checked(&solo).epoch_peak, memory(20));
}
