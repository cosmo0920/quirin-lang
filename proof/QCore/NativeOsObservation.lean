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

-- Conditional projection of the fixed one-shot msync suppression profiles.
-- Other size/trim/checksum failures are outside this model. The down operation
-- assumes successful unmap, size update and close; the native adapter checks
-- their concrete identities and results separately.
structure NativeSyncState where
  file : NativeFileState
  synced : Bool
  deriving DecidableEq, Repr

def nativeCommitSync (s : NativeSyncState) (syncSucceeded : Bool) : Int × NativeSyncState :=
  if !s.file.mapped || s.synced then
    (0, { s with file := { s.file with transaction := false } })
  else if syncSucceeded then
    (0, { file := { s.file with transaction := false }, synced := true })
  else (-1, s)

def nativeDownSync (s : NativeSyncState) (syncSucceeded : Bool) : NativeSyncState :=
  { file := nativeDown s.file, synced := s.synced || syncSucceeded }

theorem failed_native_commit_preserves_entire_state {s : NativeSyncState}
    (mapped : s.file.mapped = true) (dirty : s.synced = false) :
    nativeCommitSync s false = (-1, s) := by
  simp [nativeCommitSync, mapped, dirty]

theorem retried_native_commit_clears_transaction {s : NativeSyncState}
    (mapped : s.file.mapped = true) (dirty : s.synced = false) :
    (nativeCommitSync s true).1 = 0 ∧
    (nativeCommitSync s true).2.file.transaction = false ∧
    (nativeCommitSync s true).2.synced = true := by
  simp [nativeCommitSync, mapped, dirty]

theorem unmapped_native_commit_can_succeed_without_sync {s : NativeSyncState}
    (unmapped : s.file.mapped = false) :
    (nativeCommitSync s false).1 = 0 ∧
    (nativeCommitSync s false).2.file.transaction = false ∧
    (nativeCommitSync s false).2.synced = s.synced := by
  simp [nativeCommitSync, unmapped]

theorem failed_sync_down_preserves_transaction_and_header (s : NativeSyncState) :
    (nativeDownSync s false).file.transaction = s.file.transaction ∧
    (nativeDownSync s false).file.headerLength = s.file.headerLength ∧
    (nativeDownSync s false).file.mapped = false ∧
    (nativeDownSync s false).synced = s.synced := by
  simp [nativeDownSync, nativeDown]

theorem observed_failed_sync_down_commit_does_not_prove_sync :
    let dirty : NativeSyncState := ⟨⟨7,7,4,true,false,true,1⟩,false⟩
    let down := nativeDownSync dirty false
    let committed := nativeCommitSync down false
    committed.1 = 0 ∧ committed.2.synced = false ∧
    committed.2.file.transaction = false ∧
    (nativeReload committed.2.file).logicalLength = 7 := by decide

-- The input is a mapped chunk. Failure is an explicitly supplied suppressed
-- unmap result; successful close/size update remain premises of the other arm.
def nativeDownUnmap (s : NativeSyncState) (syncSucceeded unmapSucceeded : Bool) :
    Int × NativeSyncState :=
  if unmapSucceeded then (0, nativeDownSync s syncSucceeded)
  else (-1, { s with synced := s.synced || syncSucceeded })

theorem failed_unmap_preserves_file_state (s : NativeSyncState) (synced : Bool) :
    (nativeDownUnmap s synced false).1 = -1 ∧
    (nativeDownUnmap s synced false).2.file = s.file := ⟨rfl, rfl⟩

theorem successful_sync_survives_failed_unmap (s : NativeSyncState) :
    (nativeDownUnmap s true false).2.synced = true := by
  simp [nativeDownUnmap]

theorem combined_sync_unmap_failure_retains_state (s : NativeSyncState) :
    nativeDownUnmap s false false = (-1, s) := by
  simp [nativeDownUnmap]

theorem retry_after_failed_unmap_releases_same_epoch (s : NativeSyncState) (synced : Bool) :
    let retry := nativeDownUnmap (nativeDownUnmap s synced false).2 true true
    retry.1 = 0 ∧ retry.2.file.mapped = false ∧
    retry.2.file.epoch = s.file.epoch ∧ retry.2.file.logicalLength = 0 :=
  ⟨rfl, rfl, rfl, rfl⟩

