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

/-!
Card O01: the bounded chunk-file integrity observation relation.

Model of `crates/qcore-core/src/chunk_integrity.rs`. It states, in the model,
the three separations the Rust validator enforces:

* a **content class** is derived from the recorded image alone, and an
  **injected operation failure** is a property of the harness wrapper; neither
  determines the other, and the fine-grained error code cannot tell a clean
  image with a suppressed call from a clean image without one;
* **library releases and harness reclamations are separate counters**, and no
  arm lets one stand in for the other;
* **no arm publishes a certificate or a durability claim** — the only verdict
  is `Verdict.observed`.

Premises, not results: that the recorded `Image` fields really are the bytes'
size, identification bytes, declared lengths and checksums (the Rust side
recomputes them, this side takes them as given), that a recorded execution is
the execution it claims to be, and that a suppressed call really did not reach
the kernel. Nothing here verifies ChunkIO, the filesystem or the kernel.
-/

namespace QCore.ChunkIntegrityObservation

/-- The class an image belongs to. -/
inductive ContentClass where
  | clean
  | truncatedHeader
  | badMagic
  | truncatedContent
  | badChecksum
  deriving DecidableEq, Repr

/-- The suppressed operation, if any. -/
inductive Injection where
  | none
  | mmap
  | close
  deriving DecidableEq, Repr

/-- Complete enumerations, used to decide statements over the whole domain. -/
def allClasses : List ContentClass :=
  [.clean, .truncatedHeader, .badMagic, .truncatedContent, .badChecksum]

def allInjections : List Injection := [.none, .mmap, .close]

theorem mem_allClasses (c : ContentClass) : c ∈ allClasses := by
  cases c <;> simp [allClasses]

theorem mem_allInjections (i : Injection) : i ∈ allInjections := by
  cases i <;> simp [allInjections]

/-- The facts a recorded image carries. Extracting them from the bytes is a
premise; the Rust side recomputes every one of them independently. -/
structure Image where
  size : Nat
  magicOk : Bool
  metadataLength : Nat
  declaredContentLength : Nat
  storedChecksum : Nat
  computedChecksum : Nat
  deriving DecidableEq, Repr

/-- `CIO_FILE_HEADER_MIN`. -/
def headerMin : Nat := 24

/-- Derive the class of an image. The order of the tests is the order the
observed format check decides them. -/
def classify (i : Image) : ContentClass :=
  if i.size < headerMin then .truncatedHeader
  else if i.magicOk = false then .badMagic
  else if i.size < headerMin + i.metadataLength + i.declaredContentLength then .truncatedContent
  else if i.storedChecksum ≠ i.computedChecksum then .badChecksum
  else .clean

/-- Whether a class is a corruption at all. -/
def ContentClass.isCorrupt : ContentClass → Bool
  | .clean => false
  | _ => true

/-- `CIO_OK`, `CIO_ERROR`, `CIO_CORRUPTED`. -/
def cioOk : Int := 0
def cioError : Int := -1
def cioCorrupted : Int := -3

/-- The fine-grained `cio_error` code a class leaves on the chunk. -/
def ContentClass.errorCode : ContentClass → Int
  | .clean => 0
  | .truncatedHeader => -13
  | .truncatedContent => -13
  | .badMagic => -11
  | .badChecksum => -10

/-- What the constructor reported. -/
structure Outcome where
  chunk : Bool
  openError : Int
  chunkError : Int
  deriving DecidableEq, Repr

/-- The outcome the two axes determine. The class decides whether the open is
refused as corrupted; the injection decides whether a clean open is refused as
an ordinary error. A suppressed `close` is not reached until rollback or
teardown, so it leaves the constructor's report alone. -/
def expectedOutcome (c : ContentClass) (i : Injection) : Outcome :=
  if c.isCorrupt then
    { chunk := false, openError := cioCorrupted, chunkError := c.errorCode }
  else
    match i with
    | .mmap => { chunk := false, openError := cioError, chunkError := 0 }
    | .none | .close => { chunk := true, openError := cioOk, chunkError := 0 }

