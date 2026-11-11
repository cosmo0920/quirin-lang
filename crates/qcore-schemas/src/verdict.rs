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

//! The four-value verdict and its composition (card
//! **W45-USER-CONTRACT-PLUGINS**; language specification §L21.2).
//!
//! # The values
//!
//! [`Verdict`] is `Safe < Pending < Unknown < Unsafe`:
//!
//! * `Safe` — every obligation of the scoped claim discharged;
//! * `Pending` — not decided by the tool, **awaiting a named human act**: a
//!   review edit, a re-pin after review, or a checkable certificate for a
//!   plugin's claim ([`PendingReason`]);
//! * `Unknown` — not decidable by the tool as configured;
//! * `Unsafe` — a validated reachable violation, provisional under the
//!   existing policy and never presented as an upstream defect.
//!
//! `Pending` is fail-closed: it is not `Safe`, it stays in every denominator
//! exactly as `Unknown` does, and no composition can lower it to `Safe`.
//!
//! # The semigroup
//!
//! Composition along a chain, a group or a unit is the **join** (the maximum
//! in the order above): associative, commutative and idempotent, with `Safe`
//! as its identity, so it is a commutative idempotent monoid. A
//! [`ReasonedVerdict`] carries a finite multiset of [`VerdictReason`]s beside
//! its value; composition joins the values and unions the multisets, which is
//! again a commutative monoid (no longer idempotent on reasons, by design:
//! two links that fail for the same reason are two reasons). The value
//! component is what every roll-up in this workspace computes; the reasons
//! keep *why*.
//!
//! # What this module does not do
//!
//! It decides nothing about any program. Which verdict a diagnostic, a held
//! call or a plugin's answer denotes is decided by the trusted core
//! (`qcore_c_gateway::legacy_c::verdict`, `qcore_verifier_host::plugin_firewall`);
//! this is the shared algebra those producers and every consumer fold with.
//! The Lean model is `proof/QCore/VerdictSemigroup.lean`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::reason::ReasonCategory;

/// The four-value verdict, ordered `Safe < Pending < Unknown < Unsafe`.
///
/// The derived order is the declaration order, and it is the order the join
/// uses. The wire spelling is the report spelling (`"Safe"`, `"Pending"`,
/// `"Unknown"`, `"Unsafe"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Verdict {
    Safe,
    Pending,
    Unknown,
    Unsafe,
}

impl Verdict {
    /// Every value, in order.
    pub const ALL: [Self; 4] = [Self::Safe, Self::Pending, Self::Unknown, Self::Unsafe];

    /// The report spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Pending => "Pending",
            Self::Unknown => "Unknown",
            Self::Unsafe => "Unsafe",
        }
    }

    /// The composition: the larger value in `Safe < Pending < Unknown <
    /// Unsafe`.
    #[must_use]
    pub fn join(self, other: Self) -> Self {
        self.max(other)
    }

    /// The fold of any number of verdicts; `Safe`, the identity, for none.
    #[must_use]
    pub fn fold(verdicts: impl IntoIterator<Item = Self>) -> Self {
        verdicts.into_iter().fold(Self::Safe, Self::join)
    }

    /// `true` only for `Safe`. `Pending` is not: it stays in the denominator.
    #[must_use]
    pub const fn is_safe(self) -> bool {
        matches!(self, Self::Safe)
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for Verdict {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|verdict| verdict.as_str() == text)
            .ok_or_else(|| format!("`{text}` is not a verdict (Safe, Pending, Unknown, Unsafe)"))
    }
}

/// The named, human-resolvable reasons that alone may produce `Pending`.
///
/// Each names the act that resolves it. There is deliberately no other way to
/// construct a `Pending` [`ReasonedVerdict`] ([`ReasonedVerdict::pending`]
/// takes one of these), so `Pending` is never a free-form label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PendingReason {
    /// A declared contract the row depends on is marked
    /// `resource_review_pending` (contract spec §C06.1). Resolved by a person
    /// editing the text.
    AwaitingHumanReview,
    /// A pinned contract the row depends on is not in force because the
    /// audited source does not match its lock (contract spec §C06.2).
    /// Resolved by a person reviewing the changed source and re-pinning.
    ContractPinMismatch,
    /// A checker plugin claimed `Safe` or `Unsafe` without a certificate or
    /// witness the audit checks independently (contract spec §C11.1).
    /// Resolved by a person supplying a checkable certificate or witness.
    PluginClaimUncertified,
    /// Card OUTER-CONTRACT (gateway rule 31): in the runtime projection, a
    /// closed-world fact the row rests on is vouched for by an outer contract
    /// (contract spec §C04.20), named `NAME@DIGEST` in the reason. Resolved by
    /// a person reviewing the loaded code against the contract.
    OuterContract,
}

