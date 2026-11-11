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

import Init.Data.List.Pairwise
import Init.Data.List.Lemmas

/-!
# Checked same-item detach

Abstract decision/ownership model of `detach_same_item::decide` and the
`legacy_detach` adapter. Physical C extraction, the complete unlink-template
match and Rust/Lean correspondence are maintained review premises (boundary
28), not an extraction theorem. The named list representation premise states
finite acyclic child/next membership, first.prev = tail, predecessor links and
live objects; it does not state the conclusion of the unlink or supply caller
membership. The caller separately establishes one settled contained identity
and its unique live parent before the same identity is returned. No result
site is allocated. Concrete Rust controls execute the C write sequence over
every position of lists of lengths 1 through 6, including two-node head whose
prev equals next. Other lists and the parent's identity stay framed.
-/
namespace QCore.DetachSameItem

structure Facts where
  parentLiveIdentity : Bool
  itemIdentity : Bool
  nullable : Bool
  itemNull : Bool
  itemMayBeNull : Bool
  exactMembership : Bool
  itemSettled : Bool
  deriving DecidableEq, Repr

inductive Decision where
  | nothing
  | restore
  | refuse
  deriving DecidableEq, Repr

def decide (f : Facts) : Decision :=
  if !f.parentLiveIdentity || !f.itemIdentity then .refuse
  else if f.itemNull then if f.nullable then .nothing else .refuse
  else if f.exactMembership && f.itemSettled && (f.nullable || !f.itemMayBeNull)
    then .restore else .refuse

def decisionVectors : List (Facts × Decision) := [
  (⟨true, true, true, true, true, false, false⟩, .nothing),
  (⟨false, true, true, true, true, false, false⟩, .refuse),
  (⟨true, true, false, true, true, false, false⟩, .refuse),
  (⟨true, true, false, false, false, true, true⟩, .restore),
  (⟨true, false, false, false, false, true, true⟩, .refuse),
  (⟨true, true, false, false, false, false, true⟩, .refuse),
  (⟨true, true, false, false, false, true, false⟩, .refuse),
  (⟨true, true, false, false, true, true, true⟩, .refuse)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (f, d) => decide f == d) = true := by decide

theorem restoration_needs_checked_membership (f : Facts)
    (h : decide f = .restore) :
    f.parentLiveIdentity = true ∧ f.itemIdentity = true ∧
    f.exactMembership = true ∧ f.itemSettled = true ∧ f.itemNull = false := by
  rcases f with ⟨p, i, n, z, m, c, s⟩
  cases p <;> cases i <;> cases n <;> cases z <;> cases m <;> cases c <;> cases s <;>
    simp_all [decide]

structure State where
  parent : Nat
  children : List Nat
  direct : List Nat
  live : List Nat
  aliases : List (Nat × Nat)
  deriving DecidableEq, Repr

structure Outcome where
  state : State
  result : Option Nat
  deriving DecidableEq, Repr

/-- The ownership transition of the checked, successful physical unlink. -/
def restore (s : State) (item : Nat) : Outcome :=
  ⟨{ s with children := s.children.filter (· != item), direct := item :: s.direct }, some item⟩

def nullOutcome (s : State) : Outcome := ⟨s, none⟩

theorem the_result_is_the_same_item (s : State) (item : Nat) :
    (restore s item).result = some item := rfl

theorem restoration_does_not_allocate_or_release (s : State) (item : Nat) :
    (restore s item).state.live = s.live := rfl

theorem the_parent_identity_and_aliases_are_preserved (s : State) (item : Nat) :
    (restore s item).state.parent = s.parent ∧
    (restore s item).state.aliases = s.aliases := by constructor <;> rfl

theorem the_item_leaves_parent_custody (s : State) (item : Nat) :
    item ∉ (restore s item).state.children := by simp [restore]

theorem every_other_child_is_preserved (s : State) (item other : Nat)
    (h : other ≠ item) :
    other ∈ (restore s item).state.children ↔ other ∈ s.children := by
  simp [restore, h]

theorem the_same_item_has_direct_custody (s : State) (item : Nat) :
    item ∈ (restore s item).state.direct := by simp [restore]

theorem null_transfers_nothing (s : State) :
    (nullOutcome s).state = s ∧ (nullOutcome s).result = none := by constructor <;> rfl

