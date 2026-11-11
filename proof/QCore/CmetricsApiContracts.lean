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

import QCore.TrackedSelfInit

/-! Declared CMetrics API lifecycle models. Source extraction, representation,
SDS allocation/reallocation, synchronization and Rust/model correspondence are
premises. These definitions confer no resource-v1 call permission. -/
namespace QCore.CmetricsApiContracts

structure Opts where
  storageLive : Bool := true
  stored : List Nat := []
  owned : List Nat := []
  deriving DecidableEq, Repr

/-- Failure preserves committed handles; the caller must still clean them up. -/
def partialInit (committed : List Nat) : Opts :=
  { stored := committed, owned := committed }

def canExit (o : Opts) : Bool :=
  o.storageLive && decide o.stored.Nodup &&
    o.stored.all (fun id => o.owned.contains id)

def cleanup (o : Opts) : Option Opts :=
  if canExit o then some { o with owned := [] } else none

theorem failure_keeps_committed_obligations (ids : List Nat) (id : Nat)
    (h : id ∈ ids) : id ∈ (partialInit ids).owned := by
  exact h

theorem cleanup_discharges_contents (o out : Opts) (h : cleanup o = some out) :
    out.owned = [] := by
  unfold cleanup at h
  split at h
  · injection h with hout
    cases hout
    rfl
  · contradiction

theorem cleanup_preserves_enclosing_storage (o out : Opts)
    (h : cleanup o = some out) : out.storageLive = o.storageLive := by
  unfold cleanup at h
  split at h
  · injection h with hout
    cases hout
    rfl
  · contradiction

theorem cleanup_does_not_clear_pointer_fields (o out : Opts)
    (h : cleanup o = some out) : out.stored = o.stored := by
  unfold cleanup at h
  split at h
  · injection h with hout
    cases hout
    rfl
  · contradiction

theorem a_nonempty_cleanup_is_not_generally_idempotent (o : Opts)
    (nonempty : o.stored ≠ []) : (cleanup o).bind cleanup = none := by
  have emptyCan : canExit { o with owned := [] } = false := by
    cases fields : o.stored with
    | nil => exact False.elim (nonempty fields)
    | cons id rest => simp [canExit]
  by_cases h : canExit o = true
  · simp [cleanup, h, emptyCan]
  · simp [cleanup, h]

structure MapState where
  live : Bool := true
  epoch : Nat := 0
  primary : List Nat := []
  secondary : List Nat := []
  cached : Option Nat := none
  deriving DecidableEq, Repr

structure Borrow where
  epoch : Nat
  child : Option Nat
  deriving DecidableEq, Repr

/-- none denotes the embedded record; some denotes a primary-owned child. -/
def borrowValid (m : MapState) (b : Borrow) : Bool :=
  m.live && (b.epoch == m.epoch) &&
    (match b.child with | none => true | some id => m.primary.contains id)

def attach (m : MapState) (id : Nat) : MapState :=
  { m with primary := id :: m.primary }

/-- Storage preparation can fail after a newly created metric was attached. -/
def failedGetAfterAttach (m : MapState) (id : Nat) : MapState × Option Borrow :=
  (attach m id, none)

def destroy (m : MapState) : MapState :=
  { m with live := false, epoch := m.epoch + 1,
           primary := [], secondary := [], cached := none }

/-- Conservative abstract invalidation: retirement ends all older borrow epochs. -/
def expire (m : MapState) (retired : List Nat) : MapState :=
  { m with epoch := m.epoch + 1,
           primary := m.primary.filter (fun id => !retired.contains id),
           secondary := m.secondary.filter (fun id => !retired.contains id),
           cached := m.cached.filter (fun id => !retired.contains id) }

theorem getter_failure_can_keep_new_map_owned_metric (m : MapState) (id : Nat) :
    id ∈ (failedGetAfterAttach m id).1.primary ∧
      (failedGetAfterAttach m id).2 = none := by
  simp [failedGetAfterAttach, attach]

theorem attaching_does_not_retire_the_parent (m : MapState) (id : Nat) :
    (attach m id).live = m.live := by rfl

theorem attaching_preserves_old_primary_members (m : MapState) (id old : Nat)
    (h : old ∈ m.primary) : old ∈ (attach m id).primary := by
  simp [attach, h]

theorem embedded_result_needs_live_map (m : MapState) (e : Nat)
    (h : m.live = false) : borrowValid m ⟨e, none⟩ = false := by
  simp [borrowValid, h]

theorem destroy_invalidates_every_metric_borrow (m : MapState) (b : Borrow) :
    borrowValid (destroy m) b = false := by
  simp [borrowValid, destroy]

theorem destroy_discharges_primary_ownership (m : MapState) :
    (destroy m).primary = [] ∧ (destroy m).secondary = [] ∧
      (destroy m).cached = none := by
  simp [destroy]

theorem expiration_preserves_the_enclosing_map (m : MapState) (ids : List Nat) :
    (expire m ids).live = m.live := by rfl

theorem expiration_invalidates_an_old_borrow_epoch (m : MapState)
    (ids : List Nat) (b : Borrow) (h : b.epoch = m.epoch) :
    borrowValid (expire m ids) b = false := by
  simp [borrowValid, expire, h]

