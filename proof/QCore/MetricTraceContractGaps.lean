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

import QCore.DeclaredModeAgreement
import QCore.NeutralFrameRecursion

/-! W46-METRIC-TRACE-CONTRACT-GAPS models the custody vector of the nineteen
reviewed declarations. The C bodies and their ABI are reviewed source premises;
this model proves the claimed neutral effect and the fail-closed disagreement
case over those vectors, not the source-to-model correspondence. -/

namespace QCore.MetricTraceContractGaps

open QCore.DeclaredModeAgreement

def selectedArities : List Nat :=
  -- CMetrics: four value reads, two value writes, five timestamp helpers,
  -- two bit conversions. CTraces: two ID reads and four span scalar writes.
  [1, 1, 1, 1, 3, 3, 2, 2, 1, 1, 1, 1, 1,
   2, 1, 2, 2, 2, 2]

theorem selected_count : selectedArities.length = 19 := by decide

def neutralContract (arity : Nat) : Contract :=
  { parameters := List.replicate arity .borrowed, result := .absent }

/-- Every selected formal retains caller custody; the result produces none. -/
theorem selected_contracts_have_no_custody_transfer
    (arity : Nat) (_h : arity ∈ selectedArities) :
    (neutralContract arity).parameters.map parameterDuty =
      List.replicate arity .retained ∧
    resultDuty (neutralContract arity).result = .absent := by
  simp [neutralContract, List.map_replicate, parameterDuty, resultDuty]

/-- An owned actual passed to a borrowed position still has its duty; this
    model deliberately does not turn a scalar field write into a release. -/
theorem borrowed_does_not_discharge_owned :
    parameterDuty (.borrowed : ParameterMode) =
      parameterDuty (.owned false : ParameterMode) := by decide

/-- A derived release contradicts any selected neutral declaration of
    positive arity; the unit's gate must withdraw the declaration. -/
theorem releasing_a_selected_formal_contradicts
    (arity : Nat) (positive : 0 < arity) :
    contradiction (neutralContract arity)
      { parameters := .released false :: List.replicate (arity - 1) .borrowed,
        result := .absent } = some .parameterCustody := by
  cases arity with
  | zero => omega
  | succ n =>
      simp [neutralContract, contradiction, parameterAxis,
        parametersContradict, parameterDuty, List.replicate_succ]

/-- With no validated summary, a selected declaration remains an explicit
    assumption; absence of source evidence never becomes a proof of a body. -/
theorem missing_body_evidence_keeps_premise (arity : Nat) :
    applied { contracts := fun _ => some (neutralContract arity),
              derived := fun _ => none } noCarry 0 = some (neutralContract arity) := by
  exact absence_of_evidence_applies_the_declaration rfl

-- W46-METRIC-AGGREGATE-CONTRACTS: histogram bucket and summary quantile
-- queries take an index; the other four selected scalar queries take a metric.
-- This records only the contract vectors, not bounds or source-body semantics.
def aggregateReadArities : List Nat := [2, 1, 1, 2, 1, 1]

theorem aggregate_read_count_and_arity :
    aggregateReadArities.length = 6 ∧
    aggregateReadArities.count 2 = 2 ∧
    aggregateReadArities.count 1 = 4 := by decide

theorem aggregate_reads_retain_custody (arity : Nat)
    (_selected : arity ∈ aggregateReadArities) :
    (neutralContract arity).parameters.map parameterDuty =
      List.replicate arity .retained ∧
    resultDuty (neutralContract arity).result = .absent := by
  simp [neutralContract, List.map_replicate, parameterDuty, resultDuty]

end QCore.MetricTraceContractGaps
