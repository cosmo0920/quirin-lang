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

/-! Card `REASON-CODES`: the closed reason-code vocabulary for uncovered
obligations (`docs/QCORE_SPEC_r0_3.md` §L21.1).

This models the *classification relation* of
`crates/qcore-schemas/src/reason.rs`: the twelve closed categories, the
three-way subject partition, and the fail-closed mapping from the free-text
reason families this workspace really emits.

What is proved here is exactly the structural half — that the partition is a
partition, that `environmentUnavailable` is its sole environment member so an
environment row can never be read as a statement about the analysed source,
that the mapping is total and single-valued on the enumerated families and
non-vacuous on every category, and that a refused input yields no category at
all rather than a defaulted one.

What is **not** proved here, and remains a Rust boundary: that the Rust marker
table recognises exactly these families (pinned by
`crates/qcore-schemas/tests/reason_codes.rs`), and that every production
emission site names the category this relation assigns (pinned by
`crates/qcore-verification/tests/unit/obligation_derivation.rs`). The family
enumeration below is a transcription of those corpora, not an extraction from
them. -/
namespace QCore.ReasonCategory

/-- What a reason row speaks about. §L21.1 rule 1 protects this axis. -/
inductive Subject where
  /-- The row names a construct or an unresolved obligation in the analysed
  source. Only these may be read as saying something about the subject
  program. -/
  | analysedSource
  /-- The row names a limitation, refused premise or identity mismatch in the
  analysis itself. -/
  | analyser
  /-- The row names a missing or mismatched tool, build tree or configuration
  input. It says nothing about the analysed source. -/
  | environment
  deriving DecidableEq, Repr

/-- The closed twelve-category vocabulary. `awaitingHumanReview` was added by
card W45-CONTRACT-DEFERRED-REVIEW: a declared contract the row depends on is
marked as awaiting human review. `contractPinMismatch` was added by card
W45-CONTRACT-REVISION-MODE: a pinned contract the row depends on is not in
force, because the audited source does not match its lock.
`pluginClaimUncertified` was added by card W45-USER-CONTRACT-PLUGINS: a checker
plugin claimed `Safe` or `Unsafe` without a certificate or witness the audit
checks independently. `outerContract` was added by card OUTER-CONTRACT: in the
runtime projection, a closed-world fact the row rests on is vouched for by an
outer contract (gateway rule 31). `compositionConflict` was added by card
AUDIT-COMPOSITION: two components of one row under one provenance key differ,
so both are kept and the row is at least `Unknown` (`QCore.AuditComposition`).
`contractConflict` was added by card CONTRACT-SUPERSESSION: the pin stamps of
the bundles declaring a called function do not decide which declaration is in
force, so none is (`QCore.ContractSupersession`). -/
inductive Category where
  | unsupportedSyntax
  | missingPremise
  | incompleteObligationCoverage
  | staleEvidence
  | providerFailure
  | residualCustody
  | environmentUnavailable
  | awaitingHumanReview
  | contractPinMismatch
  | pluginClaimUncertified
  | outerContract
  | compositionConflict
  | contractConflict
  deriving DecidableEq, Repr

namespace Category

/-- Every category, mirroring `ReasonCategory::ALL`. -/
def all : List Category :=
  [ .unsupportedSyntax, .missingPremise, .incompleteObligationCoverage,
    .staleEvidence, .providerFailure, .residualCustody, .environmentUnavailable,
    .awaitingHumanReview, .contractPinMismatch, .pluginClaimUncertified,
    .outerContract, .compositionConflict, .contractConflict ]

/-- The spec spelling, identical to the Rust serde representation. -/
def spelling : Category → String
  | .unsupportedSyntax => "unsupported-syntax"
  | .missingPremise => "missing-premise"
  | .incompleteObligationCoverage => "incomplete-obligation-coverage"
  | .staleEvidence => "stale-evidence"
  | .providerFailure => "provider-failure"
  | .residualCustody => "residual-custody"
  | .environmentUnavailable => "environment-unavailable"
  | .awaitingHumanReview => "awaiting-human-review"
  | .contractPinMismatch => "contract-pin-mismatch"
  | .pluginClaimUncertified => "plugin-claim-uncertified"
  | .outerContract => "outer-contract"
  | .compositionConflict => "composition-conflict"
  | .contractConflict => "contract-conflict"

