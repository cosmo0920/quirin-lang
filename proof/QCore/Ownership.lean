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

namespace QCore.Ownership

abbrev ResourceId := Nat
abbrev Owner := Nat

structure ResourceEntry where
  id : ResourceId
  owner : Owner
  footprint : QCore.QuantVector

abbrev Ledger := List ResourceEntry

namespace Ledger

def ids (ledger : Ledger) : List ResourceId := ledger.map ResourceEntry.id

def Valid (ledger : Ledger) : Prop := ledger.ids.Nodup

def live : Ledger -> QCore.QuantVector
  | [] => QCore.QuantVector.zero
  | entry :: rest => QCore.QuantVector.add entry.footprint (live rest)

def acquire (entry : ResourceEntry) (ledger : Ledger) : Ledger := entry :: ledger

def moveOwner (resource : ResourceId) (newOwner : Owner) : Ledger -> Ledger
  | [] => []
  | entry :: rest =>
      (if entry.id = resource then { entry with owner := newOwner } else entry) ::
        moveOwner resource newOwner rest

def releaseAt (before after : Ledger) : Ledger := before ++ after

theorem live_append (left right : Ledger) :
    live (left ++ right) = QCore.QuantVector.add (live left) (live right) := by
  funext axis
  induction left with
  | nil => simp [live, QCore.QuantVector.zero, QCore.QuantVector.add]
  | cons entry rest ih =>
      simp [live, QCore.QuantVector.add, ih, Nat.add_assoc]

theorem acquire_preserves_valid (entry : ResourceEntry) (ledger : Ledger)
    (fresh : entry.id ∉ ledger.ids) (valid : ledger.Valid) :
    (acquire entry ledger).Valid := by
  simpa [Valid, ids, acquire] using List.nodup_cons.mpr ⟨fresh, valid⟩

theorem live_acquire (entry : ResourceEntry) (ledger : Ledger) :
    live (acquire entry ledger) =
      QCore.QuantVector.add entry.footprint (live ledger) := by
  rfl

theorem move_preserves_ids (resource : ResourceId) (newOwner : Owner)
    (ledger : Ledger) :
    (moveOwner resource newOwner ledger).ids = ledger.ids := by
  induction ledger with
  | nil => rfl
  | cons entry rest ih =>
      simp only [moveOwner, ids, List.map_cons]
      split <;> simp_all [ids]

theorem move_preserves_valid (resource : ResourceId) (newOwner : Owner)
    (ledger : Ledger) (valid : ledger.Valid) :
    (moveOwner resource newOwner ledger).Valid := by
  simpa [Valid, move_preserves_ids] using valid

theorem move_preserves_live (resource : ResourceId) (newOwner : Owner)
    (ledger : Ledger) :
    live (moveOwner resource newOwner ledger) = live ledger := by
  funext axis
  induction ledger with
  | nil => rfl
  | cons entry rest ih =>
      simp only [moveOwner, live, QCore.QuantVector.add]
      split <;> simp_all

theorem release_preserves_valid (before after : Ledger) (entry : ResourceEntry)
    (valid : (before ++ entry :: after).Valid) :
    (releaseAt before after).Valid := by
  induction before with
  | nil =>
      have source : (entry.id :: after.ids).Nodup := by
        simpa [Valid, ids] using valid
      simpa [Valid, ids, releaseAt] using (List.nodup_cons.mp source).2
  | cons head tail ih =>
      have source : (head.id :: ids (tail ++ entry :: after)).Nodup := by
        simpa [Valid, ids] using valid
      have parts := List.nodup_cons.mp source
      have tailValid : Valid (tail ++ entry :: after) := by
        simpa [Valid, ids] using parts.2
      have targetTail := ih tailValid
      have headFresh : head.id ∉ ids (tail ++ after) := by
        intro member
        apply parts.1
        simp [ids, List.mem_append] at member ⊢
        rcases member with inTail | inAfter
        · exact Or.inl inTail
        · exact Or.inr (Or.inr inAfter)
      simpa [Valid, ids, releaseAt] using
        (List.nodup_cons.mpr ⟨headFresh, targetTail⟩)

