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
# Conditional transport of a W4 summary across translation units

Card **W44-CALLEE-CROSS-UNIT-SUMMARY**. A source corpus alone does not
establish a linked program. The call-to-home relation below is an explicit
premise, and the source and environment identities are equalities checked by
both replays. The caller receives a summary only if the home derivation is
valid, the selected home is unique, and no declaration takes precedence.
-/

namespace QCore.CrossUnitSummary

abbrev Name := String
abbrev Unit := String

structure Home (σ : Type) where
  function : Name
  unit : Unit
  sourceDigest : String
  environmentDigest : String
  summary : σ

/-- A digest names the caller's explicit linked-program resolution premise. -/
structure LinkPremise where
  digest : String
  /-- Every call of this name resolves to that home with a compatible
  function type and calling convention in the actual linked program. -/
  resolves : Name → Unit → Prop

def uniqueHome (definitions : List (Unit × Name)) (h : Home σ) : Prop :=
  definitions.filter (fun row => row.2 == h.function) = [(h.unit, h.function)]

def admitted (definitions : List (Unit × Name)) (link : LinkPremise)
    (declared : Name → Option σ) (home : Home σ)
    (replayedSource replayedEnvironment : String) (valid : Home σ → Prop) : Prop :=
  uniqueHome definitions home ∧
  link.resolves home.function home.unit ∧
  declared home.function = none ∧
  home.sourceDigest = replayedSource ∧
  home.environmentDigest = replayedEnvironment ∧
  valid home

/-- Imported summaries occupy a separate channel; declarations win. -/
def lookup (declared : Name → Option σ) (imports : List (Home σ))
    (name : Name) : Option σ :=
  match declared name with
  | some contract => some contract
  | none => (imports.find? (fun home => home.function == name)).map (·.summary)

theorem declaration_wins {σ : Type} (declared : Name → Option σ)
    (imports : List (Home σ)) (name : Name) (contract : σ)
    (h : declared name = some contract) :
    lookup declared imports name = some contract := by
  simp [lookup, h]

