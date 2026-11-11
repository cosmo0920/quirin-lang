-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.Quant

/-! Quantitative accounting for one flat concurrent epoch (card T08).

One creator, a fixed list of independent tasks, fixed-shape shared storage,
declared runtime overhead. The model answers one question: how much of each
axis can be live at once, anywhere in the epoch.

Three rules are proved rather than assumed:

* the shared entry allocation is a **union** — a cell named by many tasks is
  counted once, and a double count is strictly larger;
* concurrent private peaks **overlap**, so they are summed; the componentwise
  maximum is exceeded by the configuration in which every task runs;
* a completed child's result is live until an actual transfer or release, so
  the completed-but-unjoined configuration still holds it.

What is *not* here: any schedule. There is no core count, no queueing and no
preemption, and `work_total_does_not_determine_finish_time` exhibits two
schedules of the same work with different finish times, which is exactly why a
work bound is never a wall-clock bound. Source import, allocator behaviour, OS
stacks, dynamic heaps and inter-task communication are outside the model, and
the per-task/per-epoch overhead figures are declared premises, never
measurements. -/
namespace QCore.EpochAccounting

open QCore

/-- One task's declared quantitative shape. `retained` is what its result costs
between completion and the creator's join; `transferred` is what survives that
join under the creator. -/
structure Task where
  privatePeak : QuantVector
  retained : QuantVector
  transferred : QuantVector
  work : QuantVector
  credit : QuantVector

/-- One shared storage cell: an identity and a footprint. Distinct identities
are a declared premise to be disjoint storage, so their footprints add. -/
structure Cell where
  id : Nat
  footprint : QuantVector

/-- One flat epoch. `cells` are the shared entry allocation the tasks read. -/
structure Epoch where
  cells : List Cell
  tasks : List Task
  perTask : QuantVector
  perEpoch : QuantVector
  allocation : QuantVector

/-- A task's result never exceeds the peak that task was accounted for, and what
survives the join never exceeds what the result was. -/
def Task.Wellformed (task : Task) : Prop :=
  QuantVector.Le task.transferred task.retained ∧
    QuantVector.Le task.retained task.privatePeak

def Epoch.Wellformed (epoch : Epoch) : Prop :=
  ∀ task ∈ epoch.tasks, task.Wellformed

/-- The union of a cell list: a repeated identity contributes once. -/
def unionOf : List Cell → QuantVector
  | [] => QuantVector.zero
  | cell :: rest =>
    if rest.any (fun other => other.id = cell.id) then unionOf rest
    else QuantVector.add cell.footprint (unionOf rest)

/-- The same cells counted once per occurrence. Never a bound of this profile;
it is here to show that the double count is a different, larger number. -/
def sumOf : List Cell → QuantVector
  | [] => QuantVector.zero
  | cell :: rest => QuantVector.add cell.footprint (sumOf rest)

def sumBy (field : Task → QuantVector) : List Task → QuantVector
  | [] => QuantVector.zero
  | task :: rest => QuantVector.add (field task) (sumBy field rest)

def maxBy (field : Task → QuantVector) : List Task → QuantVector
  | [] => QuantVector.zero
  | task :: rest => QuantVector.join (field task) (maxBy field rest)

/-- `perEpoch + |tasks| * perTask`: the declared overhead of the whole epoch. -/
def overhead (epoch : Epoch) : QuantVector :=
  QuantVector.add epoch.perEpoch (QuantVector.scale epoch.tasks.length epoch.perTask)

/-- The checked bound: shared union, summed private peaks, declared overhead. -/
def epochPeak (epoch : Epoch) : QuantVector :=
  QuantVector.add (unionOf epoch.cells)
    (QuantVector.add (sumBy Task.privatePeak epoch.tasks) (overhead epoch))

/-- Every task finished, none joined. Results are still live and so is every
per-task overhead. -/
def completedResidency (epoch : Epoch) : QuantVector :=
  QuantVector.add (unionOf epoch.cells)
    (QuantVector.add (sumBy Task.retained epoch.tasks) (overhead epoch))

