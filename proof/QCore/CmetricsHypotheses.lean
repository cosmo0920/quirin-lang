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

import QCore.CmetricsApiContracts

/-! Mathematical antecedents for the declared CMetrics contracts.
These predicates describe one independently interpreted entry state and complete
call trace. Source extraction, exact heap/field and trace-domain interpretation,
SDS/atomic backend correctness, target ABI and Rust/model correspondence remain
open inputs. In particular, no declaration proves a relation input true and a
finite observed trace does not establish a complete-call exclusion property.
Float conversion is parameterized by an exact finite-value/truncation model;
it does not approximate C conversion by requiring the original value nonnegative.
No resource-v1 permission or verdict is produced by this module. -/
namespace QCore.CmetricsHypotheses

inductive SdsField where
  | ns | subsystem | name | description | fqname
  deriving DecidableEq, Repr

structure SdsEntry where
  fields : SdsField → Option Nat
  owner : Nat
  live : Nat → Prop
  sdsHandle : Nat → Prop
  owns : Nat → Nat → Prop

def allFiveNull (entry : SdsEntry) : Prop :=
  ∀ field, entry.fields field = none

theorem five_null_fields_decompose (entry : SdsEntry) :
    allFiveNull entry ↔
      entry.fields .ns = none ∧ entry.fields .subsystem = none ∧
      entry.fields .name = none ∧ entry.fields .description = none ∧
      entry.fields .fqname = none := by
  constructor
  · intro h
    exact ⟨h .ns, h .subsystem, h .name, h .description, h .fqname⟩
  · rintro ⟨ns, subsystem, name, description, fqname⟩ field
    cases field
    · exact ns
    · exact subsystem
    · exact name
    · exact description
    · exact fqname

def handlesValid (entry : SdsEntry) : Prop :=
  ∀ field handle, entry.fields field = some handle →
    entry.live handle ∧ entry.sdsHandle handle ∧ entry.owns entry.owner handle

/-- Distinct occupied fields denote distinct allocations, not just pointer bytes. -/
def distinctHandles (entry : SdsEntry) : Prop :=
  ∀ left right handle, entry.fields left = some handle →
    entry.fields right = some handle → left = right

/-- The caller supplies uniqueness of the semantic ownership relation. -/
def ownershipFunctional (entry : SdsEntry) : Prop :=
  ∀ left right handle, entry.owns left handle → entry.owns right handle → left = right

theorem valid_handles_decompose (entry : SdsEntry) :
    handlesValid entry ↔
      (∀ field handle, entry.fields field = some handle → entry.live handle) ∧
      (∀ field handle, entry.fields field = some handle → entry.sdsHandle handle) ∧
      (∀ field handle, entry.fields field = some handle → entry.owns entry.owner handle) := by
  constructor
  · intro h
    exact ⟨fun field handle stored => (h field handle stored).1,
      fun field handle stored => (h field handle stored).2.1,
      fun field handle stored => (h field handle stored).2.2⟩
  · rintro ⟨live, sds, owned⟩ field handle stored
    exact ⟨live field handle stored, sds field handle stored, owned field handle stored⟩

theorem occupied_fields_have_distinct_identities
    (entry : SdsEntry) (distinct : distinctHandles entry)
    (left right : SdsField) (leftHandle rightHandle : Nat)
    (leftStored : entry.fields left = some leftHandle)
    (rightStored : entry.fields right = some rightHandle) (different : left ≠ right) :
    leftHandle ≠ rightHandle := by
  intro same
  subst rightHandle
  exact different (distinct left right leftHandle leftStored rightStored)

theorem valid_handle_has_unique_owner
    (entry : SdsEntry) (valid : handlesValid entry)
    (functional : ownershipFunctional entry) (field : SdsField) (handle : Nat)
    (stored : entry.fields field = some handle) :
    entry.live handle ∧ entry.sdsHandle handle ∧
      entry.owns entry.owner handle ∧
      ∀ other, entry.owns other handle → other = entry.owner := by
  obtain ⟨live, sds, owned⟩ := valid field handle stored
  exact ⟨live, sds, owned, fun other otherOwned => functional other entry.owner handle otherOwned owned⟩