theorem observed_sync_before_unmap_failure_changes_only_sync :
    let dirty : NativeSyncState := ⟨⟨7,7,0,false,false,true,1⟩,false⟩
    let failed := nativeDownUnmap dirty true false
    failed.1 = -1 ∧ failed.2.file = dirty.file ∧ failed.2.synced = true ∧
    failed.2 ≠ dirty := by decide

end QCore.NativeOsObservation

namespace QCore.NativeOsObservation

-- Bounded descriptor-failure projection, separate from arbitrary kernel errors.
-- Close suppression leaves the actual descriptor open. A down operation here
-- assumes successful unmap and size update on an initially mapped clean chunk.
-- The epoch names the same descriptor/inode witness checked by the adapter.
structure NativeDescriptorState where
  mapped : Bool
  upMember : Bool
  descriptorHeld : Bool
  epoch : Nat
  deriving DecidableEq, Repr

def nativeDescriptorDown (s : NativeDescriptorState) (closeSuppressed : Bool) :
    Int × NativeDescriptorState :=
  if !s.mapped then (-1, s)
  else (if closeSuppressed then -1 else 0,
        { s with mapped := false, upMember := false, descriptorHeld := closeSuppressed })

-- Open failure executes no syscall in this profile. Successful reopen assumes
-- the observed size/map/header operations succeed before up membership changes.
def nativeDescriptorUp (s : NativeDescriptorState) (openSuppressed : Bool) :
    Int × NativeDescriptorState :=
  if s.mapped || s.descriptorHeld || openSuppressed then (-1, s)
  else (0, ⟨true, true, true, s.epoch + 1⟩)

-- Parent allocations are released independently; the output is the remaining
-- descriptor epoch ledger, before any separately observed harness reclamation.
def nativeDescriptorParentResidual (s : NativeDescriptorState) (closeSuppressed : Bool) :
    List Nat :=
  if s.descriptorHeld && closeSuppressed then [s.epoch] else []

theorem suppressed_close_moves_down_but_retains_identity {s : NativeDescriptorState}
    (mapped : s.mapped = true) :
    (nativeDescriptorDown s true).1 = -1 ∧
    (nativeDescriptorDown s true).2 = ⟨false, false, true, s.epoch⟩ := by
  simp [nativeDescriptorDown, mapped]

theorem down_retry_after_suppressed_close_preserves_descriptor {s : NativeDescriptorState}
    (mapped : s.mapped = true) :
    let down := (nativeDescriptorDown s true).2
    nativeDescriptorDown down false = (-1, down) := by
  simp [nativeDescriptorDown, mapped]

theorem reopen_after_suppressed_close_rejects_without_new_epoch {s : NativeDescriptorState}
    (mapped : s.mapped = true) :
    let down := (nativeDescriptorDown s true).2
    nativeDescriptorUp down false = (-1, down) := by
  simp [nativeDescriptorDown, nativeDescriptorUp, mapped]

theorem suppressed_open_preserves_descriptor_state (s : NativeDescriptorState) :
    nativeDescriptorUp s true = (-1, s) := by simp [nativeDescriptorUp]

theorem successful_reopen_acquires_fresh_epoch {s : NativeDescriptorState}
    (unmapped : s.mapped = false) (closed : s.descriptorHeld = false) :
    nativeDescriptorUp s false = (0, ⟨true, true, true, s.epoch + 1⟩) := by
  simp [nativeDescriptorUp, unmapped, closed]

theorem second_suppressed_close_retains_original_parent_residual {s : NativeDescriptorState}
    (mapped : s.mapped = true) :
    nativeDescriptorParentResidual (nativeDescriptorDown s true).2 true = [s.epoch] := by
  simp [nativeDescriptorDown, nativeDescriptorParentResidual, mapped]

theorem successful_parent_close_discharges_descriptor (s : NativeDescriptorState) :
    nativeDescriptorParentResidual s false = [] := by simp [nativeDescriptorParentResidual]

end QCore.NativeOsObservation

namespace QCore.NativeOsObservation

-- Bounded reopen-failure projection. The input is a chunk already taken down,
-- with its mapping and descriptor released. Every arm assumes the observed
-- reopen `open` call succeeds; the failure arms are explicitly supplied
-- suppressed results for the size lookup and for the mapping call. Arbitrary
-- kernel errors, other flags and production callers stay outside the model.

