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

import QCore.DerivedTextAdmission

namespace QCore.TwoDerivationAdmission

/-! Card `P04-COMPOSE-FS`: **two** derived units in one question, and two
composition prerequisites checked over two different translation units.

Model of `qcore_c_gateway::composition_source::import_filesystem_chunk_close_source`
and of `qcore_verification::composition_source::FilesystemChunkCloseCompositionSourceTask`'s
`derive` and `check` bodies. The single-derivation half — that
`for_derived_text` erases a declared file, that a derivation is a change, and
that a substituted recipe or substituted base text is refused — is
`QCore.DerivedTextAdmission` and is reused rather than restated.

Three things are modelled here, and nothing else:

1. **The two derivations are two.** Both derive from one base text, under two
   recipes that must differ, into two texts that must differ. A filesystem
   probe derived under the memory recipe would read the memory discriminant,
   and the two arms would collapse into one — so the separation is a checked
   conjunct, not an assumption.
2. **The cross-unit tie is exactly what it is.** The memory prerequisite must
   have been checked over this base text; the backend prerequisite deliberately
   must not, because it lives in another translation unit. What ties it is its
   identity, its revision and its prototype — three things this model carries
   and nothing more. Recorded as `P04-COMPOSE-FS-CROSS-UNIT`.
3. **Admission is fail-closed in every conjunct**, and an admitted proposal
   *is* the core's own reading.

Deliberately outside every statement here: whether either derived text
preprocesses as its base unit does (a declared premise the task publishes),
whether the linker resolves the backend prerequisite's wrapper to the body
checked in the other unit, and anything about the OS. Digests are perfectly
injective in this model — a text *is* its own digest — so collision resistance
is neither modelled nor claimed. -/

open QCore.DerivedTextAdmission

/-! ### Two derivations over one base -/

/-- The two recipes this family's question involves: its own filesystem probe
recipe and the nested memory question's memory probe recipe. -/
structure Recipes where
  filesystem : Text
  filesystemFunction : String
  memory : Text
  memoryFunction : String
  deriving DecidableEq, Repr

/-- The separation the core checks: two different recipes, defining two
different functions, deriving two different texts from one base. -/
def derivationsAreSeparate (own nested : DerivationRecord) : Bool :=
  (own.baseDigest = nested.baseDigest)
    && (own.recipe ≠ nested.recipe)
    && (own.recipeFunction ≠ nested.recipeFunction)
    && (own.derivedDigest ≠ nested.derivedDigest)

/-- Appending different nonempty recipes to one base derives different texts,
so the separation is achievable rather than vacuous. -/
theorem different_recipes_derive_different_texts
    (base r₁ r₂ : Text) (f₁ f₂ : String) (input : InputClass)
    (hr : r₁ ≠ r₂) (hf : f₁ ≠ f₂) (hne : derivedText base r₁ ≠ derivedText base r₂) :
    derivationsAreSeparate (measuredDerivation base r₁ f₁ input)
      (measuredDerivation base r₂ f₂ input) = true := by
  simp [derivationsAreSeparate, measuredDerivation, hr, hf, hne]

/-- **The collapse the rule exists to prevent.** One recipe used twice is not
two derivations: the separation refuses it. -/
theorem one_recipe_twice_is_not_two_derivations
    (base recipe : Text) (f : String) (input : InputClass) :
    derivationsAreSeparate (measuredDerivation base recipe f input)
      (measuredDerivation base recipe f input) = false := by
  simp [derivationsAreSeparate, measuredDerivation]

/-- Two derivations over different base texts are refused: the two probes must
read the same unit the wrapper came from. -/
theorem two_base_texts_are_not_one_question
    (own nested : DerivationRecord) (h : own.baseDigest ≠ nested.baseDigest) :
    derivationsAreSeparate own nested = false := by
  simp [derivationsAreSeparate, h]

/-- Two recipes deriving one text are refused even when the recipes differ:
what matters downstream is the text the bodies came from. -/
theorem two_recipes_deriving_one_text_are_refused
    (own nested : DerivationRecord) (h : own.derivedDigest = nested.derivedDigest) :
    derivationsAreSeparate own nested = false := by
  simp [derivationsAreSeparate, h]