/-- This bridge is about the audited abstract cleanup state, not C extraction. -/
theorem explicit_cleanup_inputs_establish_canExit
    (state : CmetricsApiContracts.Opts) (storage : state.storageLive = true)
    (distinct : state.stored.Nodup)
    (owned : ∀ handle ∈ state.stored, handle ∈ state.owned) :
    CmetricsApiContracts.canExit state = true := by
  simp only [CmetricsApiContracts.canExit, storage, Bool.true_and, Bool.and_eq_true]
  constructor
  · exact decide_eq_true distinct
  · exact List.all_eq_true.mpr (fun handle member => by simpa using owned handle member)

/-- Reuse the existing audited cleanup theorems after its antecedent is checked. -/
theorem admitted_cleanup_preserves_record_and_releases_contents
    (state : CmetricsApiContracts.Opts)
    (ready : CmetricsApiContracts.canExit state = true) :
    ∃ out, CmetricsApiContracts.cleanup state = some out ∧
      out.owned = [] ∧ out.stored = state.stored ∧ out.storageLive = state.storageLive := by
  let out : CmetricsApiContracts.Opts := { state with owned := [] }
  have accepted : CmetricsApiContracts.cleanup state = some out := by
    simp [CmetricsApiContracts.cleanup, ready, out]
  exact ⟨out, accepted, CmetricsApiContracts.cleanup_discharges_contents state out accepted,
    CmetricsApiContracts.cleanup_does_not_clear_pointer_fields state out accepted,
    CmetricsApiContracts.cleanup_preserves_enclosing_storage state out accepted⟩

def quantileActive (metric array : Option Nat) (index count : Int) : Prop :=
  metric ≠ none ∧ array ≠ none ∧ 0 ≤ index ∧ index < count

/-- Null and out-of-range cases remain admitted no-ops; storage is conditional. -/
def guardedQuantile (metric array : Option Nat) (index count : Int)
    (storageLive writable atomicLayout : Prop) : Prop :=
  quantileActive metric array index count → storageLive ∧ writable ∧ atomicLayout

theorem guarded_quantile_decomposition
    (metric array : Option Nat) (index count : Int)
    (storageLive writable atomicLayout : Prop) :
    guardedQuantile metric array index count storageLive writable atomicLayout ↔
      (quantileActive metric array index count → storageLive) ∧
      (quantileActive metric array index count → writable) ∧
      (quantileActive metric array index count → atomicLayout) := by
  constructor
  · intro h
    exact ⟨fun active => (h active).1, fun active => (h active).2.1,
      fun active => (h active).2.2⟩
  · rintro ⟨live, write, atomic⟩ active
    exact ⟨live active, write active, atomic active⟩

theorem null_metric_needs_no_quantile_storage (array : Option Nat) (index count : Int)
    (storageLive writable atomicLayout : Prop) :
    guardedQuantile none array index count storageLive writable atomicLayout := by
  intro active
  exact False.elim (active.1 rfl)

theorem null_array_needs_no_quantile_storage (metric : Option Nat) (index count : Int)
    (storageLive writable atomicLayout : Prop) :
    guardedQuantile metric none index count storageLive writable atomicLayout := by
  intro active
  exact False.elim (active.2.1 rfl)

theorem invalid_index_needs_no_quantile_storage
    (metric array : Option Nat) (index count : Int) (outside : index < 0 ∨ count ≤ index)
    (storageLive writable atomicLayout : Prop) :
    guardedQuantile metric array index count storageLive writable atomicLayout := by
  intro active
  have lower := active.2.2.1
  have upper := active.2.2.2
  rcases outside with negative | after
  · omega
  · omega

/-- Call footprint includes owner, selected metric and all used nested storage. -/
def completeCallExclusion (footprint : Object → Prop) (during : Event → Prop)
    (conflict : Event → Object → Prop) : Prop :=
  ∀ object, footprint object → ∀ event, during event → ¬ conflict event object

theorem exclusion_union_decomposition
    (left right : Object → Prop) (during : Event → Prop)
    (conflict : Event → Object → Prop) :
    completeCallExclusion (fun object => left object ∨ right object) during conflict ↔
      completeCallExclusion left during conflict ∧ completeCallExclusion right during conflict := by
  constructor
  · intro h
    exact ⟨fun object member => h object (Or.inl member),
      fun object member => h object (Or.inr member)⟩
  · rintro ⟨leftExcluded, rightExcluded⟩ object member
    rcases member with leftMember | rightMember
    · exact leftExcluded object leftMember
    · exact rightExcluded object rightMember