def workTotal (epoch : Epoch) : QuantVector := sumBy Task.work epoch.tasks

def grantedCredits (epoch : Epoch) : QuantVector := sumBy Task.credit epoch.tasks

/-- A task's whole demand: what it holds at once, its own overhead, its work. -/
def demand (epoch : Epoch) (task : Task) : QuantVector :=
  QuantVector.add task.privatePeak (QuantVector.add epoch.perTask task.work)

/-- Where one task is. Because the tasks are independent, every assignment of
phases to tasks is an admitted configuration. -/
inductive Phase where
  | ready
  | running
  | completed
  | joined
  deriving DecidableEq, Repr

def liveOf (task : Task) : Phase → QuantVector
  | .ready => QuantVector.zero
  | .running => task.privatePeak
  | .completed => task.retained
  | .joined => task.transferred

/-- A started, unjoined task still costs its per-task overhead. -/
def outstanding : Phase → Nat
  | .ready => 0
  | .running => 1
  | .completed => 1
  | .joined => 0

def liveTasks : List Task → List Phase → QuantVector
  | task :: tasks, phase :: phases =>
    QuantVector.add (liveOf task phase) (liveTasks tasks phases)
  | _, _ => QuantVector.zero

def outstandingTasks : List Task → List Phase → Nat
  | _ :: tasks, phase :: phases => outstanding phase + outstandingTasks tasks phases
  | _, _ => 0

/-- The live quantity of one configuration. -/
def residency (epoch : Epoch) (phases : List Phase) : QuantVector :=
  QuantVector.add (unionOf epoch.cells)
    (QuantVector.add epoch.perEpoch
      (QuantVector.add (liveTasks epoch.tasks phases)
        (QuantVector.scale (outstandingTasks epoch.tasks phases) epoch.perTask)))

def allPhases (phase : Phase) : List Task → List Phase
  | [] => []
  | _ :: rest => phase :: allPhases phase rest

-- ---------------------------------------------------------------------------
-- The shared entry allocation is a union, counted once.
-- ---------------------------------------------------------------------------

/-- A repeated cell identity contributes its footprint once. -/
theorem union_counts_a_repeated_cell_once (cell : Cell) (rest : List Cell) :
    unionOf (cell :: cell :: rest) = unionOf (cell :: rest) := by
  simp [unionOf]

/-- Any cell whose identity already occurs later in the list contributes
nothing: the union is fixed by the *identities* present, not by how many
entries — or how many readers — name them. -/
theorem union_ignores_a_repeated_identity {cell : Cell} {rest : List Cell}
    (repeated : rest.any (fun other => other.id = cell.id)) :
    unionOf (cell :: rest) = unionOf rest := by
  simp [unionOf, repeated]

/-- The union never exceeds the double count. -/
theorem union_is_at_most_the_double_count (cells : List Cell) :
    QuantVector.Le (unionOf cells) (sumOf cells) := by
  induction cells with
  | nil => exact QuantVector.zero_le _
  | cons cell rest step =>
    intro axis
    by_cases repeated : rest.any (fun other => other.id = cell.id)
    · have : unionOf (cell :: rest) = unionOf rest := by simp [unionOf, repeated]
      rw [this]
      exact Nat.le_trans (step axis) (Nat.le_add_left _ _)
    · have : unionOf (cell :: rest) = QuantVector.add cell.footprint (unionOf rest) := by
        simp [unionOf, repeated]
      rw [this]
      exact Nat.add_le_add_left (step axis) _

/-- And a double count of a nonempty cell is strictly larger, so counting the
shared entry allocation per task is a different number, not a conservative
reading of the same one. -/
theorem a_double_counted_cell_is_strictly_larger (cell : Cell) (axis : Axis)
    (nonempty : 0 < cell.footprint axis) :
    unionOf [cell, cell] axis < sumOf [cell, cell] axis := by
  simp [unionOf, sumOf, QuantVector.add, QuantVector.zero]
  omega

