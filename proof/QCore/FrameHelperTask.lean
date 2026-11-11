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

import QCore.ChunkSource

namespace QCore.FrameHelperTask

/-! Card `P04-COMPOSE-FRAME`: the frame helper's verification task.

Model of `qcore_verification::frame_source`. Two things are modelled, and
nothing else:

1. **Task derivation is a function of its bound inputs, and every bound input
   is load-bearing.** The task the core derives is determined by the five
   inputs it binds; moving any one of them alone gives a different task, so a
   `PrerequisiteIdentity` naming the old one is refused. The obligation
   identifier alone is *not* an identity — the shared scheme does not read the
   bound inputs — and the triple is what separates two tasks the identifier
   cannot.
2. **The task composes with the unedited frame-preservation theorems.** A
   derivable helper's writes are allowed by `QCore.ChunkSource`'s
   `frameWritesAllowed`, so `checkedFrameWrites` succeeds and every protected
   field is preserved; the containment condition `derive` enforces carries the
   preservation from the imported protection to the evaluated one.

Deliberately outside every statement here: whether the C body Clang parsed is
the body that runs, cross-unit record ABI equality, live storage and exclusive
access. Those are the pinned contract's declared premises and the audit's own,
and the task publishes them as assumptions rather than discharging them.

The model gives a digest perfect injectivity — a task *is* its own digest.
Collision resistance of SHA-256 is not modelled and is not claimed. -/

open QCore.ChunkSource

/-! ### The bound inputs -/

/-- The five keys `derive` writes into `contract_sources`, each standing for
the digest of exactly what the Rust key binds. -/
structure BoundInputs where
  /-- `qcore:frame-helper-summary:v1` — the serialized schema record. -/
  summary : String
  /-- `qcore:frame-helper-import:v1` — the captured source/build identity. -/
  importIdentity : String
  /-- `qcore:frame-helper-contract:v1` — the pinned contract bytes. -/
  contract : String
  /-- `qcore:frame-helper-binding:v1` — flavour, record, imported protection. -/
  binding : String
  /-- `qcore:frame-helper-profile:v1` — the declared profile selection. -/
  profile : String
  deriving DecidableEq, Repr

structure UnitIdentity where
  translationUnit : String
  sourceDigest : String
  buildDigest : String
  deriving DecidableEq, Repr

/-- What a derived task is, for identity purposes. -/
structure Task where
  unit : UnitIdentity
  semanticProfile : String
  inputs : BoundInputs
  deriving DecidableEq, Repr

/-- The task digest is taken over the whole serialized task, so in this model
it is the task. -/
def taskDigest (t : Task) : Task := t

/-- `VerificationTask::obligation_id` reads the task version, the three
`UnitIdentity` fields, the semantic profile, the capability, the property and
the subject. It does **not** read `contract_sources`. -/
structure ObligationId where
  unit : UnitIdentity
  semanticProfile : String
  property : String
  subject : String
  deriving DecidableEq, Repr

def obligationId (t : Task) (property subject : String) : ObligationId :=
  { unit := t.unit, semanticProfile := t.semanticProfile,
    property := property, subject := subject }

/-- The three-field lookup key a dependent names a prerequisite by. `none`
models the empty field an incomplete claim carries. -/
structure PrerequisiteIdentity where
  semanticProfile : String
  taskDigest : Option Task
  obligationId : Option ObligationId
  deriving DecidableEq, Repr

def isComplete (claim : PrerequisiteIdentity) : Bool :=
  !claim.semanticProfile.isEmpty && claim.taskDigest.isSome && claim.obligationId.isSome

def prerequisiteIdentity (t : Task) (property subject : String) : PrerequisiteIdentity :=
  { semanticProfile := t.semanticProfile,
    taskDigest := some (taskDigest t),
    obligationId := some (obligationId t property subject) }

