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

namespace QCore.DerivedTextAdmission

/-! Card `P04-COMPOSE-MEMCLOSE`: importing from **derived** text, and the
fail-closed directions of the derived-identity binding.

Model of `qcore_c_gateway::legacy_c`'s `derived_source_text` /
`derived_translation_unit` / `CSourceInput::for_derived_text`, of
`qcore_c_gateway::composition_source::import_memory_close_source`, and of
`qcore_verification::composition_source::MemoryCloseCompositionSourceTask`'s
`derive` and `check` bodies.

Three things are modelled, and nothing else:

1. **`for_derived_text` erases the declared file, and the erasure is real.**
   Two base units with equal bytes and different declared identities derive
   *byte-identical* derived identities. A route that bound only the derived
   identity would therefore treat them as one task; binding the base import
   separately is what separates them.
2. **The derivation is the core's own.** The transported record is compared
   against the one the core's own importer measured under the core's own
   recipe. A substituted recipe, a substituted base text and a "derivation"
   that derives nothing are each refused, and the two substitution directions
   are refused at different places.
3. **Admission is fail-closed in every conjunct.** Every check the core does
   for itself refuses on its own, before any transported figure is compared,
   and an admitted proposal *is* the core's own reading.

Deliberately outside every statement here: whether the derived text
preprocesses as the base unit does (a declared premise the task publishes),
whether the C body Clang parsed is the body that runs, and anything about the
OS. The model gives digests perfect injectivity — a text *is* its own digest —
so SHA-256 collision resistance is not modelled and is not claimed. -/

/-! ### Text, recipes and the derivation -/

/-- Text is modelled as a list of tokens; a digest is the text itself, which
gives the model perfect injectivity. -/
abbrev Text := List String

/-- The one derivation this repository performs: append a recipe to a base. -/
def derivedText (base recipe : Text) : Text := base ++ recipe

/-- A recipe that appends nothing derives nothing. -/
def isARecipe (recipe : Text) : Bool := !recipe.isEmpty

/-- How a unit's primary input is declared. -/
inductive InputClass where
  | anonymousText
  | declaredFile (spelled directory : String)
  | historicalStdin
  deriving DecidableEq, Repr

/-- `CSourceInput::for_derived_text`: derived text is not the declared file, so
a declared identity is dropped rather than reused for different bytes. -/
def forDerivedText : InputClass → InputClass
  | .historicalStdin => .historicalStdin
  | .anonymousText => .anonymousText
  | .declaredFile _ _ => .anonymousText

/-- The class name the wire record carries. -/
def className : InputClass → String
  | .anonymousText => "anonymous_text"
  | .declaredFile _ _ => "declared_file"
  | .historicalStdin => "historical_stdin"

/-- What the importer captures for one unit: the text it analysed and the input
identity it was given. The declared input is part of the captured
configuration exactly when the unit declares a file. -/
structure Identity where
  text : Text
  input : InputClass
  deriving DecidableEq, Repr

def identityOf (text : Text) (input : InputClass) : Identity :=
  { text := text, input := input }

/-- The identity of the unit the bodies were actually imported from. -/
def derivedIdentity (base recipe : Text) (input : InputClass) : Identity :=
  identityOf (derivedText base recipe) (forDerivedText input)

/-! ### The measured erasure -/

/-- **The crux.** Two base units with equal bytes and different declared
identities derive byte-identical derived identities: `for_derived_text` records
neither the spelled input nor the directory. -/
theorem two_declared_files_with_equal_bytes_derive_equal_derived_identities
    (base recipe : Text) (s₁ d₁ s₂ d₂ : String) :
    derivedIdentity base recipe (.declaredFile s₁ d₁)
      = derivedIdentity base recipe (.declaredFile s₂ d₂) := by
  rfl

/-- The same erasure against an anonymous base: a declared file and an
anonymous text over the same bytes are one derived identity. -/
theorem a_declared_file_and_an_anonymous_text_derive_one_identity
    (base recipe : Text) (s d : String) :
    derivedIdentity base recipe (.declaredFile s d)
      = derivedIdentity base recipe .anonymousText := by
  rfl