impl PendingReason {
    /// Every reason, in order.
    pub const ALL: [Self; 4] = [
        Self::AwaitingHumanReview,
        Self::ContractPinMismatch,
        Self::PluginClaimUncertified,
        Self::OuterContract,
    ];

    /// The closed §L21.1 category this reason is reported under.
    #[must_use]
    pub const fn category(self) -> ReasonCategory {
        match self {
            Self::AwaitingHumanReview => ReasonCategory::AwaitingHumanReview,
            Self::ContractPinMismatch => ReasonCategory::ContractPinMismatch,
            Self::PluginClaimUncertified => ReasonCategory::PluginClaimUncertified,
            Self::OuterContract => ReasonCategory::OuterContract,
        }
    }

    /// The pending reason a category names, if it is a human-resolvable one.
    #[must_use]
    pub fn from_category(category: ReasonCategory) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|reason| reason.category() == category)
    }
}

/// One reason a verdict is what it is: a code and its free text.
///
/// For a `Pending` or `Unknown` contribution the code is a §L21.1 category
/// spelling; for an `Unsafe` contribution it is the violation's diagnostic
/// category. Ordered so a multiset of reasons has one canonical order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictReason {
    pub code: String,
    pub detail: String,
}

/// A verdict with the multiset of reasons that produced it.
///
/// Constructed only through the named constructors, so a `Pending` always
/// carries a [`PendingReason`] and an `Unsafe` always names its violation.
/// `reasons` is kept sorted, which makes equal multisets equal values.
///
/// Serialize-only on purpose: a deserializer would be a second, unchecked
/// constructor.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ReasonedVerdict {
    verdict: Verdict,
    reasons: Vec<VerdictReason>,
}

impl Default for ReasonedVerdict {
    fn default() -> Self {
        Self::safe()
    }
}

impl ReasonedVerdict {
    /// The identity: `Safe` with no reason.
    #[must_use]
    pub const fn safe() -> Self {
        Self {
            verdict: Verdict::Safe,
            reasons: Vec::new(),
        }
    }

    /// `Pending`, for exactly one named human-resolvable reason.
    #[must_use]
    pub fn pending(reason: PendingReason, detail: impl Into<String>) -> Self {
        Self::single(Verdict::Pending, reason.category().as_str(), detail)
    }

    /// `Unknown`, under a closed §L21.1 category. A human-resolvable category
    /// is refused here: a row withheld for one of those alone is `Pending`
    /// and says so through [`Self::pending`].
    ///
    /// # Errors
    ///
    /// Returns the category back when it is a human-resolvable one.
    pub fn unknown(
        category: ReasonCategory,
        detail: impl Into<String>,
    ) -> Result<Self, ReasonCategory> {
        if category.is_human_resolvable() {
            return Err(category);
        }
        Ok(Self::single(Verdict::Unknown, category.as_str(), detail))
    }

    /// `Unsafe`, naming the violation's category. The caller is the trusted
    /// producer that validated the witness; this constructor records, it does
    /// not validate.
    #[must_use]
    pub fn violation(category: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            verdict: Verdict::Unsafe,
            reasons: vec![VerdictReason {
                code: category.into(),
                detail: detail.into(),
            }],
        }
    }

    fn single(verdict: Verdict, code: &str, detail: impl Into<String>) -> Self {
        Self {
            verdict,
            reasons: vec![VerdictReason {
                code: code.to_owned(),
                detail: detail.into(),
            }],
        }
    }

    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The reasons, in canonical order, with multiplicity.
    #[must_use]
    pub fn reasons(&self) -> &[VerdictReason] {
        &self.reasons
    }

    /// The composition: join of the values, union of the reason multisets.
    #[must_use]
    pub fn join(mut self, other: Self) -> Self {
        self.verdict = self.verdict.join(other.verdict);
        self.reasons.extend(other.reasons);
        self.reasons.sort();
        self
    }

    /// The fold of any number of reasoned verdicts; [`Self::safe`] for none.
    #[must_use]
    pub fn fold(verdicts: impl IntoIterator<Item = Self>) -> Self {
        verdicts.into_iter().fold(Self::safe(), Self::join)
    }
}

