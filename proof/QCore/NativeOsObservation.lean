import QCore.NativeTeardownObservation

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

namespace QCore.NativeOsObservation

-- Independent resource ledgers: accepted allocation release never consumes
-- descriptor/mapping residuals recorded after the parent object is freed.
def acceptedTeardown (plan released : List Nat)
    (expectedResidual observedResidual : List (Resource × Nat))
    (durability : Bool) : Prop :=
  QCore.NativeTeardownObservation.accepted plan released ∧
  observedResidual = expectedResidual ∧ durability = false

theorem accepted_teardown_releases_exact_generations {plan released : List Nat}
    {expectedResidual observedResidual : List (Resource × Nat)} {durability : Bool}
    (h : acceptedTeardown plan released expectedResidual observedResidual durability) :
    released = plan ∧ released.Nodup :=
  ⟨h.1.2, QCore.NativeTeardownObservation.accepted_has_no_duplicate_release h.1⟩

theorem accepted_teardown_preserves_residual_identity {plan released : List Nat}
    {expectedResidual observedResidual : List (Resource × Nat)} {durability : Bool}
    (h : acceptedTeardown plan released expectedResidual observedResidual durability) :
    observedResidual = expectedResidual := h.2.1

theorem live_residual_prevents_empty_terminal {plan released : List Nat}
    {expectedResidual observedResidual : List (Resource × Nat)} {durability : Bool}
    (h : acceptedTeardown plan released expectedResidual observedResidual durability)
    (live : expectedResidual ≠ []) : observedResidual ≠ [] := by
  rw [h.2.1]
  exact live

theorem released_allocations_can_coexist_with_mapping_residual :
    acceptedTeardown [1] [1] [(.mapping, 7)] [(.mapping, 7)] false := by
  unfold acceptedTeardown QCore.NativeTeardownObservation.accepted
  decide

end QCore.NativeOsObservation

namespace QCore.NativeOsObservation

-- Partition an observed execution prefix. The process exit does not append
-- releases: instrumentation and the Python replay remain external premises.
structure ConstructorPrefix where
  transient : List Nat
  rollback : List Nat
  teardown : List Nat
  outstanding : List Nat
  acquired : List Nat
  interrupted : Bool
  staticCertificate : Bool

def acceptedPrefix (p : ConstructorPrefix) : Prop :=
  (p.transient ++ p.rollback ++ p.teardown ++ p.outstanding).Nodup ∧
  p.acquired.Perm (p.transient ++ p.rollback ++ p.teardown ++ p.outstanding) ∧
  (p.interrupted = true → p.teardown = [] ∧ p.outstanding ≠ []) ∧
  (p.interrupted = false → p.outstanding = []) ∧
  p.staticCertificate = false

theorem accepted_prefix_conserves_generations {p : ConstructorPrefix}
    (h : acceptedPrefix p) :
    p.acquired.length = p.transient.length + p.rollback.length +
      p.teardown.length + p.outstanding.length := by
  have length := h.2.1.length_eq
  simpa only [List.length_append] using length

theorem interrupted_prefix_cannot_claim_teardown {p : ConstructorPrefix}
    (h : acceptedPrefix p) (interrupted : p.interrupted = true) :
    p.teardown = [] ∧ p.outstanding ≠ [] := h.2.2.1 interrupted

theorem completed_prefix_has_no_outstanding_generations {p : ConstructorPrefix}
    (h : acceptedPrefix p) (completed : p.interrupted = false) :
    p.outstanding = [] := h.2.2.2.1 completed

theorem accepted_prefix_never_promotes_static {p : ConstructorPrefix}
    (h : acceptedPrefix p) : p.staticCertificate = false := h.2.2.2.2

end QCore.NativeOsObservation

namespace QCore.NativeOsObservation

-- A single observed open per exclusive execution. Allocation generations and
-- descriptor/inode identity occupy independent ledgers. These witnesses do not
-- infer the post-state of an arbitrary kernel close failure.
structure DescriptorWitness where
  execution : Nat
  descriptor : Nat
  device : Nat
  inode : Nat
  deriving DecidableEq, Repr

structure ConstructorRollback where
  plan : List Nat
  released : List Nat
  returnedNull : Bool
  expectedResidual : Option DescriptorWitness
  afterReturn : Option DescriptorWitness
  afterParents : Option DescriptorWitness
  staticCertificate : Bool

def acceptedRollback (o : ConstructorRollback) : Prop :=
  QCore.NativeTeardownObservation.accepted o.plan o.released ∧
  o.returnedNull = true ∧ o.afterReturn = o.expectedResidual ∧
  o.afterParents = o.afterReturn ∧ o.staticCertificate = false

theorem accepted_rollback_preserves_descriptor_identity {o : ConstructorRollback}
    (h : acceptedRollback o) : o.afterParents = o.expectedResidual :=
  h.2.2.2.1.trans h.2.2.1

theorem null_constructor_does_not_erase_observed_descriptor {o : ConstructorRollback}
    {w : DescriptorWitness} (h : acceptedRollback o)
    (residual : o.expectedResidual = some w) : o.afterParents = some w := by
  rw [accepted_rollback_preserves_descriptor_identity h, residual]

theorem accepted_rollback_separately_releases_allocation_generations {o : ConstructorRollback}
    (h : acceptedRollback o) : o.released = o.plan ∧ o.released.Nodup :=
  ⟨h.1.2, QCore.NativeTeardownObservation.accepted_has_no_duplicate_release h.1⟩