/-- A nonempty recipe really changes the text, stated as a proposition so the
non-vacuity proof below can use it directly. -/
theorem a_nonempty_recipe_changes_the_text
    (base recipe : Text) (f : String) (input : InputClass)
    (h : isARecipe recipe = true) : base ≠ derivedText base recipe := by
  have hd : (decide (base ≠ derivedText base recipe)) = true :=
    a_nonempty_recipe_derives_new_text base recipe f input h
  exact of_decide_eq_true hd

/-! ### The two composition prerequisites -/

/-- One composition prerequisite, as the dependent's core sees it: the identity
it derives, whether its own whole re-establishment succeeds, the unit it was
checked over, the question it imported, its declared revision and the declared
prototype of the frame the dependent resolves it by. -/
structure CompositionPrerequisite where
  identity : String
  evaluates : Bool
  unitDigest : Text
  question : String
  revision : String
  prototype : String
  deriving DecidableEq, Repr

/-- The two prerequisites this family names. Both are compositions. -/
structure Prerequisites where
  memory : CompositionPrerequisite
  backend : CompositionPrerequisite
  /-- The deleting entry point is the only admissible backend arm: this family
  crosses its entry states with **both** case vectors, and only the deleting
  composition publishes both. -/
  backendIsDeleting : Bool
  deriving DecidableEq, Repr

def bothEvaluate (p : Prerequisites) : Bool :=
  p.memory.evaluates && p.backend.evaluates

def claimedIdentities (p : Prerequisites) : List String :=
  [p.memory.identity, p.backend.identity]

def claimedQuestions (p : Prerequisites) : List String :=
  [p.memory.question, p.backend.question]

/-- **The cross-unit tie**, in both directions. The memory prerequisite must be
over the very text this family's derivation derives from; the backend
prerequisite must *not* be, because it is expected in another translation unit.
What ties it instead is its identity, its declared revision and its declared
prototype. -/
def unitTie (p : Prerequisites) (base : Text) : Bool :=
  (p.memory.unitDigest = base) && (p.backend.unitDigest ≠ base)

/-- The cross-unit substitute for a text tie: the backend prerequisite's own
declared revision and prototype must be the ones this family requires. -/
def crossUnitTie (p : Prerequisites) (revision prototype : String) : Bool :=
  (p.backend.revision = revision) && (p.backend.prototype = prototype)

/-! ### The task and its admission -/

/-- The core's own pinned selection for this family. None of it travels. -/
structure Contracts where
  recipe : Text
  recipeFunction : String
  composition : String
  /-- The backend prerequisite's whole pinned contract selection, which this
  route re-runs its recheck under. -/
  backendSelection : String
  revision : String
  prototype : String
  deriving DecidableEq, Repr

/-- The whole context one admission decides in. -/
structure Context where
  base : Text
  input : InputClass
  contracts : Contracts
  prerequisites : Prerequisites
  /-- The nested memory question's own derivation, as the core's own memory
  context measured it. -/
  nestedDerivation : DerivationRecord
  /-- The reading the core's own composition computes, when it computes one. -/
  coreReading : Option Reading
  deriving DecidableEq, Repr

/-- What travels: this family's own transported derivation, the nested
question's derivation, the two claimed identities, the two transported nested
questions and the proposed reading. No derived text and no composed result, at
any level, is in this record. -/
structure Proposal where
  derivation : DerivationRecord
  nestedDerivation : DerivationRecord
  identities : List String
  questions : List String
  reading : Reading
  deriving DecidableEq, Repr

/-- The derivation the core's own importer measures for **this** family. -/
def ownDerivation (c : Context) : DerivationRecord :=
  measuredDerivation c.base c.contracts.recipe c.contracts.recipeFunction c.input

/-- This family's own derivation check, reusing the single-derivation rule. -/
def ownDerivationAdmitted (c : Context) (p : Proposal) : Bool :=
  (p.derivation = ownDerivation c) && isComplete p.derivation && isADerivation p.derivation

/-- The nested derivation check: it must be the one the core's own nested
context measured, and the two must be separate derivations over one base. -/
def nestedDerivationAdmitted (c : Context) (p : Proposal) : Bool :=
  (p.nestedDerivation = c.nestedDerivation)
    && derivationsAreSeparate p.derivation p.nestedDerivation