/// A verdict with the **named assumptions** it holds under (card
/// `W48-QR-C-STRUCT-IMPORT`; §L20's assumptions field, §L21.2's join).
///
/// Composition joins the values and **unions** the assumptions, so an
/// assumption any part rests on is carried by every composed verdict: the
/// join never drops one. A *plain* `Safe` is `Safe` with no assumption, and
/// only a result with no named assumption can be one — so a result from a
/// file that imports C (`std.ffi.unsafe`) is at best "Safe under the
/// unsafe-FFI assumption", never plain `Safe`. The Lean model is
/// `proof/QCore/FfiStructImport.lean`.
///
/// Serialize-only, like [`ReasonedVerdict`]: a deserializer would be a
/// second, unchecked constructor.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct AssumedVerdict {
    verdict: Verdict,
    assumptions: std::collections::BTreeSet<String>,
}

impl AssumedVerdict {
    /// `verdict`, holding under `assumptions`.
    #[must_use]
    pub fn new(verdict: Verdict, assumptions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            verdict,
            assumptions: assumptions.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        self.verdict
    }

    /// The named assumptions, in canonical order.
    #[must_use]
    pub fn assumptions(&self) -> &std::collections::BTreeSet<String> {
        &self.assumptions
    }

    /// Whether this is `Safe` with no assumption at all.
    #[must_use]
    pub fn is_plain_safe(&self) -> bool {
        self.verdict.is_safe() && self.assumptions.is_empty()
    }

    /// Whether the named assumption is carried.
    #[must_use]
    pub fn holds_under(&self, assumption: &str) -> bool {
        self.assumptions.contains(assumption)
    }

    /// The composition: join of the values, union of the assumptions.
    #[must_use]
    pub fn join(mut self, other: Self) -> Self {
        self.verdict = self.verdict.join(other.verdict);
        self.assumptions.extend(other.assumptions);
        self
    }

    /// How a report says it: `Safe`, or `Safe under the unsafe-ffi
    /// assumption`, the assumptions named in order.
    #[must_use]
    pub fn render(&self) -> String {
        if self.assumptions.is_empty() {
            return self.verdict.as_str().to_owned();
        }
        let names = self.assumptions.iter().cloned().collect::<Vec<_>>();
        format!(
            "{} under the {} assumption{}",
            self.verdict,
            names.join(", "),
            if names.len() == 1 { "" } else { "s" }
        )
    }
}

/// What a chain composes (§L21.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChainKind {
    /// A function and the contracts of the callees it applies: the function's
    /// own analysis first, then each applied contract's attached checks.
    CalleeChain,
    /// The obligations one claim requires, in derivation order.
    ObligationPipeline,
    /// A group of units or functions reported under one verdict (a unit's
    /// functions; a checkpoint's units).
    UnitGroup,
}

/// One link of a chain: what it is and the verdict it contributes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ChainLink {
    pub label: String,
    pub verdict: ReasonedVerdict,
}

/// An ordered composition of contract steps. Its verdict is the fold of its
/// links' verdicts; the order is kept for the report, and the fold does not
/// depend on it (the join is commutative).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Chain {
    pub kind: ChainKind,
    pub links: Vec<ChainLink>,
}

impl Chain {
    #[must_use]
    pub const fn new(kind: ChainKind) -> Self {
        Self {
            kind,
            links: Vec::new(),
        }
    }

    /// Append one link.
    pub fn push(&mut self, label: impl Into<String>, verdict: ReasonedVerdict) {
        self.links.push(ChainLink {
            label: label.into(),
            verdict,
        });
    }