/-- Declared model inspection never supplies an executable contract permission. -/
def modelPermission (_sourceParsed : Bool) : Bool := false

theorem parsed_model_is_not_a_trusted_call (b : Bool) : modelPermission b = false := by rfl

def vectors : List Bool :=
  [canExit (partialInit [1,2]),
   canExit { (partialInit [1,2]) with owned := [] },
   canExit {},
   borrowValid {} ⟨0, none⟩,
   borrowValid { primary := [8] } ⟨0, some 8⟩,
   borrowValid { primary := [8] } ⟨0, some 9⟩,
   borrowValid (destroy { primary := [8] }) ⟨0, some 8⟩,
   borrowValid (expire { primary := [8] } [8]) ⟨0, some 8⟩,
   (failedGetAfterAttach {} 8).1.primary.contains 8,
   modelPermission true,
   canExit (partialInit [1,1])]

theorem shared_vectors : vectors =
    [true,false,true,true,true,false,false,false,true,false,false] := by decide


/-- Setter values are abstract bit-pattern tokens, not a C float refinement. -/
structure SetterState where
  map : MapState := {}
  value : Nat := 0
  timestamp : Nat := 0
  arrays : List Nat := []
  retired : List Nat := []
  quantilesSet : Bool := false
  deriving DecidableEq, Repr

/-- Lookup effects are already committed even when the later setter fails. -/
def counterSet (afterLookup : SetterState) (lookupOK allowReset : Bool)
    (value timestamp : Nat) : SetterState × Int :=
  if !lookupOK || (!allowReset && decide (value < afterLookup.value)) then
    (afterLookup, -1)
  else ({ afterLookup with value := value, timestamp := timestamp }, 0)

/-- Fresh/distinct replacement ownership and preparation cleanup are premises.
External coordination excludes conflicting metric users through return.
No new arrays escape on failure; successful replacement retires old aliases. -/
def replaceArrays (afterLookup : SetterState) (prepared : Bool)
    (fresh : List Nat) (timestamp : Nat) : SetterState × Int :=
  if prepared then
    ({ afterLookup with arrays := fresh, retired := afterLookup.arrays ++ afterLookup.retired, timestamp := timestamp }, 0)
  else (afterLookup, -1)

def arrayBorrowValid (s : SetterState) (id : Nat) : Bool :=
  s.map.live && s.arrays.contains id && !s.retired.contains id

/-- NULL quantile input preserves the flag; preparation is modeled separately. -/
def summarySet (afterLookup : SetterState) (hasQuantiles : Bool)
    (value timestamp : Nat) : SetterState :=
  { afterLookup with value := value, timestamp := timestamp, quantilesSet := afterLookup.quantilesSet || hasQuantiles }

theorem refused_counter_keeps_lookup_effects (s : SetterState) (ok reset : Bool)
    (v t : Nat) (h : (!ok || (!reset && decide (v < s.value))) = true) :
    counterSet s ok reset v t = (s, -1) := by
  simp only [counterSet, h, ite_true]

theorem counter_success_preserves_map_ownership (s : SetterState) (v t : Nat) :
    (counterSet s true true v t).1.map = s.map := by
  simp [counterSet]

theorem preparation_failure_preserves_old_array_ownership (s : SetterState)
    (fresh : List Nat) (t : Nat) :
    replaceArrays s false fresh t = (s, -1) := by
  simp [replaceArrays]

theorem replacement_retires_every_old_array (s : SetterState) (fresh : List Nat)
    (t id : Nat) (h : id ∈ s.arrays) :
    id ∈ (replaceArrays s true fresh t).1.retired := by
  simp [replaceArrays, h]

theorem replacement_invalidates_old_array_borrow (s : SetterState)
    (fresh : List Nat) (t id : Nat) (h : id ∈ s.arrays) :
    arrayBorrowValid (replaceArrays s true fresh t).1 id = false := by
  simp [arrayBorrowValid, replaceArrays, h]

theorem replacement_preserves_metric_parent (s : SetterState) (fresh : List Nat)
    (t : Nat) : (replaceArrays s true fresh t).1.map = s.map := by
  simp [replaceArrays]

theorem null_quantile_input_preserves_flag (s : SetterState) (v t : Nat) :
    (summarySet s false v t).quantilesSet = s.quantilesSet := by
  simp [summarySet]

def setterVectors : List Bool :=
  [((counterSet { value := 7 } true false 2 9).2 == -1),
   ((counterSet { value := 7 } true true 2 9).2 == 0),
   ((counterSet { value := 7 } false true 2 9).1.value == 7),
   ((replaceArrays { arrays := [1,2] } false [3,4] 9).1.arrays == [1,2]),
   arrayBorrowValid (replaceArrays { arrays := [1,2] } true [3,4] 9).1 1,
   arrayBorrowValid (replaceArrays { arrays := [1,2] } true [3,4] 9).1 3,
   (summarySet { quantilesSet := true } false 2 9).quantilesSet,
   (summarySet {} false 2 9).quantilesSet,
   (summarySet {} true 2 9).quantilesSet]

theorem shared_setter_vectors : setterVectors =
    [true,true,true,true,false,true,true,false,true] := by decide

end QCore.CmetricsApiContracts
