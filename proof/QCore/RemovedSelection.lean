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

import Init.Data.List.Lemmas
import QCore.ListLookup
import QCore.DetachSameItem

/-!
# Checked removed-selection composition (gateway rule 33)

Abstract model of `removed_selection` and the `legacy_selection` adapter.
A selection wrapper's exactly checked frame computes
`sel P W = (W < 0) ? NULL : L(P, (size_t)W)`; its detach composes the existing
checked lookup with the existing same-item transition, and its release
composes that with one nullable release. The selected child is a caller
prerequisite: this model states that the selection, when present, is a member
of the represented child list (`selected_is_a_member`), which is what the caller
must establish, and never derives it from the detach representation premise or
from borrowed provenance. Source extraction (the exact frame match), the
analyzer's abstract-state reading and Rust/Lean correspondence are maintained
boundary-28 premises; the shared vectors bind the two decision tables.
-/
namespace QCore.RemovedSelection

open QCore.DetachSameItem (State Outcome restore nullOutcome)

/-- The frame's own selection value. -/
def sel (children : List Nat) (w : Int) : Option Nat :=
  if w < 0 then none else QCore.ListLookup.lookup children w.toNat

theorem negative_selects_nothing (children : List Nat) (w : Int) (h : w < 0) :
    sel children w = none := by
  simp [sel, h]

/-- A caller's decimal constant drops the guard: its selection is the lookup. -/
theorem nonnegative_constant_is_the_lookup (children : List Nat) (k : Nat) :
    sel children (k : Int) = QCore.ListLookup.lookup children k := by
  have nonnegative : ¬ ((k : Int) < 0) := by omega
  simp only [sel, nonnegative, ite_false, Int.toNat_natCast]

theorem selected_is_a_member (children : List Nat) (w : Int) (c : Nat)
    (h : sel children w = some c) : c ∈ children := by
  unfold sel at h
  by_cases neg : w < 0
  · simp [neg] at h
  · simp [neg] at h
    exact QCore.ListLookup.nonnull_is_a_member children w.toNat c h

/-- `F`: detach the selection and return it. -/
def detachFromArray (s : State) (w : Int) : Outcome :=
  match sel s.children w with
  | none => nullOutcome s
  | some c => restore s c

theorem the_result_is_the_selection (s : State) (w : Int) :
    (detachFromArray s w).result = sel s.children w := by
  unfold detachFromArray
  cases h : sel s.children w <;> simp [nullOutcome, restore]

theorem no_selection_moves_nothing (s : State) (w : Int)
    (h : sel s.children w = none) : (detachFromArray s w).state = s := by
  simp [detachFromArray, h, nullOutcome]

theorem the_selected_child_leaves_parent_custody (s : State) (w : Int) (c : Nat)
    (h : sel s.children w = some c) :
    c ∉ (detachFromArray s w).state.children ∧ c ∈ (detachFromArray s w).state.direct := by
  simp [detachFromArray, h, restore]

theorem every_other_child_is_preserved (s : State) (w : Int) (c other : Nat)
    (h : sel s.children w = some c) (distinct : other ≠ c) :
    other ∈ (detachFromArray s w).state.children ↔ other ∈ s.children := by
  simp [detachFromArray, h, restore, distinct]

theorem detach_does_not_allocate_or_release (s : State) (w : Int) :
    (detachFromArray s w).state.live = s.live ∧
    (detachFromArray s w).state.parent = s.parent := by
  unfold detachFromArray
  cases sel s.children w <;> simp [nullOutcome, restore]

/-- The ownership partition changes and its identity inventory does not. The
membership used is exactly the transported prerequisite. -/
theorem detach_inventory_is_preserved (s : State) (w : Int) (object : Nat) :
    (object ∈ (detachFromArray s w).state.children ∨
      object ∈ (detachFromArray s w).state.direct) ↔
    (object ∈ s.children ∨ object ∈ s.direct) := by
  cases h : sel s.children w with
  | none => simp [detachFromArray, h, nullOutcome]
  | some c =>
    have member := selected_is_a_member s.children w c h
    simpa [detachFromArray, h] using
      QCore.DetachSameItem.custody_identity_inventory_is_preserved s c object member

/-- Release of a detached result through one nullable release. -/
def releaseResult (o : Outcome) : State × Option Nat :=
  match o.result with
  | none => (o.state, none)
  | some c =>
    ({ o.state with direct := o.state.direct.filter (· != c),
                    live := o.state.live.filter (· != c) }, some c)

/-- `G`: detach the selection, then release the result. -/
def deleteFromArray (s : State) (w : Int) : State × Option Nat :=
  releaseResult (detachFromArray s w)

theorem delete_releases_exactly_the_selection (s : State) (w : Int) :
    (deleteFromArray s w).2 = sel s.children w := by
  unfold deleteFromArray detachFromArray
  cases sel s.children w <;> simp [releaseResult, nullOutcome, restore]

theorem delete_without_selection_changes_nothing (s : State) (w : Int)
    (h : sel s.children w = none) : (deleteFromArray s w).1 = s := by
  simp [deleteFromArray, detachFromArray, h, releaseResult, nullOutcome]

theorem delete_leaves_no_obligation_for_the_child (s : State) (w : Int) (c : Nat)
    (h : sel s.children w = some c) :
    c ∉ (deleteFromArray s w).1.children ∧ c ∉ (deleteFromArray s w).1.direct ∧
    c ∉ (deleteFromArray s w).1.live := by
  simp [deleteFromArray, detachFromArray, h, releaseResult, restore]

