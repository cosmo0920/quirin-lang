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

import QCore.LoopLifetimeSummary
/-! Complete traces replay removals against incoming records in order. Source
extraction, exact symbolic snapshot correspondence and Rust/Lean correspondence
remain declared trust boundaries. A cross-unit home must be independently
recomputed and compared, including effects; a custody-only projection cannot
stand in for its full result. These theorems do not prove C extraction or link
resolution. -/
namespace QCore.OrderedLifetimeReplay
open LoopLifetimeSummary
inductive Step where
  | register (record : Registration)
  | remove (object holder : Option Path)
  | destroy (holder : Path)
  | release (object : Path)
  deriving DecidableEq, Repr
structure State where
  records : List Registration
  refused : Bool
  deriving DecidableEq, Repr

def remove (object holder : Option Path) (records : List Registration) : List Registration :=
  records.filterMap fun r =>
    if !r.poisoned && concrete r.object && concrete r.holder &&
        object == some r.object && holder == some r.holder then none
    else some { r with poisoned := true }

def apply (state : State) : Step → State
  | .register r => { state with records := r :: state.records }
  | .remove o h => { state with records := remove o h state.records }
  | .destroy h => { state with records := LoopLifetimeSummary.destroy h state.records }
  | .release _ => { state with refused := state.refused || !state.records.isEmpty }

def replay (steps : List Step) (state : State) : State := steps.foldl apply state

theorem summary_composition_preserves_the_complete_order
    (earlier later : List Step) (state : State) :
    replay (earlier ++ later) state = replay later (replay earlier state) := by
  simp [replay, List.foldl_append]

theorem refusal_survives_every_later_step
    (state : State) (step : Step) (old : state.refused = true) :
    (apply state step).refused = true := by
  cases step <;> simp [apply, old]

theorem a_later_ending_never_repairs_an_earlier_refusal
    (steps : List Step) (state : State) (old : state.refused = true) :
    (replay steps state).refused = true := by
  induction steps generalizing state with
  | nil => exact old
  | cons first rest ih =>
    exact ih (apply state first) (refusal_survives_every_later_step state first old)

def object : Path := ⟨.formal 0 0, []⟩
def holder : Path := ⟨.formal 0 1, []⟩
def other : Path := ⟨.formal 0 2, []⟩
def record (poison : Bool) : Registration := ⟨object, holder, poison⟩

theorem exact_removal_before_release_decides :
    replay [.remove (some object) (some holder), .release object]
      ⟨[record false], false⟩ = ⟨[], false⟩ := by decide

theorem poison_cannot_be_cleared_by_exact_removal :
    (replay [.remove (some object) (some holder), .release object]
      ⟨[record true], false⟩).refused = true := by decide

theorem unmatched_then_matching_removal_still_refuses :
    (replay [.remove (some object) (some other),
      .remove (some object) (some holder), .release object]
      ⟨[record false], false⟩).refused = true := by decide

/-- `replayed` is the separately computed home result, not the imported claim.
The home/source/link identities also come from independent expected inputs. -/
def admitImport (homePresent : Bool) (actualIdentity expectedIdentity : Nat)
    (imported replayed : List Nat) : Bool :=
  homePresent && actualIdentity == expectedIdentity &&
    !imported.isEmpty && imported == replayed

theorem an_admitted_import_preserves_every_replayed_effect
    (present : Bool) (actual expected : Nat) (imported recomputed : List Nat)
    (admitted : admitImport present actual expected imported recomputed = true) :
    imported = recomputed := by
  simp_all [admitImport]

theorem a_missing_home_cannot_authorize_lifetime_import
    (actual expected : Nat) (imported recomputed : List Nat) :
    admitImport false actual expected imported recomputed = false := by
  simp [admitImport]

theorem an_empty_projection_cannot_authorize_lifetime_import
    (present : Bool) (actual expected : Nat) (recomputed : List Nat) :
    admitImport present actual expected [] recomputed = false := by
  simp [admitImport]
def joinTrace (left right : Option (List Step)) : Option (List Step) :=
  if left == right then left else none

theorem an_admitted_join_retains_the_same_trace_on_both_exits
    (left right : Option (List Step)) (trace : List Step)
    (accepted : joinTrace left right = some trace) :
    left = some trace ∧ right = some trace := by
  simp only [joinTrace] at accepted
  split at accepted <;> simp_all

def boundedTrace (steps : List Step) : Option (List Step) :=
  if steps.length ≤ 4096 then some steps else none

theorem a_bounded_trace_is_complete_and_within_its_limit
    (steps trace : List Step) (accepted : boundedTrace steps = some trace) :
    steps = trace ∧ steps.length ≤ 4096 := by
  simp only [boundedTrace] at accepted
  split at accepted <;> simp_all

theorem admitted_imports_keep_the_independently_expected_identity
    (present : Bool) (actual expected : Nat) (imported recomputed : List Nat)
    (admitted : admitImport present actual expected imported recomputed = true) :
    actual = expected := by simp_all [admitImport]
/-- An ordinary borrow record has nowhere to retain removal order. -/
def ordinaryPublication (hasRemoval : Bool) : Bool := !hasRemoval

theorem a_removal_cannot_be_published_as_an_ordinary_borrow :
    ordinaryPublication true = false := by decide

def effectClass (registrations removals : List Nat) : Bool :=
    (registrations.isEmpty && !removals.isEmpty) ||
    (!registrations.isEmpty && removals.isEmpty)

theorem mixed_effect_vectors_do_not_establish_an_order
    (registration removal : Nat) (registrations removals : List Nat) :
    effectClass (registration :: registrations) (removal :: removals) = false := by
  simp [effectClass]
def removalCallAllowed (internal replayedHome : Bool) : Bool := internal || replayedHome

theorem an_unresolved_external_call_cannot_supply_a_removal :
    removalCallAllowed false false = false := by decide

def sourceAdmission (seededLocal : Bool) (homePresent : Bool)
    (actual expected : Nat) (imported recomputed : List Nat) : Bool :=
    !seededLocal && admitImport homePresent actual expected imported recomputed

theorem preseeded_local_inference_is_not_source_authority
    (homePresent : Bool) (actual expected : Nat) (imported recomputed : List Nat) :
    sourceAdmission true homePresent actual expected imported recomputed = false := by
  simp [sourceAdmission]
end QCore.OrderedLifetimeReplay
