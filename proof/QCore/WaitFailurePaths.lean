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

import Std

namespace QCore.WaitFailurePaths

/-! Audited parity model for the executable timed-wait failure policy. The
source inventory and native observations remain separate evidence classes. -/

inductive Outcome where | violated | unknown
  deriving DecidableEq, Repr

inductive EvidenceClass where | instrumentedSourceHarness | sourceModel
  deriving DecidableEq, Repr

inductive Scenario where
  | contextAllocationFailure | baseCreationFailure | firedAllocationFailure
  | timerCreationFailure | timerRegistrationFailure | successObservation
  | loopInitFailure | loopSizeBounds
  deriving DecidableEq, Repr

structure Finding where
  scenario : Scenario
  outcome : Outcome
  evidenceClass : EvidenceClass
  deriving DecidableEq, Repr

structure SourceInventory where
  createPathCount : Nat
  timedWaitPathCount : Nat
  createSelectedPath : Nat
  timedWaitSelectedPath : Nat
  deriving DecidableEq, Repr

def exactSource : SourceInventory := ⟨4, 3, 3, 2⟩

def sourceAccepted (source : SourceInventory) : Bool := source == exactSource

def findings : List Finding := [
  ⟨.contextAllocationFailure, .unknown, .instrumentedSourceHarness⟩,
  ⟨.baseCreationFailure, .violated, .instrumentedSourceHarness⟩,
  ⟨.firedAllocationFailure, .violated, .instrumentedSourceHarness⟩,
  ⟨.timerCreationFailure, .violated, .instrumentedSourceHarness⟩,
  ⟨.timerRegistrationFailure, .unknown, .instrumentedSourceHarness⟩,
  ⟨.successObservation, .unknown, .instrumentedSourceHarness⟩,
  ⟨.loopInitFailure, .unknown, .sourceModel⟩,
  ⟨.loopSizeBounds, .unknown, .sourceModel⟩
]

def accepted (source : SourceInventory) (observed : List Finding) : Bool :=
  sourceAccepted source && observed == findings

theorem exact_inventory_and_findings_are_accepted :
    accepted exactSource findings = true := by decide

theorem missing_creation_path_is_rejected :
    accepted { exactSource with createPathCount := 3 } findings = false := by decide

def registrationUnsafe : List Finding := findings.modify 4 fun finding =>
  { finding with outcome := .violated }

theorem registration_failure_cannot_be_promoted_to_unsafe :
    accepted exactSource registrationUnsafe = false := by decide

def successSafeSubstitution : List Finding := findings.eraseIdx 5

theorem success_observation_cannot_discharge_the_inventory :
    accepted exactSource successSafeSubstitution = false := by decide

theorem every_unsafe_finding_has_harness_evidence
    (finding : Finding) (member : finding ∈ findings)
    (violated : finding.outcome = .violated) :
    finding.evidenceClass = .instrumentedSourceHarness := by
  simp [findings] at member
  rcases member with h | h | h | h | h | h | h | h <;>
    simp_all

theorem registration_failure_remains_unknown :
    findings[4]?.map Finding.outcome = some .unknown := by decide

theorem successful_observation_remains_unknown :
    findings[5]?.map Finding.outcome = some .unknown := by decide

end QCore.WaitFailurePaths
