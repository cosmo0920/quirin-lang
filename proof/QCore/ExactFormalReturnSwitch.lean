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

import QCore.ExactFormalReturn

/-! Card RULE36-PILOT: rule 36 as a switchable package.

The gateway asks rule 36's package (`rule-36-exact-formal-return`) at four
host points, each only while registry rule `36` is on
(`QCORE_GATEWAY_RULES_OFF`): the lexical candidate (whether a derived-result
CFG is built for the rule and a neutral summary is refined), the return route
of the derived-result CFG, and the exit classifier read by the transfer and
the exit observation. Off, the candidate is false, the route is not asked and
the classifier refuses. These theorems state that the switch on is exactly
rule 36's model (`QCore.ExactFormalReturn`), that off is the pre-rule-36
reading, and that off never admits what on refuses: no exit validates, no
summary is refined, every owed resource, binding and frame is kept, and the
refusal's unknown is added. The Rust/model correspondence and faithful
extraction remain premise 28. -/
namespace QCore.ExactFormalReturnSwitch

open QCore.DerivedResultInference
open QCore.ExactFormalReturn

/-- The exit classifier with rule 36 on or off. -/
def classifyUnder (on : Bool) (s : State α) (bases : List (Nat × Nat)) (parameter : Nat) :
    Flow :=
  if on then classify s bases parameter else .refuse

/-- The exit transfer with rule 36 on or off. -/
def transferUnder (on : Bool) (s : State α) (bases : List (Nat × Nat)) (parameter : Nat)
    (exposure : List Nat) : State α :=
  match classifyUnder on s bases parameter with
  | .refuse =>
      { s with
        unresolved := s.unresolved ++ exposure
        unmodeled := true
        diagnostics := 0 :: s.diagnostics }
  | _ => s

/-- The return route of the derived-result CFG: the confirmed formal of a
bare return, asked only while the rule is on and only in that CFG. -/
def routeUnder (on derived : Bool) (confirmed : Option Nat) : Option Nat :=
  if on && derived then confirmed else none

/-- The neutral summary's refinement: off, the lexical candidate is false, so
the source check never holds. -/
def refineUnder (on : Bool) (old candidate : Contract) (sourceChecked validated : Bool) :
    Contract :=
  refine old candidate (on && sourceChecked) validated

theorem the_switch_on_is_the_classifier (s : State α) (bases : List (Nat × Nat))
    (parameter : Nat) : classifyUnder true s bases parameter = classify s bases parameter := by
  simp [classifyUnder]

theorem the_switch_on_is_the_transfer (s : State α) (bases : List (Nat × Nat))
    (parameter : Nat) (exposure : List Nat) :
    transferUnder true s bases parameter exposure = transfer s bases parameter exposure := by
  simp only [transferUnder, transfer, classifyUnder, ite_true]
  cases classify s bases parameter <;> rfl

theorem the_switch_off_refuses (s : State α) (bases : List (Nat × Nat)) (parameter : Nat) :
    classifyUnder false s bases parameter = .refuse := by
  simp [classifyUnder]

/-- Off, no exit the rule observes validates any derived summary. -/
theorem the_switch_off_never_validates_an_exit (spec : Spec) (base : Nat) (s : State α)
    (bases : List (Nat × Nat)) (parameter : Nat) (touched : Bool) :
    exitMatches spec base ⟨observe (classifyUnder false s bases parameter), touched⟩ = false := by
  simp [the_switch_off_refuses, refused_exit_never_validates]

/-- Off never widens the exit: an exit accepted with the rule off is accepted
with it on (vacuously: off accepts none). -/
theorem the_switch_off_never_widens_an_exit (spec : Spec) (base : Nat) (s : State α)
    (bases : List (Nat × Nat)) (parameter : Nat) (touched : Bool)
    (h : exitMatches spec base ⟨observe (classifyUnder false s bases parameter), touched⟩ = true) :
    exitMatches spec base ⟨observe (classifyUnder true s bases parameter), touched⟩ = true := by
  simp [the_switch_off_never_validates_an_exit] at h

/-- Off, the transfer is the refusal: owed resources, the binding and the
frame are kept, and the unknown and the exposure are added. -/
theorem the_switch_off_keeps_custody_and_adds_unknown (s : State α)
    (bases : List (Nat × Nat)) (parameter : Nat) (exposure : List Nat) :
    (transferUnder false s bases parameter exposure).ledger = s.ledger ∧
    (transferUnder false s bases parameter exposure).pointer = s.pointer ∧
    (transferUnder false s bases parameter exposure).frame = s.frame ∧
    (transferUnder false s bases parameter exposure).unresolved = s.unresolved ++ exposure ∧
    (transferUnder false s bases parameter exposure).unmodeled = true := by
  simp [transferUnder, classifyUnder]

/-- Under either switch the transfer keeps every owed resource. -/
theorem every_switched_transfer_keeps_owed_resources (on : Bool) (s : State α)
    (bases : List (Nat × Nat)) (parameter : Nat) (exposure : List Nat) :
    (transferUnder on s bases parameter exposure).ledger = s.ledger := by
  unfold transferUnder
  split <;> rfl

theorem the_switch_off_asks_no_route (derived : Bool) (confirmed : Option Nat) :
    routeUnder false derived confirmed = none := by
  simp [routeUnder]

theorem the_route_is_only_the_derived_cfgs (on : Bool) (confirmed : Option Nat) :
    routeUnder on false confirmed = none := by
  simp [routeUnder]

theorem the_switch_on_is_the_confirmed_route (confirmed : Option Nat) :
    routeUnder true true confirmed = confirmed := by
  simp [routeUnder]

theorem the_switch_off_retains_the_old_summary (old candidate : Contract)
    (sourceChecked validated : Bool) :
    refineUnder false old candidate sourceChecked validated = old := by
  simp [refineUnder, refine]

theorem the_switch_on_is_the_refinement (old candidate : Contract)
    (sourceChecked validated : Bool) :
    refineUnder true old candidate sourceChecked validated =
      refine old candidate sourceChecked validated := by
  simp [refineUnder]

theorem every_switched_refinement_keeps_parameter_custody (on : Bool)
    (old candidate : Contract) (sourceChecked validated : Bool) :
    (refineUnder on old candidate sourceChecked validated).parameters = old.parameters :=
  refinement_preserves_parameter_custody old candidate (on && sourceChecked) validated

end QCore.ExactFormalReturnSwitch