/-- The whole admission rule, as a decided conjunction. Every conjunct is work
the core does for itself; the proposal's reading is compared last. -/
def admits (c : Context) (p : Proposal) : Bool :=
  ownDerivationAdmitted c p
    && nestedDerivationAdmitted c p
    && (p.identities = claimedIdentities c.prerequisites)
    && (p.questions = claimedQuestions c.prerequisites)
    && bothEvaluate c.prerequisites
    && c.prerequisites.backendIsDeleting
    && unitTie c.prerequisites c.base
    && crossUnitTie c.prerequisites c.contracts.revision c.contracts.prototype
    && (match c.coreReading with
        | none => false
        | some reading => reading = p.reading)

/-! ### Fail-closed directions -/

theorem admission_is_decided (c : Context) (p : Proposal) :
    admits c p = true ∨ admits c p = false := by
  cases admits c p <;> simp

/-- A core that computes no reading admits nothing, whatever was sent. -/
theorem a_refusing_core_admits_nothing (c : Context) (p : Proposal)
    (h : c.coreReading = none) : admits c p = false := by
  simp [admits, h]

/-- A transported derivation that is not the one the core's own importer
measured is refused. -/
theorem a_foreign_own_derivation_is_refused (c : Context) (p : Proposal)
    (h : p.derivation ≠ ownDerivation c) : admits c p = false := by
  simp [admits, ownDerivationAdmitted, h]

/-- **A substituted recipe over honest text is refused**: the core's own recipe
constant decides what this family's derivation must be. -/
theorem a_substituted_recipe_is_refused (c : Context) (p : Proposal)
    (h : p.derivation.recipe ≠ c.contracts.recipe) : admits c p = false := by
  apply a_foreign_own_derivation_is_refused
  intro contra
  exact h (by rw [contra]; rfl)

/-- **An honest recipe over substituted base text is refused.** -/
theorem substituted_base_text_is_refused (c : Context) (p : Proposal)
    (h : p.derivation.baseDigest ≠ c.base) : admits c p = false := by
  apply a_foreign_own_derivation_is_refused
  intro contra
  exact h (by rw [contra]; rfl)

/-- A derivation that derives nothing is refused. -/
theorem a_derivation_that_derives_nothing_is_refused (c : Context) (p : Proposal)
    (h : isADerivation p.derivation = false) : admits c p = false := by
  simp [admits, ownDerivationAdmitted, h]

/-- An incomplete derivation record is refused. -/
theorem an_incomplete_derivation_is_refused (c : Context) (p : Proposal)
    (h : isComplete p.derivation = false) : admits c p = false := by
  simp [admits, ownDerivationAdmitted, h]

/-- A nested derivation that is not the one the core's own nested context
measured is refused. -/
theorem a_foreign_nested_derivation_is_refused (c : Context) (p : Proposal)
    (h : p.nestedDerivation ≠ c.nestedDerivation) : admits c p = false := by
  simp [admits, nestedDerivationAdmitted, h]

/-- **The collapse row.** Two derivations that are not separate — one recipe
used twice, two base texts, or two recipes deriving one text — are refused. -/
theorem unseparated_derivations_are_refused (c : Context) (p : Proposal)
    (h : derivationsAreSeparate p.derivation p.nestedDerivation = false) :
    admits c p = false := by
  simp [admits, nestedDerivationAdmitted, h]

/-- Each of the two composition prerequisites refuses on its own. -/
theorem a_refusing_memory_prerequisite_refuses (c : Context) (p : Proposal)
    (h : c.prerequisites.memory.evaluates = false) : admits c p = false := by
  simp [admits, bothEvaluate, h]

theorem a_refusing_backend_prerequisite_refuses (c : Context) (p : Proposal)
    (h : c.prerequisites.backend.evaluates = false) : admits c p = false := by
  simp [admits, bothEvaluate, h]

/-- A non-deleting backend prerequisite is refused: this family crosses both
case vectors and only the deleting composition publishes both. -/
theorem a_non_deleting_backend_prerequisite_is_refused (c : Context) (p : Proposal)
    (h : c.prerequisites.backendIsDeleting = false) : admits c p = false := by
  simp [admits, h]

