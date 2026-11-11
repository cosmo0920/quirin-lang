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

import QCore.CmetricsMetricConditions

/-! Conditional scalar-wrapper control projection. `decrease` is the observed
C floating comparison, including its NaN behavior, supplied independently;
it is not inferred from integer or real-number order. Lookup source/effects,
exact direct/nested traces and all C/Lean correspondence remain open. -/
namespace QCore.CmetricsScalarWrapperConditions
open CmetricsMetricConditions

structure Control where
  result : Int
  readsValue : Bool
  callsScalar : Bool
  deriving DecidableEq, Repr

def counterControl (lookupOK resetEnabled decrease : Bool) : Control :=
  if !lookupOK then ⟨-1, false, false⟩
  else if decrease && !resetEnabled then ⟨-1, true, false⟩
  else ⟨0, true, true⟩

def plainControl (lookupOK : Bool) : Control :=
  if lookupOK then ⟨0, false, true⟩ else ⟨-1, false, false⟩

theorem failed_lookup_has_no_scalar_call (resetEnabled decrease : Bool) :
    (counterControl false resetEnabled decrease).result = -1 ∧
    (counterControl false resetEnabled decrease).readsValue = false ∧
    (counterControl false resetEnabled decrease).callsScalar = false := by
  simp [counterControl]

theorem refused_decrease_has_no_scalar_call :
    counterControl true false true = ⟨-1, true, false⟩ := by
  rfl

theorem reset_enabled_still_reads_observed_value (decrease : Bool) :
    counterControl true true decrease = ⟨0, true, true⟩ := by
  cases decrease <;> rfl

theorem no_decrease_allows_scalar_call (resetEnabled : Bool) :
    counterControl true resetEnabled false = ⟨0, true, true⟩ := by
  cases resetEnabled <;> rfl

theorem counter_result_domain (lookupOK resetEnabled decrease : Bool) :
    (counterControl lookupOK resetEnabled decrease).result = 0 ∨
    (counterControl lookupOK resetEnabled decrease).result = -1 := by
  cases lookupOK <;> cases resetEnabled <;> cases decrease <;> decide

theorem plain_success_calls_scalar_without_value_read :
    plainControl true = ⟨0, false, true⟩ := by rfl

theorem plain_failure_has_no_scalar_call :
    plainControl false = ⟨-1, false, false⟩ := by rfl

def scalarPhase (afterLookup : State) (control : Control) (stores : List Store) : State :=
  if control.callsScalar then execute afterLookup stores else afterLookup

theorem refused_phase_preserves_lookup_effects (afterLookup : State)
    (control : Control) (stores : List Store) (refused : control.callsScalar = false) :
    scalarPhase afterLookup control stores = afterLookup := by
  simp [scalarPhase, refused]

theorem scalar_phase_preserves_post_lookup_ownership (afterLookup : State)
    (control : Control) (stores : List Store) :
    (scalarPhase afterLookup control stores).ownership = afterLookup.ownership := by
  unfold scalarPhase
  split
  · exact stores_preserve_ownership afterLookup stores
  · rfl

end QCore.CmetricsScalarWrapperConditions