/-- The subject partition, mirroring `ReasonCategory::subject`. -/
def subject : Category → Subject
  | .unsupportedSyntax => .analysedSource
  | .residualCustody => .analysedSource
  | .missingPremise => .analyser
  | .incompleteObligationCoverage => .analyser
  | .staleEvidence => .analyser
  | .providerFailure => .analyser
  | .awaitingHumanReview => .analyser
  | .contractPinMismatch => .analyser
  | .pluginClaimUncertified => .analyser
  | .outerContract => .analyser
  | .compositionConflict => .analyser
  | .contractConflict => .analyser
  | .environmentUnavailable => .environment

def describesAnalysedSource (c : Category) : Bool := subject c == .analysedSource

def isEnvironment (c : Category) : Bool := subject c == .environment

/-- A row may support a provisional `Unsafe` only when it speaks about the
analysed source. -/
def maySupportUnsafe (c : Category) : Bool := describesAnalysedSource c

/-- Card W45-USER-CONTRACT-PLUGINS: the categories a person resolves by a
named act, mirroring `ReasonCategory::is_human_resolvable`. A row withheld for
one of these alone is `Pending` (`QCore.VerdictSemigroup`). -/
def isHumanResolvable : Category → Bool
  | .awaitingHumanReview => true
  | .contractPinMismatch => true
  | .pluginClaimUncertified => true
  | .outerContract => true
  | _ => false

end Category

open Category

/-- The vocabulary is closed and `all` really enumerates it. -/
theorem all_is_complete (c : Category) : c ∈ all := by
  cases c <;> simp [all]

theorem all_has_no_duplicates : all.length = 13 ∧ all.eraseDups.length = 13 := by
  constructor <;> rfl

theorem spellings_are_pairwise_distinct :
    (all.map spelling).eraseDups.length = 13 := by decide

/-- Every human-resolvable category speaks about the analyser, so none can
support a provisional `Unsafe`. -/
theorem a_human_resolvable_category_never_supports_unsafe (c : Category)
    (resolvable : isHumanResolvable c = true) : maySupportUnsafe c = false := by
  cases c <;> simp_all [isHumanResolvable, maySupportUnsafe, describesAnalysedSource, subject]

/-- Exactly four categories are human-resolvable (card OUTER-CONTRACT added
`outerContract`). -/
theorem exactly_four_categories_are_human_resolvable :
    (all.filter isHumanResolvable) =
      [.awaitingHumanReview, .contractPinMismatch, .pluginClaimUncertified,
        .outerContract] := rfl

/-- §L21.1 rule 1, structurally: `environmentUnavailable` is the sole member of
the environment subject. -/
theorem environment_subject_is_inhabited_only_by_environment_unavailable
    (c : Category) : subject c = .environment ↔ c = .environmentUnavailable := by
  cases c <;> simp [subject]

/-- The partition is a partition: no category has two subjects, and in
particular none is both an environment row and a statement about source. -/
theorem no_category_is_both_environment_and_source (c : Category) :
    ¬(isEnvironment c = true ∧ describesAnalysedSource c = true) := by
  cases c <;> simp [isEnvironment, describesAnalysedSource, subject]

/-- An environment failure can never contribute to a provisional `Unsafe`. -/
theorem an_environment_row_never_supports_unsafe :
    maySupportUnsafe .environmentUnavailable = false := rfl

theorem environment_unavailable_says_nothing_about_the_source :
    describesAnalysedSource .environmentUnavailable = false := rfl

/-- All three subjects are actually reached, so the partition is not degenerate. -/
theorem every_subject_is_reached :
    (∃ c, subject c = .analysedSource) ∧ (∃ c, subject c = .analyser) ∧
      (∃ c, subject c = .environment) :=
  ⟨⟨.unsupportedSyntax, rfl⟩, ⟨.missingPremise, rfl⟩, ⟨.environmentUnavailable, rfl⟩⟩