theorem admitted_import_is_valid {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (admission : admitted definitions link declared home source environment valid) :
    valid home ∧ link.resolves home.function home.unit ∧
      uniqueHome definitions home := by
  exact ⟨admission.2.2.2.2.2, admission.2.1, admission.1⟩

theorem missing_resolution_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (missing : ¬ link.resolves home.function home.unit) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact missing admission.2.1

theorem ambiguous_home_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (ambiguous : ¬ uniqueHome definitions home) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact ambiguous admission.1

theorem changed_source_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (changed : home.sourceDigest ≠ source) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact changed admission.2.2.2.1

theorem changed_environment_refuses {σ : Type} (definitions : List (Unit × Name))
    (link : LinkPremise) (declared : Name → Option σ) (home : Home σ)
    (source environment : String) (valid : Home σ → Prop)
    (changed : home.environmentDigest ≠ environment) :
    ¬ admitted definitions link declared home source environment valid := by
  intro admission
  exact changed admission.2.2.2.2.1

/-- ABI, checked-specification, and recursive W4 passes may rebuild the
declared table. They must carry the separate imports and their independent
caller-expected identities and link premise into that rebuilt environment. -/
structure CallerEnvironment (σ : Type) where
  declared : Name → Option σ
  imports : List (Home σ)
  expectedHomeDigest : Name → Option String
  linkDigest : String

def rebuildDeclared (environment : CallerEnvironment σ)
    (kept : Name → Option σ) : CallerEnvironment σ :=
  { declared := kept
    imports := environment.imports
    expectedHomeDigest := environment.expectedHomeDigest
    linkDigest := environment.linkDigest }

theorem rebuilt_environment_retains_import_evidence
    (environment : CallerEnvironment σ) (kept : Name → Option σ) :
    (rebuildDeclared environment kept).imports = environment.imports ∧
    (rebuildDeclared environment kept).expectedHomeDigest =
      environment.expectedHomeDigest ∧
    (rebuildDeclared environment kept).linkDigest = environment.linkDigest := by
  exact ⟨rfl, rfl, rfl⟩

/-! ## Card LINK-STANDING (rule 40): a derived resolution

A link-resolution artifact read from the linked executable replaces the
stated premise above by facts checked for each (caller, home) pair. The
conditions are listed in decision order. The first condition that fails is
the refusal (`standing`). A pair resolves exactly when none fails, and then it
discharges the same `admitted` obligation that a stated map discharged. -/

/-- The facts rule 40 reads for one caller and one home, in decision order
(`Facts` in `cross_unit_links.rs`). -/
structure DerivedFacts where
  /-- The artifact's compile database is the route's. -/
  databaseMatches : Bool
  /-- Every file the artifact binds by digest still has that digest. -/
  digestsVerify : Bool
  /-- The caller's configuration is linked into the executable, reproduced. -/
  callerLinked : Bool
  /-- The image is an executable, not a shared object. -/
  executable : Bool
  /-- The definition is in the executable, not a shared object. -/
  inExecutable : Bool
  /-- A strong global function definition (not weak, common, unique or an
  indirect function). -/
  strongGlobal : Bool
  /-- Default visibility. -/
  defaultVisibility : Bool
  /-- The only definition among the linked objects. -/
  onlyDefinition : Bool
  /-- No dynamic relocation in the executable names it. -/
  noDynamicRelocation : Bool
  /-- The artifact refused none of the link's own facts. -/
  factsAgree : Bool
  /-- The home's configuration is linked, reproduced. -/
  homeLinked : Bool
  /-- That configuration is one the route analyses. -/
  routeUnit : Bool
  /-- The route's `library/file` key names exactly that configuration. -/
  uniqueKey : Bool
  /-- Both units give Clang's type of the callee. -/
  typeAvailable : Bool
  /-- The caller's declaration in force has the home definition's type. -/
  typeEqual : Bool
  deriving DecidableEq, Repr

/-- Rule 40's refusals, one per condition, in the same order. -/
inductive Refusal where
  | databaseDiffers
  | digestUnverified
  | callerUnlinked
  | notExecutable
  | notInExecutable
  | notStrong
  | notDefaultVisibility
  | duplicate
  | dynamicRelocation
  | factsRefused
  | homeUnlinked
  | notRouteUnit
  | ambiguousKey
  | typeUnavailable
  | typeDiffers
  deriving DecidableEq, Repr

def DerivedFacts.conditions (f : DerivedFacts) : List (Bool × Refusal) :=
  [(f.databaseMatches, .databaseDiffers), (f.digestsVerify, .digestUnverified),
   (f.callerLinked, .callerUnlinked), (f.executable, .notExecutable),
   (f.inExecutable, .notInExecutable), (f.strongGlobal, .notStrong),
   (f.defaultVisibility, .notDefaultVisibility), (f.onlyDefinition, .duplicate),
   (f.noDynamicRelocation, .dynamicRelocation), (f.factsAgree, .factsRefused),
   (f.homeLinked, .homeUnlinked), (f.routeUnit, .notRouteUnit),
   (f.uniqueKey, .ambiguousKey), (f.typeAvailable, .typeUnavailable),
   (f.typeEqual, .typeDiffers)]

def firstFailure : List (Bool × Refusal) → Option Refusal
  | [] => none
  | (true, _) :: rest => firstFailure rest
  | (false, refusal) :: _ => some refusal

/-- The first failing condition's refusal, or `none` when the derived
resolution resolves the call to the home. -/
def standing (f : DerivedFacts) : Option Refusal := firstFailure f.conditions

theorem firstFailure_none_iff (conditions : List (Bool × Refusal)) :
    firstFailure conditions = none ↔ ∀ c ∈ conditions, c.1 = true := by
  induction conditions with
  | nil => simp [firstFailure]
  | cons head rest ih =>
    obtain ⟨holds, refusal⟩ := head
    cases holds <;> simp [firstFailure, ih]

/-- Rule 40 resolves a pair exactly when every condition holds. -/
theorem standing_none_iff (f : DerivedFacts) :
    standing f = none ↔ ∀ c ∈ f.conditions, c.1 = true :=
  firstFailure_none_iff f.conditions

/-- Any failed condition refuses. -/
theorem failed_condition_refuses (f : DerivedFacts) (c : Bool × Refusal)
    (listed : c ∈ f.conditions) (failed : c.1 = false) : standing f ≠ none := by
  intro resolved
  have := (standing_none_iff f).1 resolved c listed
  rw [failed] at this
  exact Bool.false_ne_true this

theorem database_mismatch_refuses (f : DerivedFacts) (h : f.databaseMatches = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.databaseMatches, .databaseDiffers)
    (by simp [DerivedFacts.conditions]) h

theorem unverified_digest_refuses (f : DerivedFacts) (h : f.digestsVerify = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.digestsVerify, .digestUnverified)
    (by simp [DerivedFacts.conditions]) h

theorem unlinked_caller_refuses (f : DerivedFacts) (h : f.callerLinked = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.callerLinked, .callerUnlinked)
    (by simp [DerivedFacts.conditions]) h

theorem shared_object_refuses (f : DerivedFacts) (h : f.inExecutable = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.inExecutable, .notInExecutable)
    (by simp [DerivedFacts.conditions]) h

theorem weak_definition_refuses (f : DerivedFacts) (h : f.strongGlobal = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.strongGlobal, .notStrong)
    (by simp [DerivedFacts.conditions]) h

theorem duplicate_definition_refuses (f : DerivedFacts) (h : f.onlyDefinition = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.onlyDefinition, .duplicate)
    (by simp [DerivedFacts.conditions]) h

theorem interposable_refuses (f : DerivedFacts) (h : f.noDynamicRelocation = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.noDynamicRelocation, .dynamicRelocation)
    (by simp [DerivedFacts.conditions]) h

theorem unlinked_home_configuration_refuses (f : DerivedFacts) (h : f.homeLinked = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.homeLinked, .homeUnlinked)
    (by simp [DerivedFacts.conditions]) h

/-- A home whose analysed configuration is not the linked one is refused by
the general condition, whichever library it belongs to. -/
theorem unanalysed_linked_configuration_refuses (f : DerivedFacts)
    (h : f.routeUnit = false) : standing f ≠ none :=
  failed_condition_refuses f (f.routeUnit, .notRouteUnit)
    (by simp [DerivedFacts.conditions]) h

theorem ambiguous_key_refuses (f : DerivedFacts) (h : f.uniqueKey = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.uniqueKey, .ambiguousKey)
    (by simp [DerivedFacts.conditions]) h

theorem unavailable_type_refuses (f : DerivedFacts) (h : f.typeAvailable = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.typeAvailable, .typeUnavailable)
    (by simp [DerivedFacts.conditions]) h

theorem unequal_type_refuses (f : DerivedFacts) (h : f.typeEqual = false) :
    standing f ≠ none :=
  failed_condition_refuses f (f.typeEqual, .typeDiffers)
    (by simp [DerivedFacts.conditions]) h

/-- The type condition (`link_call_type::agree`): both units give a type and
the two are equal. A missing type on either side fails it. -/
def typeAgrees {τ : Type} [DecidableEq τ] (caller home : Option τ) : Bool :=
  match caller, home with
  | some left, some right => decide (left = right)
  | _, _ => false

theorem typeAgrees_iff {τ : Type} [DecidableEq τ] (caller home : Option τ) :
    typeAgrees caller home = true ↔ ∃ t, caller = some t ∧ home = some t := by
  cases caller <;> cases home <;> simp [typeAgrees, eq_comm]

theorem missing_caller_type_disagrees {τ : Type} [DecidableEq τ] (home : Option τ) :
    typeAgrees (none : Option τ) home = false := by
  cases home <;> rfl

/-- One caller's derived premise: a name resolves to a unit exactly when
rule 40's `standing` is `none` for the pair. Its digest is the artifact's. -/
def derivedPremise (artifactDigest : String) (facts : Name → Unit → DerivedFacts) :
    LinkPremise :=
  { digest := artifactDigest
    resolves := fun name unit => standing (facts name unit) = none }

/-- A derived resolution, together with rule 4c's unchanged per-home checks,
discharges the obligation that the stated map discharged. -/
theorem derived_resolution_discharges {σ : Type} (definitions : List (Unit × Name))
    (artifactDigest : String) (facts : Name → Unit → DerivedFacts)
    (declared : Name → Option σ) (home : Home σ) (source environment : String)
    (valid : Home σ → Prop)
    (resolved : standing (facts home.function home.unit) = none)
    (unique : uniqueHome definitions home) (undeclared : declared home.function = none)
    (sameSource : home.sourceDigest = source)
    (sameEnvironment : home.environmentDigest = environment) (validHome : valid home) :
    admitted definitions (derivedPremise artifactDigest facts) declared home source
      environment valid :=
  ⟨unique, resolved, undeclared, sameSource, sameEnvironment, validHome⟩

/-- An admitted derived import carries exactly what `admitted_import_is_valid`
gives a stated one. -/
theorem derived_admission_is_valid {σ : Type} (definitions : List (Unit × Name))
    (artifactDigest : String) (facts : Name → Unit → DerivedFacts)
    (declared : Name → Option σ) (home : Home σ) (source environment : String)
    (valid : Home σ → Prop)
    (admission : admitted definitions (derivedPremise artifactDigest facts) declared home
      source environment valid) :
    valid home ∧ standing (facts home.function home.unit) = none ∧
      uniqueHome definitions home :=
  admitted_import_is_valid definitions _ declared home source environment valid admission

/-- Every refusal is a missing resolution: the import is not admitted. -/
theorem derived_refusal_is_not_admitted {σ : Type} (definitions : List (Unit × Name))
    (artifactDigest : String) (facts : Name → Unit → DerivedFacts)
    (declared : Name → Option σ) (home : Home σ) (source environment : String)
    (valid : Home σ → Prop) (refused : standing (facts home.function home.unit) ≠ none) :
    ¬ admitted definitions (derivedPremise artifactDigest facts) declared home source
      environment valid :=
  missing_resolution_refuses definitions _ declared home source environment valid refused

/-- A caller holds only the homes rule 40 resolves for it
(`cross_unit_retain_held`). -/
def heldDerived {σ : Type} (resolvesFor : Name → Bool) (imports : List (Home σ)) :
    List (Home σ) :=
  imports.filter (fun home => resolvesFor home.function)

/-- A refused home never supplies a summary: an undeclared call to it stays
uncontracted, as it was without the artifact. -/
theorem refused_home_supplies_nothing {σ : Type} (declared : Name → Option σ)
    (imports : List (Home σ)) (resolvesFor : Name → Bool) (name : Name)
    (undeclared : declared name = none) (refused : resolvesFor name = false) :
    lookup declared (heldDerived resolvesFor imports) name = none := by
  unfold lookup heldDerived
  rw [undeclared]
  simp only
  have : (imports.filter (fun home => resolvesFor home.function)).find?
      (fun home => home.function == name) = none := by
    rw [List.find?_eq_none]
    intro home member
    have kept := (List.mem_filter.1 member).2
    intro same
    have : home.function = name := by simpa using same
    rw [this, refused] at kept
    exact Bool.false_ne_true kept
  rw [this]
  rfl

/-! ### Vectors bound to the Rust (`STANDING_VECTORS`) -/

def failing (indices : List Nat) : DerivedFacts :=
  { databaseMatches := !indices.contains 0, digestsVerify := !indices.contains 1,
    callerLinked := !indices.contains 2, executable := !indices.contains 3,
    inExecutable := !indices.contains 4, strongGlobal := !indices.contains 5,
    defaultVisibility := !indices.contains 6, onlyDefinition := !indices.contains 7,
    noDynamicRelocation := !indices.contains 8, factsAgree := !indices.contains 9,
    homeLinked := !indices.contains 10, routeUnit := !indices.contains 11,
    uniqueKey := !indices.contains 12, typeAvailable := !indices.contains 13,
    typeEqual := !indices.contains 14 }

def Refusal.index : Refusal → Nat
  | .databaseDiffers => 0
  | .digestUnverified => 1
  | .callerUnlinked => 2
  | .notExecutable => 3
  | .notInExecutable => 4
  | .notStrong => 5
  | .notDefaultVisibility => 6
  | .duplicate => 7
  | .dynamicRelocation => 8
  | .factsRefused => 9
  | .homeUnlinked => 10
  | .notRouteUnit => 11
  | .ambiguousKey => 12
  | .typeUnavailable => 13
  | .typeDiffers => 14

/-- `(failing conditions, refusal index)`. -/
def standingVectors : List (List Nat × Option Nat) := [
  ([], none), ([0], some 0), ([1], some 1), ([2], some 2), ([3], some 3),
  ([4], some 4), ([5], some 5), ([6], some 6), ([7], some 7), ([8], some 8),
  ([9], some 9), ([10], some 10), ([11], some 11), ([12], some 12),
  ([13], some 13), ([14], some 14), ([2, 14], some 2), ([12, 13], some 12),
  ([0, 1, 2], some 0), ([9, 11], some 9)
]

theorem standing_vectors_hold :
    standingVectors.all (fun (indices, expected) =>
      (standing (failing indices)).map Refusal.index == expected) = true := by
  decide

end QCore.CrossUnitSummary
