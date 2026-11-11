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

/-!
Card **B01-RETURN-EXPRESSION**: one modeled resource-free call inside an
otherwise custody-neutral scalar return expression.

The model includes the load-bearing gates: the call is unique and
unconditional, the residual expression is neutral, the result carries no
custody, and the reserved synthetic place does not collide with source. A
refused shape has no modeled transition. An admitted shape performs exactly the
ordinary call effect; returning its scalar result adds no custody effect.

This is an abstract rule theorem. Faithful C extraction and correspondence to
`legacy_c.rs` remain the standing review premises (constraint 28).
-/

namespace QCore.ReturnExpressionEvaluation

abbrev Site := Nat
abbrev State := Site → Bool
abbrev CallEffect := State → State

structure Shape where
  calls : Nat
  conditional : Bool
  shortCircuit : Bool
  assignment : Bool
  mutation : Bool
  typeOperator : Bool
  collision : Bool
  resultCarriesCustody : Bool
  residualNeutral : Bool
  externallyDeclared : Bool
  deriving DecidableEq, Repr

def Admitted (s : Shape) : Prop :=
  s.calls = 1 ∧ s.conditional = false ∧ s.shortCircuit = false ∧
  s.assignment = false ∧ s.mutation = false ∧ s.typeOperator = false ∧
  s.collision = false ∧ s.resultCarriesCustody = false ∧
  s.residualNeutral = true ∧ s.externallyDeclared = true

instance admittedDecidable (s : Shape) : Decidable (Admitted s) := by
  unfold Admitted
  infer_instance

def lower (s : Shape) (effect : CallEffect) (st : State) : Option State :=
  if Admitted s then some (effect st) else none

theorem an_admitted_call_is_unconditional (s : Shape) (h : Admitted s) :
    s.conditional = false ∧ s.shortCircuit = false := ⟨h.2.1, h.2.2.1⟩

theorem an_admitted_call_has_no_synthetic_collision (s : Shape) (h : Admitted s) :
    s.collision = false := h.2.2.2.2.2.2.1

theorem a_conditional_call_stays_refused
    (s : Shape) (effect : CallEffect) (st : State)
    (h : s.conditional = true ∨ s.shortCircuit = true) :
    lower s effect st = none := by
  have hn : ¬ Admitted s := by
    intro ha
    rcases h with hc | hs
    · rw [ha.2.1] at hc
      contradiction
    · rw [ha.2.2.1] at hs
      contradiction
  simp [lower, hn]

theorem a_collision_stays_refused
    (s : Shape) (effect : CallEffect) (st : State)
    (h : s.collision = true) : lower s effect st = none := by
  have hn : ¬ Admitted s := by
    intro ha
    rw [ha.2.2.2.2.2.2.1] at h
    contradiction
  simp [lower, hn]

theorem a_local_summary_stays_refused
    (s : Shape) (effect : CallEffect) (st : State)
    (h : s.externallyDeclared = false) : lower s effect st = none := by
  have hn : ¬ Admitted s := by
    intro ha
    rw [ha.2.2.2.2.2.2.2.2.2] at h
    contradiction
  simp [lower, hn]

theorem an_admitted_return_is_exactly_the_call_effect
    (s : Shape) (effect : CallEffect) (st : State) (h : Admitted s) :
    lower s effect st = some (effect st) := by simp [lower, h]

theorem a_preserved_obligation_remains_visible
    (s : Shape) (effect : CallEffect) (st : State) (site : Site)
    (ha : Admitted s) (hp : effect st site = st site) :
    ∃ post, lower s effect st = some post ∧ post site = st site := by
  refine ⟨effect st, an_admitted_return_is_exactly_the_call_effect s effect st ha, hp⟩

def arithmeticShape : Shape where
  calls := 1
  conditional := false
  shortCircuit := false
  assignment := false
  mutation := false
  typeOperator := false
  collision := false
  resultCarriesCustody := false
  residualNeutral := true
  externallyDeclared := true

theorem the_positive_family_is_admitted : Admitted arithmeticShape := by
  simp [Admitted, arithmeticShape]

theorem a_declared_release_is_applied (st : State) (site : Site) :
    lower arithmeticShape
      (fun prior s => if s = site then false else prior s) st =
      some (fun s => if s = site then false else st s) := by
  simp [lower, the_positive_family_is_admitted]

/-- The second B01 arm: a complete opaque member read, carrying the assignment
    classifier's retained alias check and rooted at an untracked base. -/
def OpaqueAdmitted
    (baseUntracked aliasValid completeRead collision : Bool) : Prop :=
  baseUntracked = true ∧ aliasValid = true ∧ completeRead = true ∧ collision = false

instance opaqueAdmittedDecidable (b a r c : Bool) :
    Decidable (OpaqueAdmitted b a r c) := by
  unfold OpaqueAdmitted
  infer_instance

def lowerOpaque
    (baseUntracked aliasValid completeRead collision : Bool) (st : State) : Option State :=
  if OpaqueAdmitted baseUntracked aliasValid completeRead collision
  then some st
  else none

/-- `OpaqueValue` followed by B01's `NoOp` return escapes no identity. -/
theorem an_admitted_opaque_read_changes_no_custody (st : State) :
    lowerOpaque true true true false st = some st := by
  simp [lowerOpaque, OpaqueAdmitted]

/-- The alias obligation is retained rather than erased by the return. -/
theorem an_unvalidated_alias_read_stays_refused (st : State) :
    lowerOpaque true false true false st = none := by
  simp [lowerOpaque, OpaqueAdmitted]

/-- A protected or body-bound base is not the untracked getter family. -/
theorem a_tracked_base_stays_refused (st : State) :
    lowerOpaque false true true false st = none := by
  simp [lowerOpaque, OpaqueAdmitted]

/-- A conditional pointer result is not one complete opaque member read. -/
theorem a_conditional_opaque_result_stays_refused (st : State) :
    lowerOpaque true true false false st = none := by
  simp [lowerOpaque, OpaqueAdmitted]

end QCore.ReturnExpressionEvaluation