/-- A foreign identity set is refused. -/
theorem a_foreign_identity_set_is_refused (c : Context) (p : Proposal)
    (h : p.identities ≠ claimedIdentities c.prerequisites) : admits c p = false := by
  simp [admits, h]

/-- A transported nested question that is not the one the corresponding
composition context imported is refused. -/
theorem a_foreign_nested_question_is_refused (c : Context) (p : Proposal)
    (h : p.questions ≠ claimedQuestions c.prerequisites) : admits c p = false := by
  simp [admits, h]

/-- The memory prerequisite must be over this base text. -/
theorem a_memory_prerequisite_over_another_unit_is_refused (c : Context) (p : Proposal)
    (h : c.prerequisites.memory.unitDigest ≠ c.base) : admits c p = false := by
  simp [admits, unitTie, h]

/-- **And the backend prerequisite must not be.** The cross-unit expectation is
checked in both directions, so a backend composition over the wrapper's own
text is refused rather than silently accepted as a stronger tie. -/
theorem a_backend_prerequisite_over_this_unit_is_refused (c : Context) (p : Proposal)
    (h : c.prerequisites.backend.unitDigest = c.base) : admits c p = false := by
  simp [admits, unitTie, h]

/-- The cross-unit substitute is load-bearing in both of its parts. -/
theorem a_backend_prerequisite_at_another_revision_is_refused (c : Context) (p : Proposal)
    (h : c.prerequisites.backend.revision ≠ c.contracts.revision) : admits c p = false := by
  simp [admits, crossUnitTie, h]

theorem a_backend_prerequisite_at_another_prototype_is_refused (c : Context) (p : Proposal)
    (h : c.prerequisites.backend.prototype ≠ c.contracts.prototype) : admits c p = false := by
  simp [admits, crossUnitTie, h]

/-- A proposal that differs from the core's own reading is refused. -/
theorem a_differing_proposal_is_refused (c : Context) (p : Proposal) (reading : Reading)
    (hc : c.coreReading = some reading) (h : reading ≠ p.reading) :
    admits c p = false := by
  simp [admits, hc, h]

/-! ### What an admission means -/

/-- An admission names the core's own derivation, for **both** derived units.

Proved by the contrapositive of the two refusal directions, so the statement is
tied to the rules that decide it rather than to a normal form. -/
theorem an_admission_names_both_of_the_cores_own_derivations (c : Context) (p : Proposal)
    (h : admits c p = true) :
    p.derivation = ownDerivation c ∧ p.nestedDerivation = c.nestedDerivation := by
  constructor
  · by_cases hd : p.derivation = ownDerivation c
    · exact hd
    · rw [a_foreign_own_derivation_is_refused c p hd] at h; cases h
  · by_cases hn : p.nestedDerivation = c.nestedDerivation
    · exact hn
    · rw [a_foreign_nested_derivation_is_refused c p hn] at h; cases h

/-- An admission carries the two-derivation separation, so no admitted question
ever read one discriminant twice. -/
theorem an_admission_separates_the_two_derivations (c : Context) (p : Proposal)
    (h : admits c p = true) :
    derivationsAreSeparate p.derivation p.nestedDerivation = true := by
  cases hs : derivationsAreSeparate p.derivation p.nestedDerivation with
  | false => rw [unseparated_derivations_are_refused c p hs] at h; cases h
  | true => rfl

/-- An admission establishes **both** compositions, each of which established
its own prerequisites in turn. -/
theorem an_admission_establishes_both_compositions (c : Context) (p : Proposal)
    (h : admits c p = true) :
    c.prerequisites.memory.evaluates = true ∧ c.prerequisites.backend.evaluates = true := by
  constructor
  · cases hm : c.prerequisites.memory.evaluates with
    | false => rw [a_refusing_memory_prerequisite_refuses c p hm] at h; cases h
    | true => rfl
  · cases hb : c.prerequisites.backend.evaluates with
    | false => rw [a_refusing_backend_prerequisite_refuses c p hb] at h; cases h
    | true => rfl

