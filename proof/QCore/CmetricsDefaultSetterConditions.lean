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

import QCore.CmetricsApiContracts

/-! Conditional value, preparation and retirement projections for the three
CMetrics default setters. `convert` is the independently bound u64/double/u64
conversion, and input lists contain the caller-proved readable elements.
Storage objects, allocation epochs, C traces, event completeness, floating
representations and Rust/Lean correspondence remain open inputs. These model
properties grant no executable call permission or joint atomic snapshot. -/
namespace QCore.CmetricsDefaultSetterConditions
open CmetricsApiContracts

/-- Histogram storage contains converted values, not necessarily the inputs. -/
def histogramValues (convert : Nat → Nat) (input : List Nat) : List Nat :=
  input.map convert

theorem histogram_preserves_bucket_count (convert : Nat → Nat) (input : List Nat) :
    (histogramValues convert input).length = input.length := by
  simp [histogramValues]

theorem histogram_values_have_conversion_witness (convert : Nat → Nat)
    (input : List Nat) (value : Nat) :
    value ∈ histogramValues convert input ↔ ∃ original ∈ input, convert original = value := by
  simp [histogramValues, List.mem_map]

structure SummaryStorage where
  values : Option (List Nat) := none
  flag : Bool := false
  deriving DecidableEq, Repr

/-- Called before checking whether quantile input is null. -/
def prepareSummary (s : SummaryStorage) (count : Nat) (allocated : Bool) :
    Option SummaryStorage :=
  if s.values.isSome || count == 0 then some s
  else if allocated then some { s with values := some (List.replicate count 0) }
  else none

theorem existing_summary_storage_needs_no_allocation (values : List Nat)
    (flag allocated : Bool) (count : Nat) :
    prepareSummary ⟨some values, flag⟩ count allocated = some ⟨some values, flag⟩ := by
  simp [prepareSummary]

theorem zero_summary_count_needs_no_allocation (s : SummaryStorage) (allocated : Bool) :
    prepareSummary s 0 allocated = some s := by
  simp [prepareSummary]

theorem summary_preparation_zeroes_new_storage (count : Nat) (positive : count ≠ 0)
    (flag : Bool) :
    prepareSummary ⟨none, flag⟩ count true =
      some ⟨some (List.replicate count 0), flag⟩ := by
  simp [prepareSummary, positive]

theorem summary_allocation_failure_has_no_success (count : Nat) (positive : count ≠ 0)
    (flag : Bool) : prepareSummary ⟨none, flag⟩ count false = none := by
  simp [prepareSummary, positive]

/-- An admitted slot needs both the parent loop bound and metric storage bound.
The supplied function represents all caller-proved readable input values. -/
def summarySlot (parentCount metricCount index old : Nat)
    (input : Option (Nat → Nat)) : Nat :=
  match input with
  | none => old
  | some values => if index < parentCount ∧ index < metricCount then values index else old

def summaryFlag (old : Bool) (input : Option (Nat → Nat)) : Bool :=
  old || input.isSome

theorem null_summary_input_preserves_values_and_flag (parentCount metricCount index old : Nat)
    (flag : Bool) : summarySlot parentCount metricCount index old none = old ∧
      summaryFlag flag none = flag := by
  simp [summarySlot, summaryFlag]

theorem admitted_summary_slot_receives_input (parentCount metricCount index old : Nat)
    (input : Nat → Nat) (parentBound : index < parentCount)
    (metricBound : index < metricCount) :
    summarySlot parentCount metricCount index old (some input) = input index := by
  simp [summarySlot, parentBound, metricBound]

theorem unadmitted_summary_slot_is_preserved (parentCount metricCount index old : Nat)
    (input : Nat → Nat) (outside : ¬(index < parentCount ∧ index < metricCount)) :
    summarySlot parentCount metricCount index old (some input) = old := by
  simp [summarySlot, outside]

/-- A zero count ignores its pointer. Positive counts need both readable input
and allocation; a successful copy takes exactly the admitted input prefix. -/
def prepareCopy (count : Nat) (input : Option (List Nat)) (allocated : Bool) :
    Option (Option (List Nat)) :=
  if count == 0 then some none
  else match input with
    | none => none
    | some values => if allocated then some (some (values.take count)) else none

theorem zero_copy_count_ignores_pointer (input : Option (List Nat)) (allocated : Bool) :
    prepareCopy 0 input allocated = some none := by
  simp [prepareCopy]

theorem positive_copy_count_rejects_null (count : Nat) (positive : count ≠ 0)
    (allocated : Bool) : prepareCopy count none allocated = none := by
  simp [prepareCopy, positive]

theorem successful_copy_has_requested_length (count : Nat) (positive : count ≠ 0)
    (input : List Nat) (readable : count ≤ input.length) :
    prepareCopy count (some input) true = some (some (input.take count)) ∧
      (input.take count).length = count := by
  simp [prepareCopy, positive, List.length_take, Nat.min_eq_left readable]

structure FailedPreparation where
  state : SetterState
  released : List Nat
  remainingTemps : List Nat
  deriving DecidableEq, Repr

/-- The list contains distinct non-null temporary allocation epochs. -/
def failPreparation (afterLookup : SetterState) (temps : List Nat) : FailedPreparation :=
  ⟨afterLookup, temps, []⟩

theorem failed_preparation_keeps_lookup_and_discharges_temporaries
    (afterLookup : SetterState) (temps : List Nat) :
    (failPreparation afterLookup temps).state = afterLookup ∧
      (failPreparation afterLookup temps).released = temps ∧
      (failPreparation afterLookup temps).remainingTemps = [] := by
  simp [failPreparation]

theorem failed_preparation_releases_each_temporary_once (s : SetterState)
    (temps : List Nat) (distinctEpochs : temps.Nodup) (epoch : Nat)
    (allocated : epoch ∈ temps) :
    (failPreparation s temps).released.count epoch = 1 := by
  exact distinctEpochs.count_of_mem allocated

theorem successful_swap_invalidates_prior_array_epochs (s : SetterState)
    (fresh : List Nat) (timestamp old : Nat) (wasOwned : old ∈ s.arrays) :
    arrayBorrowValid (replaceArrays s true fresh timestamp).1 old = false ∧
      (replaceArrays s true fresh timestamp).1.map = s.map := by
  exact ⟨replacement_invalidates_old_array_borrow s fresh timestamp old wasOwned,
    replacement_preserves_metric_parent s fresh timestamp⟩

inductive ArrayEvent where
  | swap
  | unlock
  | release (epoch : Nat)
  deriving DecidableEq, Repr

def replacementTrace (old : List Nat) : List ArrayEvent :=
  [.swap, .unlock] ++ old.map ArrayEvent.release

theorem replacement_releases_follow_swap_and_unlock (old : List Nat) :
    (replacementTrace old).take 2 = [.swap, .unlock] ∧
      (replacementTrace old).drop 2 = old.map ArrayEvent.release := by
  simp [replacementTrace]

end QCore.CmetricsDefaultSetterConditions
