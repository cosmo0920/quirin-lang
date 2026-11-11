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

/-! Card `G03-F1`: admission of a declared source root, and the manifest
version that gates it.

This models the decision procedure of
`qcore_contract::profile_loader::is_admissible_source_root` together with
`source_root_path`: which spellings of `LibraryProfile::source_roots[].root`
a profile manifest may declare at each schema version, and where an admitted
root lands inside the pinned checkout.

What is proved here:

* the version-2 rule is a *widening* — every root version 1 admits, version 2
  admits, with the same meaning;
* the widening is exactly one spelling, the checkout root itself, and the
  literal spelling is what is tested, not a normalised path;
* **no admitted root escapes the checkout**: walking the components of any
  admissible root from the checkout never steps above it and never reaches a
  filesystem root, so admitting `"."` opens no traversal the pin does not
  already contain;
* refusal is fail-closed in the version direction too: an unsupported version
  admits nothing at all, and the same bytes that load at version 2 are refused
  at version 1.

What is **not** proved here, and stays a Rust boundary: that Rust's
`Path::components` produces the component lists transcribed below (pinned
behaviourally by `crates/qcore-audit/tests/root_source_set.rs`), and that the
audit's unit selection reads no path outside the checkout for other reasons
(the audited-file, clean-path, exact-unit and provider-manifest validators are
unchanged and keep their own rules). The corpus below is a transcription of
that test matrix, not an extraction from it. -/
namespace QCore.SourceRootAdmission

/-- One component of a path, as `std::path::Components` yields them. -/
inductive Component where
  | normal (name : String)
  | curDir
  | parentDir
  | rootDir
  deriving DecidableEq, Repr

/-- A declared source root: the literal manifest string together with the
components the path library yields for it. Both matter — the checkout-root
rule tests the literal, every other rule tests the components. -/
structure Root where
  text : String
  components : List Component
  deriving DecidableEq, Repr

/-- The one spelling that names the checkout root, mirroring
`CHECKOUT_ROOT_SOURCE_ROOT`. -/
def checkoutRoot : String := "."

/-- `is_relative_without_traversal`: nonempty, and every component ordinary. -/
def relativeWithoutTraversal (root : Root) : Bool :=
  !root.components.isEmpty &&
    root.components.all fun component =>
      match component with
      | .normal _ => true
      | _ => false

/-- The versions this loader defines, mirroring
`SUPPORTED_PROFILE_MANIFEST_VERSIONS`. -/
def supportedVersions : List Nat := [1, 2]

/-- `is_admissible_source_root`. The version-2 clause is literal equality, not
a normalisation rule. -/
def admissible (version : Nat) (root : Root) : Bool :=
  relativeWithoutTraversal root || (2 ≤ version && root.text == checkoutRoot)

/-- Whole-profile admission: an unsupported version rejects before any root is
read, so no root is admissible at it (design §3, F3). -/
def loadable (version : Nat) (root : Root) : Bool :=
  supportedVersions.contains version && admissible version root

/-- Where a root lands, walking from the checkout. `none` is an escape: a step
above the checkout, or an absolute path that leaves it entirely. -/
def walk : List Component → Option (List String)
  | [] => some []
  | .normal name :: rest => (walk rest).map fun tail => name :: tail
  | .curDir :: rest => walk rest
  | .parentDir :: _ => none
  | .rootDir :: _ => none

/-- `source_root_path`: the checkout-root spelling resolves to the checkout
itself, never to a `checkout/.` whose text would compare unequal to a
canonicalised entry path. -/
def resolve (root : Root) : Option (List String) :=
  if root.text = checkoutRoot then some [] else walk root.components