/-- An admission establishes the deleting backend arm and both halves of the
cross-unit tie. -/
theorem an_admission_establishes_the_cross_unit_tie (c : Context) (p : Proposal)
    (h : admits c p = true) :
    c.prerequisites.backendIsDeleting = true
      ∧ c.prerequisites.memory.unitDigest = c.base
      ∧ c.prerequisites.backend.unitDigest ≠ c.base
      ∧ c.prerequisites.backend.revision = c.contracts.revision
      ∧ c.prerequisites.backend.prototype = c.contracts.prototype := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · cases hd : c.prerequisites.backendIsDeleting with
    | false => rw [a_non_deleting_backend_prerequisite_is_refused c p hd] at h; cases h
    | true => rfl
  · by_cases hm : c.prerequisites.memory.unitDigest = c.base
    · exact hm
    · rw [a_memory_prerequisite_over_another_unit_is_refused c p hm] at h; cases h
  · by_cases hb : c.prerequisites.backend.unitDigest = c.base
    · rw [a_backend_prerequisite_over_this_unit_is_refused c p hb] at h; cases h
    · exact hb
  · by_cases hr : c.prerequisites.backend.revision = c.contracts.revision
    · exact hr
    · rw [a_backend_prerequisite_at_another_revision_is_refused c p hr] at h; cases h
  · by_cases hp : c.prerequisites.backend.prototype = c.contracts.prototype
    · exact hp
    · rw [a_backend_prerequisite_at_another_prototype_is_refused c p hp] at h; cases h

/-- An admission names the core's own two nested questions and identities. -/
theorem an_admission_names_the_cores_own_questions (c : Context) (p : Proposal)
    (h : admits c p = true) :
    p.identities = claimedIdentities c.prerequisites
      ∧ p.questions = claimedQuestions c.prerequisites := by
  constructor
  · by_cases hi : p.identities = claimedIdentities c.prerequisites
    · exact hi
    · rw [a_foreign_identity_set_is_refused c p hi] at h; cases h
  · by_cases hq : p.questions = claimedQuestions c.prerequisites
    · exact hq
    · rw [a_foreign_nested_question_is_refused c p hq] at h; cases h

/-- An admitted proposal **is** the core's own reading. -/
theorem an_admitted_proposal_is_the_cores_own_reading (c : Context) (p : Proposal)
    (h : admits c p = true) : c.coreReading = some p.reading := by
  cases hc : c.coreReading with
  | none => rw [a_refusing_core_admits_nothing c p hc] at h; cases h
  | some reading =>
      by_cases hr : reading = p.reading
      · exact congrArg some hr
      · rw [a_differing_proposal_is_refused c p reading hc hr] at h; cases h