-- The backend mapping field, which the library tests only for non-nullness.
-- `failed` is the MAP_FAILED value a failed mmap leaves in that field.
inductive ReopenMapField where
  | absent | failed | live
  deriving DecidableEq, Repr

def ReopenMapField.isMapped : ReopenMapField → Bool
  | .absent => false
  | .failed => true
  | .live => true

inductive ReopenOutcome where
  | sizeLookupFailed | mappingFailed | mapped
  deriving DecidableEq, Repr

structure NativeReopenState where
  mapping : ReopenMapField
  descriptorHeld : Bool
  epoch : Nat
  mappingEpoch : Nat
  -- Observed state-ring membership, recorded separately from the counter.
  upMember : Bool
  upCounter : Nat
  recordedSize : Nat
  deriving DecidableEq, Repr

-- `cio_file_is_up` is descriptor presence conjoined with a non-null mapping
-- field, so a stored MAP_FAILED still classifies the chunk as up.
def NativeReopenState.isUp (s : NativeReopenState) : Bool :=
  s.descriptorHeld && s.mapping.isMapped

def nativeReopen (s : NativeReopenState) (outcome : ReopenOutcome) :
    Int × NativeReopenState :=
  if s.mapping.isMapped || s.descriptorHeld then (-1, s)
  else match outcome with
    | .sizeLookupFailed =>
        (-1, { s with descriptorHeld := true, epoch := s.epoch + 1,
                      recordedSize := 0, upMember := false })
    | .mappingFailed =>
        (-1, { s with descriptorHeld := true, epoch := s.epoch + 1,
                      mapping := .failed, upMember := true })
    | .mapped =>
        (0, { s with descriptorHeld := true, epoch := s.epoch + 1,
                     mapping := .live, mappingEpoch := s.mappingEpoch + 1,
                     upMember := true, upCounter := s.upCounter + 1 })

-- Down rejects an absent mapping outright, and an unmap of the stored
-- MAP_FAILED value is refused by the kernel, so neither retry changes state.
def nativeReopenDown (s : NativeReopenState) : Int × NativeReopenState :=
  match s.mapping with
  | .absent => (-1, s)
  | .failed => (-1, s)
  | .live => (0, { s with mapping := .absent, descriptorHeld := false,
                          upMember := false, upCounter := s.upCounter - 1 })

-- Sync reports success whenever the mapping field is non-null and the chunk is
-- already flagged synced; it issues no syscall, so it witnesses no durability.
def nativeReopenSync (s : NativeReopenState) : Int × Bool :=
  (0, s.mapping == ReopenMapField.live)

def nativeReopenParentResidual (s : NativeReopenState) (closeSuppressed : Bool) :
    List Nat :=
  if s.descriptorHeld && closeSuppressed then [s.epoch] else []

-- The recorded post-down input: one released descriptor epoch, one released
-- mapping epoch, no membership and a size carried over from the down update.
def reopenDown : NativeReopenState :=
  { mapping := .absent, descriptorHeld := false, epoch := 1, mappingEpoch := 1,
    upMember := false, upCounter := 0, recordedSize := 4096 }

theorem reopen_membership_matches_library_predicate {s : NativeReopenState}
    (unmapped : s.mapping = .absent) (closed : s.descriptorHeld = false)
    (outcome : ReopenOutcome) :
    (nativeReopen s outcome).2.upMember = (nativeReopen s outcome).2.isUp := by
  cases outcome <;>
    simp [nativeReopen, NativeReopenState.isUp, ReopenMapField.isMapped, unmapped, closed]

theorem failed_size_lookup_retains_new_descriptor_without_mapping
    {s : NativeReopenState} (unmapped : s.mapping = .absent)
    (closed : s.descriptorHeld = false) :
    nativeReopen s .sizeLookupFailed =
      (-1, { s with descriptorHeld := true, epoch := s.epoch + 1,
                    recordedSize := 0, upMember := false }) := by
  simp [nativeReopen, ReopenMapField.isMapped, unmapped, closed]

theorem failed_size_lookup_keeps_chunk_down {s : NativeReopenState}
    (unmapped : s.mapping = .absent) (closed : s.descriptorHeld = false) :
    (nativeReopen s .sizeLookupFailed).2.upMember = false ∧
    (nativeReopen s .sizeLookupFailed).2.upCounter = s.upCounter ∧
    (nativeReopen s .sizeLookupFailed).2.mappingEpoch = s.mappingEpoch := by
  simp [nativeReopen, ReopenMapField.isMapped, unmapped, closed]