/-- Exactly two categories are statements about the analysed source. -/
theorem exactly_two_categories_describe_the_analysed_source :
    (all.filter describesAnalysedSource) = [.unsupportedSyntax, .residualCustody] := rfl

/-- The free-text reason families this workspace emits, transcribed from
`crates/qcore-schemas/tests/reason_codes.rs`'s corpus. Grouped by emitter. -/
inductive Family where
  -- qcore-verification: obligation derivation
  | noCoreDerivation
  | refusedObservationsTrusted
  | unparseableContractModule
  | ambiguousObservationContract
  | missingExternHookContract
  | gatewayCustodyDerivation
  | crossFunctionOrderBound
  | lifecycleAbiImportUnavailable
  -- qcore-verification: LifecycleScan source gaps
  | structuredControlFlow
  | operationsAfterReturn
  | unrecoverableCallStructure
  | unknownCallEffect
  | unsupportedModelCallArity
  | shadowedLifecycleIdentity
  | unresolvedLifecycleArgument
  -- qcore-c-gateway
  | projectionUnresolvedWithUnmodeledOperations
  | projectionUnresolvedCustodyOnly
  | functionNotFound
  | malformedFunctionBody
  | productionParserUnavailable
  | productionParserRejected
  | unsupportedInputIdentity
  | outsideBooleanSourceProfile
  | helperSignatureMismatch
  | unmodeledConstruct
  -- qcore-audit: compile command / checkout / tool
  | compileCommandUnusable
  | configuredClangMismatch
  | sourceOutsideCheckout
  | notUtf8Source
  | pinnedBytesMismatch
  | historicalEvidenceNotAdmitted
  | absentFromTypedImport
  | sourceContractConfiguration
  -- W45-CONTRACT-DEFERRED-REVIEW: a projection summary naming held calls, and
  -- the held-call statement itself
  | projectionHeldAwaitingHumanReview
  | heldCallAwaitingHumanReview
  -- W45-CONTRACT-REVISION-MODE: a projection summary naming calls held for a
  -- contract pin mismatch, and the held-call statement itself
  | projectionHeldContractPinMismatch
  | heldCallContractPinMismatch
  -- W45-USER-CONTRACT-PLUGINS: the plugin firewall's uncertified claim
  | pluginClaimUncertified
  -- OUTER-CONTRACT: the runtime projection's covered closed-world fact
  | outerContractCovered
  -- AUDIT-COMPOSITION: the conflict leaf of a composed row
  | compositionConflictLeaf
  -- CONTRACT-SUPERSESSION: the held call of a function whose bundles conflict
  | heldCallContractConflict
  deriving DecidableEq, Repr

namespace Family

def all : List Family :=
  [ .noCoreDerivation, .refusedObservationsTrusted, .unparseableContractModule,
    .ambiguousObservationContract, .missingExternHookContract,
    .gatewayCustodyDerivation, .crossFunctionOrderBound,
    .lifecycleAbiImportUnavailable, .structuredControlFlow,
    .operationsAfterReturn, .unrecoverableCallStructure, .unknownCallEffect,
    .unsupportedModelCallArity, .shadowedLifecycleIdentity,
    .unresolvedLifecycleArgument, .projectionUnresolvedWithUnmodeledOperations,
    .projectionUnresolvedCustodyOnly, .functionNotFound, .malformedFunctionBody,
    .productionParserUnavailable, .productionParserRejected,
    .unsupportedInputIdentity, .outsideBooleanSourceProfile,
    .helperSignatureMismatch, .unmodeledConstruct, .compileCommandUnusable,
    .configuredClangMismatch, .sourceOutsideCheckout, .notUtf8Source,
    .pinnedBytesMismatch, .historicalEvidenceNotAdmitted,
    .absentFromTypedImport, .sourceContractConfiguration,
    .projectionHeldAwaitingHumanReview, .heldCallAwaitingHumanReview,
    .projectionHeldContractPinMismatch, .heldCallContractPinMismatch,
    .pluginClaimUncertified, .outerContractCovered, .compositionConflictLeaf,
    .heldCallContractConflict ]

