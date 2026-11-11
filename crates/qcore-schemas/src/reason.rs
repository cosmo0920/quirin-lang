// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The closed reason-code vocabulary for uncovered obligations
//! (`docs/QCORE_SPEC_r0_3.md` §L21.1).
//!
//! Every `Unknown` row in this project carries a free-text reason. Free text is
//! not queryable and — the point of the vocabulary — does not distinguish a
//! statement about the *analysed source* from a statement about the *machine
//! analysing it*. This module adds the closed classification alongside that
//! text. It refines the row; it replaces nothing. No source span, `property`,
//! semantic profile, scope, bound, assumption or obligation identifier is
//! reachable from here (§L21.1 rule 2).
//!
//! Two structural rules hold by construction rather than by review:
//!
//! 1. [`ReasonCategory::EnvironmentUnavailable`] is the sole inhabitant of
//!    [`ReasonSubject::Environment`], and [`ReasonSubject`] is a partition of a
//!    closed enumeration, so an environment failure can never carry a category
//!    whose subject is the analysed source (§L21.1 rule 1).
//! 2. [`ReasonCategory::classify`] is fail-closed. A string the documented
//!    mapping does not recognise is *refused*; there is no fallback category,
//!    and in particular no fallback onto a subject-`AnalysedSource` category.
//!
//! Production emission sites name their category at construction rather than
//! recovering it from prose. [`ReasonCategory::classify`] is the documented
//! mapping for string-only rows, and the crate's tests require it to agree with
//! every typed emission site (see `crates/qcore-schemas/tests/reason_codes.rs`
//! and `crates/qcore-verification/tests/unit/obligation_derivation.rs`).

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

/// What a reason row speaks about.
///
/// This is the axis §L21.1 rule 1 protects: a row about the environment says
/// nothing about the analysed source, so its category must never be one whose
/// subject is [`Self::AnalysedSource`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReasonSubject {
    /// The row names a construct or an unresolved obligation *in the analysed
    /// source*. Only these categories may be read as saying something about the
    /// subject program.
    AnalysedSource,
    /// The row names a limitation, refused premise or identity mismatch in the
    /// analysis itself. It says nothing about the analysed source.
    Analyser,
    /// The row names a missing or mismatched tool, build tree or configuration
    /// input. It says nothing about the analysed source and can never
    /// contribute to a provisional `Unsafe`.
    Environment,
}