/-- Allocation generations, split by who released them. -/
structure AllocationLedger where
  acquired : Nat
  libraryReleased : Nat
  harnessReleased : Nat
  outstanding : Nat
  deriving DecidableEq, Repr

/-- One endpoint kind's epochs, split by who released them. -/
structure EndpointLedger where
  epochs : Nat
  libraryReleased : Nat
  harnessReleased : Nat
  residualAfterParents : Bool
  deriving DecidableEq, Repr

/-- One recorded fixture execution. -/
structure Record where
  declaredClass : ContentClass
  imageBefore : Image
  imageAfter : Image
  injection : Injection
  injectedErrno : Int
  suppressedCalls : Nat
  outcome : Outcome
  allocations : AllocationLedger
  descriptors : EndpointLedger
  mappings : EndpointLedger
  reclaimedDescriptors : Nat
  reclaimedMappings : Nat
  inodePreserved : Bool
  sizePreserved : Bool
  deriving DecidableEq, Repr

/-- The only verdict this relation can reach. -/
inductive Verdict where
  | observed
  deriving DecidableEq, Repr

/-- What a validated case establishes. -/
structure CaseVerdict where
  derivedClass : ContentClass
  injection : Injection
  residualEndpoints : Nat
  harnessReclaimed : Nat
  verdict : Verdict
  deriving DecidableEq, Repr

/-- One endpoint ledger is accounted for exactly when its epochs are split
between the two counters, its residual matches a harness reclamation, and a
suppressed call on that endpoint really left the harness something to do. -/
def endpointAccounted (l : EndpointLedger) (reclaimed : Nat) (suppressedHere : Bool) : Bool :=
  l.epochs == l.libraryReleased + l.harnessReleased &&
  (l.residualAfterParents == decide (0 < l.harnessReleased)) &&
  l.harnessReleased == reclaimed &&
  (!suppressedHere || 0 < l.harnessReleased)

/-- The validation relation, decided. -/
def accepted (r : Record) : Bool :=
  (classify r.imageBefore == r.declaredClass) &&
  (classify r.imageAfter == classify r.imageBefore) &&
  r.inodePreserved && r.sizePreserved &&
  ((r.injection != Injection.none) == decide (0 < r.suppressedCalls)) &&
  ((r.injection != Injection.none) == decide (r.injectedErrno ≠ 0)) &&
  (expectedOutcome (classify r.imageBefore) r.injection == r.outcome) &&
  (r.allocations.acquired ==
    r.allocations.libraryReleased + r.allocations.harnessReleased + r.allocations.outstanding) &&
  (r.allocations.outstanding == 0) &&
  (r.allocations.harnessReleased == 0) &&
  endpointAccounted r.descriptors r.reclaimedDescriptors (r.injection == Injection.close) &&
  endpointAccounted r.mappings r.reclaimedMappings false &&
  (r.descriptors.residualAfterParents == (r.injection == Injection.close))

/-- The verdict a validated record publishes. -/
def validate (r : Record) : Option CaseVerdict :=
  if accepted r then
    some { derivedClass := classify r.imageBefore,
           injection := r.injection,
           residualEndpoints :=
             (if r.descriptors.residualAfterParents then 1 else 0) +
             (if r.mappings.residualAfterParents then 1 else 0),
           harnessReclaimed := r.reclaimedDescriptors + r.reclaimedMappings,
           verdict := Verdict.observed }
  else none

-- ---------------------------------------------------------------------------
-- Classification
-- ---------------------------------------------------------------------------

/-- Every class is reachable, so the taxonomy is not vacuous. -/
theorem every_content_class_is_reachable :
    ∀ c : ContentClass, ∃ i : Image, classify i = c := by
  intro c
  cases c
  · exact ⟨⟨24, true, 0, 0, 7, 7⟩, by decide⟩
  · exact ⟨⟨8, true, 0, 0, 0, 0⟩, by decide⟩
  · exact ⟨⟨24, false, 0, 0, 0, 0⟩, by decide⟩
  · exact ⟨⟨24, true, 0, 4, 0, 0⟩, by decide⟩
  · exact ⟨⟨24, true, 0, 0, 7, 9⟩, by decide⟩