theorem all_is_complete (f : Family) : f ∈ all := by
  cases f <;> simp [all]

end Family

/-- The documented mapping. A total function, so it is single-valued by
construction; the content is that each family lands on the category the Rust
table and the typed emission sites agree on. -/
def categoryOf : Family → Category
  | .noCoreDerivation => .incompleteObligationCoverage
  | .gatewayCustodyDerivation => .incompleteObligationCoverage
  | .crossFunctionOrderBound => .incompleteObligationCoverage
  | .lifecycleAbiImportUnavailable => .incompleteObligationCoverage
  | .unsupportedInputIdentity => .incompleteObligationCoverage
  | .functionNotFound => .incompleteObligationCoverage
  | .absentFromTypedImport => .incompleteObligationCoverage
  | .refusedObservationsTrusted => .missingPremise
  | .unparseableContractModule => .missingPremise
  | .ambiguousObservationContract => .missingPremise
  | .missingExternHookContract => .missingPremise
  | .structuredControlFlow => .unsupportedSyntax
  | .operationsAfterReturn => .unsupportedSyntax
  | .unrecoverableCallStructure => .unsupportedSyntax
  | .unknownCallEffect => .unsupportedSyntax
  | .unsupportedModelCallArity => .unsupportedSyntax
  | .shadowedLifecycleIdentity => .unsupportedSyntax
  | .unresolvedLifecycleArgument => .unsupportedSyntax
  | .projectionUnresolvedWithUnmodeledOperations => .unsupportedSyntax
  | .malformedFunctionBody => .unsupportedSyntax
  | .productionParserRejected => .unsupportedSyntax
  | .outsideBooleanSourceProfile => .unsupportedSyntax
  | .helperSignatureMismatch => .unsupportedSyntax
  | .unmodeledConstruct => .unsupportedSyntax
  | .notUtf8Source => .unsupportedSyntax
  | .projectionUnresolvedCustodyOnly => .residualCustody
  | .productionParserUnavailable => .environmentUnavailable
  | .compileCommandUnusable => .environmentUnavailable
  | .configuredClangMismatch => .environmentUnavailable
  | .sourceOutsideCheckout => .environmentUnavailable
  | .pinnedBytesMismatch => .staleEvidence
  | .historicalEvidenceNotAdmitted => .staleEvidence
  | .sourceContractConfiguration => .providerFailure
  | .projectionHeldAwaitingHumanReview => .awaitingHumanReview
  | .heldCallAwaitingHumanReview => .awaitingHumanReview
  | .projectionHeldContractPinMismatch => .contractPinMismatch
  | .heldCallContractPinMismatch => .contractPinMismatch
  | .pluginClaimUncertified => .pluginClaimUncertified
  | .outerContractCovered => .outerContract
  | .compositionConflictLeaf => .compositionConflict
  | .heldCallContractConflict => .contractConflict

/-- Totality: every enumerated family receives a category of the closed set. -/
theorem the_mapping_is_total (f : Family) : categoryOf f ∈ Category.all :=
  all_is_complete _

/-- Non-vacuity: every category is reached by some real family, so no arm of
the vocabulary is decorative. -/
theorem every_category_is_reached (c : Category) :
    ∃ f : Family, categoryOf f = c := by
  cases c
  · exact ⟨.structuredControlFlow, rfl⟩
  · exact ⟨.refusedObservationsTrusted, rfl⟩
  · exact ⟨.noCoreDerivation, rfl⟩
  · exact ⟨.pinnedBytesMismatch, rfl⟩
  · exact ⟨.sourceContractConfiguration, rfl⟩
  · exact ⟨.projectionUnresolvedCustodyOnly, rfl⟩
  · exact ⟨.productionParserUnavailable, rfl⟩
  · exact ⟨.projectionHeldAwaitingHumanReview, rfl⟩
  · exact ⟨.projectionHeldContractPinMismatch, rfl⟩
  · exact ⟨.pluginClaimUncertified, rfl⟩
  · exact ⟨.outerContractCovered, rfl⟩
  · exact ⟨.compositionConflictLeaf, rfl⟩
  · exact ⟨.heldCallContractConflict, rfl⟩