-- ---------------------------------------------------------------------------
-- Overlapping private peaks are summed, not maximized.
-- ---------------------------------------------------------------------------

theorem sum_dominates_each_task {field : Task → QuantVector} {task : Task}
    {tasks : List Task} (mem : task ∈ tasks) :
    QuantVector.Le (field task) (sumBy field tasks) := by
  induction tasks with
  | nil => cases mem
  | cons head rest step =>
    intro axis
    rcases List.mem_cons.mp mem with rfl | tail
    · exact Nat.le_add_right _ _
    · exact Nat.le_trans (step tail axis) (Nat.le_add_left _ _)

theorem maximum_is_at_most_the_sum (field : Task → QuantVector) (tasks : List Task) :
    QuantVector.Le (maxBy field tasks) (sumBy field tasks) := by
  induction tasks with
  | nil => exact QuantVector.zero_le _
  | cons task rest step =>
    intro axis
    exact Nat.max_le.mpr ⟨Nat.le_add_right _ _,
      Nat.le_trans (step axis) (Nat.le_add_left _ _)⟩

/-- Two concurrent tasks with positive private peaks exceed the componentwise
maximum, so a maximized bound is undercounted rather than merely imprecise. -/
theorem the_maximum_undercounts_two_concurrent_tasks (task : Task) (axis : Axis)
    (nonempty : 0 < task.privatePeak axis) :
    maxBy Task.privatePeak [task, task] axis < sumBy Task.privatePeak [task, task] axis := by
  simp [maxBy, sumBy, QuantVector.join, QuantVector.add, QuantVector.zero]
  omega

-- ---------------------------------------------------------------------------
-- The bound covers every configuration, and the all-running one attains it.
-- ---------------------------------------------------------------------------

theorem live_is_at_most_the_private_peak {task : Task} (wellformed : task.Wellformed)
    (phase : Phase) : QuantVector.Le (liveOf task phase) task.privatePeak := by
  intro axis
  cases phase with
  | ready => exact Nat.zero_le _
  | running => exact Nat.le_refl _
  | completed => exact wellformed.2 axis
  | joined => exact Nat.le_trans (wellformed.1 axis) (wellformed.2 axis)

theorem live_tasks_le_private_sum {tasks : List Task} (phases : List Phase)
    (wellformed : ∀ task ∈ tasks, task.Wellformed) :
    QuantVector.Le (liveTasks tasks phases) (sumBy Task.privatePeak tasks) := by
  induction tasks generalizing phases with
  | nil => intro axis; cases phases <;> exact Nat.zero_le _
  | cons task rest step =>
    intro axis
    cases phases with
    | nil => exact Nat.zero_le _
    | cons phase tail =>
      have head := live_is_at_most_the_private_peak (wellformed task (by simp)) phase axis
      have others := step tail (fun t mem => wellformed t (by simp [mem])) axis
      exact Nat.add_le_add head others

theorem outstanding_le_task_count (tasks : List Task) (phases : List Phase) :
    outstandingTasks tasks phases ≤ tasks.length := by
  induction tasks generalizing phases with
  | nil => cases phases <;> simp [outstandingTasks]
  | cons task rest step =>
    cases phases with
    | nil => simp [outstandingTasks]
    | cons phase tail =>
      have tailBound := step tail
      have headBound : outstanding phase ≤ 1 := by cases phase <;> simp [outstanding]
      simp only [outstandingTasks, List.length_cons]
      omega

/-- The soundness claim of the profile: no admitted configuration of a
wellformed epoch exceeds the checked bound. -/
theorem residency_is_bounded_by_the_epoch_peak {epoch : Epoch}
    (wellformed : epoch.Wellformed) (phases : List Phase) :
    QuantVector.Le (residency epoch phases) (epochPeak epoch) := by
  intro axis
  have live := live_tasks_le_private_sum (tasks := epoch.tasks) phases wellformed axis
  have count := outstanding_le_task_count epoch.tasks phases
  have scaled :
      QuantVector.scale (outstandingTasks epoch.tasks phases) epoch.perTask axis
        ≤ QuantVector.scale epoch.tasks.length epoch.perTask axis :=
    Nat.mul_le_mul_right _ count
  simp only [residency, epochPeak, overhead, QuantVector.add]
  omega