/-- A short image is a size failure before anything can be read from it, even
when its identification bytes are also destroyed. -/
theorem a_short_image_is_a_size_failure_whatever_else_is_wrong (i : Image)
    (short : i.size < headerMin) : classify i = ContentClass.truncatedHeader := by
  simp [classify, short]

/-- Destroyed identification bytes are decided before the checksum, so a file
that is both is reported as a layout failure. -/
theorem broken_magic_is_decided_before_the_checksum (i : Image)
    (long : ¬ i.size < headerMin) (magic : i.magicOk = false) :
    classify i = ContentClass.badMagic := by
  simp [classify, long, magic]

/-- A clean image satisfies every rule, checksum included. -/
theorem a_clean_image_satisfies_every_rule (i : Image) (h : classify i = ContentClass.clean) :
    ¬ i.size < headerMin ∧ i.magicOk = true ∧
      ¬ i.size < headerMin + i.metadataLength + i.declaredContentLength ∧
      i.storedChecksum = i.computedChecksum := by
  unfold classify at h
  by_cases short : i.size < headerMin
  · simp [short] at h
  by_cases magic : i.magicOk = false
  · simp [short, magic] at h
  by_cases truncated : i.size < headerMin + i.metadataLength + i.declaredContentLength
  · simp [short, magic, truncated] at h
  by_cases sum : i.storedChecksum ≠ i.computedChecksum
  · simp [short, magic, truncated, sum] at h
  · exact ⟨short, by simpa using magic, truncated, by simpa using sum⟩

/-- A disagreeing stored checksum on an otherwise well-formed image is a
checksum failure, and nothing else. -/
theorem a_disagreeing_checksum_is_a_checksum_failure (i : Image)
    (long : ¬ i.size < headerMin) (magic : i.magicOk = true)
    (fits : ¬ i.size < headerMin + i.metadataLength + i.declaredContentLength)
    (sum : i.storedChecksum ≠ i.computedChecksum) :
    classify i = ContentClass.badChecksum := by
  simp [classify, long, magic, fits, sum]

-- ---------------------------------------------------------------------------
-- The two axes stay separate
-- ---------------------------------------------------------------------------

/-- An injection never changes the derived class: the class is a function of
the image alone. -/
theorem an_injection_never_changes_the_derived_class (r : Record) (i : Injection) :
    classify { r with injection := i }.imageBefore = classify r.imageBefore := rfl

/-- A corrupted input and a clean input with a suppressed mapping are different
recorded outcomes, over the whole class domain. -/
theorem a_corruption_is_never_an_injected_failure :
    ∀ c ∈ allClasses, c.isCorrupt = true →
      expectedOutcome c Injection.none ≠ expectedOutcome ContentClass.clean Injection.mmap := by
  decide

/-- The fine-grained error code alone cannot separate the axes: a clean image
with a suppressed call and a clean image without one leave the same code. That
is the measured reason the class must be derived from the bytes. -/
theorem the_error_code_alone_cannot_separate_the_axes :
    (expectedOutcome ContentClass.clean Injection.mmap).chunkError =
      (expectedOutcome ContentClass.clean Injection.none).chunkError := by
  decide

/-- A corrupted image decides the outcome no matter what was suppressed, so an
injection can never be read out of a corrupted case's report. -/
theorem a_corrupted_image_decides_the_outcome_alone :
    ∀ c ∈ allClasses, ∀ i ∈ allInjections, ∀ j ∈ allInjections,
      c.isCorrupt = true → expectedOutcome c i = expectedOutcome c j := by
  decide

/-- Every corrupted class reports `CIO_CORRUPTED`, and no clean case does. -/
theorem only_a_corrupted_image_reports_corruption :
    ∀ c ∈ allClasses, ∀ i ∈ allInjections,
      ((expectedOutcome c i).openError = cioCorrupted) = c.isCorrupt := by
  decide

