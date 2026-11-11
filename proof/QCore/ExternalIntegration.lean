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

namespace QCore.ExternalIntegration

/-! Executable certificate boundaries for audited external-library custody,
ChunkIO import composition, and Monkey/Libevent observation composition. -/

structure LibraryCustody where
  identitiesUnique : Bool
  referenceHoldersBalanced : Bool
  containerTransfersChecked : Bool
  arenaTeardownCascades : Bool
  synchronizationEvidenceComplete : Bool
  reclamationEvidenceComplete : Bool
  noLiveObligations : Bool
  sourcePinned : Bool
  summaryProfileAccepted : Bool
  sourceContractsChecked : Bool
  deriving DecidableEq, Repr

def LibraryCustody.Valid (custody : LibraryCustody) : Prop :=
  custody.identitiesUnique = true ∧
  custody.referenceHoldersBalanced = true ∧
  custody.containerTransfersChecked = true ∧
  custody.arenaTeardownCascades = true ∧
  custody.synchronizationEvidenceComplete = true ∧
  custody.reclamationEvidenceComplete = true ∧
  custody.noLiveObligations = true ∧
  custody.sourcePinned = true ∧
  custody.summaryProfileAccepted = true ∧
  custody.sourceContractsChecked = true

instance (custody : LibraryCustody) : Decidable custody.Valid := by
  unfold LibraryCustody.Valid
  infer_instance

structure LibraryCustodyCertificate where
  checked : LibraryCustody
  deriving DecidableEq, Repr

def checkLibraryCustody
    (custody : LibraryCustody) : Option LibraryCustodyCertificate :=
  if custody.Valid then some { checked := custody } else none

private theorem checkLibraryCustody_valid
    {custody : LibraryCustody} {certificate : LibraryCustodyCertificate}
    (result : checkLibraryCustody custody = some certificate) : custody.Valid := by
  unfold checkLibraryCustody at result
  split at result
  · assumption
  · contradiction

theorem accepted_library_custody_balances_holders_and_containers
    {custody : LibraryCustody} {certificate : LibraryCustodyCertificate}
    (result : checkLibraryCustody custody = some certificate) :
    custody.referenceHoldersBalanced = true ∧
      custody.containerTransfersChecked = true ∧
      custody.arenaTeardownCascades = true := by
  have valid := checkLibraryCustody_valid result
  exact ⟨valid.2.1, valid.2.2.1, valid.2.2.2.1⟩

theorem accepted_publication_and_reclamation_have_evidence
    {custody : LibraryCustody} {certificate : LibraryCustodyCertificate}
    (result : checkLibraryCustody custody = some certificate) :
    custody.synchronizationEvidenceComplete = true ∧
      custody.reclamationEvidenceComplete = true := by
  have valid := checkLibraryCustody_valid result
  exact ⟨valid.2.2.2.2.1, valid.2.2.2.2.2.1⟩

theorem accepted_library_custody_has_no_live_obligation
    {custody : LibraryCustody} {certificate : LibraryCustodyCertificate}
    (result : checkLibraryCustody custody = some certificate) :
    custody.noLiveObligations = true :=
  (checkLibraryCustody_valid result).2.2.2.2.2.2.1

theorem accepted_library_summary_profile_is_pinned_and_scoped
    {custody : LibraryCustody} {certificate : LibraryCustodyCertificate}
    (result : checkLibraryCustody custody = some certificate) :
    custody.sourcePinned = true ∧ custody.summaryProfileAccepted = true := by
  have valid := checkLibraryCustody_valid result
  exact ⟨valid.2.2.2.2.2.2.2.1, valid.2.2.2.2.2.2.2.2.1⟩

theorem accepted_library_source_contracts_are_checked
    {custody : LibraryCustody} {certificate : LibraryCustodyCertificate}
    (result : checkLibraryCustody custody = some certificate) :
    custody.sourceContractsChecked = true :=
  (checkLibraryCustody_valid result).2.2.2.2.2.2.2.2.2

structure ChunkImport where
  sourcePinned : Bool
  controlFlowComplete : Bool
  summariesComplete : Bool
  lifecycleAccepted : Bool
  noLiveObligations : Bool
  deriving DecidableEq, Repr