/// The closed twelve-category vocabulary of §L21.1.
///
/// The wire spelling is the spec's kebab-case spelling; it is part of the
/// schema surface and moving it is a compatibility event.
/// There is deliberately no `Default`: a defaulted category is exactly the
/// silent relabeling §L21.1 rule 1 forbids, and every construction site in this
/// workspace names its category explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReasonCategory {
    /// The input is well-formed but contains a construct outside the modeled
    /// grammar. Subject: the analysed source.
    UnsupportedSyntax,
    /// A declared assumption or trusted input the analysis requires was refused
    /// or not supplied. Subject: the analyser.
    MissingPremise,
    /// A capability has no derivation, or only a partial one, in the current
    /// increment. Subject: the analyser.
    IncompleteObligationCoverage,
    /// Evidence exists but is bound to a different identity than the subject
    /// under check. Subject: the analyser.
    StaleEvidence,
    /// The selected provider was absent, failed, returned a malformed or
    /// rejected response, or failed attribution at the firewall. Subject: the
    /// analyser.
    ProviderFailure,
    /// Analysis completed within its model but left an unresolved custody,
    /// containment or leak obligation. Subject: the analysed source.
    ResidualCustody,
    /// The analysis could not run because a required tool, build tree or
    /// configuration input was missing or mismatched. Subject: the environment.
    EnvironmentUnavailable,
    /// Card W45-CONTRACT-DEFERRED-REVIEW: a declared contract the row depends
    /// on is marked as awaiting human review (`resource_review_pending`,
    /// contract spec §C06), so it discharges nothing and accuses nothing.
    /// Subject: the analyser — the row says nothing about the analysed source
    /// and never contributes to a provisional `Unsafe`.
    AwaitingHumanReview,
    /// Card W45-CONTRACT-REVISION-MODE: a pinned contract the row depends on
    /// is not in force, because the audited source does not match the
    /// identity its lock records (contract spec §C06.2). Subject: the analyser
    /// — the row says nothing about the analysed source and never contributes
    /// to a provisional `Unsafe`.
    ContractPinMismatch,
    /// Card W45-USER-CONTRACT-PLUGINS: an out-of-process checker plugin
    /// attached to a declared contract claimed `Safe` or `Unsafe` without a
    /// certificate or witness the audit checks independently (contract spec
    /// §C11.1). The claim is recorded and decides nothing: the row is
    /// `Pending` until a person supplies a checkable certificate or witness.
    /// Subject: the analyser — the row says nothing about the analysed source
    /// and never contributes to a provisional `Unsafe`.
    PluginClaimUncertified,
    /// Card OUTER-CONTRACT (gateway rule 31): in the runtime projection, the
    /// row rests on a closed-world fact that code loaded at runtime could
    /// falsify, and an outer contract (`quant #outer [[contract]] NAME`)
    /// vouches for it. The contract is the reason: a person resolves it by
    /// reviewing the loaded code against the contract, or by a deployment that
    /// loads nothing. Subject: the analyser — the row says nothing about the
    /// analysed source and never contributes to a provisional `Unsafe`.
    OuterContract,
    /// Card AUDIT-COMPOSITION (§L21.2, composition across results): two
    /// components of one row under one provenance key differ byte for byte
    /// (`qcore_schemas::composition`). Both are kept and neither is chosen, so
    /// the row is at least `Unknown`. Subject: the analyser — the row says
    /// nothing about the analysed source and never contributes to a
    /// provisional `Unsafe`; it is not human-resolvable, because no single
    /// named act decides which producer's account is right.
    CompositionConflict,
    /// Card CONTRACT-SUPERSESSION (contract spec §C06.6): more than one
    /// contract bundle declares the function a call relies on and their pin
    /// stamps do not decide which is in force (equal stamps on different
    /// texts, or a missing, malformed or stale stamp, or an unpinned
    /// declaration), so no declaration is in force and every candidate is
    /// recorded. Subject: the analyser — the row says nothing about the
    /// analysed source and never contributes to a provisional `Unsafe`; it is
    /// not human-resolvable as such, because which bundle should win is not
    /// one named act on this row.
    ContractConflict,
}

impl ReasonCategory {
    /// Every category, in declaration order. The vocabulary is closed: adding a
    /// member is a compatibility event, and the totality tests enumerate this
    /// array rather than a hand-written list.
    pub const ALL: [Self; 13] = [
        Self::UnsupportedSyntax,
        Self::MissingPremise,
        Self::IncompleteObligationCoverage,
        Self::StaleEvidence,
        Self::ProviderFailure,
        Self::ResidualCustody,
        Self::EnvironmentUnavailable,
        Self::AwaitingHumanReview,
        Self::ContractPinMismatch,
        Self::PluginClaimUncertified,
        Self::OuterContract,
        Self::CompositionConflict,
        Self::ContractConflict,
    ];

