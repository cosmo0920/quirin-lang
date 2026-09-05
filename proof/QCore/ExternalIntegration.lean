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

structure MonkeyComposition where
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

def MonkeyComposition.Valid (composition : MonkeyComposition) : Prop :=
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

instance (composition : MonkeyComposition) : Decidable composition.Valid := by
  unfold MonkeyComposition.Valid
  infer_instance

structure MonkeyCompositionCertificate where
  checked : MonkeyComposition
  deriving DecidableEq, Repr

def checkMonkeyComposition
    (composition : MonkeyComposition) : Option MonkeyCompositionCertificate :=
  if composition.Valid then some { checked := composition } else none

private theorem checkMonkeyComposition_valid
    {composition : MonkeyComposition}
    {certificate : MonkeyCompositionCertificate}
    (result : checkMonkeyComposition composition = some certificate) :
    composition.Valid := by
  unfold checkMonkeyComposition at result
  split at result
  · assumption
  · contradiction

theorem accepted_monkey_callbacks_cover_failure_and_map_paths
    {composition : MonkeyComposition}
    {certificate : MonkeyCompositionCertificate}
    (result : checkMonkeyComposition composition = some certificate) :
    composition.callbackSummariesExact = true ∧
      composition.mapCustodyChecked = true ∧
      composition.nullableFailuresChecked = true ∧
      composition.registrationFailureChecked = true := by
  have valid := checkMonkeyComposition_valid result
  exact ⟨valid.2.1, valid.2.2.1, valid.2.2.2.1, valid.2.2.2.2.1⟩

theorem accepted_monkey_observation_binds_backend_dispatcher_and_trace
    {composition : MonkeyComposition}
    {certificate : MonkeyCompositionCertificate}
    (result : checkMonkeyComposition composition = some certificate) :
    composition.backendObserved = true ∧
      composition.dispatcherMatches = true ∧
      composition.traceRefines = true := by
  have valid := checkMonkeyComposition_valid result
  exact ⟨valid.2.2.2.2.2.1, valid.2.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.2.1⟩

theorem accepted_monkey_composition_has_event_teardown
    {composition : MonkeyComposition}
    {certificate : MonkeyCompositionCertificate}
    (result : checkMonkeyComposition composition = some certificate) :
    composition.eventTeardownAccepted = true :=
  (checkMonkeyComposition_valid result).2.2.2.2.2.2.2.2.1

theorem accepted_monkey_event_build_matches_profile
    {composition : MonkeyComposition}
    {certificate : MonkeyCompositionCertificate}
    (result : checkMonkeyComposition composition = some certificate) :
    composition.eventBuildMatches = true :=
  (checkMonkeyComposition_valid result).2.2.2.2.2.2.2.2.2.1

theorem accepted_monkey_source_is_pinned
    {composition : MonkeyComposition}
    {certificate : MonkeyCompositionCertificate}
    (result : checkMonkeyComposition composition = some certificate) :
    composition.sourcePinned = true :=
  (checkMonkeyComposition_valid result).2.2.2.2.2.2.2.2.2.2

end QCore.ExternalIntegration