def ChunkImport.Valid (input : ChunkImport) : Prop :=
  input.sourcePinned = true ∧
  input.controlFlowComplete = true ∧
  input.summariesComplete = true ∧
  input.lifecycleAccepted = true ∧
  input.noLiveObligations = true

instance (input : ChunkImport) : Decidable input.Valid := by
  unfold ChunkImport.Valid
  infer_instance

structure ChunkImportCertificate where
  checked : ChunkImport
  deriving DecidableEq, Repr

def checkChunkImport (input : ChunkImport) : Option ChunkImportCertificate :=
  if input.Valid then some { checked := input } else none

private theorem checkChunkImport_valid
    {input : ChunkImport} {certificate : ChunkImportCertificate}
    (result : checkChunkImport input = some certificate) : input.Valid := by
  unfold checkChunkImport at result
  split at result
  · assumption
  · contradiction

theorem accepted_chunk_import_is_pinned_and_complete
    {input : ChunkImport} {certificate : ChunkImportCertificate}
    (result : checkChunkImport input = some certificate) :
    input.sourcePinned = true ∧ input.controlFlowComplete = true ∧
      input.summariesComplete = true := by
  have valid := checkChunkImport_valid result
  exact ⟨valid.1, valid.2.1, valid.2.2.1⟩

theorem accepted_chunk_import_composes_lifecycle_safety
    {input : ChunkImport} {certificate : ChunkImportCertificate}
    (result : checkChunkImport input = some certificate) :
    input.lifecycleAccepted = true ∧ input.noLiveObligations = true := by
  have valid := checkChunkImport_valid result
  exact ⟨valid.2.2.2.1, valid.2.2.2.2⟩

structure EventComposition where
  buildProfileAccepted : Bool
  callbackSummariesExact : Bool
  mapCustodyChecked : Bool
  nullableFailuresChecked : Bool
  registrationFailureChecked : Bool
  backendObserved : Bool
  dispatcherMatches : Bool
  traceRefines : Bool
  eventTeardownAccepted : Bool
  eventBuildMatches : Bool
  sourcePinned : Bool
  deriving DecidableEq, Repr

def EventComposition.Valid (composition : EventComposition) : Prop :=
  composition.buildProfileAccepted = true ∧
  composition.callbackSummariesExact = true ∧
  composition.mapCustodyChecked = true ∧
  composition.nullableFailuresChecked = true ∧
  composition.registrationFailureChecked = true ∧
  composition.backendObserved = true ∧
  composition.dispatcherMatches = true ∧
  composition.traceRefines = true ∧
  composition.eventTeardownAccepted = true ∧
  composition.eventBuildMatches = true ∧
  composition.sourcePinned = true

instance (composition : EventComposition) : Decidable composition.Valid := by
  unfold EventComposition.Valid
  infer_instance

structure EventCompositionCertificate where
  checked : EventComposition
  deriving DecidableEq, Repr

def checkEventComposition
    (composition : EventComposition) : Option EventCompositionCertificate :=
  if composition.Valid then some { checked := composition } else none

private theorem checkEventComposition_valid
    {composition : EventComposition}
    {certificate : EventCompositionCertificate}
    (result : checkEventComposition composition = some certificate) :
    composition.Valid := by
  unfold checkEventComposition at result
  split at result
  · assumption
  · contradiction

theorem accepted_monkey_callbacks_cover_failure_and_map_paths
    {composition : EventComposition}
    {certificate : EventCompositionCertificate}
    (result : checkEventComposition composition = some certificate) :
    composition.callbackSummariesExact = true ∧
      composition.mapCustodyChecked = true ∧
      composition.nullableFailuresChecked = true ∧
      composition.registrationFailureChecked = true := by
  have valid := checkEventComposition_valid result
  exact ⟨valid.2.1, valid.2.2.1, valid.2.2.2.1, valid.2.2.2.2.1⟩

theorem accepted_monkey_observation_binds_backend_dispatcher_and_trace
    {composition : EventComposition}
    {certificate : EventCompositionCertificate}
    (result : checkEventComposition composition = some certificate) :
    composition.backendObserved = true ∧
      composition.dispatcherMatches = true ∧
      composition.traceRefines = true := by
  have valid := checkEventComposition_valid result
  exact ⟨valid.2.2.2.2.2.1, valid.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.1⟩