    /// The spec spelling, identical to the serde representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSyntax => "unsupported-syntax",
            Self::MissingPremise => "missing-premise",
            Self::IncompleteObligationCoverage => "incomplete-obligation-coverage",
            Self::StaleEvidence => "stale-evidence",
            Self::ProviderFailure => "provider-failure",
            Self::ResidualCustody => "residual-custody",
            Self::EnvironmentUnavailable => "environment-unavailable",
            Self::AwaitingHumanReview => "awaiting-human-review",
            Self::ContractPinMismatch => "contract-pin-mismatch",
            Self::PluginClaimUncertified => "plugin-claim-uncertified",
            Self::OuterContract => "outer-contract",
            Self::CompositionConflict => "composition-conflict",
            Self::ContractConflict => "contract-conflict",
        }
    }

    /// What this category speaks about (§L21.1 rule 1).
    #[must_use]
    pub const fn subject(self) -> ReasonSubject {
        match self {
            Self::UnsupportedSyntax | Self::ResidualCustody => ReasonSubject::AnalysedSource,
            Self::MissingPremise
            | Self::IncompleteObligationCoverage
            | Self::StaleEvidence
            | Self::ProviderFailure
            | Self::AwaitingHumanReview
            | Self::ContractPinMismatch
            | Self::PluginClaimUncertified
            | Self::OuterContract
            | Self::CompositionConflict
            | Self::ContractConflict => ReasonSubject::Analyser,
            Self::EnvironmentUnavailable => ReasonSubject::Environment,
        }
    }

    /// `true` when the row makes a claim about the analysed source. §L21.1 rule
    /// 1 forbids an environment failure from ever carrying such a category.
    #[must_use]
    pub const fn describes_analysed_source(self) -> bool {
        matches!(self.subject(), ReasonSubject::AnalysedSource)
    }

    /// `true` for the single environment category.
    #[must_use]
    pub const fn is_environment(self) -> bool {
        matches!(self.subject(), ReasonSubject::Environment)
    }

    /// `true` when a row of this category may contribute to a provisional
    /// `Unsafe`. §L21.1 rule 1 states an environment row never may; and no
    /// analyser-subject row ever may either, because an analyser limitation is
    /// not a validated reachable violation (AGENTS.md invariant 2).
    #[must_use]
    pub const fn may_support_unsafe(self) -> bool {
        self.describes_analysed_source()
    }

    /// Card W45-USER-CONTRACT-PLUGINS: `true` for the categories a person
    /// resolves by a named act — a review edit (`awaiting-human-review`), a
    /// re-pin after review (`contract-pin-mismatch`) or a checkable
    /// certificate for a plugin's claim (`plugin-claim-uncertified`) or a
    /// review of loaded code against an outer contract (`outer-contract`,
    /// card OUTER-CONTRACT). A row
    /// withheld for one of these alone is `Pending`
    /// ([`crate::verdict::Verdict::Pending`]); every other category leaves its
    /// row `Unknown`. Exactly the categories of
    /// [`crate::verdict::PendingReason`].
    #[must_use]
    pub const fn is_human_resolvable(self) -> bool {
        matches!(
            self,
            Self::AwaitingHumanReview
                | Self::ContractPinMismatch
                | Self::PluginClaimUncertified
                | Self::OuterContract
        )
    }

    /// The documented mapping from the free-text reasons in force onto the
    /// closed vocabulary (§L21.1, "Representative reasons in force").
    ///
    /// The table is ordered and first-match-wins; each marker is a stable
    /// fragment of a real emitted string, not a guess. Every string emitted by
    /// a production site is required to classify to exactly the category that
    /// site names — the totality tests enumerate the sites and check the
    /// agreement, so this table cannot drift from the typed carriers unnoticed.
    ///
    /// Fail-closed: an unrecognised string is refused. There is no fallback
    /// category and, in particular, no fallback onto a category whose subject
    /// is the analysed source.
    ///
    /// # Errors
    ///
    /// Returns [`UnclassifiedReason`] when no rule matches.
    pub fn classify(reason: &str) -> Result<Self, UnclassifiedReason> {
        // The gateway's dominant projection summary carries its own component
        // counts, so it is decided exactly rather than by a marker. Both arms
        // speak about the analysed source; an unparseable count is refused.
        if let Some(rest) = reason.strip_prefix(PROJECTION_UNRESOLVED_PREFIX) {
            let unsupported = rest.split(' ').next().and_then(|n| n.parse::<u64>().ok());
            // Card W45-CONTRACT-DEFERRED-REVIEW: the summary names its calls
            // held for a pending review only when there is at least one, so
            // every summary written before the card classifies as it did.
            // Card W45-CONTRACT-REVISION-MODE: a second optional component
            // names calls held for a pinned contract that is not in force, in
            // the same way and after the review component.
            let held = match rest.split_once(PROJECTION_HELD_SEPARATOR) {
                None => Some((0, 0)),
                Some((_, tail)) => held_components(tail),
            };
            return match (held, unsupported) {
                (None, _) | (_, None) => Err(UnclassifiedReason),
                // A call held for review withholds a declared contract, and
                // whatever the projection then leaves open downstream of it
                // is not a statement about the source: the held premise is
                // named first, so no held row reaches a source category.
                (Some((review, _)), Some(_)) if review > 0 => Ok(Self::AwaitingHumanReview),
                // A call held for a pin mismatch withholds a declared contract
                // in the same way.
                (Some((_, pinned)), Some(_)) if pinned > 0 => Ok(Self::ContractPinMismatch),
                // No unmodeled operation: the model ran and left custody open.
                (Some(_), Some(0)) => Ok(Self::ResidualCustody),
                // At least one construct the model does not cover.
                (Some(_), Some(_)) => Ok(Self::UnsupportedSyntax),
            };
        }
        for (marker, category) in REASON_RULES {
            if reason.contains(marker) {
                return Ok(*category);
            }
        }
        Err(UnclassifiedReason)
    }
}

