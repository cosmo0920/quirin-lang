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

import QCore.QuantCondition
import QCore.ContractViewProjection

/-! Conditional integration boundaries. Typed entry predicates remain assumed
unless an independent source/model interpretation discharges them. The legacy
arithmetic entailer does not interpret relation parameters, graph versions do
not supply evidence, and neutral metadata attachment supplies no C permission. -/
namespace QCore.QuantIntegrationBoundaries

/-- A projection retains both legacy and explicit typed entry predicates. -/
def viewGuard {State : Type u} (legacy typed : List (State → Prop)) (state : State) : Prop :=
  ∀ predicate ∈ legacy ++ typed, predicate state

theorem typed_entry_obligations_remain_required {State : Type u}
    (legacy typed : List (State → Prop)) (state : State)
    (guard : viewGuard legacy typed state) (predicate : State → Prop)
    (entry : predicate ∈ typed) : predicate state :=
  guard predicate (List.mem_append.mpr (Or.inr entry))

theorem legacy_obligations_are_preserved {State : Type u}
    (legacy typed : List (State → Prop)) (state : State)
    (guard : viewGuard legacy typed state) (predicate : State → Prop)
    (entry : predicate ∈ legacy) : predicate state :=
  guard predicate (List.mem_append.mpr (Or.inl entry))

theorem a_legacy_guard_cannot_discharge_an_arbitrary_typed_predicate :
    viewGuard [fun _ : Nat => True] [] 0 ∧
      ¬ viewGuard [fun _ : Nat => True] [fun n => n = 1] 0 := by
  simp [viewGuard]

/-- Canonical versions describe encoding forms only. -/
def graphVersion (hasEntry hasFormula hasRefinement : Bool) : Nat :=
  if hasEntry then 4 else if hasFormula then 3 else if hasRefinement then 2 else 1

theorem typed_atoms_require_version_three (refinement : Bool) :
    graphVersion false true refinement = 3 := by rfl

theorem entry_atoms_require_version_four (formula refinement : Bool) :
    graphVersion true formula refinement = 4 := by rfl

/-- Classification and metadata transport never discharge source binding. -/
def metadataPermission (_version : Nat) (_syntaxValid : Bool) : Bool := false

theorem canonical_encoding_is_not_a_call_permission (entry formula refinement valid : Bool) :
    metadataPermission (graphVersion entry formula refinement) valid = false := by rfl

end QCore.QuantIntegrationBoundaries
