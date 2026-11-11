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

import QCore.HolderFrameIdentity
/-! First-match owner/key selection and constructor-local conditional loop
identity. Source extraction, list well-formedness and memory validity are
premises of a source application. No ambient invariance follows from a
constructor snapshot or a numeric key. -/
namespace QCore.CollectorIdentity
structure Record where
  object : Nat
  owner : Nat
  key : Nat
  loop : Nat
  deriving DecidableEq, Repr

def scan (key : Nat) : List Record → Option Record
  | [] => none
  | first :: rest => if first.key = key then some first else scan key rest

def select (owner key : Nat) (lists : Nat → List Record) : Option Record := scan key (lists owner)

theorem selection_keeps_the_exact_key (key : Nat) (records : List Record)
    (selected : Record) (found : scan key records = some selected) : selected.key = key := by
  induction records with
  | nil => simp [scan] at found
  | cons first rest ih =>
    by_cases matching : first.key = key
    · simp [scan, matching] at found
      subst selected
      exact matching
    · simp [scan, matching] at found
      exact ih found

theorem selection_returns_an_actual_entry (key : Nat) (records : List Record)
    (selected : Record) (found : scan key records = some selected) : selected ∈ records := by
  induction records with
  | nil => simp [scan] at found
  | cons first rest ih =>
    by_cases matching : first.key = key
    · simp [scan, matching] at found
      simp [← found]
    · simp [scan, matching] at found
      exact List.mem_cons_of_mem first (ih found)

theorem the_first_match_wins_with_duplicate_keys (first second : Record) (rest : List Record)
    (key : Nat) (matching : first.key = key) :
    scan key (first :: second :: rest) = some first := by simp [scan, matching]

theorem an_empty_list_has_no_selected_record (key : Nat) : scan key [] = none := rfl

theorem selection_uses_the_exact_owner_list (owner key : Nat) (lists : Nat → List Record)
    (selected : Record) (found : select owner key lists = some selected) :
    selected ∈ lists owner := selection_returns_an_actual_entry key (lists owner) selected found

theorem another_key_cannot_supply_the_selected_record (key : Nat) (records : List Record)
    (selected : Record) (other : selected.key ≠ key) : scan key records ≠ some selected := by
  intro found
  exact other (selection_keeps_the_exact_key key records selected found)

theorem equal_keys_do_not_identify_records :
    let first : Record := ⟨1, 3, 7, 10⟩
    let second : Record := ⟨2, 3, 7, 20⟩
    first.key = second.key ∧ first ≠ second := by decide

theorem changing_the_list_can_change_the_selected_record :
    let first : Record := ⟨1, 3, 7, 10⟩
    let second : Record := ⟨2, 3, 7, 20⟩
    scan 7 [first, second] ≠ scan 7 [second, first] := by decide

theorem list_selection_alone_does_not_prove_the_instance_member :
    let record : Record := ⟨1, 99, 7, 10⟩
    let lists : Nat → List Record := fun _ => [record]
    select 3 7 lists = some record ∧ record.owner ≠ 3 := by decide

theorem a_separate_membership_invariant_establishes_the_instance
    (owner key : Nat) (lists : Nat → List Record) (selected : Record)
    (found : select owner key lists = some selected)
    (membership : ∀ entry ∈ lists owner, entry.owner = owner) : selected.owner = owner :=
    membership selected (selection_uses_the_exact_owner_list owner key lists selected found)

structure Snapshot where
  ownerRoot : Nat
  threaded : Bool
  loop : Nat
  deriving DecidableEq, Repr

def construct (ownerRoot workerLoop configLoop : Nat) (threaded : Bool) : Snapshot :=
  ⟨ownerRoot, threaded, if threaded then workerLoop else configLoop⟩

theorem construction_keeps_the_exact_instance (ownerRoot worker config : Nat) (threaded : Bool) :
    (construct ownerRoot worker config threaded).ownerRoot = ownerRoot := rfl

theorem the_threaded_snapshot_uses_that_workers_loop (ownerRoot worker config : Nat) :
    (construct ownerRoot worker config true).loop = worker := rfl

theorem the_nonthreaded_snapshot_uses_the_config_actual (ownerRoot worker config : Nat) :
    (construct ownerRoot worker config false).loop = config := rfl

theorem a_different_config_is_not_the_instance_config (ownerRoot worker config instanceConfig : Nat)
    (different : config ≠ instanceConfig) :
    (construct ownerRoot worker config false).loop ≠ instanceConfig := different

theorem construction_does_not_prove_the_future_loop :
    let made := construct 1 10 20 false
    let later : Snapshot := { made with loop := 30 }
    made.loop ≠ later.loop := by decide

theorem a_changed_flag_does_not_reconstruct_the_loop :
    let made := construct 1 10 20 false
    let changed : Snapshot := { made with threaded := true }
    changed.loop ≠ (construct 1 10 20 changed.threaded).loop := by decide

def reusable (sourceReplay sameOwner sameKey listStable recordStable flagStable : Bool) : Bool :=
  sourceReplay && sameOwner && sameKey && listStable && recordStable && flagStable

theorem replay_requires_all_identity_and_stability_obligations (a b c d e f : Bool)
    (accepted : reusable a b c d e f = true) :
    a = true ∧ b = true ∧ c = true ∧ d = true ∧ e = true ∧ f = true := by simp_all [reusable]

theorem an_unstable_list_supplies_no_cross_call_identity (a b c e f : Bool) :
    reusable a b c false e f = false := by simp [reusable]

theorem an_unstable_record_supplies_no_persistent_loop_identity (a b c d f : Bool) :
    reusable a b c d false f = false := by simp [reusable]

theorem an_unstable_flag_supplies_no_conditional_reuse (a b c d e : Bool) :
    reusable a b c d e false = false := by simp [reusable]
theorem a_disjoint_subobject_frame_preserves_both_outer_members
    (heap : QCore.HolderFrameIdentity.Heap)
    (ownerKey loopKey : QCore.HolderFrameIdentity.Key)
    (writes : List (QCore.HolderFrameIdentity.Key × Nat))
    (ownerClosed : ∀ write ∈ writes, write.1 ≠ ownerKey)
    (loopClosed : ∀ write ∈ writes, write.1 ≠ loopKey) :
    QCore.HolderFrameIdentity.run heap writes ownerKey = heap ownerKey ∧
    QCore.HolderFrameIdentity.run heap writes loopKey = heap loopKey :=
  ⟨QCore.HolderFrameIdentity.a_closed_frame_preserves_the_exact_holder heap ownerKey writes ownerClosed,
   QCore.HolderFrameIdentity.a_closed_frame_preserves_the_exact_holder heap loopKey writes loopClosed⟩

end QCore.CollectorIdentity