/// `qcore_c_gateway::legacy_c`'s resource-projection summary prefix.
const PROJECTION_UNRESOLVED_PREFIX: &str = "resource projection unresolved: ";

/// The separator before the optional held-call component of that summary
/// (card W45-CONTRACT-DEFERRED-REVIEW).
const PROJECTION_HELD_SEPARATOR: &str = "custody summary conflict(s), ";

/// The held-call component's suffix.
const PROJECTION_HELD_SUFFIX: &str = " call(s) held awaiting human review";

/// The suffix of the component naming calls held for a pinned contract that
/// is not in force (card W45-CONTRACT-REVISION-MODE).
const PROJECTION_PIN_HELD_SUFFIX: &str = " call(s) held for a contract pin mismatch";

/// The held components after the summary's conflict count: an optional review
/// component, then an optional pin component, each named only when its count
/// is positive. `None` for anything else, so a malformed or zero count refuses.
fn held_components(tail: &str) -> Option<(u64, u64)> {
    let positive = |text: &str, suffix: &str| {
        text.strip_suffix(suffix)
            .and_then(|n| n.parse::<u64>().ok())
            .filter(|n| *n > 0)
    };
    match tail.split_once(", ") {
        Some((review, pinned)) => Some((
            positive(review, PROJECTION_HELD_SUFFIX)?,
            positive(pinned, PROJECTION_PIN_HELD_SUFFIX)?,
        )),
        None => positive(tail, PROJECTION_HELD_SUFFIX)
            .map(|review| (review, 0))
            .or_else(|| positive(tail, PROJECTION_PIN_HELD_SUFFIX).map(|pinned| (0, pinned))),
    }
}