/-- The base identities of those same two units are *not* equal, which is what
makes the erasure a loss rather than a normalisation. -/
theorem the_base_identities_are_not_equal
    (base : Text) (s d : String) :
    identityOf base (.declaredFile s d) ≠ identityOf base .anonymousText := by
  intro contra
  have : InputClass.declaredFile s d = InputClass.anonymousText :=
    congrArg Identity.input contra
  cases this

/-- The historical standard-input projection is the one class `for_derived_text`
does not collapse, so it is recorded rather than silently folded in. -/
theorem the_historical_projection_is_not_collapsed :
    forDerivedText .historicalStdin = .historicalStdin := by
  rfl

/-- A derived identity is a function of the base text and the recipe alone,
once the base is anonymous. -/
theorem the_derived_identity_reads_only_the_text_and_the_recipe
    (base recipe : Text) (input : InputClass) :
    (derivedIdentity base recipe input).text = base ++ recipe := by
  rfl

/-! ### The transported derivation record -/

/-- `DerivedUnitRecord`: every field is a *claim* the core checks against its
own derivation. -/
structure DerivationRecord where
  recipe : Text
  recipeFunction : String
  baseDigest : Text
  derivedDigest : Text
  baseInputClass : String
  derivedInputClass : String
  deriving DecidableEq, Repr

def isComplete (r : DerivationRecord) : Bool :=
  !r.recipe.isEmpty && !r.recipeFunction.isEmpty && !r.baseInputClass.isEmpty
    && !r.derivedInputClass.isEmpty

/-- A derivation is a *change*: derived text equal to base text would mean the
recipe did nothing. -/
def isADerivation (r : DerivationRecord) : Bool := r.baseDigest ≠ r.derivedDigest

/-- The record the core's own importer measures. -/
def measuredDerivation (base recipe : Text) (recipeFunction : String)
    (input : InputClass) : DerivationRecord :=
  { recipe := recipe,
    recipeFunction := recipeFunction,
    baseDigest := base,
    derivedDigest := derivedText base recipe,
    baseInputClass := className input,
    derivedInputClass := className (forDerivedText input) }

/-- A nonempty recipe always yields a real derivation. -/
theorem a_nonempty_recipe_derives_new_text
    (base recipe : Text) (f : String) (input : InputClass)
    (h : isARecipe recipe = true) :
    isADerivation (measuredDerivation base recipe f input) = true := by
  simp [isARecipe, isADerivation, measuredDerivation, derivedText] at *
  intro contra
  exact h (by simpa using contra.symm)

/-- An empty recipe derives nothing, and the importer refuses it. -/
theorem an_empty_recipe_is_not_a_derivation
    (base : Text) (f : String) (input : InputClass) :
    isADerivation (measuredDerivation base [] f input) = false := by
  simp [isADerivation, measuredDerivation, derivedText]

/-! ### The task and its admission -/

/-- The core's own pinned selection. None of it travels. -/
structure Contracts where
  recipe : Text
  recipeFunction : String
  composition : String
  frame : String
  backend : String
  unlink : String
  counter : String
  deriving DecidableEq, Repr

/-- One prerequisite context, as the dependent's core sees it: the identity it
derives, whether its own evaluation succeeds, and the unit it was checked
over. -/
structure Prerequisite where
  identity : String
  evaluates : Bool
  unitDigest : Text
  deriving DecidableEq, Repr

/-- The four prerequisites this composition names. -/
structure Prerequisites where
  reset : Prerequisite
  backend : Prerequisite
  unlink : Prerequisite
  counter : Prerequisite
  deriving DecidableEq, Repr

def allEvaluate (p : Prerequisites) : Bool :=
  p.reset.evaluates && p.backend.evaluates && p.unlink.evaluates && p.counter.evaluates