theorem accepted_monkey_composition_has_event_teardown
    {composition : EventComposition}
    {certificate : EventCompositionCertificate}
    (result : checkEventComposition composition = some certificate) :
    composition.eventTeardownAccepted = true :=
  (checkEventComposition_valid result).2.2.2.2.2.2.2.2.1

theorem accepted_monkey_event_build_matches_profile
    {composition : EventComposition}
    {certificate : EventCompositionCertificate}
    (result : checkEventComposition composition = some certificate) :
    composition.eventBuildMatches = true :=
  (checkEventComposition_valid result).2.2.2.2.2.2.2.2.2.1

theorem accepted_monkey_source_is_pinned
    {composition : EventComposition}
    {certificate : EventCompositionCertificate}
    (result : checkEventComposition composition = some certificate) :
    composition.sourcePinned = true :=
  (checkEventComposition_valid result).2.2.2.2.2.2.2.2.2.2

/-- Historical names retained as definitional aliases for existing adapters. -/
abbrev MonkeyComposition := EventComposition
abbrev MonkeyCompositionCertificate := EventCompositionCertificate

namespace MonkeyComposition
abbrev buildProfileAccepted (composition : MonkeyComposition) : Bool :=
  EventComposition.buildProfileAccepted composition
abbrev callbackSummariesExact (composition : MonkeyComposition) : Bool :=
  EventComposition.callbackSummariesExact composition
abbrev mapCustodyChecked (composition : MonkeyComposition) : Bool :=
  EventComposition.mapCustodyChecked composition
abbrev nullableFailuresChecked (composition : MonkeyComposition) : Bool :=
  EventComposition.nullableFailuresChecked composition
abbrev registrationFailureChecked (composition : MonkeyComposition) : Bool :=
  EventComposition.registrationFailureChecked composition
abbrev backendObserved (composition : MonkeyComposition) : Bool :=
  EventComposition.backendObserved composition
abbrev dispatcherMatches (composition : MonkeyComposition) : Bool :=
  EventComposition.dispatcherMatches composition
abbrev traceRefines (composition : MonkeyComposition) : Bool :=
  EventComposition.traceRefines composition
abbrev eventTeardownAccepted (composition : MonkeyComposition) : Bool :=
  EventComposition.eventTeardownAccepted composition
abbrev eventBuildMatches (composition : MonkeyComposition) : Bool :=
  EventComposition.eventBuildMatches composition
abbrev sourcePinned (composition : MonkeyComposition) : Bool :=
  EventComposition.sourcePinned composition
abbrev Valid (composition : MonkeyComposition) : Prop := EventComposition.Valid composition
export EventComposition (
  _sizeOf_1
  _sizeOf_inst
  casesOn
  ctorIdx
  mk
  mk._flat_ctor
  mk.inj
  mk.injEq
  mk.noConfusion
  mk.sizeOf_spec
  noConfusion
  noConfusionType
  rec
  recOn
)
end MonkeyComposition

namespace MonkeyCompositionCertificate
abbrev checked (certificate : MonkeyCompositionCertificate) : MonkeyComposition :=
  EventCompositionCertificate.checked certificate
export EventCompositionCertificate (
  _sizeOf_1
  _sizeOf_inst
  casesOn
  ctorIdx
  mk
  mk._flat_ctor
  mk.inj
  mk.injEq
  mk.noConfusion
  mk.sizeOf_spec
  noConfusion
  noConfusionType
  rec
  recOn
)
end MonkeyCompositionCertificate

abbrev checkMonkeyComposition (composition : MonkeyComposition) :
    Option MonkeyCompositionCertificate := checkEventComposition composition

abbrev instDecidableEqMonkeyComposition : DecidableEq MonkeyComposition :=
  instDecidableEqEventComposition
abbrev instReprMonkeyComposition : Repr MonkeyComposition := instReprEventComposition
abbrev instDecidableEqMonkeyCompositionCertificate : DecidableEq MonkeyCompositionCertificate :=
  instDecidableEqEventCompositionCertificate
abbrev instReprMonkeyCompositionCertificate : Repr MonkeyCompositionCertificate :=
  instReprEventCompositionCertificate

end QCore.ExternalIntegration