/-- `revalidate_as_prerequisite`: refuse an incomplete claim, refuse a claim
that is not the identity this context derives, and otherwise re-run the core's
own evaluation. `evaluates` abstracts that evaluation's outcome — the model
never assumes it succeeds. -/
def revalidate (t : Task) (property subject : String) (evaluates : Bool)
    (claim : PrerequisiteIdentity) : Bool :=
  isComplete claim && claim == prerequisiteIdentity t property subject && evaluates

/-! ### Derivation is a function, and every bound input is load-bearing -/

/- `prerequisiteIdentity` is a pure function. Its self-equality says nothing
about the Rust derivation; `derivation_is_a_function_of_its_inputs` remains a
Rust observation. The sensitivity theorems below concern this model's inputs. -/

theorem equal_digests_are_equal_tasks (left right : Task)
    (equal : taskDigest left = taskDigest right) : left = right := equal

theorem a_changed_summary_retires_the_task (t : Task) (other : String)
    (different : other ≠ t.inputs.summary) :
    { t with inputs := { t.inputs with summary := other } } ≠ t := by
  intro equal
  apply different
  have := congrArg (fun task => task.inputs.summary) equal
  exact this

theorem a_changed_import_identity_retires_the_task (t : Task) (other : String)
    (different : other ≠ t.inputs.importIdentity) :
    { t with inputs := { t.inputs with importIdentity := other } } ≠ t := by
  intro equal
  exact different (congrArg (fun task => task.inputs.importIdentity) equal)

theorem a_changed_contract_retires_the_task (t : Task) (other : String)
    (different : other ≠ t.inputs.contract) :
    { t with inputs := { t.inputs with contract := other } } ≠ t := by
  intro equal
  exact different (congrArg (fun task => task.inputs.contract) equal)

theorem a_changed_binding_retires_the_task (t : Task) (other : String)
    (different : other ≠ t.inputs.binding) :
    { t with inputs := { t.inputs with binding := other } } ≠ t := by
  intro equal
  exact different (congrArg (fun task => task.inputs.binding) equal)

theorem a_changed_profile_selection_retires_the_task (t : Task) (other : String)
    (different : other ≠ t.inputs.profile) :
    { t with inputs := { t.inputs with profile := other } } ≠ t := by
  intro equal
  exact different (congrArg (fun task => task.inputs.profile) equal)

/-- No bound input can be absorbed by another's slot: two tasks whose bound
inputs differ in any component are different tasks. -/
theorem every_bound_input_is_load_bearing (left right : Task)
    (different : left.inputs ≠ right.inputs) : left ≠ right := by
  intro equal
  exact different (congrArg (fun task => task.inputs) equal)

/-! ### The recorded boundary `P04-FRAME-OBLIGATION-ID` -/

/-- The obligation identifier is blind to the bound inputs. Proved, not
assumed: two tasks over the same body under *different pinned contracts* carry
the same identifier. -/
theorem the_obligation_identifier_does_not_read_the_bound_inputs
    (t : Task) (other : BoundInputs) (property subject : String) :
    obligationId { t with inputs := other } property subject =
      obligationId t property subject := rfl

/-- Which is sound only because the triple carries the task digest as well.
The identifier alone does not separate these two tasks; the identity does. -/
theorem the_triple_separates_what_the_identifier_cannot
    (t : Task) (other : BoundInputs) (property subject : String)
    (different : other ≠ t.inputs) :
    obligationId { t with inputs := other } property subject =
        obligationId t property subject ∧
      prerequisiteIdentity { t with inputs := other } property subject ≠
        prerequisiteIdentity t property subject := by
  refine ⟨rfl, ?_⟩
  intro equal
  apply different
  have := congrArg (fun claim => claim.taskDigest) equal
  simp [prerequisiteIdentity, taskDigest] at this
  exact congrArg (fun task => task.inputs) this

/-! ### Re-establishment is fail-closed -/

theorem an_incomplete_identity_is_refused (t : Task) (property subject : String)
    (evaluates : Bool) (claim : PrerequisiteIdentity)
    (incomplete : isComplete claim = false) :
    revalidate t property subject evaluates claim = false := by
  simp [revalidate, incomplete]