    /// The chain's verdict: the semigroup fold of its links.
    #[must_use]
    pub fn verdict(&self) -> ReasonedVerdict {
        ReasonedVerdict::fold(self.links.iter().map(|link| link.verdict.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_order_is_safe_pending_unknown_unsafe() {
        assert!(Verdict::Safe < Verdict::Pending);
        assert!(Verdict::Pending < Verdict::Unknown);
        assert!(Verdict::Unknown < Verdict::Unsafe);
        let mut sorted = Verdict::ALL;
        sorted.sort();
        assert_eq!(sorted, Verdict::ALL);
    }

    #[test]
    fn the_join_is_a_commutative_idempotent_monoid_with_safe_as_identity() {
        for a in Verdict::ALL {
            assert_eq!(a.join(Verdict::Safe), a);
            assert_eq!(Verdict::Safe.join(a), a);
            assert_eq!(a.join(a), a);
            for b in Verdict::ALL {
                assert_eq!(a.join(b), b.join(a));
                assert!(a.join(b) >= a);
                for c in Verdict::ALL {
                    assert_eq!(a.join(b).join(c), a.join(b.join(c)));
                }
            }
        }
        assert_eq!(Verdict::fold([]), Verdict::Safe);
    }

    #[test]
    fn an_assumption_survives_every_join_and_forbids_plain_safe() {
        let ffi = AssumedVerdict::new(Verdict::Safe, ["unsafe-ffi"]);
        assert!(!ffi.is_plain_safe());
        assert_eq!(ffi.render(), "Safe under the unsafe-ffi assumption");
        for verdict in Verdict::ALL {
            let other = AssumedVerdict::new(verdict, std::iter::empty::<String>());
            let joined = ffi.clone().join(other.clone());
            assert!(joined.holds_under("unsafe-ffi"));
            assert!(!joined.is_plain_safe());
            assert_eq!(joined.verdict(), verdict);
            assert_eq!(joined, other.join(ffi.clone()));
        }
        let plain = AssumedVerdict::new(Verdict::Safe, std::iter::empty::<String>());
        assert!(plain.is_plain_safe());
        assert_eq!(plain.render(), "Safe");
    }

    #[test]
    fn pending_is_never_folded_into_safe() {
        for other in Verdict::ALL {
            assert_ne!(Verdict::Pending.join(other), Verdict::Safe);
            assert!(!Verdict::Pending.join(other).is_safe());
        }
    }

    #[test]
    fn reasons_are_a_multiset_and_composition_keeps_them() {
        let review = ReasonedVerdict::pending(PendingReason::AwaitingHumanReview, "k_close");
        let unknown =
            ReasonedVerdict::unknown(ReasonCategory::UnsupportedSyntax, "a goto").unwrap();
        let joined = review.clone().join(unknown.clone());
        assert_eq!(joined.verdict(), Verdict::Unknown);
        assert_eq!(joined.reasons().len(), 2);
        assert_eq!(joined, unknown.clone().join(review.clone()));
        assert_eq!(review.clone().join(review.clone()).reasons().len(), 2);
        assert_eq!(ReasonedVerdict::safe().join(review.clone()), review);
        assert_eq!(ReasonedVerdict::fold([]), ReasonedVerdict::safe());
    }

    #[test]
    fn pending_needs_a_named_reason_and_unknown_refuses_one() {
        for reason in PendingReason::ALL {
            let verdict = ReasonedVerdict::pending(reason, "x");
            assert_eq!(verdict.verdict(), Verdict::Pending);
            assert_eq!(verdict.reasons()[0].code, reason.category().as_str());
            assert_eq!(
                PendingReason::from_category(reason.category()),
                Some(reason)
            );
            assert!(reason.category().is_human_resolvable());
            assert!(!reason.category().may_support_unsafe());
            assert_eq!(
                ReasonedVerdict::unknown(reason.category(), "x"),
                Err(reason.category())
            );
        }
        let resolvable = ReasonCategory::ALL
            .into_iter()
            .filter(|category| category.is_human_resolvable())
            .count();
        assert_eq!(resolvable, PendingReason::ALL.len());
    }

    #[test]
    fn a_chain_is_the_fold_of_its_links() {
        let mut chain = Chain::new(ChainKind::CalleeChain);
        assert_eq!(chain.verdict().verdict(), Verdict::Safe);
        chain.push("body", ReasonedVerdict::safe());
        chain.push(
            "pool_get",
            ReasonedVerdict::pending(PendingReason::PluginClaimUncertified, "no certificate"),
        );
        assert_eq!(chain.verdict().verdict(), Verdict::Pending);
        chain.push("free", ReasonedVerdict::violation("leak", "line 4"));
        assert_eq!(chain.verdict().verdict(), Verdict::Unsafe);
        assert_eq!(chain.verdict().reasons().len(), 2);
    }

    #[test]
    fn the_wire_spelling_is_the_report_spelling() {
        for verdict in Verdict::ALL {
            assert_eq!(
                serde_json::to_string(&verdict).unwrap(),
                format!("\"{verdict}\"")
            );
            assert_eq!(verdict.as_str().parse::<Verdict>(), Ok(verdict));
        }
        assert!("pending".parse::<Verdict>().is_err());
    }
}
