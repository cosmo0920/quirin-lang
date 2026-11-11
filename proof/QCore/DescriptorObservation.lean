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

namespace QCore.DescriptorObservation

/-! Executable generation-slot projection of native descriptor observations.
This models one descriptor number. It does not authenticate observations or
prove the Rust/JSON bridge, C callbacks, or platform close-failure semantics. -/

structure Slot where
  generation : Nat := 0
  live : Bool := false
  deriving DecidableEq, Repr

def acquire (slot : Slot) (generation : Nat) : Option Slot :=
  if slot.live = false ∧ generation = slot.generation + 1 then
    some { generation, live := true }
  else none

def useDescriptor (slot : Slot) (generation : Nat) : Bool :=
  slot.live && decide (generation = slot.generation)

-- Only a recorded successful close is accepted by the native projection.
-- Failed native closes are unresolved, not assumed to preserve custody.
def close (slot : Slot) (generation : Nat) (succeeded : Bool) : Option Slot :=
  if useDescriptor slot generation && succeeded then
    some { slot with live := false }
  else none

theorem accepted_acquisition_advances_generation
    {slot next : Slot} {generation : Nat}
    (accepted : acquire slot generation = some next) :
    slot.live = false ∧ generation = slot.generation + 1 ∧
      next.generation = generation ∧ next.live = true := by
  unfold acquire at accepted
  split at accepted
  · rename_i valid
    cases accepted
    exact ⟨valid.1, valid.2, rfl, rfl⟩
  · contradiction

theorem stale_generation_cannot_be_used (slot : Slot) (generation : Nat)
    (stale : generation ≠ slot.generation) :
    useDescriptor slot generation = false := by
  simp [useDescriptor, stale]

theorem accepted_close_discharges_slot
    {slot next : Slot} {generation : Nat} {succeeded : Bool}
    (accepted : close slot generation succeeded = some next) :
    next.live = false ∧ next.generation = slot.generation := by
  unfold close at accepted
  split at accepted
  · cases accepted
    exact ⟨rfl, rfl⟩
  · contradiction

theorem failed_native_close_is_unresolved (slot : Slot) (generation : Nat) :
    close slot generation false = none := by
  simp [close]

theorem accepted_close_prevents_duplicate_close
    {slot next : Slot} {generation : Nat}
    (accepted : close slot generation true = some next) :
    close next generation true = none := by
  have discharged := (accepted_close_discharges_slot accepted).1
  simp [close, useDescriptor, discharged]

theorem reuse_rejects_previous_generation (generation : Nat) :
    useDescriptor { generation := generation + 1, live := true } generation = false := by
  simp [useDescriptor]

end QCore.DescriptorObservation