/-- Restoring a contained item never duplicates a direct obligation. -/
theorem direct_custody_stays_unique (s : State) (item : Nat)
    (unique : s.direct.Nodup) (notDirect : item ∉ s.direct) :
    (restore s item).state.direct.Nodup := by
  simpa [restore] using List.nodup_cons.mpr ⟨notDirect, unique⟩

/-- The ownership partition changes, its identity inventory does not. -/
theorem custody_identity_inventory_is_preserved (s : State) (item object : Nat)
    (member : item ∈ s.children) :
    (object ∈ (restore s item).state.children ∨ object ∈ (restore s item).state.direct) ↔
    (object ∈ s.children ∨ object ∈ s.direct) := by
  by_cases same : object = item
  · subst object
    simp [restore, member]
  · simp [restore, same]

/-- The checked distinct parent is not returned in the item's place. -/
theorem the_distinct_parent_is_not_the_result (s : State) (item : Nat)
    (distinct : item ≠ s.parent) :
    (restore s item).result ≠ some s.parent := by
  simp [restore, distinct]

/-! The physical link writes, over a finite list's already checked local
neighbour identities. `prev` is head.prev (the tail) at the first node, and
otherwise the predecessor. `next` is the successor or NULL. The representation
premise establishes these readings; this model does not assume the unlink
conclusion. The Rust concrete adapter executes this same order. -/
structure Links where
  prev : Option Nat
  next : Option Nat
  deriving DecidableEq, Repr

def putPrev (nodes : Nat → Links) (target : Nat) (value : Option Nat) : Nat → Links :=
  fun object => if object = target then { nodes object with prev := value } else nodes object

def putNext (nodes : Nat → Links) (target : Nat) (value : Option Nat) : Nat → Links :=
  fun object => if object = target then { nodes object with next := value } else nodes object

def physicalPatch (nodes : Nat → Links) (head item prev : Nat) (next : Option Nat) :
    Option Nat × (Nat → Links) :=
  let first := if item = head then nodes else putNext nodes prev next
  let second := match next with
    | none => first
    | some successor => putPrev first successor (some prev)
  let third := if item ≠ head ∧ next = none then putPrev second head (some prev) else second
  let cleared := putNext (putPrev third item none) item none
  (if item = head then next else some head, cleared)

theorem the_item_links_are_cleared (nodes : Nat → Links) (head item prev : Nat)
    (next : Option Nat) :
    (physicalPatch nodes head item prev next).2 item = ⟨none, none⟩ := by
  simp [physicalPatch, putNext, putPrev]

theorem the_head_changes_only_when_detaching_the_first (nodes : Nat → Links)
    (head item prev : Nat) (next : Option Nat) :
    (physicalPatch nodes head item prev next).1 =
      (if item = head then next else some head) := rfl

theorem the_successor_links_to_the_old_predecessor_or_tail (nodes : Nat → Links)
    (head item prev successor : Nat) (distinct : successor ≠ item) :
    ((physicalPatch nodes head item prev (some successor)).2 successor).prev = some prev := by
  simp [physicalPatch, putNext, putPrev, distinct]

theorem detaching_the_tail_updates_the_head_tail_link (nodes : Nat → Links)
    (head item prev : Nat) (notHead : item ≠ head) :
    ((physicalPatch nodes head item prev none).2 head).prev = some prev := by
  simp [physicalPatch, putNext, putPrev, notHead, Ne.symm notHead]

theorem every_object_outside_the_write_frame_is_preserved (nodes : Nat → Links)
    (head item prev object : Nat) (next : Option Nat)
    (notHead : object ≠ head) (notItem : object ≠ item) (notPrev : object ≠ prev)
    (notNext : ∀ successor, next = some successor → object ≠ successor) :
    (physicalPatch nodes head item prev next).2 object = nodes object := by
  cases next with
  | none =>
    by_cases first : item = head <;>
      simp [physicalPatch, putNext, putPrev, first, notHead, notItem, notPrev]
  | some successor =>
    have different := notNext successor rfl
    by_cases first : item = head <;>
      simp [physicalPatch, putNext, putPrev, first, notHead, notItem, notPrev, different]

end QCore.DetachSameItem
