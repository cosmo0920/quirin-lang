import QCore.CleanupOutcome

namespace QCore.FileClose

open CleanupOutcome
structure State where
  backendLive : Bool
  mapping : Custody
  descriptor : Custody
  deriving DecidableEq

def releaseBackend (state : State) : Option State :=
  if state.backendLive then some { state with backendLive := false } else none

def nestedDischarged (state : State) : Bool :=
  state.mapping == .absent && state.descriptor == .absent

theorem backend_release_preserves_mapping {before after : State}
    (h : releaseBackend before = some after) : after.mapping = before.mapping := by
  unfold releaseBackend at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem backend_release_preserves_descriptor {before after : State}
    (h : releaseBackend before = some after) : after.descriptor = before.descriptor := by
  unfold releaseBackend at h
  split at h
  · cases Option.some.inj h
    rfl
  · contradiction

theorem unresolved_mapping_survives_backend_release (descriptor : Custody) :
    releaseBackend ⟨true, .unresolved, descriptor⟩ =
      some ⟨false, .unresolved, descriptor⟩ := rfl

theorem unknown_custody_rejects_discharge {state : State}
    (h : state.mapping = .unresolved ∨ state.descriptor = .unresolved) :
    nestedDischarged state = false := by
  rcases h with h | h <;> simp [nestedDischarged, h]

theorem accepted_nested_discharge_has_no_unresolved_resource {state : State}
    (h : nestedDischarged state = true) :
    state.mapping = .absent ∧ state.descriptor = .absent := by
  simpa [nestedDischarged] using h

theorem duplicate_backend_release_rejected (mapping descriptor : Custody) :
    releaseBackend ⟨false, mapping, descriptor⟩ = none := rfl

end QCore.FileClose