def claimedIdentities (p : Prerequisites) : List String :=
  [p.reset.identity, p.backend.identity, p.unlink.identity, p.counter.identity]

/-- Every prerequisite must have been checked over the very unit the derivation
derives from. -/
def baseTie (p : Prerequisites) (base : Text) : Bool :=
  p.reset.unitDigest = base && p.backend.unitDigest = base
    && p.unlink.unitDigest = base && p.counter.unitDigest = base

/-- What a producer sends, and what the core computes for itself. Readings are
modelled abstractly: what matters is whether the two agree. -/
structure Reading where
  value : String
  deriving DecidableEq, Repr

/-- The whole context one admission decides in. -/
structure Context where
  base : Text
  input : InputClass
  contracts : Contracts
  prerequisites : Prerequisites
  /-- The reading the core's own composition computes, when it computes one. -/
  coreReading : Option Reading
  deriving DecidableEq, Repr

/-- What travels: the transported derivation, the claimed identities and the
proposed reading. No derived text and no composed result is in this record. -/
structure Proposal where
  derivation : DerivationRecord
  identities : List String
  reading : Reading
  deriving DecidableEq, Repr

/-- The derivation the core's own importer measures in this context. -/
def ownDerivation (c : Context) : DerivationRecord :=
  measuredDerivation c.base c.contracts.recipe c.contracts.recipeFunction c.input

/-- The core's derivation check: the transported record must be the one the
core's own importer measured, complete, and a real derivation. -/
def derivationAdmitted (c : Context) (p : Proposal) : Bool :=
  (p.derivation = ownDerivation c) && isComplete p.derivation && isADerivation p.derivation

/-- The whole admission rule, as a decided conjunction. Every conjunct is work
the core does for itself; the proposal's reading is compared last. -/
def admits (c : Context) (p : Proposal) : Bool :=
  derivationAdmitted c p
    && (p.identities = claimedIdentities c.prerequisites)
    && allEvaluate c.prerequisites
    && baseTie c.prerequisites c.base
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
theorem a_foreign_derivation_is_refused (c : Context) (p : Proposal)
    (h : p.derivation ≠ ownDerivation c) : admits c p = false := by
  simp [admits, derivationAdmitted, h]

/-- **A substituted recipe over honest text is refused**: the core's own recipe
constant decides what the derivation must be. -/
theorem a_substituted_recipe_is_refused (c : Context) (p : Proposal)
    (h : p.derivation.recipe ≠ c.contracts.recipe) : admits c p = false := by
  apply a_foreign_derivation_is_refused
  intro contra
  exact h (by rw [contra]; rfl)

/-- **An honest recipe over substituted base text is refused**: the derivation
names the base text, so a projection over other bytes is not this one. -/
theorem substituted_base_text_is_refused (c : Context) (p : Proposal)
    (h : p.derivation.baseDigest ≠ c.base) : admits c p = false := by
  apply a_foreign_derivation_is_refused
  intro contra
  exact h (by rw [contra]; rfl)

/-- A derivation that derives nothing is refused, so a plain import can never
be presented as a derived one. -/
theorem a_derivation_that_derives_nothing_is_refused (c : Context) (p : Proposal)
    (h : isADerivation p.derivation = false) : admits c p = false := by
  simp [admits, derivationAdmitted, h]

/-- An incomplete derivation record is refused. -/
theorem an_incomplete_derivation_is_refused (c : Context) (p : Proposal)
    (h : isComplete p.derivation = false) : admits c p = false := by
  simp [admits, derivationAdmitted, h]

/-- Each of the four prerequisites refuses on its own. -/
theorem a_refusing_reset_prerequisite_refuses (c : Context) (p : Proposal)
    (h : c.prerequisites.reset.evaluates = false) : admits c p = false := by
  simp [admits, allEvaluate, h]

theorem a_refusing_backend_prerequisite_refuses (c : Context) (p : Proposal)
    (h : c.prerequisites.backend.evaluates = false) : admits c p = false := by
  simp [admits, allEvaluate, h]

