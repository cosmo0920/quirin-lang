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

/// The closed seven-category vocabulary of §L21.1.
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
}

impl ReasonCategory {
    /// Every category, in declaration order. The vocabulary is closed: adding a
    /// member is a compatibility event, and the totality tests enumerate this
    /// array rather than a hand-written list.
    pub const ALL: [Self; 7] = [
        Self::UnsupportedSyntax,
        Self::MissingPremise,
        Self::IncompleteObligationCoverage,
        Self::StaleEvidence,
        Self::ProviderFailure,
        Self::ResidualCustody,
        Self::EnvironmentUnavailable,
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
            | Self::ProviderFailure => ReasonSubject::Analyser,
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
            return match unsupported {
                // No unmodeled operation: the model ran and left custody open.
                Some(0) => Ok(Self::ResidualCustody),
                // At least one construct the model does not cover.
                Some(_) => Ok(Self::UnsupportedSyntax),
                None => Err(UnclassifiedReason),
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

/// The text is not one of the seven closed category spellings.
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