/-- The three fine-grained corruption codes are pairwise distinct, so the class
survives the projection onto codes. -/
theorem the_corruption_codes_are_pairwise_distinct :
    ContentClass.badChecksum.errorCode ≠ ContentClass.badMagic.errorCode ∧
    ContentClass.badChecksum.errorCode ≠ ContentClass.truncatedHeader.errorCode ∧
    ContentClass.badMagic.errorCode ≠ ContentClass.truncatedContent.errorCode := by
  decide

/-- A returned chunk is possible only for a clean image with no suppressed
mapping, over the whole domain. -/
theorem a_chunk_is_returned_only_for_a_clean_unsuppressed_open :
    ∀ c ∈ allClasses, ∀ i ∈ allInjections,
      (expectedOutcome c i).chunk = true → c = ContentClass.clean ∧ i ≠ Injection.mmap := by
  decide

-- ---------------------------------------------------------------------------
-- Custody
-- ---------------------------------------------------------------------------

/-- An accepted record accounts for every allocation generation it acquired. -/
theorem accepted_allocations_balance {r : Record} (h : accepted r = true) :
    r.allocations.acquired =
      r.allocations.libraryReleased + r.allocations.harnessReleased +
        r.allocations.outstanding := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.2

/-- An accepted record leaves no generation live. -/
theorem accepted_leaves_no_outstanding_generation {r : Record} (h : accepted r = true) :
    r.allocations.outstanding = 0 := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.2

/-- The harness never releases a library allocation: that counter is zero in
every accepted record, so the two are separated rather than merged. -/
theorem the_harness_never_releases_a_library_allocation {r : Record}
    (h : accepted r = true) : r.allocations.harnessReleased = 0 := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.2

/-- An accepted endpoint ledger splits its epochs between the library and the
harness; neither counter absorbs the other. -/
theorem accepted_endpoint_epochs_are_split {l : EndpointLedger} {reclaimed : Nat} {s : Bool}
    (h : endpointAccounted l reclaimed s = true) :
    l.epochs = l.libraryReleased + l.harnessReleased := by
  unfold endpointAccounted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1

/-- A residual is exactly a harness reclamation: an endpoint cannot be left
outstanding without someone recorded as reclaiming it, and a reclamation cannot
be recorded without a residual. -/
theorem a_residual_is_exactly_a_harness_reclamation {l : EndpointLedger} {reclaimed : Nat}
    {s : Bool} (h : endpointAccounted l reclaimed s = true) :
    l.residualAfterParents = decide (0 < l.harnessReleased) ∧ l.harnessReleased = reclaimed := by
  unfold endpointAccounted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact ⟨h.1.1.2, h.1.2⟩

/-- A suppressed call on an endpoint always leaves the harness something to
reclaim, so a suppressed close can never be credited to the library. -/
theorem a_suppressed_call_is_never_credited_to_the_library {l : EndpointLedger} {reclaimed : Nat}
    (h : endpointAccounted l reclaimed true = true) : 0 < l.harnessReleased := by
  unfold endpointAccounted at h
  simp only [Bool.and_eq_true] at h
  simpa using h.2

/-- Only a suppressed close leaves a descriptor residual in an accepted record;
a forwarded profile that claims one is refused. -/
theorem only_a_suppressed_close_leaves_a_descriptor_residual {r : Record}
    (h : accepted r = true) :
    r.descriptors.residualAfterParents = (r.injection == Injection.close) := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.2

/-- An accepted record's declared class really is the class of its bytes. -/
theorem accepted_declares_the_class_its_bytes_have {r : Record} (h : accepted r = true) :
    classify r.imageBefore = r.declaredClass := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.1.1.1.1.1.1.1

/-- An accepted record's image did not change class across the execution. -/
theorem accepted_content_keeps_its_class {r : Record} (h : accepted r = true) :
    classify r.imageAfter = classify r.imageBefore := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.1.1.1.1.1.1.2