theorem a_refusing_unlink_prerequisite_refuses (c : Context) (p : Proposal)
    (h : c.prerequisites.unlink.evaluates = false) : admits c p = false := by
  simp [admits, allEvaluate, h]

theorem a_refusing_counter_prerequisite_refuses (c : Context) (p : Proposal)
    (h : c.prerequisites.counter.evaluates = false) : admits c p = false := by
  simp [admits, allEvaluate, h]

/-- A claimed identity set that is not the one the core derives is refused. -/
theorem a_foreign_identity_set_is_refused (c : Context) (p : Proposal)
    (h : p.identities ≠ claimedIdentities c.prerequisites) : admits c p = false := by
  simp [admits, h]

/-- **The base tie.** A prerequisite checked over another unit than the one this
derivation derives from is refused, even when every identity is honest and the
derivation is the core's own. -/
theorem a_prerequisite_over_another_unit_is_refused (c : Context) (p : Proposal)
    (h : baseTie c.prerequisites c.base = false) : admits c p = false := by
  simp [admits, h]

/-- A proposal that differs from the core's own reading is refused. Agreement
alone discharges nothing: the core has already recomputed everything above. -/
theorem a_differing_proposal_is_refused (c : Context) (p : Proposal) (r : Reading)
    (h : c.coreReading = some r) (hne : r ≠ p.reading) : admits c p = false := by
  simp [admits, h]
  intro _ _ _ _
  exact hne

/-! ### What an admission means -/

/-- Admission implies every conjunct. -/
theorem admission_implies_every_conjunct (c : Context) (p : Proposal)
    (h : admits c p = true) :
    derivationAdmitted c p = true
      ∧ p.identities = claimedIdentities c.prerequisites
      ∧ allEvaluate c.prerequisites = true
      ∧ baseTie c.prerequisites c.base = true
      ∧ c.coreReading = some p.reading := by
  simp [admits] at h
  obtain ⟨⟨⟨⟨hd, hi⟩, ha⟩, hb⟩, hr⟩ := h
  refine ⟨hd, hi, ha, hb, ?_⟩
  cases hc : c.coreReading with
  | none => rw [hc] at hr; simp at hr
  | some reading => rw [hc] at hr; simp at hr; rw [hr]

/-- An admitted proposal **is** the core's own reading. -/
theorem an_admitted_proposal_is_the_cores_own_reading (c : Context) (p : Proposal)
    (h : admits c p = true) : c.coreReading = some p.reading :=
  (admission_implies_every_conjunct c p h).2.2.2.2