/// The ordered, documented string → category rules used by
/// [`ReasonCategory::classify`].
///
/// Ordering matters only where one marker is a substring of another; the
/// schema tests assert that no earlier rule shadows a later one for any string
/// the production sites actually emit.
const REASON_RULES: &[(&str, ReasonCategory)] = &[
    // ---- environment: the compile command, compilation database, checkout
    // selection or tool could not be used. These are matched FIRST, because
    // several of them contain the word "unsupported" and must never fall
    // through to a rule whose subject is the analysed source (§L21.1 rule 1).
    // Sites: `qcore_audit::compile_db`, `qcore_audit::current`,
    // `qcore_audit::lib`, `qcore_c_gateway::legacy_c::LegacyCError`.
    (
        "audit requires the configured Clang executable",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "unsupported compilation dialect",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "unterminated quoting in compile command",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "shell expansion in compile command is unsupported",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "shell expression in compile command is unsupported",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "empty compiler executable",
        ReasonCategory::EnvironmentUnavailable,
    ),
    ("cannot be resolved", ReasonCategory::EnvironmentUnavailable),
    (
        "require the Clang C driver",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "compile directory and source must be absolute",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "empty or unexpanded compiler option is unsupported",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "sysroot-relative include paths are unsupported",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "invalid -Wundef-prefix argument",
        ReasonCategory::EnvironmentUnavailable,
    ),
    ("-arch argument", ReasonCategory::EnvironmentUnavailable),
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): a target, ABI or `-target`
    // selection the reader cannot admit under the readers' LP64 premise.
    (
        "unsupported compilation target",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "unsupported compilation ABI",
        ReasonCategory::EnvironmentUnavailable,
    ),
    ("-target argument", ReasonCategory::EnvironmentUnavailable),
    (
        "compile output argument",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "cannot be preserved by the bounded gateway",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "compile command requires one -c",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "compile command has no recorded source input",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "require an explicit supported -std= dialect",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "compilation database contains no command for",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "distinct configurations for",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "source outside selected checkout",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "source outside current checkout",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "production C parser is unavailable",
        ReasonCategory::EnvironmentUnavailable,
    ),
    // `qcore_c_gateway::legacy_c::enumeration` (unit E01-F1): the definition
    // enumeration could not read the unit's macro expansion, or could not tell
    // which file the expansion belongs to. Both are failures of the analysis
    // environment and say nothing about the analysed source, so they are
    // matched here, ahead of every source-subject rule.
    (
        "enumeration completeness refused: macro expansion",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "line-marked preprocessing does not name its primary input",
        ReasonCategory::EnvironmentUnavailable,
    ),
    // `qcore_c_gateway::typed_c_ast` could not run or read Clang at all.
    (
        "Clang AST unavailable",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "Clang stdin unavailable",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "Clang input writer panicked",
        ReasonCategory::EnvironmentUnavailable,
    ),
    ("Clang input failed", ReasonCategory::EnvironmentUnavailable),
    ("Clang AST failed", ReasonCategory::EnvironmentUnavailable),
    (
        "unsupported Clang AST JSON",
        ReasonCategory::EnvironmentUnavailable,
    ),
    (
        "Clang identity changed during Boolean source import",
        ReasonCategory::EnvironmentUnavailable,
    ),
    // ---- stale evidence: an identity mismatch, never a source defect. ----
    (
        "does not match its pinned bytes",
        ReasonCategory::StaleEvidence,
    ),
    (
        "does not match the pinned revision",
        ReasonCategory::StaleEvidence,
    ),
    ("source revision mismatch", ReasonCategory::StaleEvidence),
    (
        "source/build/profile selection mismatch",
        ReasonCategory::StaleEvidence,
    ),
    (
        "requires evidence unavailable in current-revision resource projection",
        ReasonCategory::StaleEvidence,
    ),
    (
        "historical source/native/provider evidence is not admitted",
        ReasonCategory::StaleEvidence,
    ),
    // ---- provider failure (`qcore_audit::source_contract` Display forms). ----
    (
        "source-contract configuration",
        ReasonCategory::ProviderFailure,
    ),
    ("provider package", ReasonCategory::ProviderFailure),
    ("selected provider", ReasonCategory::ProviderFailure),
    // ---- awaiting human review (card W45-CONTRACT-DEFERRED-REVIEW): a
    // declared contract marked `resource_review_pending` was withheld. Matched
    // before every source-subject rule: the phrase describes a declared
    // premise, never the analysed source. Sites: `qcore_c_gateway::legacy_c`'s
    // held-call statement, `qcore_verification::driver`'s extern-call route.
    ("awaits human review", ReasonCategory::AwaitingHumanReview),
    // ---- contract pin mismatch (card W45-CONTRACT-REVISION-MODE): a pinned
    // contract whose lock the audited source does not match was withheld.
    // Matched before every source-subject rule for the same reason. Site:
    // `qcore_c_gateway::legacy_c`'s held-call statement.
    (
        "pinned contract is not in force",
        ReasonCategory::ContractPinMismatch,
    ),
    // ---- a declared contract its own analysed body refutes (card
    // W45-REFUTED-CONTRACT-WITHDRAWAL, contract spec C06.4): the call was
    // held, and the declaration is evidence about a body other than the one
    // it names, so the category is constraint 8's `stale-evidence`; never a
    // human-resolvable one. Site: `qcore_c_gateway::legacy_c`'s held-call
    // statement.
    (
        "its own analysed body refutes",
        ReasonCategory::StaleEvidence,
    ),
    // ---- a call inside its own caller frame (card W48-RK-LOG-CALLER-FRAME,
    // gateway rule 32a): a caller-frame declaration serves only callers
    // outside the frame, and the frame's own obligations have no derivation,
    // so the held call is this increment's bound, never a statement about the
    // source. Matched before every source-subject rule. Site:
    // `qcore_c_gateway::legacy_c`'s held-call statement.
    (
        "inside its own caller frame",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    // ---- plugin claim uncertified (card W45-USER-CONTRACT-PLUGINS): a
    // checker plugin's `Safe` or `Unsafe` claim with no independently checked
    // certificate or witness. Matched before every source-subject rule: the
    // phrase describes a plugin's assertion, never the analysed source. Site:
    // `qcore_verifier_host::plugin_firewall`.
    (
        "plugin claim uncertified",
        ReasonCategory::PluginClaimUncertified,
    ),
    // ---- outer contract (card OUTER-CONTRACT, rule 31): a runtime-projected
    // row resting on a closed-world fact an outer contract vouches for.
    // Matched before every source-subject rule: the phrase names a contract
    // about loaded code, never the analysed source. Site:
    // `qcore_audit::outer_selection`.
    ("rests on outer contract", ReasonCategory::OuterContract),
    // ---- composition conflict (card AUDIT-COMPOSITION): two components of
    // one row under one provenance key differ. Matched before every
    // source-subject rule: the phrase names the composed record, never the
    // analysed source. Site: `qcore_schemas::composition`'s conflict leaf.
    (
        "composition-conflict: ",
        ReasonCategory::CompositionConflict,
    ),
    // ---- contract conflict (card CONTRACT-SUPERSESSION, contract spec
    // C06.6): the declarations of a called function's bundles are not
    // decided by their pin stamps, so none is in force. Matched before every
    // source-subject rule: the phrase names the contract environment, never
    // the analysed source. Site: `qcore_c_gateway::legacy_c`'s held-call
    // statement.
    (
        "whose contract bundles conflict",
        ReasonCategory::ContractConflict,
    ),
    // ---- missing premise: a declared assumption or trusted contract input
    // the analysis requires was refused or is unusable. The subject is the
    // declared premise, not the analysed source.
    // Sites: `qcore_verification::obligation_derivation`. ----
    (
        "refuses the observations_trusted assumption",
        ReasonCategory::MissingPremise,
    ),
    (
        "has no `extern fn` contract in the",
        ReasonCategory::MissingPremise,
    ),
    (
        "ambiguous observation contract",
        ReasonCategory::MissingPremise,
    ),
    ("does not parse", ReasonCategory::MissingPremise),
    // ---- incomplete obligation coverage: this increment's own bound. ----
    (
        "no core derivation for this capability",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "the evidence-v2.0 order bound",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "source lifecycle effect/ABI import is unavailable",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "per-allocation-site custody derivation runs in the unrestricted-C gateway",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "cannot answer a filename-sensitive construct",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "absent from the selected typed import",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "expected one selected definition after preprocessing",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "exceeds the import depth limit",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "duplicate helper identity",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    (
        "was not found",
        ReasonCategory::IncompleteObligationCoverage,
    ),
    // ---- residual custody: the model ran and left an obligation open. ----
    ("residual leak", ReasonCategory::ResidualCustody),
    ("unresolved custody", ReasonCategory::ResidualCustody),
    // ---- unsupported syntax: a construct in the analysed source that the
    // model does not cover. Deliberately LAST, so no environment, premise,
    // provider or identity row can reach a source-subject category.
    // Sites: `qcore_verification::obligation_derivation`,
    // `qcore_c_gateway::{legacy_c, boolean_source}`. ----
    (
        "contains structured control flow",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "has operations after a return",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "whose call structure could not be recovered",
        ReasonCategory::UnsupportedSyntax,
    ),
    ("has unknown call effect", ReasonCategory::UnsupportedSyntax),
    (
        "has unsupported model-call arity",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "shadows a lifecycle identity",
        ReasonCategory::UnsupportedSyntax,
    ),
    ("is not bound to a name", ReasonCategory::UnsupportedSyntax),
    ("does not name a created", ReasonCategory::UnsupportedSyntax),
    ("does not name an opened", ReasonCategory::UnsupportedSyntax),
    (
        "could not project body for C function",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "production C parser rejected the translation unit",
        ReasonCategory::UnsupportedSyntax,
    ),
    ("source is not UTF-8", ReasonCategory::UnsupportedSyntax),
    (
        "outside single borrowed chunk",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper signature or parameter binding mismatch",
        ReasonCategory::UnsupportedSyntax,
    ),
    ("unmodeled ", ReasonCategory::UnsupportedSyntax),
    // `qcore_c_gateway::legacy_c::enumeration` (unit E01-F1): a definition the
    // enumeration counted whose shape no implemented analysis reads, and the
    // two boundary refusals that name a construct in the analysed token
    // stream. The environment grounds of the same family are matched far
    // earlier, so neither of them can reach these rules.
    (
        "identifier-list (K&R) parameter list",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "a file-scope delimiter has no match",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "is neither a definition nor a declaration this scanner reads",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "outside selected Unix profile",
        ReasonCategory::UnsupportedSyntax,
    ),
    // `qcore_c_gateway::boolean_source` typed-import refusals. Every one names
    // a declaration, record, field, statement or expression in the analysed
    // translation unit that the Boolean field model does not cover. The
    // identity, revision, depth-bound and Clang-tool refusals above are
    // matched earlier, so none of them can reach these rules.
    (
        "Clang AST rejected source",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "expected one ordinary declared record",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "expected one complete chunk record definition",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "expected one direct lifecycle field",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "has unmodeled attributes or fields",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "lifecycle field must be an ordinary non-volatile int",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "field assignment leaves the Boolean lifecycle domain",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "ambiguous Boolean field models for one record ABI",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "unsupported Boolean field operation mapping",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "unsupported Boolean field result ABI",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "function has no modeled chunk field access",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "source path falls through without an explicit return",
        ReasonCategory::UnsupportedSyntax,
    ),
    // The helper-import refusals, enumerated rather than caught by a
    // "helper" prefix rule: a prefix catch-all onto a source-subject category
    // is the exact hazard this table is ordered to avoid.
    (
        "helper record has no selected declared Boolean model",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper record is not a selected Boolean model",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper record differs from caller model record",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper requires one declared record pointer and void result",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper requires direct constant field assignments",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper requires a direct assignment",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper requires direct borrowed parameter",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper field is not bound to its borrowed parameter",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper assignment is not a constant",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper has no checked field writes",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper writes a declared Boolean model field",
        ReasonCategory::UnsupportedSyntax,
    ),
    (
        "helper declaration, structural layout, or uniqueness mismatch",
        ReasonCategory::UnsupportedSyntax,
    ),
];

impl fmt::Display for ReasonCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// No rule in the documented mapping recognised the reason text.
///
/// This is a refusal, not a category. Callers must not substitute a category:
/// the closed vocabulary has no "unclassified" member, and defaulting one in
/// would be exactly the silent relabeling §L21.1 rule 1 forbids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnclassifiedReason;

impl fmt::Display for UnclassifiedReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "reason text matches no rule of the closed L21.1 vocabulary; \
             classification is refused rather than defaulted",
        )
    }
}

