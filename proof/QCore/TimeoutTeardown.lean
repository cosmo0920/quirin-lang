import QCore.EventDriven

namespace QCore.TimeoutTeardown

open QCore.EventDriven

structure Evidence where
  callbackRunning : Bool
  sameDispatcher : Bool
  currentCallbackDeleted : Bool
  eventDescriptorAbsent : Bool
  eventFreedOnce : Bool
  mapFreedOnce : Bool
  writeEndpointRetired : Bool
  peerEndpointHandledByCaller : Bool
  callbackExited : Bool
  trace : List Nat
  expectedTrace : List Nat
  deriving DecidableEq, Repr

def Evidence.Valid (evidence : Evidence) : Prop :=
  evidence.callbackRunning = true ∧
  evidence.sameDispatcher = true ∧
  evidence.currentCallbackDeleted = true ∧
  evidence.eventDescriptorAbsent = true ∧
  evidence.eventFreedOnce = true ∧
  evidence.mapFreedOnce = true ∧
  evidence.writeEndpointRetired = true ∧
  evidence.peerEndpointHandledByCaller = true ∧
  evidence.callbackExited = true ∧
  evidence.trace = evidence.expectedTrace

instance (evidence : Evidence) : Decidable evidence.Valid := by
  unfold Evidence.Valid
  infer_instance

structure Certificate where
  checked : Evidence
  deriving DecidableEq, Repr

def check (evidence : Evidence) : Option Certificate :=
  if evidence.Valid then some { checked := evidence } else none

private theorem checked_valid {evidence : Evidence} {certificate : Certificate}
    (result : check evidence = some certificate) : evidence.Valid := by
  unfold check at result
  split at result
  · assumption
  · contradiction

theorem accepted_self_teardown_has_distinct_custody
    {evidence : Evidence} {certificate : Certificate}
    (result : check evidence = some certificate) :
    evidence.eventFreedOnce = true ∧
    evidence.mapFreedOnce = true ∧
    evidence.writeEndpointRetired = true ∧
    evidence.peerEndpointHandledByCaller = true := by
  have valid := checked_valid result
  exact ⟨valid.2.2.2.2.1, valid.2.2.2.2.2.1,
    valid.2.2.2.2.2.2.1, valid.2.2.2.2.2.2.2.1⟩

theorem accepted_self_teardown_is_same_dispatcher_and_deleted
    {evidence : Evidence} {certificate : Certificate}
    (result : check evidence = some certificate) :
    evidence.callbackRunning = true ∧
    evidence.sameDispatcher = true ∧
    evidence.currentCallbackDeleted = true ∧
    evidence.eventDescriptorAbsent = true := by
  have valid := checked_valid result
  exact ⟨valid.1, valid.2.1, valid.2.2.1, valid.2.2.2.1⟩

theorem accepted_trace_exits_callback
    {evidence : Evidence} {certificate : Certificate}
    (result : check evidence = some certificate) :
    evidence.callbackExited = true ∧ evidence.trace = evidence.expectedTrace := by
  have valid := checked_valid result
  exact ⟨valid.2.2.2.2.2.2.2.2.1, valid.2.2.2.2.2.2.2.2.2⟩

end QCore.TimeoutTeardown