/-- Two admitted proposals cannot disagree. -/
theorem two_admitted_proposals_agree (c : Context) (p q : Proposal)
    (hp : admits c p = true) (hq : admits c q = true) : p.reading = q.reading := by
  have hp' := an_admitted_proposal_is_the_cores_own_reading c p hp
  have hq' := an_admitted_proposal_is_the_cores_own_reading c q hq
  rw [hp'] at hq'
  exact Option.some.inj hq'

/-- An admitted proposal establishes all four prerequisites. -/
theorem an_admission_establishes_all_four_prerequisites (c : Context) (p : Proposal)
    (h : admits c p = true) :
    c.prerequisites.reset.evaluates = true ∧ c.prerequisites.backend.evaluates = true
      ∧ c.prerequisites.unlink.evaluates = true ∧ c.prerequisites.counter.evaluates = true := by
  have := (admission_implies_every_conjunct c p h).2.2.1
  simp [allEvaluate] at this
  exact ⟨this.1.1.1, this.1.1.2, this.1.2, this.2⟩

/-- An admitted proposal's derivation is the core's own, so the derived text it
describes is the text the core derived. -/
theorem an_admission_names_the_cores_own_derivation (c : Context) (p : Proposal)
    (h : admits c p = true) :
    p.derivation.derivedDigest = derivedText c.base c.contracts.recipe := by
  have hd := (admission_implies_every_conjunct c p h).1
  simp [derivationAdmitted] at hd
  rw [hd.1.1]
  rfl

/-! ### The two substitution directions are separated -/

/-- Substituting the base text moves the derived digest, so the two contexts
are genuinely different questions. -/
theorem substituted_base_text_moves_the_derived_digest
    (base other recipe : Text) (f : String) (input : InputClass)
    (h : base ≠ other) :
    (measuredDerivation base recipe f input).derivedDigest
      ≠ (measuredDerivation other recipe f input).derivedDigest := by
  simp [measuredDerivation, derivedText]
  intro contra
  exact h contra

/-- Substituting the recipe moves the derived digest too, so the two directions
are separated by measurement rather than by convention. -/
theorem a_substituted_recipe_moves_the_derived_digest
    (base recipe other : Text) (f : String) (input : InputClass)
    (h : recipe ≠ other) :
    (measuredDerivation base recipe f input).derivedDigest
      ≠ (measuredDerivation base other f input).derivedDigest := by
  simp [measuredDerivation, derivedText]
  intro contra
  exact h contra

/-- And the recipe field itself separates them even when a hostile pair of
texts would collide, which is why the record carries the recipe as well as the
two digests. -/
theorem the_recipe_field_separates_two_recipes
    (base recipe other : Text) (f : String) (input : InputClass)
    (h : recipe ≠ other) :
    measuredDerivation base recipe f input ≠ measuredDerivation base other f input := by
  intro contra
  exact h (congrArg DerivationRecord.recipe contra)

/-! ### Non-vacuity -/

/-- A context in which the core composed, every prerequisite evaluates over the
right unit, and the proposal is the core's own reading. -/
def honestContext : Context :=
  { base := ["cio_chunk_close"],
    input := .declaredFile "cio_chunk.c" "/build",
    contracts :=
      { recipe := ["probe"], recipeFunction := "probe", composition := "memory_close",
        frame := "error_state", backend := "memory_backend", unlink := "list_unlink",
        counter := "counters" },
    prerequisites :=
      { reset := { identity := "reset", evaluates := true, unitDigest := ["cio_chunk_close"] },
        backend := { identity := "backend", evaluates := true, unitDigest := ["cio_chunk_close"] },
        unlink := { identity := "unlink", evaluates := true, unitDigest := ["cio_chunk_close"] },
        counter := { identity := "counter", evaluates := true, unitDigest := ["cio_chunk_close"] } },
    coreReading := some { value := "two entry states" } }

def honestProposal : Proposal :=
  { derivation := ownDerivation honestContext,
    identities := claimedIdentities honestContext.prerequisites,
    reading := { value := "two entry states" } }

/-- The rule really admits something: it is not vacuously refusing. -/
theorem the_honest_proposal_is_admitted : admits honestContext honestProposal = true := by
  decide

/-- And the honest context's derivation records the erasure: a declared base
unit, an anonymous derived one. -/
theorem the_honest_derivation_records_the_erasure :
    (ownDerivation honestContext).baseInputClass = "declared_file"
      ∧ (ownDerivation honestContext).derivedInputClass = "anonymous_text" := by
  decide

/-- Every refusal above really refuses on the honest context, so none of them is
vacuous: substituting the recipe, the base text or one prerequisite each
refuses a proposal that is otherwise the core's own. -/
theorem the_refusals_are_not_vacuous :
    admits honestContext { honestProposal with
      derivation := { honestProposal.derivation with recipe := ["other"] } } = false
    ∧ admits honestContext { honestProposal with
        derivation := { honestProposal.derivation with baseDigest := ["other"] } } = false
    ∧ admits { honestContext with
        prerequisites := { honestContext.prerequisites with
          counter := { honestContext.prerequisites.counter with evaluates := false } } }
        honestProposal = false
    ∧ admits { honestContext with
        prerequisites := { honestContext.prerequisites with
          unlink := { honestContext.prerequisites.unlink with
            unitDigest := ["another unit"] } } }
        honestProposal = false := by
  decide

end QCore.DerivedTextAdmission