theorem exclusion_preserved_by_footprint_subset
    (small large : Object → Prop) (during : Event → Prop)
    (conflict : Event → Object → Prop) (subset : ∀ object, small object → large object)
    (excluded : completeCallExclusion large during conflict) :
    completeCallExclusion small during conflict := by
  intro object member
  exact excluded object (subset object member)

/-- Integers model pre-wrap arithmetic; limits/width come from independently bound ABI. -/
def allocationBounds (count width sizeMaximum loopMaximum : Int) : Prop :=
  0 ≤ count ∧ 0 < width ∧ count + 1 ≤ loopMaximum ∧
    (count + 1) * width ≤ sizeMaximum

theorem allocation_bounds_imply_element_and_byte_range
    (count width sizeMaximum loopMaximum index : Int)
    (bounded : allocationBounds count width sizeMaximum loopMaximum)
    (lower : 0 ≤ index) (upper : index < count + 1) :
    index < loopMaximum ∧ 0 ≤ index * width ∧ index * width < sizeMaximum := by
  obtain ⟨countNonnegative, widthPositive, loopBound, byteBound⟩ := bounded
  have productNonnegative : 0 ≤ index * width :=
    Int.mul_nonneg lower (by omega)
  have productStrict : index * width < (count + 1) * width :=
    Int.mul_lt_mul_of_pos_right upper widthPositive
  exact ⟨by omega, productNonnegative, by omega⟩

theorem positive_width_allocation_implies_element_count_fits
    (count width sizeMaximum loopMaximum : Int)
    (bounded : allocationBounds count width sizeMaximum loopMaximum) :
    count + 1 ≤ sizeMaximum := by
  obtain ⟨countNonnegative, widthPositive, _, byteBound⟩ := bounded
  have oneWidth : 1 ≤ width := by omega
  have product := Int.mul_le_mul_of_nonneg_left oneWidth (by omega : 0 ≤ count + 1)
  have smaller : count + 1 ≤ (count + 1) * width := by simpa using product
  exact Int.le_trans smaller byteBound

/-- `trunc` must be independently interpreted as target C truncation toward zero. -/
structure TruncationModel (Floating : Type u) where
  finite : Floating → Prop
  trunc : Floating → Int

def conversionDefined (model : TruncationModel Floating) (value : Floating)
    (minimum maximum : Int) : Prop :=
  model.finite value ∧ minimum ≤ model.trunc value ∧ model.trunc value ≤ maximum

theorem signed_unsigned_conversion_intersection
    (model : TruncationModel Floating) (value : Floating)
    (signedMinimum signedMaximum unsignedMaximum : Int)
    (signedLower : signedMinimum ≤ 0) (unsignedWider : signedMaximum ≤ unsignedMaximum) :
    conversionDefined model value signedMinimum signedMaximum ∧
      conversionDefined model value 0 unsignedMaximum ↔
      model.finite value ∧ 0 ≤ model.trunc value ∧ model.trunc value ≤ signedMaximum := by
  constructor
  · rintro ⟨⟨finite, _, signedUpper⟩, ⟨_, unsignedLower, _⟩⟩
    exact ⟨finite, unsignedLower, signedUpper⟩
  · rintro ⟨finite, lower, upper⟩
    exact ⟨⟨finite, by omega, upper⟩, ⟨finite, lower, by omega⟩⟩

/-- In particular, an exact negative fraction with truncated value zero is allowed.
There is deliberately no premise that the original floating value is nonnegative. -/
theorem zero_truncated_finite_value_has_unsigned_conversion
    (model : TruncationModel Floating) (value : Floating) (maximum : Int)
    (finite : model.finite value) (zeroTruncated : model.trunc value = 0)
    (nonnegativeMaximum : 0 ≤ maximum) :
    conversionDefined model value 0 maximum := by
  exact ⟨finite, by omega, by omega⟩

end QCore.CmetricsHypotheses