impl std::error::Error for UnclassifiedReason {}

impl FromStr for ReasonCategory {
    type Err = UnknownReasonCategory;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|category| category.as_str() == text)
            .ok_or(UnknownReasonCategory)
    }
}

/// The text is not one of the closed category spellings (`ReasonCategory::ALL`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnknownReasonCategory;

impl fmt::Display for UnknownReasonCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("not a category of the closed L21.1 reason vocabulary")
    }
}

impl std::error::Error for UnknownReasonCategory {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_category_is_the_sole_environment_subject() {
        let environment: Vec<_> = ReasonCategory::ALL
            .into_iter()
            .filter(|category| category.subject() == ReasonSubject::Environment)
            .collect();
        assert_eq!(environment, vec![ReasonCategory::EnvironmentUnavailable]);
    }

    #[test]
    fn no_environment_row_can_describe_the_analysed_source() {
        for category in ReasonCategory::ALL {
            assert!(
                !(category.is_environment() && category.describes_analysed_source()),
                "{category} would be readable as a source defect"
            );
        }
        assert!(!ReasonCategory::EnvironmentUnavailable.describes_analysed_source());
        assert!(!ReasonCategory::EnvironmentUnavailable.may_support_unsafe());
    }

    #[test]
    fn spellings_round_trip_and_are_distinct() {
        let mut seen = std::collections::BTreeSet::new();
        for category in ReasonCategory::ALL {
            assert!(seen.insert(category.as_str()), "duplicate spelling");
            assert_eq!(category.as_str().parse::<ReasonCategory>(), Ok(category));
            let json = serde_json::to_string(&category).unwrap();
            assert_eq!(json, format!("\"{}\"", category.as_str()));
            assert_eq!(
                serde_json::from_str::<ReasonCategory>(&json).unwrap(),
                category
            );
        }
        assert_eq!(seen.len(), ReasonCategory::ALL.len());
        assert!("source-defect".parse::<ReasonCategory>().is_err());
        assert!(serde_json::from_str::<ReasonCategory>("\"unclassified\"").is_err());
    }

