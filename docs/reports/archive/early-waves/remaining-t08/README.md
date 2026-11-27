# T08 — quantitative accounting for one flat concurrent epoch

Date: 2026-09-08. Baseline: `c60fd91` (second delegated increment consolidated).
Branch: `claude/cycle-19-task-delegation-f20028` (isolated worktree).
Workstream W6, lane Core, kind implementation, scope M.

**What landed:** a new dependency-free core profile, `FlatEpochBoundV1`
(`crates/qcore-core/src/epoch_accounting.rs`), that derives and independently
rechecks a quantitative residency and work bound for one flat epoch of a fixed
number of independent tasks over fixed-shape shared storage; 14 new Rust tests;
20 new Lean theorems in a new `QCore.EpochAccounting` namespace with its own
audit module. **Nothing is promoted.** No source lowering, no CT mapping, no
verdict change, no diagnostic-code change to any existing pass, no
`qcore-schemas` change, no native measurement.

## 1. The admitted epoch shape

An epoch is **one** creator that opens **one** scope, creates a **fixed** number
of **independent** tasks over **fixed-shape** shared storage and joins every one
of them. `FlatEpochProgram` declares it completely:

| Field | Meaning |
|---|---|
| `declared_task_count` | The fixed count. It must equal `tasks.len()`; the duplication exists so a plan cannot disagree with itself (`TaskCountMismatch`). |
| `shared_storage: Vec<SharedStorageCell>` | The shared entry allocation, by cell identity and footprint. |
| `overhead: RuntimeOverhead` | `per_task`, `per_epoch`, and the `basis` those figures came from. |
| `allocation: Quantity` | What the whole epoch may consume, per axis. |
| `tasks: Vec<EpochTask>` | Per task: the shared cells it reads at entry, its `private_peak`, its `work`, its `result`, its `credit`, and whether the creator's join is recorded. |
| `unsupported: Vec<UnsupportedEpochShape>` | Constructs present in this epoch that the profile does not model. Each becomes a refusal. |

Quantities are `qcore_core::resource_flow::Quantity` — the existing
multi-axis nonneg­ative vector — so "memory" and "cpu" are ordinary axes and
every comparison is componentwise. The module adds no arithmetic to
`resource_flow`; it composes `Quantity::from_axes` and `componentwise_le`.

A task's result is `ReleasedInWorker` or `Retained { footprint, disposal }`,
where `disposal` is `TransferredToCreator`, `ReleasedByCreator` or
`DroppedAtCompletion`. The third is representable precisely so it can be
refused rather than being an unrepresentable case nobody checks.

## 2. The bound, exactly

Let `N` be the task count, `cells(t)` the shared cells task `t` reads,
`peak(t)` its private peak, `retained(t)` its result footprint (zero for
`ReleasedInWorker`) and `transferred(t)` the part the creator keeps after join
(zero unless `TransferredToCreator`).

```
shared_entry_union     = Σ over DISTINCT cells referenced by any task
concurrent_private_peak = Σ_t peak(t)                     (summed, not maxed)
runtime_overhead       = per_epoch + N * per_task
epoch_peak             = shared_entry_union + concurrent_private_peak + runtime_overhead
completed_residency    = shared_entry_union + Σ_t retained(t) + runtime_overhead
exit_residency         = shared_entry_union + per_epoch + Σ_t transferred(t)
work_total             = Σ_t work(t)
granted_credits        = Σ_t credit(t)
demand(t)              = peak(t) + per_task + work(t)
```

Checked, componentwise, in this order — structural refusals first, so a
malformed epoch is never given a bound at all, not even a conservative one:

1. `retained(t) ≤ peak(t)` — a result that outlives its task must fit inside
   the peak that task was accounted for, or its allocation was never counted.
2. every task is joined, and no `disposal` is `DroppedAtCompletion`.
3. `demand(t) ≤ credit(t)` for every task.
4. `granted_credits ≤ allocation`.
5. `epoch_peak + work_total ≤ allocation`. Equivalently
   `shared_entry_union + per_epoch + Σ_t demand(t) ≤ allocation`: shared storage
   and the epoch's own overhead belong to no task's credit, so credits fitting
   is not by itself an epoch that fits, and a test pins that case.

