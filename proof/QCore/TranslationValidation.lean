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

import QCore.Compiler
import QCore.LegacyCAnalyzer

namespace QCore.TranslationValidation

/-! Cycle 13 models translation validation as an independently checked
certificate connecting source, cleanup CFG, and target observations. -/

abbrev Observation := Nat
abbrev CfgId := Nat

structure PassBoundary where
  expected : CfgId
  actual : CfgId
  deriving DecidableEq, Repr

def PassBoundary.Valid (boundary : PassBoundary) : Prop :=
  boundary.expected = boundary.actual

instance (boundary : PassBoundary) : Decidable boundary.Valid := by
  unfold PassBoundary.Valid
  infer_instance

structure Certificate where
  source : Observation
  cfg : Observation
  target : Observation
  sourceCfg : CfgId
  targetCfg : CfgId
  deriving DecidableEq, Repr

def validate (boundary : PassBoundary) (source cfg target : Observation) :
    Option Certificate :=
  if boundary.Valid ∧ source = cfg ∧ cfg = target then
    some {
      source := source
      cfg := cfg
      target := target
      sourceCfg := boundary.expected
      targetCfg := boundary.actual
    }
  else none

private theorem validate_condition
    {boundary : PassBoundary} {source cfg target : Observation}
    {certificate : Certificate}
    (result : validate boundary source cfg target = some certificate) :
    boundary.Valid ∧ source = cfg ∧ cfg = target := by
  unfold validate at result
  split at result
  · assumption
  · contradiction

theorem validation_preserves_pass_boundary
    {boundary : PassBoundary} {source cfg target : Observation}
    {certificate : Certificate}
    (result : validate boundary source cfg target = some certificate) :
    boundary.Valid :=
  (validate_condition result).1

theorem validation_source_matches_cfg
    {boundary : PassBoundary} {source cfg target : Observation}
    {certificate : Certificate}
    (result : validate boundary source cfg target = some certificate) :
    source = cfg :=
  (validate_condition result).2.1

theorem validation_cfg_matches_target
    {boundary : PassBoundary} {source cfg target : Observation}
    {certificate : Certificate}
    (result : validate boundary source cfg target = some certificate) :
    cfg = target :=
  (validate_condition result).2.2

theorem validation_source_matches_target
    {boundary : PassBoundary} {source cfg target : Observation}
    {certificate : Certificate}
    (result : validate boundary source cfg target = some certificate) :
    source = target := by
  exact (validation_source_matches_cfg result).trans
    (validation_cfg_matches_target result)

def widenState (state : LegacyCAnalyzer.AbstractState) :
    LegacyCAnalyzer.AbstractState :=
  { state with precise := false }

/-- Widening is conservative: once precision is lost the legacy-C analyzer
cannot issue a safe verdict. -/
theorem widened_legacy_state_is_never_safe
    (state : LegacyCAnalyzer.AbstractState) :
    LegacyCAnalyzer.classify (widenState state) ≠ .safe := by
  apply LegacyCAnalyzer.imprecise_is_never_safe
  rfl

end QCore.TranslationValidation