theorem retries_after_failed_size_lookup_reject {s : NativeReopenState}
    (unmapped : s.mapping = .absent) (closed : s.descriptorHeld = false)
    (outcome : ReopenOutcome) :
    let stuck := (nativeReopen s .sizeLookupFailed).2
    nativeReopenDown stuck = (-1, stuck) ∧ nativeReopen stuck outcome = (-1, stuck) := by
  simp [nativeReopen, nativeReopenDown, ReopenMapField.isMapped, unmapped, closed]

theorem failed_mapping_retains_descriptor_and_failed_field {s : NativeReopenState}
    (unmapped : s.mapping = .absent) (closed : s.descriptorHeld = false) :
    nativeReopen s .mappingFailed =
      (-1, { s with descriptorHeld := true, epoch := s.epoch + 1,
                    mapping := .failed, upMember := true }) := by
  simp [nativeReopen, ReopenMapField.isMapped, unmapped, closed]

-- The recorded divergence: the stream reports the chunk up while the context
-- counter was never incremented, so ring membership discharges nothing.
theorem failed_mapping_reports_up_without_counter {s : NativeReopenState}
    (unmapped : s.mapping = .absent) (closed : s.descriptorHeld = false) :
    (nativeReopen s .mappingFailed).2.upMember = true ∧
    (nativeReopen s .mappingFailed).2.upCounter = s.upCounter ∧
    (nativeReopen s .mappingFailed).2.mappingEpoch = s.mappingEpoch := by
  simp [nativeReopen, ReopenMapField.isMapped, unmapped, closed]

theorem retries_after_failed_mapping_reject {s : NativeReopenState}
    (unmapped : s.mapping = .absent) (closed : s.descriptorHeld = false)
    (outcome : ReopenOutcome) :
    let stuck := (nativeReopen s .mappingFailed).2
    nativeReopenDown stuck = (-1, stuck) ∧ nativeReopen stuck outcome = (-1, stuck) := by
  simp [nativeReopen, nativeReopenDown, ReopenMapField.isMapped, unmapped, closed]

theorem sync_after_failed_mapping_succeeds_without_live_mapping
    {s : NativeReopenState} (unmapped : s.mapping = .absent)
    (closed : s.descriptorHeld = false) :
    nativeReopenSync (nativeReopen s .mappingFailed).2 = (0, false) := by
  simp [nativeReopen, nativeReopenSync, ReopenMapField.isMapped, unmapped, closed]

theorem stat_fallback_reopen_acquires_both_epochs {s : NativeReopenState}
    (unmapped : s.mapping = .absent) (closed : s.descriptorHeld = false) :
    (nativeReopen s .mapped).1 = 0 ∧
    (nativeReopen s .mapped).2.epoch = s.epoch + 1 ∧
    (nativeReopen s .mapped).2.mappingEpoch = s.mappingEpoch + 1 ∧
    (nativeReopen s .mapped).2.upMember = true ∧
    (nativeReopen s .mapped).2.upCounter = s.upCounter + 1 := by
  simp [nativeReopen, ReopenMapField.isMapped, unmapped, closed]

-- Neither failed reopen releases the descriptor it acquired; only the parent
-- close does, and a suppressed parent close leaves that exact epoch behind.
theorem reopen_failures_leave_descriptor_for_parent (outcome : ReopenOutcome) :
    nativeReopenParentResidual (nativeReopen reopenDown outcome).2 true = [2] ∧
    nativeReopenParentResidual (nativeReopen reopenDown outcome).2 false = [] := by
  cases outcome <;>
    simp [nativeReopen, nativeReopenParentResidual, ReopenMapField.isMapped, reopenDown]

theorem reopen_observation_does_not_prove_durability :
    let stuck := (nativeReopen reopenDown .mappingFailed).2
    (nativeReopenSync stuck).1 = 0 ∧ stuck.mapping ≠ ReopenMapField.live ∧
    stuck.mappingEpoch = reopenDown.mappingEpoch := by
  simp [nativeReopen, nativeReopenSync, ReopenMapField.isMapped, reopenDown]

end QCore.NativeOsObservation