/-- Two admitted proposals cannot disagree. -/
theorem two_admitted_proposals_agree (c : Context) (p q : Proposal)
    (hp : admits c p = true) (hq : admits c q = true) : p.reading = q.reading := by
  have hp' := an_admitted_proposal_is_the_cores_own_reading c p hp
  have hq' := an_admitted_proposal_is_the_cores_own_reading c q hq
  rw [hp'] at hq'
  exact Option.some.inj hq'

/-- Every conjunct of the rule, gathered, so the whole meaning of an admission
is one statement. -/
theorem admission_implies_every_conjunct (c : Context) (p : Proposal)
    (h : admits c p = true) :
    (p.derivation = ownDerivation c ∧ p.nestedDerivation = c.nestedDerivation)
      ∧ derivationsAreSeparate p.derivation p.nestedDerivation = true
      ∧ (p.identities = claimedIdentities c.prerequisites
          ∧ p.questions = claimedQuestions c.prerequisites)
      ∧ (c.prerequisites.memory.evaluates = true
          ∧ c.prerequisites.backend.evaluates = true)
      ∧ c.coreReading = some p.reading := by
  exact ⟨an_admission_names_both_of_the_cores_own_derivations c p h,
    an_admission_separates_the_two_derivations c p h,
    an_admission_names_the_cores_own_questions c p h,
    an_admission_establishes_both_compositions c p h,
    an_admitted_proposal_is_the_cores_own_reading c p h⟩

/-! ### Non-vacuity -/

/-- The honest context, over one base text and two separate recipes. -/
def honestContext (base own nested : Text) (fOwn fNested : String)
    (input : InputClass) (composition backendSelection revision prototype : String)
    (memoryId backendId memoryQ backendQ : String) (backendUnit : Text)
    (reading : Reading) : Context :=
  { base := base,
    input := input,
    contracts :=
      { recipe := own, recipeFunction := fOwn, composition := composition,
        backendSelection := backendSelection, revision := revision,
        prototype := prototype },
    prerequisites :=
      { memory :=
          { identity := memoryId, evaluates := true, unitDigest := base,
            question := memoryQ, revision := revision, prototype := prototype },
        backend :=
          { identity := backendId, evaluates := true, unitDigest := backendUnit,
            question := backendQ, revision := revision, prototype := prototype },
        backendIsDeleting := true },
    nestedDerivation := measuredDerivation base nested fNested input,
    coreReading := some reading }

def honestProposal (base own nested : Text) (fOwn fNested : String)
    (input : InputClass) (memoryId backendId memoryQ backendQ : String)
    (reading : Reading) : Proposal :=
  { derivation := measuredDerivation base own fOwn input,
    nestedDerivation := measuredDerivation base nested fNested input,
    identities := [memoryId, backendId],
    questions := [memoryQ, backendQ],
    reading := reading }

/-- **The rule is not vacuous**: the honest proposal over the honest context is
admitted, with both derivations separate and the backend prerequisite in
another unit. -/
theorem the_honest_proposal_is_admitted
    (base own nested backendUnit : Text) (fOwn fNested : String)
    (input : InputClass) (composition backendSelection revision prototype : String)
    (memoryId backendId memoryQ backendQ : String) (reading : Reading)
    (hOwn : isARecipe own = true)
    (hComplete : isComplete (measuredDerivation base own fOwn input) = true)
    (hRecipes : own ≠ nested) (hFunctions : fOwn ≠ fNested)
    (hTexts : derivedText base own ≠ derivedText base nested)
    (hUnit : backendUnit ≠ base) :
    admits
      (honestContext base own nested fOwn fNested input composition backendSelection
        revision prototype memoryId backendId memoryQ backendQ backendUnit reading)
      (honestProposal base own nested fOwn fNested input memoryId backendId memoryQ
        backendQ reading) = true := by
  simp [admits, honestContext, honestProposal, ownDerivation, ownDerivationAdmitted,
    nestedDerivationAdmitted, derivationsAreSeparate, measuredDerivation,
    claimedIdentities, claimedQuestions, bothEvaluate, unitTie, crossUnitTie,
    hRecipes, hFunctions, hTexts, hUnit]
  exact ⟨hComplete, a_nonempty_recipe_derives_new_text base own fOwn input hOwn⟩

/-- And each refusal really refuses on that same honest context, so the
fail-closed theorems are not about unreachable states. -/
theorem the_refusals_are_not_vacuous
    (base own nested backendUnit : Text) (fOwn fNested : String)
    (input : InputClass) (composition backendSelection revision prototype : String)
    (memoryId backendId memoryQ backendQ : String) (reading : Reading) :
    admits
      { honestContext base own nested fOwn fNested input composition backendSelection
          revision prototype memoryId backendId memoryQ backendQ backendUnit reading with
        coreReading := none }
      (honestProposal base own nested fOwn fNested input memoryId backendId memoryQ
        backendQ reading) = false := by
  apply a_refusing_core_admits_nothing
  rfl

/-- The cross-unit expectation is genuinely two-sided on the honest context: a
backend prerequisite moved onto the wrapper's own unit is refused. -/
theorem the_cross_unit_expectation_is_two_sided
    (base own nested : Text) (fOwn fNested : String)
    (input : InputClass) (composition backendSelection revision prototype : String)
    (memoryId backendId memoryQ backendQ : String) (reading : Reading) :
    admits
      (honestContext base own nested fOwn fNested input composition backendSelection
        revision prototype memoryId backendId memoryQ backendQ base reading)
      (honestProposal base own nested fOwn fNested input memoryId backendId memoryQ
        backendQ reading) = false := by
  apply a_backend_prerequisite_over_this_unit_is_refused
  rfl

end QCore.TwoDerivationAdmission