`epoch_residency(program, phases)` computes the live quantity of one
configuration — an assignment of `Ready | Running | Completed | Joined` to every
task. A `Ready` task costs nothing, a `Running` task its private peak, a
`Completed` task its retained result, a `Joined` task its transferred result;
`per_task` overhead is live exactly while a task is started and unjoined. This
is what makes the bound falsifiable rather than definitional: the tests
enumerate **all 64** configurations of the three-task fixture and assert every
one is `≤ epoch_peak`, that all-`Running` **attains** it, that all-`Completed`
equals `completed_residency`, and that all-`Joined` equals `exit_residency`.

## 3. Acceptance, item by item

### Acceptance 1 — union once, overlapping peaks, declared overhead

*Union once.* The union is computed over distinct cell identities. Three tasks
reading cell 1 (100) plus one also reading cell 2 (40) give **140**, not 340.
Adding a fourth task that reads only already-referenced cells does not change
the union; a cell no task reads is not part of the entry allocation. One task
naming the same cell twice is refused (`DuplicateSharedEntry`, `Q630`) rather
than silently deduplicated — a repeated entry is evidence that the plan is
summing.

*Overlapping incremental peaks.* Peaks are summed (10 + 20 + 30 = 60), and the
undercount is exhibited, not argued: `epoch_residency` on the all-`Running`
configuration returns 211, which is **not** `≤` the 181 a maximized reading
claims.

*Declared runtime overhead.* `per_epoch + N * per_task` (5 + 3·2 = 11), scaling
to 7 for one task and 13 for four. The figures are a **premise**:
`EpochAssumption::DeclaredRuntimeOverhead` is on every certificate and a
certificate that drops it is refused
(`DischargedPremiseNotEstablished`). Offering a measurement instead of a
declaration is a refusal, not an upgrade:
`OverheadBasis::MeasuredNativeRun { runs }` yields
`MeasuredOverheadIsNotADeclaredBound` (`Q640`). Declared overhead and measured
native memory are kept apart by construction — there is no path in this module
by which an observation of some runs becomes a bound on every admitted epoch.

### Acceptance 2 — results live until transfer, not completion

`completed_residency` is the residency with every child finished and **none**
joined: shared union + Σ retained + **all** per-task overheads. In the retaining
fixture that is 165 against an exit residency of 153 and a ready residency of
145. `completion_reclaims_nothing` (Lean) and the residency tests (Rust) both
say the same thing from the model side, matching `scoped_tasks`'
`Custodian::Capsule` rule that a completed-but-unjoined child's resources
belong to no task.

Three refusals guard it:

| Situation | Diagnostic | Code |
|---|---|---|
| `disposal = DroppedAtCompletion` | `ResultDiscardedAtCompletion { task }` | `Q634` |
| retained result, task not joined | `UntransferredResultAtEpochExit { task }` | `Q635` |
| no result, task not joined (its overhead still leaks) | `UnjoinedTask { task }` | `Q635` |
| `retained(t)` exceeds `peak(t)` | `RetainedResultExceedsTaskPeak { task, axis, result, task_peak }` | `Q633` |

And on the certificate side, a residency that reclaims the results at
completion is named exactly: `ResultsDroppedFromResidency { claimed, derived }`.

### Acceptance 3 — credits, and work is not wall-clock

`granted_credits ≤ allocation` is checked (`CreditsExceedAllocation { axis,
granted, allocation }`, `Q637`), as is each task against its own credit
(`TaskExceedsItsCredit`, `Q636`) and the whole epoch against the allocation
(`EpochExceedsAllocation`, `Q638`). The work axis is checked exactly like the
memory axis; a work total beyond the allocation is refused on the work axis.

No work or CPU quantity becomes a duration anywhere. `epoch_wall_clock_bound`
**always** returns `WallClockUnavailable` (`Q639`);
`EpochAssumption::NoScheduleModel` is on every certificate and cannot be
dropped; and `UnsupportedEpochShape::SchedulerLatencyClaim` is a represented
refusal (`Q632`). In Lean, `work_total_does_not_determine_finish_time` and
`no_function_of_the_work_total_is_the_finish_time` prove the point rather than
asserting it: two schedules of the same work list finish at different times, so
no function of the work total is the finish time.