/-- The all-running configuration attains it exactly, which is why the peaks
had to be summed. -/
theorem the_all_running_configuration_attains_the_bound (epoch : Epoch) :
    residency epoch (allPhases .running epoch.tasks) = epochPeak epoch := by
  have live : ∀ tasks : List Task,
      liveTasks tasks (allPhases .running tasks) = sumBy Task.privatePeak tasks := by
    intro tasks
    induction tasks with
    | nil => rfl
    | cons task rest step => simp [allPhases, liveTasks, sumBy, liveOf, step]
  have count : ∀ tasks : List Task,
      outstandingTasks tasks (allPhases .running tasks) = tasks.length := by
    intro tasks
    induction tasks with
    | nil => rfl
    | cons task rest step =>
      simp [allPhases, outstandingTasks, outstanding, step]
      omega
  funext axis
  simp [residency, epochPeak, overhead, live, count, QuantVector.add]
  omega

/-- Nothing has started: only the shared storage and the epoch's own overhead
are live. A task that does not exist costs no per-task overhead. -/
theorem the_ready_configuration_holds_only_shared_storage_and_epoch_overhead
    (epoch : Epoch) :
    residency epoch (allPhases .ready epoch.tasks)
      = QuantVector.add (unionOf epoch.cells) epoch.perEpoch := by
  have live : ∀ tasks : List Task,
      liveTasks tasks (allPhases .ready tasks) = QuantVector.zero := by
    intro tasks
    induction tasks with
    | nil => rfl
    | cons task rest step =>
      funext axis
      simp [allPhases, liveTasks, liveOf, step, QuantVector.add, QuantVector.zero]
  have count : ∀ tasks : List Task,
      outstandingTasks tasks (allPhases .ready tasks) = 0 := by
    intro tasks
    induction tasks with
    | nil => rfl
    | cons task rest step => simp [allPhases, outstandingTasks, outstanding, step]
  funext axis
  simp [residency, live, count, QuantVector.add, QuantVector.zero, QuantVector.scale]

-- ---------------------------------------------------------------------------
-- Completed results are live until an actual transfer or release.
-- ---------------------------------------------------------------------------

/-- Completion reclaims nothing: with every child finished and none joined, the
residency is exactly the completed residency, results included. -/
theorem completion_reclaims_nothing (epoch : Epoch) :
    residency epoch (allPhases .completed epoch.tasks) = completedResidency epoch := by
  have live : ∀ tasks : List Task,
      liveTasks tasks (allPhases .completed tasks) = sumBy Task.retained tasks := by
    intro tasks
    induction tasks with
    | nil => rfl
    | cons task rest step => simp [allPhases, liveTasks, sumBy, liveOf, step]
  have count : ∀ tasks : List Task,
      outstandingTasks tasks (allPhases .completed tasks) = tasks.length := by
    intro tasks
    induction tasks with
    | nil => rfl
    | cons task rest step =>
      simp [allPhases, outstandingTasks, outstanding, step]
      omega
  funext axis
  simp [residency, completedResidency, overhead, live, count, QuantVector.add]
  omega

/-- A completed child's result is *observable* in the residency: dropping it
strictly lowers the number, so a reading that ends the result's life at
completion is an undercount, not a rounding. -/
theorem dropping_a_completed_result_undercounts {epoch : Epoch} {task : Task}
    {axis : Axis} (mem : task ∈ epoch.tasks) (nonempty : 0 < task.retained axis) :
    QuantVector.add (unionOf epoch.cells) (overhead epoch) axis
      < completedResidency epoch axis := by
  have retained := sum_dominates_each_task (field := Task.retained) mem axis
  simp only [completedResidency, QuantVector.add]
  omega