/-- An accepted record's outcome is the outcome its two axes determine; a
report that disagrees is refused. -/
theorem accepted_outcome_is_the_determined_one {r : Record} (h : accepted r = true) :
    expectedOutcome (classify r.imageBefore) r.injection = r.outcome := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.1.2

/-- An injection is accounted for by a suppressed call in both directions. -/
theorem an_injection_is_accounted_for_in_both_directions {r : Record} (h : accepted r = true) :
    (r.injection != Injection.none) = decide (0 < r.suppressedCalls) := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.1.1.1.2

/-- A suppressed call is recorded with a nonzero errno, in both directions. -/
theorem a_suppressed_call_carries_an_errno {r : Record} (h : accepted r = true) :
    (r.injection != Injection.none) = decide (r.injectedErrno ≠ 0) := by
  unfold accepted at h
  simp only [Bool.and_eq_true, beq_iff_eq] at h
  exact h.1.1.1.1.1.1.1.2

-- ---------------------------------------------------------------------------
-- The verdict, and what it is not
-- ---------------------------------------------------------------------------

/-- The only verdict any accepted record can reach is `observed`. There is no
arm of this relation that publishes a certificate. -/
theorem no_arm_publishes_anything_but_an_observation {r : Record} {v : CaseVerdict}
    (h : validate r = some v) : v.verdict = Verdict.observed := by
  unfold validate at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    rfl
  · simp at h

/-- A refused record publishes nothing at all: there is no partial verdict. -/
theorem a_refused_record_publishes_nothing {r : Record} (h : accepted r = false) :
    validate r = none := by
  unfold validate
  rw [ite_eq_right (by simp [h])]

/-- A validated verdict carries both axes, so neither can be recovered from the
other after the fact. -/
theorem a_verdict_carries_both_axes_separately {r : Record} {v : CaseVerdict}
    (h : validate r = some v) :
    v.derivedClass = classify r.imageBefore ∧ v.injection = r.injection := by
  unfold validate at h
  by_cases accepted : accepted r = true
  · rw [ite_eq_left accepted] at h
    cases h
    exact ⟨rfl, rfl⟩
  · rw [ite_eq_right accepted] at h
    simp at h

/-- A validated verdict's harness reclamation count equals the endpoints the
harness actually reclaimed, and never any library release. -/
theorem a_verdict_reports_only_harness_reclamations {r : Record} {v : CaseVerdict}
    (h : validate r = some v) :
    v.harnessReclaimed = r.reclaimedDescriptors + r.reclaimedMappings := by
  unfold validate at h
  split at h
  · simp only [Option.some.injEq] at h
    subst h
    rfl
  · simp at h

-- ---------------------------------------------------------------------------
-- Non-vacuity and refusals
-- ---------------------------------------------------------------------------

/-- The clean control of the recorded family, as an explicit witness. -/
def cleanControl : Record :=
  { declaredClass := .clean
    imageBefore := ⟨4096, true, 0, 4, 7, 7⟩
    imageAfter := ⟨4096, true, 0, 4, 7, 7⟩
    injection := .none
    injectedErrno := 0
    suppressedCalls := 0
    outcome := { chunk := true, openError := cioOk, chunkError := 0 }
    allocations := ⟨10, 10, 0, 0⟩
    descriptors := ⟨1, 1, 0, false⟩
    mappings := ⟨1, 1, 0, false⟩
    reclaimedDescriptors := 0
    reclaimedMappings := 0
    inodePreserved := true
    sizePreserved := true }

/-- The checksum-failure case with a suppressed close: both axes at once. -/
def checksumWithSuppressedClose : Record :=
  { cleanControl with
    declaredClass := .badChecksum
    imageBefore := ⟨4096, true, 0, 4, 7, 9⟩
    imageAfter := ⟨4096, true, 0, 4, 7, 9⟩
    injection := .close
    injectedErrno := 5
    suppressedCalls := 1
    outcome := { chunk := false, openError := cioCorrupted,
                 chunkError := ContentClass.badChecksum.errorCode }
    descriptors := ⟨1, 0, 1, true⟩
    reclaimedDescriptors := 1 }