/-- The families whose subject really is the environment: a missing or
mismatched tool, checkout or compile command. -/
def isEnvironmentFamily : Family → Bool
  | .productionParserUnavailable => true
  | .compileCommandUnusable => true
  | .configuredClangMismatch => true
  | .sourceOutsideCheckout => true
  | _ => false

/-- §L21.1 rule 1, over the real corpus: an environment family is mapped to the
environment category, and therefore never to a statement about the analysed
source. This is the property a naive substring table breaks — three of these
four reasons contain the word "unsupported". -/
theorem an_environment_family_never_becomes_a_source_statement (f : Family)
    (env : isEnvironmentFamily f = true) :
    categoryOf f = .environmentUnavailable ∧
      describesAnalysedSource (categoryOf f) = false := by
  cases f <;> simp_all [isEnvironmentFamily, categoryOf, describesAnalysedSource, subject]

/-- The converse: nothing but an environment family is labeled environment, so
the category is not over-applied either. -/
theorem only_an_environment_family_is_labeled_environment (f : Family)
    (labeled : categoryOf f = .environmentUnavailable) :
    isEnvironmentFamily f = true := by
  cases f <;> simp_all [isEnvironmentFamily, categoryOf]

/-- No environment family can support a provisional `Unsafe`. -/
theorem an_environment_family_never_supports_unsafe (f : Family)
    (env : isEnvironmentFamily f = true) :
    maySupportUnsafe (categoryOf f) = false := by
  cases f <;> simp_all [isEnvironmentFamily, categoryOf, maySupportUnsafe,
    describesAnalysedSource, subject]

/-- A classification input: either a family the documented mapping recognises,
or a string it does not. -/
inductive Input where
  | known : Family → Input
  | unrecognised : Input
  deriving DecidableEq, Repr

/-- Fail-closed classification, mirroring `ReasonCategory::classify`'s
`Result`: an unrecognised input yields no category, never a defaulted one. -/
def classify : Input → Option Category
  | .known f => some (categoryOf f)
  | .unrecognised => none

theorem an_unrecognised_reason_is_refused : classify .unrecognised = none := rfl

/-- The fail-closed property that matters: refusal never produces a category
whose subject is the analysed source — nor any category at all. -/
theorem refusal_never_yields_a_source_statement (c : Category) :
    classify .unrecognised ≠ some c := by simp [classify]

/-- Classification is single-valued: it is a function, so one input never
carries two categories. -/
theorem classification_is_single_valued {i : Input} {a b : Category}
    (ha : classify i = some a) (hb : classify i = some b) : a = b := by
  rw [ha] at hb; exact Option.some.inj hb

/-- Every recognised input classifies, so the mapping is total where it claims
to be. -/
theorem every_known_input_classifies (f : Family) :
    ∃ c, classify (.known f) = some c := ⟨categoryOf f, rfl⟩

/-- Composite statement of §L21.1 rule 1 over the whole input space: if a
classification result describes the analysed source, its input was a
recognised, non-environment family. -/
theorem a_source_statement_has_a_recognised_non_environment_origin {i : Input}
    {c : Category} (classified : classify i = some c)
    (source : describesAnalysedSource c = true) :
    ∃ f, i = .known f ∧ isEnvironmentFamily f = false := by
  cases i with
  | unrecognised => simp [classify] at classified
  | known f =>
    refine ⟨f, rfl, ?_⟩
    cases env : isEnvironmentFamily f with
    | false => rfl
    | true =>
      exfalso
      have notSource := (an_environment_family_never_becomes_a_source_statement f env).2
      simp only [classify, Option.some.injEq] at classified
      subst classified
      simp [notSource] at source

end QCore.ReasonCategory