## 4. Refusals and their codes

This profile occupies `Q630`–`Q641` inside the concurrency pass's existing
`Q6xx` range (`docs/QCORE_SPEC_r0_3.md` §L21 assigns `Q6xx` to the
`Concurrency` pass; `Q600` and T02's `Q610`–`Q623` are untouched). No existing
code was reused, renamed or downgraded.

| Code | Refusals |
|---|---|
| `Q630` | `EmptyEpoch`, `DuplicateTask`, `DuplicateSharedCell`, `DuplicateSharedEntry`, `TaskCountMismatch`, `EmptyRetainedResult`, `PhaseVectorMismatch` |
| `Q631` | `UnknownSharedCell` |
| `Q632` | `UnsupportedEpochShape` (nine constructs) |
| `Q633` | `RetainedResultExceedsTaskPeak` |
| `Q634` | `ResultDiscardedAtCompletion` |
| `Q635` | `UntransferredResultAtEpochExit`, `UnjoinedTask` |
| `Q636` | `TaskExceedsItsCredit` |
| `Q637` | `CreditsExceedAllocation` |
| `Q638` | `EpochExceedsAllocation` |
| `Q639` | `WallClockUnavailable` |
| `Q640` | `MeasuredOverheadIsNotADeclaredBound` |
| `Q641` | `Arithmetic` (checked overflow; never wrapped or saturated) |

The nine represented refusals of `UnsupportedEpochShape` — `NestedTask`,
`DynamicTaskCount`, `DynamicSharedStorage`, `SharedMutation`,
`InterTaskCommunication`, `RecursiveWorker`, `SchedulerLatencyClaim`,
`OsStackReservation`, `CommunicatingDynamicHeap` — are each exercised
individually and together.

`recheck_epoch_bound` rederives every certificate field from the program alone
and refuses:

| Mutation | Refusal |
|---|---|
| shared entry counted once per referencing task | `SharedUnionDoubleCounted { summed, union }` |
| private peaks maximized instead of summed | `OverlappingPeaksMaxedNotSummed { maximum, sum }` |
| claimed peak below the derived one | `PeakUndercounted { axis, claimed, derived }` |
| claimed peak above the derived one | `PeakOvercounted { axis, claimed, derived }` |
| completed residency with the results removed | `ResultsDroppedFromResidency { claimed, derived }` |
| a missing premise | `DischargedPremiseNotEstablished { assumption }` |
| any other field | `FieldMismatch { field }` |
| a program the checker refuses | `ProgramRejected(diagnostics)` (no field-by-field recheck) |

An overcount is refused as well as an undercount. That is deliberate: this is an
*exact* bound for a declared shape, and admitting "conservative" inflation would
admit the double count the card asks to refuse.

## 5. Evidence

**Rust — 14 new tests**, all in
`crates/qcore-core/tests/unit/epoch_accounting.rs` (module-private, wired by
`#[path]` exactly like `resource_accounting`):

| Test | Covers |
|---|---|
| `the_epoch_bound_covers_every_configuration_and_the_all_running_one_attains_it` | exact certificate fields; all 64 configurations `≤` bound; all-running attains it; ready/completed/joined identities; phase-vector refusal; clean recheck |
| `the_shared_entry_allocation_union_is_counted_once_not_once_per_task` | acceptance 1 union; added readers and unread cells; `SharedUnionDoubleCounted` + `PeakOvercounted`; `DuplicateSharedEntry` |
| `overlapping_incremental_peaks_are_summed_and_a_maximum_is_refused` | `OverlappingPeaksMaxedNotSummed` + `PeakUndercounted`; the reachable configuration that exceeds the maximized claim |
| `declared_runtime_overhead_scales_with_the_task_count_and_is_never_a_measurement` | overhead scaling; `MeasuredOverheadIsNotADeclaredBound`; overhead field mutation; dropped premise |
| `a_completed_child_result_stays_live_until_transfer_not_at_completion` | acceptance 2 positive: completed/exit/mixed residencies; `ResultsDroppedFromResidency` |
| `a_result_that_ends_at_completion_or_is_never_transferred_is_refused` | `Q634`, both `Q635` refusals, `Q633`, `EmptyRetainedResult` |
| `credit_sums_cannot_exceed_the_epoch_allocation` | `Q637`, `Q636`, `Q638`; credits fitting while the epoch does not; the work axis |
| `work_totals_never_become_wall_clock_bounds` | `Q639`; dropped `NoScheduleModel`; `SchedulerLatencyClaim`; equal work totals with different residency |
| `unsupported_epoch_shapes_are_represented_refusals` | all nine constructs, singly and together |
| `a_malformed_epoch_shape_is_refused_before_any_accounting` | `EmptyEpoch`, `TaskCountMismatch`, `DuplicateTask`, `DuplicateSharedCell`, `UnknownSharedCell`, and that a structural refusal suppresses accounting |
| `quantitative_overflow_is_refused` | `u64` overflow in a peak and in the overhead scale |
| `the_recheck_rederives_every_certificate_field` | nine field mutations; the two near-miss readings; premise-set substitution |
| `a_refused_program_cannot_be_rechecked` | `ProgramRejected` |
| `the_degenerate_epochs_are_accounted_exactly` | one task, no shared storage; two disjoint tasks |

**Lean — 20 new theorems**, `proof/QCore/EpochAccounting.lean`, audited by
`proof/AuditEpochAccounting.lean` (24 audits total, up from 23). All 20 depend
only on `propext` and `Quot.sound`; **no new axiom, no `sorry`, no placeholder**.

*Union:* `union_counts_a_repeated_cell_once`,
`union_ignores_a_repeated_identity`, `union_is_at_most_the_double_count`,
`a_double_counted_cell_is_strictly_larger`.
*Overlap:* `sum_dominates_each_task`, `maximum_is_at_most_the_sum`,
`the_maximum_undercounts_two_concurrent_tasks`.
*Soundness of the bound:* `live_is_at_most_the_private_peak`,
`live_tasks_le_private_sum`, `outstanding_le_task_count`,
`residency_is_bounded_by_the_epoch_peak` (every configuration of a wellformed
epoch), `the_all_running_configuration_attains_the_bound`,
`the_ready_configuration_holds_only_shared_storage_and_epoch_overhead`.
*Results live until transfer:* `completion_reclaims_nothing`,
`dropping_a_completed_result_undercounts`,
`a_transferred_result_survives_the_join`.
*Credits:* `demands_within_credits_fit_the_allocation`,
`credits_beyond_the_allocation_are_not_covered`.
*Work is not time:* `work_total_does_not_determine_finish_time`,
`no_function_of_the_work_total_is_the_finish_time`.

## 6. Standing per-change gate

| Gate | Result |
|---|---|
| `git -c protocol.file.allow=always submodule update --init --recursive` | ok, libevent at `d82464a277d0f42703702c4dfd9af6af38595a83` |
| `cargo test --workspace` | **1155 passed, 0 failed** (1141 before; +14, all in `epoch_accounting::tests`, count confirmed by `--list`) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0, `failed_expectations: 0` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0, `failed_expectations: 0` |
| `python3 -B -m unittest discover -s scripts/tests -v` | 43 tests, OK |
| `(cd proof && lake build)` | 68 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | **Passed 24 Lean audits** (23 before) |
| `cargo test` in `providers/` | 56 passed, 0 failed |

## 7. What this does not establish

* **No source route.** There is no `.qcore` syntax for an epoch declaration, no
  frontend lowering and no C11 emission. `FlatEpochProgram` is a declared model
  input; nothing here derives peaks, results, work, credits or cell footprints
  from source, from an allocator or from a runtime.
* **No scheduler, no wall clock.** No core count, queueing, preemption,
  fairness, makespan or latency claim, in either direction. That is a refusal,
  not an omission.
* **No dynamic heap, no OS stacks.** Communicating dynamic heaps and OS thread
  stack guarantees are represented refusals; thread stacks are not in the
  accounting at all.
* **Declared premises, named on every certificate:**
  `DeclaredRuntimeOverhead`, `DeclaredTaskFootprints`,
  `DisjointSharedStorageCells`, `NoScheduleModel`. The disjointness of distinct
  shared cells in particular is an assumption about real C layout, exactly like
  T02's `DeclaredFootprintShape`.
* **No native measurement was taken.** The card allows optional sanitized
  native evidence; none was produced, deliberately, so that nothing in this
  record can be read as conflating an observed run with a declared bound. Any
  future measurement must arrive as separate evidence and must not become the
  `RuntimeOverhead` figure — that substitution is refused today by `Q640`.
* **Rust/Lean correspondence is by inspection.** The Lean model transcribes the
  Rust definitions; it is not mechanically extracted and there is no digest-
  checked correspondence of the F01 kind. Trace-level refinement between this
  profile and `scoped_tasks` / `ScopedOwnedTasksV1` execution is open.
* **Nothing is promoted.** Raw Cycle 19 units remain 2 Safe / 42 provisional
  Unsafe / 137 Unknown plus four data-only units; the contract corpus remains
  59 modules; CT coverage remains 15/50 mapped; `qcore-schemas` remains 5.0.0
  and was not touched. The four pinned 185-unit artifacts were not re-measured
  and are unchanged by construction: this card adds one new core module and
  edits nothing on the audit path (the only edit outside the new files is one
  additive `ConcurrencyProfile` variant and one `pub mod` line).

## 8. Integrator decisions

1. **CT35, CT36, CT38, CT39 stay unmapped — please do not mark them mapped.**
   Their scenarios line up exactly with this card's four rules (CT35 "count 4096
   once", CT36 "bound at least 8192 plus modeled overhead", CT38 "child credit
   splits total more than original credit → reject", CT39 "work bound used as
   wall-clock bound → reject"), and each is now checked at model level with the
   diagnostic named above. But a mapped CT case needs stage-derived evidence
   through the source pipeline, and there is no source fixture, no parser stage
   and no reference execution for an epoch declaration. Mapping them is a
   follow-on that must land epoch source syntax first. CT37 is already mapped
   and is untouched; its report's "general heap-peak accounting remains open"
   note is the boundary this card addresses at model level only.
2. **Ledger text.** Suggested paragraph for the AGENTS.md/CLAUDE.md status
   ledger (integrator to place and trim):

   > Flat-epoch quantitative accounting (T08): `FlatEpochBoundV1` in
   > `crates/qcore-core/src/epoch_accounting.rs` derives and independently
   > rechecks a residency and work bound for a fixed number of independent
   > tasks over fixed-shape shared storage. The shared entry allocation is a
   > union counted once, concurrent private peaks are summed rather than
   > maximized, and declared runtime overhead scales with the fixed task count;
   > a per-task double count, a maximized peak, a discarded live
   > completed-child result and a credit sum beyond the allocation are each
   > refused with a named diagnostic in the new `Q630`–`Q641` range. A
   > completed child's result stays live until an actual transfer or release,
   > and `epoch_residency` checks the whole configuration space of the tested
   > epoch. Work totals never become wall-clock bounds: there is no schedule
   > model, `epoch_wall_clock_bound` always refuses, and two Lean theorems show
   > no function of a work total is a finish time. Declared overhead is a
   > premise and a native measurement offered in its place is refused. 14 new
   > tests (1155 workspace total), 20 new Lean theorems in a new
   > `QCore.EpochAccounting` namespace (24 audits), no new axiom. Model level
   > only: no source syntax, no CT mapping (15/50 unchanged), no scheduler,
   > dynamic heap or OS stack claim, and no raw unit promoted. Evidence:
   > `docs/reports/remaining-t08/README.md`.
3. **Spec §L21.** No change made. `Q6xx` is already assigned to the
   `Concurrency` pass and T02's `Q610`–`Q623` are likewise not enumerated
   there. If the integrator wants per-code enumeration in the spec, `Q630`–
   `Q641` and `Q610`–`Q623` should be added together in one deliberate edit.
4. **`ConcurrencyProfile::FlatEpochBoundV1`** is a new additive variant in
   `crates/qcore-core/src/concurrency.rs`. `check_concurrent`'s body and its
   pinned tests are untouched; no match over that enum was exhaustive, so
   nothing else changed. If a lane prefers this profile to carry its own
   marker type instead, say so — the certificate field is the only user.