theorem released_identity_absent (before after : Ledger) (entry : ResourceEntry)
    (valid : (before ++ entry :: after).Valid) :
    entry.id ∉ (releaseAt before after).ids := by
  induction before with
  | nil =>
      have source : (entry.id :: after.ids).Nodup := by
        simpa [Valid, ids] using valid
      simpa [releaseAt, ids] using (List.nodup_cons.mp source).1
  | cons head tail ih =>
      have source : (head.id :: ids (tail ++ entry :: after)).Nodup := by
        simpa [Valid, ids] using valid
      have parts := List.nodup_cons.mp source
      have tailValid : Valid (tail ++ entry :: after) := by
        simpa [Valid, ids] using parts.2
      have absentTail := ih tailValid
      intro member
      have splitMember : entry.id = head.id ∨ entry.id ∈ ids (tail ++ after) := by
        simpa [releaseAt, ids] using member
      rcases splitMember with same | inTail
      · apply parts.1
        have entryMember : entry.id ∈ ids (tail ++ entry :: after) := by
          simp [ids]
        exact same ▸ entryMember
      · exact absentTail inTail

theorem release_accounts_exact_footprint (before after : Ledger)
    (entry : ResourceEntry) :
    live (before ++ entry :: after) =
      QCore.QuantVector.add entry.footprint (live (releaseAt before after)) := by
  simp only [releaseAt]
  rw [live_append before (entry :: after), live_append before after]
  funext axis
  simp [live, QCore.QuantVector.add, Nat.add_assoc, Nat.add_comm]

end Ledger

inductive BorrowKind where
  | shared
  | mutable
  deriving DecidableEq

structure Loan where
  resource : ResourceId
  kind : BorrowKind
  deriving DecidableEq

abbrev Loans := List Loan

namespace Loans

def HasLoan (loans : Loans) (resource : ResourceId) : Prop :=
  ∃ loan ∈ loans, loan.resource = resource

def CanMove (loans : Loans) (resource : ResourceId) : Prop :=
  ¬loans.HasLoan resource

def CanDrop (loans : Loans) (resource : ResourceId) : Prop :=
  ¬loans.HasLoan resource

def CanMutablyBorrow (loans : Loans) (resource : ResourceId) : Prop :=
  ¬loans.HasLoan resource

def beginShared (resource : ResourceId) (loans : Loans) : Loans :=
  { resource := resource, kind := .shared } :: loans

def beginMutable (resource : ResourceId) (loans : Loans) : Loans :=
  { resource := resource, kind := .mutable } :: loans

theorem shared_blocks_move (resource : ResourceId) (loans : Loans) :
    ¬CanMove (beginShared resource loans) resource := by
  intro canMove
  apply canMove
  exact ⟨{ resource := resource, kind := .shared }, by simp [beginShared], rfl⟩

theorem shared_blocks_drop (resource : ResourceId) (loans : Loans) :
    ¬CanDrop (beginShared resource loans) resource := by
  intro canDrop
  apply canDrop
  exact ⟨{ resource := resource, kind := .shared }, by simp [beginShared], rfl⟩

theorem mutable_blocks_move (resource : ResourceId) (loans : Loans) :
    ¬CanMove (beginMutable resource loans) resource := by
  intro canMove
  apply canMove
  exact ⟨{ resource := resource, kind := .mutable }, by simp [beginMutable], rfl⟩

theorem mutable_blocks_drop (resource : ResourceId) (loans : Loans) :
    ¬CanDrop (beginMutable resource loans) resource := by
  intro canDrop
  apply canDrop
  exact ⟨{ resource := resource, kind := .mutable }, by simp [beginMutable], rfl⟩

theorem mutable_borrow_is_exclusive (resource : ResourceId) (loans : Loans)
    (available : CanMutablyBorrow loans resource) (loan : Loan)
    (member : loan ∈ beginMutable resource loans) (same : loan.resource = resource) :
    loan = { resource := resource, kind := .mutable } := by
  simp only [beginMutable, List.mem_cons] at member
  rcases member with head | tail
  · exact head
  · exact False.elim (available ⟨loan, tail, same⟩)

end Loans

def NormalExit (ledger : Ledger) : Prop := ledger = []

theorem normal_exit_has_zero_live (ledger : Ledger) (normal : NormalExit ledger) :
    Ledger.live ledger = QCore.QuantVector.zero := by
  subst ledger
  rfl

end QCore.Ownership