theorem a_foreign_identity_is_refused (t : Task) (property subject : String)
    (evaluates : Bool) (claim : PrerequisiteIdentity)
    (foreign : claim ≠ prerequisiteIdentity t property subject) :
    revalidate t property subject evaluates claim = false := by
  simp [revalidate, foreign]

/-- A refusing core evaluation is never established, however honest the claim.
This is the `a_refusing_evaluation_never_becomes_a_prerequisite` row: identity
makes a task nameable, it never makes it discharged. -/
theorem a_refusing_evaluation_is_never_established
    (t : Task) (property subject : String) (claim : PrerequisiteIdentity) :
    revalidate t property subject false claim = false := by
  simp [revalidate]

theorem the_derived_identity_is_established (t : Task) (property subject : String)
    (named : t.semanticProfile ≠ "") :
    revalidate t property subject true (prerequisiteIdentity t property subject) = true := by
  simp [revalidate, isComplete, prerequisiteIdentity, String.isEmpty, named]

/-- Each of the three fields is load-bearing on its own. -/
theorem every_prerequisite_field_is_load_bearing
    (t : Task) (property subject : String) (evaluates : Bool)
    (claim : PrerequisiteIdentity)
    (differs : claim.semanticProfile ≠ t.semanticProfile ∨
      claim.taskDigest ≠ some (taskDigest t) ∨
      claim.obligationId ≠ some (obligationId t property subject)) :
    revalidate t property subject evaluates claim = false := by
  apply a_foreign_identity_is_refused
  intro equal
  rcases differs with h | h | h
  · exact h (congrArg (fun c => c.semanticProfile) equal)
  · exact h (congrArg (fun c => c.taskDigest) equal)
  · exact h (congrArg (fun c => c.obligationId) equal)

/-! ### The tie to the unedited frame-preservation theorems -/

/-- The frame-specific half of what `derive` decides. -/
structure FrameHelper where
  record : String
  writes : List (String × Int)
  importedProtection : List String
  evaluatedProtection : List String
  deriving DecidableEq, Repr

/-- `derive`'s frame conditions, decided. -/
def derivable (h : FrameHelper) : Bool :=
  !h.record.isEmpty && !h.writes.isEmpty && !h.importedProtection.isEmpty &&
    h.evaluatedProtection.all (fun field => h.importedProtection.contains field)

theorem an_empty_write_set_is_not_derivable (h : FrameHelper) (empty : h.writes = []) :
    derivable h = false := by
  simp [derivable, empty]

theorem an_empty_imported_protection_is_not_derivable (h : FrameHelper)
    (empty : h.importedProtection = []) : derivable h = false := by
  simp [derivable, empty]

/-- The containment condition is not decorative: an evaluated protection
reaching outside the imported one refuses before any task exists. -/
theorem an_uncontained_evaluated_protection_is_not_derivable (h : FrameHelper)
    (field : String) (claimed : field ∈ h.evaluatedProtection)
    (outside : field ∉ h.importedProtection) :
    derivable h = false := by
  simp only [derivable, Bool.and_eq_false_iff, List.all_eq_false]
  refine Or.inr ⟨field, claimed, ?_⟩
  simpa using outside

theorem derivable_implies_containment (h : FrameHelper) (ok : derivable h = true)
    (field : String) (claimed : field ∈ h.evaluatedProtection) :
    field ∈ h.importedProtection := by
  simp only [derivable, Bool.and_eq_true, List.all_eq_true] at ok
  simpa using ok.2 field claimed

theorem a_derivable_helper_has_a_nonempty_write_set (h : FrameHelper)
    (ok : derivable h = true) : h.writes ≠ [] := by
  simp only [derivable, Bool.and_eq_true] at ok
  simpa using ok.1.1.2