/-- The relation is not vacuous: real shapes are accepted. -/
theorem the_relation_accepts_the_recorded_shapes :
    accepted cleanControl = true ∧ accepted checksumWithSuppressedClose = true := by
  decide

/-- The combined case keeps both axes and does not collapse into either. -/
theorem the_combined_case_keeps_both_axes :
    validate checksumWithSuppressedClose =
      some { derivedClass := .badChecksum, injection := .close, residualEndpoints := 1,
             harnessReclaimed := 1, verdict := Verdict.observed } := by
  decide

/-- Claiming a corruption that is not in the bytes is refused. -/
theorem a_claimed_corruption_that_is_not_in_the_bytes_is_refused :
    accepted { cleanControl with declaredClass := .badChecksum } = false := by
  decide

/-- The same case with the suppressed close credited to the library instead. -/
def suppressedCloseCreditedToTheLibrary : Record :=
  { checksumWithSuppressedClose with
    descriptors := ⟨1, 1, 0, false⟩
    reclaimedDescriptors := 0 }

/-- Crediting a suppressed close to the library is refused. -/
theorem a_suppressed_close_credited_to_the_library_is_refused :
    accepted suppressedCloseCreditedToTheLibrary = false := by
  decide

/-- A residual with no reclamation is refused. -/
theorem a_residual_without_a_reclamation_is_refused :
    accepted { checksumWithSuppressedClose with reclaimedDescriptors := 0 } = false := by
  decide

/-- The clean control with an unexplained descriptor residual. -/
def cleanControlWithAnUnexplainedResidual : Record :=
  { cleanControl with
    descriptors := ⟨1, 0, 1, true⟩
    reclaimedDescriptors := 1 }

/-- A residual that no suppression explains is refused. -/
theorem a_residual_no_suppression_explains_is_refused :
    accepted cleanControlWithAnUnexplainedResidual = false := by
  decide

/-- An outstanding generation is refused. -/
theorem an_outstanding_generation_is_refused :
    accepted { cleanControl with allocations := ⟨10, 9, 0, 1⟩ } = false := by
  decide

/-- A harness claim on a library allocation is refused. -/
theorem a_harness_claim_on_a_library_allocation_is_refused :
    accepted { cleanControl with allocations := ⟨10, 9, 1, 0⟩ } = false := by
  decide

/-- The combined case, reported as an ordinary injected failure. -/
def corruptionBorrowingAnInjectionReport : Record :=
  { checksumWithSuppressedClose with
    outcome := { chunk := false, openError := cioError, chunkError := 0 } }

/-- Borrowing an injected failure's report for a corrupted image is refused. -/
theorem a_corruption_may_not_borrow_an_injection_report :
    accepted corruptionBorrowingAnInjectionReport = false := by
  decide

/-- A clean image with a suppressed mapping, reported with a corruption code. -/
def injectionBorrowingACorruptionCode : Record :=
  { cleanControl with
    injection := .mmap
    injectedErrno := 5
    suppressedCalls := 1
    outcome := { chunk := false, openError := cioCorrupted,
                 chunkError := ContentClass.badChecksum.errorCode }
    mappings := ⟨0, 0, 0, false⟩ }

/-- Borrowing a corruption code for an injected failure is refused. -/
theorem an_injection_may_not_borrow_a_corruption_code :
    accepted injectionBorrowingACorruptionCode = false := by
  decide

/-- A suppressed call recorded with no errno is refused. -/
theorem a_suppressed_call_without_an_errno_is_refused :
    accepted { checksumWithSuppressedClose with injectedErrno := 0 } = false := by
  decide

/-- A fixture that lost its identity is refused. -/
theorem a_replaced_fixture_is_refused :
    accepted { cleanControl with inodePreserved := false } = false := by
  decide

end QCore.ChunkIntegrityObservation
