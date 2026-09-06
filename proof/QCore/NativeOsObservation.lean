namespace QCore.NativeOsObservation

-- Abstract final observation gate. The C observer, Python structural adapter,
-- Linux syscalls, endpoint probes and exclusive execution are trusted premises.
-- In particular, failed close does not in general imply descriptor liveness.
inductive Resource where
  | descriptor | mapping | pathname
  deriving DecidableEq, Repr

structure Observation where
  resource : Resource
  identityBefore : Nat
  identityAfter : Nat
  syscallSucceeded : Bool
  endpointAbsent : Bool
  staticCertificate : Bool
  durabilityProved : Bool

def accepted (o : Observation) : Prop :=
  o.identityBefore = o.identityAfter ∧
  (o.syscallSucceeded = true → o.endpointAbsent = true) ∧
  o.staticCertificate = false ∧ o.durabilityProved = false

theorem accepted_preserves_witness_identity {o : Observation} (h : accepted o) :
    o.identityBefore = o.identityAfter := h.1

theorem accepted_success_has_absent_endpoint {o : Observation} (h : accepted o)
    (success : o.syscallSucceeded = true) : o.endpointAbsent = true := h.2.1 success

theorem observation_does_not_promote_static_or_durability {o : Observation}
    (h : accepted o) : o.staticCertificate = false ∧ o.durabilityProved = false := h.2.2

theorem missing_endpoint_rejects_success {o : Observation}
    (success : o.syscallSucceeded = true) (present : o.endpointAbsent = false) :
    ¬ accepted o := by
  intro h
  have absent := accepted_success_has_absent_endpoint h success
  simp [present] at absent

end QCore.NativeOsObservation
