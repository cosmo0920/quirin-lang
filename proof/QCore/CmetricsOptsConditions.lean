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

import QCore.CmetricsHypotheses

/-! Conditional representation/trace model for opts postconditions. Field and
allocation-epoch correspondence, the five-field inventory, helper behavior,
exclusion, physical separation and complete call traces require independent
bindings. No C permission is established by this mathematical model. -/
namespace QCore.CmetricsOptsConditions
open CmetricsHypotheses

structure State where
  stored : SdsField → Option Nat
  pointerBytes : SdsField → List UInt8
  indexBytes : List UInt8
  recordLive : Bool
  live : Nat → Bool
  owned : Nat → Bool

/-- The caller supplies all five occupied fields in actual source order, with
no duplicate allocation epochs. This function does not derive the inventory. -/
def cleanup (before : State) (released : List Nat) : State :=
  { before with
    live := fun handle => if handle ∈ released then false else before.live handle
    owned := fun handle => if handle ∈ released then false else before.owned handle }

theorem cleanup_preserves_raw_representation (before : State) (released : List Nat) :
    (cleanup before released).pointerBytes = before.pointerBytes ∧
    (cleanup before released).indexBytes = before.indexBytes ∧
    (cleanup before released).stored = before.stored ∧
    (cleanup before released).recordLive = before.recordLive := by
  exact ⟨rfl, rfl, rfl, rfl⟩

theorem selected_contents_retire (before : State) (released : List Nat)
    (field : SdsField) (handle : Nat) (stored : before.stored field = some handle)
    (complete : ∀ f h, before.stored f = some h → h ∈ released) :
    (cleanup before released).live handle = false ∧
    (cleanup before released).owned handle = false := by
  have member := complete field handle stored
  simp [cleanup, member]

def aliasUsable (state : State) (epoch : Nat) : Bool := state.live epoch

theorem old_alias_cannot_revive_from_unchanged_pointer_bytes (before : State)
    (released : List Nat) (epoch : Nat) (retired : epoch ∈ released) :
    aliasUsable (cleanup before released) epoch = false ∧
    (cleanup before released).pointerBytes = before.pointerBytes := by
  simp [aliasUsable, cleanup, retired]

def FailedAppendFrame (eventBefore eventAfter : State) : Prop :=
  eventAfter.stored .fqname = eventBefore.stored .fqname ∧
  eventAfter.pointerBytes .fqname = eventBefore.pointerBytes .fqname

/-- Failed realloc/append helper behavior is a premise. The caller commits the
returned handle only on success, preserving the old fqname on a failed append. -/
def appendCommit (before : State) (value : Option Nat) (bytes : List UInt8) : State :=
  match value with
  | none => before
  | some handle => { before with
      stored := fun field => if field = .fqname then some handle else before.stored field
      pointerBytes := fun field => if field = .fqname then bytes else before.pointerBytes field }

theorem failed_append_has_no_field_commit (before : State) (bytes : List UInt8) :
    FailedAppendFrame before (appendCommit before none bytes) := by
  exact ⟨rfl, rfl⟩

theorem append_preserves_index_representation (before : State)
    (value : Option Nat) (bytes : List UInt8) :
    (appendCommit before value bytes).indexBytes = before.indexBytes := by
  cases value <;> rfl

def RetryReady (state : State) : Prop :=
  ∀ field handle, state.stored field = some handle → state.live handle = true

theorem nonempty_cleanup_needs_new_liveness (before : State) (released : List Nat)
    (field : SdsField) (handle : Nat) (stored : before.stored field = some handle)
    (retired : handle ∈ released) : ¬ RetryReady (cleanup before released) := by
  intro ready
  have live := ready field handle stored
  simp [cleanup, retired] at live

/-- A complete event inventory has exactly one event per occupied field when
it is an injective enumeration. Events and their source binding remain inputs. -/
theorem exact_release_event (events : List Event) (handle : Handle)
    (selected : Handle → Event) (belongs : selected handle ∈ events)
    (releases : Event → Handle → Prop)
    (classification : ∀ event, releases event handle ↔ event = selected handle) :
    ∃ event ∈ events, releases event handle ∧
      ∀ other ∈ events, releases other handle → other = event := by
  exact ⟨selected handle, belongs, (classification (selected handle)).mpr rfl,
    fun other _ hit => (classification other).mp hit⟩

end QCore.CmetricsOptsConditions
