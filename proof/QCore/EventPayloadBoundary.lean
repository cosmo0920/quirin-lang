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

namespace QCore.EventPayloadBoundary

/-! Audited abstract model of the `cb_event` payload boundary.  This proves the
closed path/target, extent arithmetic, authority and context-substitution rules.
Faithful Clang extraction, native observations, hashing and Rust/Lean
correspondence remain explicit external premises. -/

inductive Target where
  | firedFd | firedMask | firedData | firedCount
  deriving DecidableEq, Repr

def exactTargets : List Target :=
  [.firedFd, .firedMask, .firedData, .firedCount]

structure SourcePath where
  readTaken : Bool
  writeTaken : Bool
  targets : List Target
  deriving DecidableEq, Repr

def sourcePaths : List SourcePath :=
  [ ⟨true, true, exactTargets⟩,
    ⟨true, false, exactTargets⟩,
    ⟨false, true, exactTargets⟩,
    ⟨false, false, exactTargets⟩ ]

def sourceProjectionAccepted (paths : List SourcePath) : Bool :=
  paths.length == 4 && paths.all (fun path => path.targets == exactTargets)

theorem accepted_projection_has_four_paths
    {paths : List SourcePath} (accepted : sourceProjectionAccepted paths = true) :
    paths.length = 4 := by
  simp [sourceProjectionAccepted] at accepted
  exact accepted.1

theorem accepted_projection_has_exact_writes
    {paths : List SourcePath} (accepted : sourceProjectionAccepted paths = true) :
    ∀ path ∈ paths, path.targets = exactTargets := by
  simp [sourceProjectionAccepted] at accepted
  exact accepted.2

theorem all_four_guard_alternatives_are_present : sourceProjectionAccepted sourcePaths = true := by
  decide

structure AllocationShape where
  loopSize : Nat
  queueSize : Nat
  elementSize : Nat
  allocationBytes : Nat
  firedReceivesAllocation : Bool
  deriving DecidableEq, Repr

def allocationAccepted (shape : AllocationShape) : Bool :=
  shape.loopSize > 0 &&
  shape.queueSize == shape.loopSize &&
  shape.elementSize > 0 &&
  shape.allocationBytes == shape.loopSize * shape.elementSize &&
  shape.firedReceivesAllocation

theorem accepted_allocation_binds_queue_extent
    {shape : AllocationShape} (accepted : allocationAccepted shape = true) :
    shape.queueSize = shape.loopSize ∧
    shape.allocationBytes = shape.loopSize * shape.elementSize := by
  simp [allocationAccepted] at accepted
  omega

structure Access where
  index : Int
  capacity : Nat
  signedMax : Int
  elementSize : Nat
  offset : Nat
  extent : Nat
  deriving DecidableEq, Repr

def accessAccepted (access : Access) : Bool :=
  0 ≤ access.index &&
  access.index < access.signedMax &&
  access.index.toNat < access.capacity &&
  access.capacity > 0 &&
  access.elementSize > 0 &&
  access.offset == access.index.toNat * access.elementSize &&
  access.extent == access.capacity * access.elementSize &&
  access.offset < access.extent

theorem accepted_access_is_in_bounds
    {access : Access} (accepted : accessAccepted access = true) :
    0 ≤ access.index ∧ access.index.toNat < access.capacity := by
  simp [accessAccepted] at accepted
  omega

theorem accepted_access_has_source_sized_extent
    {access : Access} (accepted : accessAccepted access = true) :
    access.offset = access.index.toNat * access.elementSize ∧
    access.extent = access.capacity * access.elementSize := by
  simp [accessAccepted] at accepted
  omega

theorem negative_index_is_rejected (access : Access) (negative : access.index < 0) :
    accessAccepted access = false := by
  simp [accessAccepted, Int.not_le.mpr negative]

theorem signed_increment_overflow_is_rejected (access : Access)
    (atMax : access.index = access.signedMax) : accessAccepted access = false := by
  simp [accessAccepted, atMax]

inductive AccessKind where
  | read | write | atomicRead | atomicWrite
  deriving DecidableEq, Repr

structure Authority where
  context : AccessKind
  fired : AccessKind
  targets : List Target
  mayDefer : Bool
  maySpawn : Bool
  deriving DecidableEq, Repr

def authorityAccepted (authority : Authority) : Bool :=
  authority.context == .write &&
  authority.fired == .write &&
  authority.targets == exactTargets &&
  !authority.mayDefer && !authority.maySpawn

theorem accepted_authority_grants_both_writes
    {authority : Authority} (accepted : authorityAccepted authority = true) :
    authority.context = .write ∧ authority.fired = .write ∧
    authority.targets = exactTargets := by
  simp [authorityAccepted] at accepted
  exact ⟨accepted.1.1.1.1, accepted.1.1.1.2, accepted.1.1.2⟩

structure ContextBinding where
  sourceHash : Nat
  allocationSourceHash : Nat
  contractHash : Nat
  authorityHash : Nat
  callbackHash : Nat
  allocationEvidenceHash : Nat
  deriving DecidableEq, Repr

def contextAccepted (presented expected : ContextBinding) : Bool :=
  presented == expected

theorem accepted_context_is_exact
    {presented expected : ContextBinding}
    (accepted : contextAccepted presented expected = true) : presented = expected := by
  simpa [contextAccepted] using accepted

theorem source_substitution_is_rejected (context : ContextBinding) (different : Nat)
    (changed : different ≠ context.sourceHash) :
    contextAccepted { context with sourceHash := different } context = false := by
  simp [contextAccepted]
  intro equal
  apply changed
  exact congrArg ContextBinding.sourceHash equal

theorem authority_substitution_is_rejected (context : ContextBinding) (different : Nat)
    (changed : different ≠ context.authorityHash) :
    contextAccepted { context with authorityHash := different } context = false := by
  simp [contextAccepted]
  intro equal
  apply changed
  exact congrArg ContextBinding.authorityHash equal

theorem allocation_evidence_substitution_is_rejected
    (context : ContextBinding) (different : Nat)
    (changed : different ≠ context.allocationEvidenceHash) :
    contextAccepted { context with allocationEvidenceHash := different } context = false := by
  simp [contextAccepted]
  intro equal
  apply changed
  exact congrArg ContextBinding.allocationEvidenceHash equal

end QCore.EventPayloadBoundary