/-- A transferred result is still live after the join: the creator holds it. -/
theorem a_transferred_result_survives_the_join {epoch : Epoch} {task : Task}
    (mem : task ∈ epoch.tasks) :
    QuantVector.Le task.transferred (residency epoch (allPhases .joined epoch.tasks)) := by
  have live : ∀ tasks : List Task,
      liveTasks tasks (allPhases .joined tasks) = sumBy Task.transferred tasks := by
    intro tasks
    induction tasks with
    | nil => rfl
    | cons head rest step => simp [allPhases, liveTasks, sumBy, liveOf, step]
  intro axis
  have transferred := sum_dominates_each_task (field := Task.transferred) mem axis
  simp only [residency, live, QuantVector.add]
  omega

-- ---------------------------------------------------------------------------
-- Credits never exceed the allocation.
-- ---------------------------------------------------------------------------

/-- Credits that fit, spent within their credits, fit. -/
theorem demands_within_credits_fit_the_allocation {epoch : Epoch}
    (granted : QuantVector.Le (grantedCredits epoch) epoch.allocation)
    (within : ∀ task ∈ epoch.tasks, QuantVector.Le (demand epoch task) task.credit) :
    QuantVector.Le (sumBy (demand epoch) epoch.tasks) epoch.allocation := by
  have summed : ∀ tasks : List Task,
      (∀ task ∈ tasks, QuantVector.Le (demand epoch task) task.credit) →
        QuantVector.Le (sumBy (demand epoch) tasks) (sumBy Task.credit tasks) := by
    intro tasks
    induction tasks with
    | nil => intro _; exact QuantVector.zero_le _
    | cons task rest step =>
      intro within
      intro axis
      have head := within task (by simp) axis
      have tail := step (fun t mem => within t (by simp [mem])) axis
      exact Nat.add_le_add head tail
  exact QuantVector.le_trans (summed epoch.tasks within) granted

/-- The converse direction of the same gate: a credit sum beyond the allocation
is refused, and no per-task check can rescue it. -/
theorem credits_beyond_the_allocation_are_not_covered {epoch : Epoch} {axis : Axis}
    (excess : epoch.allocation axis < grantedCredits epoch axis) :
    ¬ QuantVector.Le (grantedCredits epoch) epoch.allocation := by
  intro granted
  exact absurd (granted axis) (Nat.not_le.mpr excess)

-- ---------------------------------------------------------------------------
-- Work is not time.
-- ---------------------------------------------------------------------------

/-- A schedule assigns each task a start time. This is *not* part of the epoch
profile: it exists only to show that the profile's work total cannot decide a
completion time. -/
def finishTime : List Nat → List Nat → Nat
  | start :: starts, work :: works => max (start + work) (finishTime starts works)
  | _, _ => 0

def totalWork : List Nat → Nat
  | [] => 0
  | work :: works => work + totalWork works

/-- Two schedules of the very same work finish at different times. A work or CPU
bound therefore never becomes a wall-clock bound without a schedule model, and
this profile has none. -/
theorem work_total_does_not_determine_finish_time :
    ∃ works starts₁ starts₂ : List Nat,
      finishTime starts₁ works ≠ finishTime starts₂ works := by
  refine ⟨[1, 1], [0, 0], [0, 1], ?_⟩
  decide

/-- The same statement from the other side: the work total is fixed while the
finish time varies, so no function of the work total can be the finish time. -/
theorem no_function_of_the_work_total_is_the_finish_time :
    ¬ ∃ latency : Nat → Nat,
        ∀ works starts : List Nat, latency (totalWork works) = finishTime starts works := by
  rintro ⟨latency, sound⟩
  have first := sound [1, 1] [0, 0]
  have second := sound [1, 1] [0, 1]
  rw [first] at second
  exact absurd second (by decide)

end QCore.EpochAccounting