    #[test]
    fn classification_refuses_rather_than_defaulting() {
        assert_eq!(
            ReasonCategory::classify("a reason nobody wrote"),
            Err(UnclassifiedReason)
        );
        assert_eq!(ReasonCategory::classify(""), Err(UnclassifiedReason));
    }

    #[test]
    fn an_environment_reason_never_classifies_as_a_source_statement() {
        for reason in [
            "audit requires the configured Clang executable /usr/bin/clang; \
             compilation database selects /usr/bin/cc",
            "production C parser is unavailable: QCORE_CLANG is empty",
            "source outside selected checkout",
        ] {
            let category = ReasonCategory::classify(reason).expect("environment rows classify");
            assert_eq!(category, ReasonCategory::EnvironmentUnavailable);
            assert!(!category.describes_analysed_source());
        }
    }

    #[test]
    fn every_category_has_exactly_one_subject_and_the_partition_is_covered() {
        let mut subjects = std::collections::BTreeSet::new();
        for category in ReasonCategory::ALL {
            subjects.insert(category.subject());
            // `subject` is a total function on a closed enum, so "exactly one"
            // holds by construction; assert the derived predicates agree.
            assert_eq!(
                category.describes_analysed_source(),
                category.subject() == ReasonSubject::AnalysedSource
            );
            assert_eq!(
                category.is_environment(),
                category.subject() == ReasonSubject::Environment
            );
            assert_eq!(
                category.may_support_unsafe(),
                category.describes_analysed_source()
            );
        }
        assert_eq!(
            subjects,
            [
                ReasonSubject::AnalysedSource,
                ReasonSubject::Analyser,
                ReasonSubject::Environment
            ]
            .into_iter()
            .collect()
        );
    }
}