/-- The spellings `crates/qcore-audit/tests/root_source_set.rs` drives, with
the components Rust's path library yields for each. -/
def corpus : List Root :=
  [ ⟨".", [.curDir]⟩,
    ⟨"..", [.parentDir]⟩,
    ⟨"./src", [.curDir, .normal "src"]⟩,
    ⟨"src/..", [.normal "src", .parentDir]⟩,
    ⟨"/", [.rootDir]⟩,
    ⟨"/abs", [.rootDir, .normal "abs"]⟩,
    ⟨"", []⟩,
    ⟨"../checkout", [.parentDir, .normal "checkout"]⟩,
    -- `Components` normalises a trailing `.` away, so this was already an
    -- ordinary relative root at version 1 and version 2 does not move it.
    ⟨"src/.", [.normal "src"]⟩,
    ⟨"src", [.normal "src"]⟩,
    ⟨"lib/chunkio/src", [.normal "lib", .normal "chunkio", .normal "src"]⟩ ]

/-- The spellings the test matrix requires to be refused at every version. -/
def refusedCorpus : List Root :=
  corpus.filter fun root =>
    root.text ∈ ["..", "./src", "src/..", "/", "/abs", "", "../checkout"]

-- ---------------------------------------------------------------- widening

/-- Version 2 is a widening: it never refuses a root version 1 admits. -/
theorem version_two_admits_everything_version_one_does (root : Root) :
    admissible 1 root = true → admissible 2 root = true := by
  intro h
  simp [admissible] at h ⊢
  exact Or.inl h

/-- The widening is monotone in the version, not special to the pair (1, 2). -/
theorem admission_is_monotone_in_the_version (root : Root) {v w : Nat}
    (h : v ≤ w) : admissible v root = true → admissible w root = true := by
  simp only [admissible, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq]
  rintro (hrel | ⟨hv, htext⟩)
  · exact Or.inl hrel
  · exact Or.inr ⟨Nat.le_trans hv h, htext⟩

/-- Nothing else moved: on a root that is not spelled `"."`, the two versions
decide identically. Version 2 changes exactly one spelling. -/
theorem only_the_checkout_root_spelling_moved (root : Root)
    (h : root.text ≠ checkoutRoot) : admissible 2 root = admissible 1 root := by
  simp [admissible, h]

/-- And that spelling really is admitted at version 2. -/
theorem the_checkout_root_is_admissible_at_version_two (root : Root)
    (h : root.text = checkoutRoot) : admissible 2 root = true := by
  simp [admissible, h]

/-- The version gate has teeth: the same manifest bytes are refused at
version 1. Non-vacuity of the version distinction. -/
theorem the_checkout_root_is_refused_at_version_one :
    admissible 1 ⟨".", [.curDir]⟩ = false := by decide

-- ------------------------------------------------------------- no escape

/-- A component list of ordinary names always walks to a location. -/
theorem walk_of_all_normal :
    ∀ cs : List Component,
      (cs.all fun component =>
        match component with
        | .normal _ => true
        | _ => false) = true → (walk cs).isSome = true := by
  intro cs
  induction cs with
  | nil => intro _; simp [walk]
  | cons head tail ih =>
    intro h
    simp only [List.all_cons, Bool.and_eq_true] at h
    obtain ⟨hhead, htail⟩ := h
    cases head with
    | normal name =>
      have hsome := ih htail
      cases hw : walk tail with
      | none => rw [hw] at hsome; exact absurd hsome (by simp)
      | some loc => simp [walk, hw]
    | curDir => exact absurd hhead (by simp)
    | parentDir => exact absurd hhead (by simp)
    | rootDir => exact absurd hhead (by simp)

/-- Every ordinary relative root walks to a location inside the checkout. -/
theorem a_relative_root_never_escapes (root : Root) :
    relativeWithoutTraversal root = true → (walk root.components).isSome = true := by
  intro h
  simp only [relativeWithoutTraversal, Bool.and_eq_true] at h
  exact walk_of_all_normal root.components h.2