theorem delete_preserves_every_other_object (s : State) (w : Int) (c other : Nat)
    (h : sel s.children w = some c) (distinct : other ≠ c) :
    (other ∈ (deleteFromArray s w).1.children ↔ other ∈ s.children) ∧
    (other ∈ (deleteFromArray s w).1.direct ↔ other ∈ s.direct) ∧
    (other ∈ (deleteFromArray s w).1.live ↔ other ∈ s.live) := by
  simp [deleteFromArray, detachFromArray, h, releaseResult, restore, distinct]

/-! ## The two decision tables shared with Rust -/

structure MaterializeFacts where
  selectionNode : Bool
  parentLiveIdentity : Bool
  pristine : Bool
  fresh : Bool
  borrowedResult : Bool
  deriving DecidableEq, Repr

def materialize (f : MaterializeFacts) : Bool :=
  f.selectionNode && f.parentLiveIdentity && f.pristine && f.fresh && f.borrowedResult

theorem materialization_needs_every_fact (f : MaterializeFacts)
    (h : materialize f = true) :
    f.selectionNode = true ∧ f.parentLiveIdentity = true ∧ f.pristine = true ∧
    f.fresh = true ∧ f.borrowedResult = true := by
  rcases f with ⟨a, b, c, d, e⟩
  cases a <;> cases b <;> cases c <;> cases d <;> cases e <;> simp_all [materialize]

def materializeVectors : List (MaterializeFacts × Bool) := [
  (⟨true, true, true, true, true⟩, true),
  (⟨false, true, true, true, true⟩, false),
  (⟨true, false, true, true, true⟩, false),
  (⟨true, true, false, true, true⟩, false),
  (⟨true, true, true, false, true⟩, false),
  (⟨true, true, true, true, false⟩, false)
]

theorem materialize_vectors_hold :
    materializeVectors.all (fun (f, r) => materialize f == r) = true := by decide

inductive Disposition where
  | returned
  | released
  deriving DecidableEq, Repr

inductive Selected where
  | absent
  | contained
  | live
  | escaped
  | released
  | mixed
  deriving DecidableEq, Repr

inductive Returned where
  | nothing
  | selected
  | other
  deriving DecidableEq, Repr

structure ExitFacts where
  parentLive : Bool
  frameClear : Bool
  selected : Selected
  returned : Returned
  deriving DecidableEq, Repr

def acceptExit (d : Disposition) (f : ExitFacts) : Bool :=
  f.parentLive && f.frameClear &&
    match d, f.selected, f.returned with
    | .returned, .absent, .nothing => true
    | .returned, .escaped, .selected => true
    | .released, .absent, .nothing => true
    | .released, .released, .nothing => true
    | _, _, _ => false

/-- An accepted `F` exit either never selected and returns nothing, or returns
exactly the selected child it detached. -/
theorem accepted_detach_exit_returns_the_selection_or_nothing (f : ExitFacts)
    (h : acceptExit .returned f = true) :
    f.parentLive = true ∧
    ((f.selected = .absent ∧ f.returned = .nothing) ∨
      (f.selected = .escaped ∧ f.returned = .selected)) := by
  rcases f with ⟨p, c, s, r⟩
  cases p <;> cases c <;> cases s <;> cases r <;> simp_all [acceptExit]

/-- An accepted `G` exit returns nothing and never keeps, leaks or returns the
selected child. -/
theorem accepted_release_exit_keeps_no_selection (f : ExitFacts)
    (h : acceptExit .released f = true) :
    f.parentLive = true ∧ f.returned = .nothing ∧
    (f.selected = .absent ∨ f.selected = .released) := by
  rcases f with ⟨p, c, s, r⟩
  cases p <;> cases c <;> cases s <;> cases r <;> simp_all [acceptExit]

def exitVectors : List (Disposition × ExitFacts × Bool) := [
  (.returned, ⟨true, true, .absent, .nothing⟩, true),
  (.returned, ⟨true, true, .escaped, .selected⟩, true),
  (.returned, ⟨false, true, .escaped, .selected⟩, false),
  (.returned, ⟨true, false, .escaped, .selected⟩, false),
  (.returned, ⟨true, true, .contained, .nothing⟩, false),
  (.returned, ⟨true, true, .live, .nothing⟩, false),
  (.returned, ⟨true, true, .escaped, .other⟩, false),
  (.released, ⟨true, true, .released, .nothing⟩, true),
  (.released, ⟨true, true, .absent, .nothing⟩, true),
  (.released, ⟨true, true, .escaped, .selected⟩, false),
  (.released, ⟨true, true, .mixed, .nothing⟩, false),
  (.released, ⟨true, true, .live, .nothing⟩, false)
]

theorem exit_vectors_hold :
    exitVectors.all (fun (d, f, r) => acceptExit d f == r) = true := by decide

def selectionVectors : List (List Nat × Int × Option Nat) := [
  ([], 0, none),
  ([], -1, none),
  ([10], 0, some 10),
  ([10], -1, none),
  ([10, 20, 30], 1, some 20),
  ([10, 20, 30], 2, some 30),
  ([10, 20, 30], 3, none),
  ([10, 20, 30], -2, none)
]

theorem selection_vectors_hold :
    selectionVectors.all (fun (xs, w, r) => sel xs w == r) = true := by decide

end QCore.RemovedSelection
