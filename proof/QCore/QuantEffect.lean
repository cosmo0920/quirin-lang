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

/-! Conditional models for one operation's own effects. These definitions do not
establish atomic execution, memory order, synchronization, actual C behavior,
source binding or evidence. Other threads' events need their own footprints;
an operation's frame does not constrain concurrent interference. -/
namespace QCore.QuantEffect

abbrev State (Object : Type u) (Value : Type v) := Object → Value

def PreservesOutside (writes : Object → Prop) (before after : State Object Value) : Prop :=
  ∀ object, ¬ writes object → after object = before object

structure Model (Object : Type u) (Value : Type v) where
  guard : State Object Value → Prop
  reads : Object → Prop
  writes : Object → Prop
  frame : State Object Value → State Object Value → Prop
  transition : State Object Value → State Object Value → Prop

/-- Conditional relation, not an assertion that a step actually occurs. -/
def Admitted (model : Model Object Value) (before after : State Object Value) : Prop :=
  model.guard before ∧ model.frame before after ∧ model.transition before after ∧
    PreservesOutside model.writes before after

def guarded (condition : State Object Value → Prop) (model : Model Object Value) : Model Object Value :=
  { model with guard := fun before => condition before ∧ model.guard before }

theorem guarded_relation_decomposition
    (condition : State Object Value → Prop) (model : Model Object Value)
    (before after : State Object Value) :
    Admitted (guarded condition model) before after ↔ condition before ∧ Admitted model before after := by
  constructor
  · rintro ⟨⟨extra, base⟩, frame, transition, outside⟩
    exact ⟨extra, base, frame, transition, outside⟩
  · rintro ⟨extra, base, frame, transition, outside⟩
    exact ⟨⟨extra, base⟩, frame, transition, outside⟩

theorem absent_guard_refuses_step
    (model : Model Object Value) (before after : State Object Value)
    (absent : ¬ model.guard before) : ¬ Admitted model before after := by
  intro step
  exact absent step.1

/-- Sequential frames compose under the union of both operations' writes. -/
theorem frame_composition
    (firstWrites secondWrites : Object → Prop) (before middle after : State Object Value)
    (first : PreservesOutside firstWrites before middle)
    (second : PreservesOutside secondWrites middle after) :
    PreservesOutside (fun object => firstWrites object ∨ secondWrites object) before after := by
  intro object outside
  have outsideFirst : ¬ firstWrites object := fun written => outside (Or.inl written)
  have outsideSecond : ¬ secondWrites object := fun written => outside (Or.inr written)
  exact (second object outsideSecond).trans (first object outsideFirst)

/-- A separate interfering event must contribute its own writes to the frame. -/
theorem operation_then_interference_frame
    (operation : Model Object Value) (otherWrites : Object → Prop)
    (before ownAfter after : State Object Value)
    (ownStep : Admitted operation before ownAfter)
    (otherEvent : PreservesOutside otherWrites ownAfter after) :
    PreservesOutside (fun object => operation.writes object ∨ otherWrites object) before after := by
  exact frame_composition operation.writes otherWrites before ownAfter after ownStep.2.2.2 otherEvent

/-- Even an empty own footprint says nothing about another event's writes. -/
theorem own_frame_does_not_constrain_other_event :
    PreservesOutside (fun _ : Nat => False) (fun _ => (0 : Int)) (fun _ => (0 : Int)) ∧
      ¬ PreservesOutside (fun _ : Nat => False) (fun _ => (0 : Int))
        (fun object => if object = 0 then (1 : Int) else 0) := by
  constructor
  · intro _ _
    rfl
  · intro unrestricted
    have impossible := unrestricted 0 (by simp)
    simp at impossible

end QCore.QuantEffect