theorem null_constructor_can_release_allocations_and_retain_descriptor :
    acceptedRollback {
      plan := [5,4,10,11,9,8,7,6,2,3,1]
      released := [5,4,10,11,9,8,7,6,2,3,1]
      returnedNull := true
      expectedResidual := some ⟨206,3,47,1⟩
      afterReturn := some ⟨206,3,47,1⟩
      afterParents := some ⟨206,3,47,1⟩
      staticCertificate := false } := by
  unfold acceptedRollback QCore.NativeTeardownObservation.accepted
  decide

end QCore.NativeOsObservation

namespace QCore.NativeOsObservation

-- Bounded existing-file observations distinguish the public constructor result
-- from the chunk-local error recorded before rollback releases that object.
-- No assertion is made about unobserved bytes, flags or native callers.
inductive ExistingProfile where
  | validRW | chmodFailure | chmodCloseFailure | badMagic | shortHeader | emptyRD | validRD
  deriving DecidableEq, Repr

def existingErrors : ExistingProfile → Int × Int
  | .validRW | .validRD => (0, 0)
  | .chmodFailure | .chmodCloseFailure => (-1, 0)
  | .badMagic => (-3, -11)
  | .shortHeader => (-3, -13)
  | .emptyRD => (-3, -12)

structure ExistingFileObservation where
  profile : ExistingProfile
  publicError : Int
  localError : Int
  initialBytes : List Nat
  finalBytes : List Nat
  before : DescriptorWitness
  after : DescriptorWitness
  staticCertificate : Bool
  durabilityProved : Bool

def acceptedExisting (o : ExistingFileObservation) : Prop :=
  (o.publicError, o.localError) = existingErrors o.profile ∧
  o.finalBytes = o.initialBytes ∧ o.after = o.before ∧
  o.staticCertificate = false ∧ o.durabilityProved = false

theorem existing_corruption_preserves_specific_error {o : ExistingFileObservation}
    (h : acceptedExisting o) (profile : o.profile = .badMagic) :
    o.publicError = -3 ∧ o.localError = -11 := by
  have errors := h.1
  simpa [profile, existingErrors, Prod.mk.injEq] using errors

theorem readonly_empty_differs_from_short_header :
    existingErrors .emptyRD ≠ existingErrors .shortHeader := by decide

theorem accepted_existing_retains_bytes_and_identity {o : ExistingFileObservation}
    (h : acceptedExisting o) : o.finalBytes = o.initialBytes ∧ o.after = o.before :=
  ⟨h.2.1, h.2.2.1⟩

theorem accepted_existing_never_promotes_static_or_durability {o : ExistingFileObservation}
    (h : acceptedExisting o) : o.staticCertificate = false ∧ o.durabilityProved = false := h.2.2.2

end QCore.NativeOsObservation

namespace QCore.NativeOsObservation

-- Executable projection of the bounded, successful-syscall native sequences.
-- The mapped header length is deliberately distinct from cf->data_size.
-- This is not a total contract for arbitrary flags, capacity or OS failures.
structure NativeFileState where
  logicalLength : Nat
  headerLength : Nat
  checkpoint : Nat
  transaction : Bool
  locked : Bool
  mapped : Bool
  epoch : Nat
  deriving DecidableEq, Repr

def nativeBegin (s : NativeFileState) : Int × NativeFileState :=
  if s.locked then (-2, s)
  else if s.transaction then (0, s)
  else (0, { s with transaction := true, checkpoint := s.logicalLength })

def nativeRollback (s : NativeFileState) : Int × NativeFileState :=
  if s.transaction then
    (0, { s with logicalLength := s.checkpoint, transaction := false })
  else (-1, s)

-- Caller supplies the fixed mapped buffer/capacity premise. The observed
-- cio_chunk_write path does not consult the chunk lock flag.
def nativeAppend (s : NativeFileState) (count : Nat) : NativeFileState :=
  { s with logicalLength := s.logicalLength + count, headerLength := s.logicalLength + count }

def nativeDown (s : NativeFileState) : NativeFileState :=
  { s with logicalLength := 0, mapped := false }

def nativeReload (s : NativeFileState) : NativeFileState :=
  { s with logicalLength := s.headerLength, mapped := true, epoch := s.epoch + 1 }

theorem native_rollback_preserves_mapped_header (s : NativeFileState) :
    (nativeRollback s).2.headerLength = s.headerLength := by
  unfold nativeRollback
  split <;> rfl

theorem native_rollback_restores_checkpoint {s : NativeFileState}
    (active : s.transaction = true) :
    (nativeRollback s).2.logicalLength = s.checkpoint ∧
    (nativeRollback s).2.transaction = false := by
  simp [nativeRollback, active]

theorem native_locked_begin_is_retry {s : NativeFileState} (locked : s.locked = true) :
    nativeBegin s = (-2, s) := by simp [nativeBegin, locked]

theorem native_append_preserves_lock_and_updates_lengths (s : NativeFileState) (count : Nat) :
    (nativeAppend s count).locked = s.locked ∧
    (nativeAppend s count).logicalLength = s.logicalLength + count ∧
    (nativeAppend s count).headerLength = s.logicalLength + count := ⟨rfl, rfl, rfl⟩

theorem native_reload_advances_epoch_and_uses_header (s : NativeFileState) :
    (nativeReload (nativeDown s)).epoch = s.epoch + 1 ∧
    (nativeReload (nativeDown s)).logicalLength = s.headerLength := ⟨rfl, rfl⟩

theorem observed_rollback_reload_restores_appended_length :
    let initial : NativeFileState := ⟨4,4,0,false,false,true,1⟩
    let appended := nativeAppend (nativeBegin initial).2 3
    let rolledBack := (nativeRollback appended).2
    rolledBack.logicalLength = 4 ∧ rolledBack.headerLength = 7 ∧
      (nativeReload (nativeDown rolledBack)).logicalLength = 7 := by decide

end QCore.NativeOsObservation