/-- The composition theorem. A derivable helper whose writes are disjoint from
its imported protection — which is exactly what the gateway importer refuses to
violate — is admitted by `QCore.ChunkSource.checkedFrameWrites`, and every
imported-protected field survives unchanged. -/
theorem a_derivable_helper_preserves_every_imported_protected_field
    (before : FieldStore) (identity : Nat) (h : FrameHelper)
    (ok : derivable h = true)
    (disjoint : ∀ write ∈ h.writes, write.1 ∉ h.importedProtection) :
    ∃ after, checkedFrameWrites before identity h.importedProtection h.writes = some after ∧
      ∀ field ∈ h.importedProtection, after identity field = before identity field := by
  have allowed : frameWritesAllowed h.importedProtection h.writes :=
    ⟨a_derivable_helper_has_a_nonempty_write_set h ok, disjoint⟩
  have checked : checkedFrameWrites before identity h.importedProtection h.writes =
      some (applyFrameWrites before identity h.writes) := by
    simp [checkedFrameWrites, allowed]
  refine ⟨applyFrameWrites before identity h.writes, checked, ?_⟩
  intro field member
  exact checked_frame_writes_preserve_every_modeled_field checked field member identity

/-- Containment carries the preservation to the fields the selected evaluator
actually checks. This is why the Chunk flavour's pinned `{"lock"}` may compose
with an import that protects `{"busy", "lock"}`. -/
theorem a_derivable_helper_preserves_every_evaluated_protected_field
    (before : FieldStore) (identity : Nat) (h : FrameHelper)
    (ok : derivable h = true)
    (disjoint : ∀ write ∈ h.writes, write.1 ∉ h.importedProtection) :
    ∀ field ∈ h.evaluatedProtection,
      applyFrameWrites before identity h.writes identity field = before identity field := by
  intro field member
  have inside : field ∈ h.importedProtection := derivable_implies_containment h ok field member
  exact frame_writes_preserve_unwritten_field before identity identity field h.writes
    (fun write hw => by
      intro same
      exact disjoint write hw (same ▸ inside))

/-- The other direction: a helper that writes a field the import protects is
refused outright, so no task over it can preserve anything vacuously. -/
theorem a_write_to_an_imported_protected_field_is_refused
    (before : FieldStore) (identity : Nat) (h : FrameHelper)
    (write : String × Int) (member : write ∈ h.writes)
    (protected_field : write.1 ∈ h.importedProtection) :
    checkedFrameWrites before identity h.importedProtection h.writes = none :=
  modeled_field_write_rejects before identity h.importedProtection h.writes write member
    protected_field

/-! ### Non-vacuity: the real ChunkIO helper -/

/-- `cio_error_reset` as the audit imports it: writes `error_n = 0`, imported
protection `{busy, lock}`, and the Chunk flavour's evaluated protection
`{lock}`. -/
def cioErrorReset : FrameHelper :=
  { record := "cio_chunk", writes := [("error_n", 0)],
    importedProtection := ["busy", "lock"], evaluatedProtection := ["lock"] }

theorem the_real_chunkio_helper_is_derivable : derivable cioErrorReset = true := by
  decide

theorem the_real_chunkio_helper_preserves_the_lock_field (before : FieldStore) (identity : Nat) :
    applyFrameWrites before identity cioErrorReset.writes identity "lock" =
      before identity "lock" := by
  simp [cioErrorReset, applyFrameWrites, writeFrame]

theorem the_real_chunkio_helper_preserves_the_busy_field (before : FieldStore) (identity : Nat) :
    applyFrameWrites before identity cioErrorReset.writes identity "busy" =
      before identity "busy" := by
  simp [cioErrorReset, applyFrameWrites, writeFrame]

/-- And it really writes: the preservation above is not the statement that
nothing happens. -/
theorem the_real_chunkio_helper_writes_its_metadata_field
    (before : FieldStore) (identity : Nat) :
    applyFrameWrites before identity cioErrorReset.writes identity "error_n" = 0 := by
  simp [cioErrorReset, applyFrameWrites, writeFrame]

end QCore.FrameHelperTask