/-- The load-bearing statement: **no admitted root escapes the checkout**, at
any version. Admitting the checkout-root spelling opens no traversal. -/
theorem an_admissible_root_never_escapes (version : Nat) (root : Root) :
    admissible version root = true → (resolve root).isSome = true := by
  intro h
  simp only [admissible, Bool.or_eq_true, Bool.and_eq_true, decide_eq_true_eq,
    beq_iff_eq] at h
  by_cases htext : root.text = checkoutRoot
  · simp [resolve, htext]
  · rcases h with hrel | ⟨_, hbad⟩
    · simpa [resolve, htext] using a_relative_root_never_escapes root hrel
    · exact absurd hbad htext

/-- The checkout root resolves to the checkout itself: the empty location, not
a trailing current-directory step. -/
theorem the_checkout_root_resolves_to_the_checkout (root : Root)
    (h : root.text = checkoutRoot) : resolve root = some [] := by
  simp [resolve, h]

/-- An ordinary relative root resolves below the checkout, never at it, so the
new spelling is the only way to name the checkout itself. -/
theorem an_ordinary_root_resolves_below_the_checkout :
    resolve ⟨"lib/chunkio/src", [.normal "lib", .normal "chunkio", .normal "src"]⟩
      = some ["lib", "chunkio", "src"] := by decide

-- ------------------------------------------------- refusals over the corpus

/-- Every traversal, absolute and empty spelling is refused at both supported
versions. -/
theorem traversal_and_absolute_roots_are_refused_at_every_version :
    supportedVersions.all (fun version =>
      refusedCorpus.all fun root => !admissible version root) = true := by
  decide

/-- The refused set is not empty, so the theorem above is not vacuous. -/
theorem the_refused_corpus_is_inhabited : refusedCorpus.length = 7 := by
  decide

/-- Refusal is conservative, not minimal, and that is deliberate: `"./src"`
would not escape, and is refused anyway because admission tests the literal
spelling. -/
theorem a_refused_root_may_still_be_harmless :
    admissible 2 ⟨"./src", [.curDir, .normal "src"]⟩ = false
      ∧ (walk [.curDir, .normal "src"]).isSome = true := by
  decide

/-- Over the whole corpus, exactly one spelling separates the two versions. -/
theorem exactly_one_corpus_spelling_separates_the_versions :
    (corpus.filter fun root => admissible 2 root && !admissible 1 root)
      = [⟨".", [.curDir]⟩] := by
  decide

/-- No admitted root in the corpus escapes, at either version. -/
theorem no_admitted_corpus_root_escapes :
    supportedVersions.all (fun version =>
      corpus.all fun root => !admissible version root || (resolve root).isSome) = true := by
  decide

-- --------------------------------------------------------- version gating

/-- An unsupported version admits nothing: the whole profile rejects before any
root is read. -/
theorem an_unsupported_version_admits_nothing (version : Nat) (root : Root)
    (h : supportedVersions.contains version = false) : loadable version root = false := by
  simp only [loadable, h, Bool.false_and]

/-- Including the checkout root, which a higher version number does not buy. -/
theorem a_higher_version_number_does_not_buy_the_checkout_root :
    loadable 3 ⟨".", [.curDir]⟩ = false := by decide

/-- Both defined versions load an ordinary relative root, so the gate is not
vacuous in the accepting direction either. -/
theorem both_supported_versions_load_an_ordinary_root :
    supportedVersions.all (fun version =>
      loadable version ⟨"lib/chunkio/src",
        [.normal "lib", .normal "chunkio", .normal "src"]⟩) = true := by
  decide

/-- The shipped profiles declare version 1 and name no checkout root, so the
version-2 rule cannot change how any of them loads. -/
theorem a_version_one_profile_is_unaffected (root : Root)
    (h : root.text ≠ checkoutRoot) : loadable 1 root = loadable 2 root := by
  simp [loadable, supportedVersions, only_the_checkout_root_spelling_moved root h]

end QCore.SourceRootAdmission
