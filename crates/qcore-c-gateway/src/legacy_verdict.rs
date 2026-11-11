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

//! The four-value verdict of the legacy-C gateway (card
//! **W45-USER-CONTRACT-PLUGINS**; language specification §L21.2).
//!
//! # What changed, and what did not
//!
//! Before this card a function's status was a three-way decision over its
//! diagnostics — `Unsafe` if any violation, else `Unknown` if any unresolved
//! construct, else `Safe` — and a unit's status the same decision over its
//! functions and projection failures. That decision **is** the join of
//! `Safe < Unknown < Unsafe` over per-diagnostic verdicts, so this module
//! computes it as a fold ([`classify`], [`unit_status`]) over the order
//! extended by `Pending`.
//!
//! The only diagnostic whose verdict moves is a **held call**: a call to a
//! callee whose declared contract is withheld for a human act — awaiting
//! review (C06.1) or a contract pin mismatch (C06.2). Its refusal was
//! `Unknown`; it is now `Pending`. Joined with anything else it stays at least
//! `Pending`, and one other unresolved construct in the same function makes
//! the function `Unknown` exactly as before. No diagnostic moves towards
//! `Safe`, and no diagnostic moves towards or away from `Unsafe`.
//!
//! [`diagnostic_verdict`] matches every diagnostic kind by name, so a new kind
//! cannot be added without deciding its verdict.

use qcore_schemas::verdict::Verdict;

use super::{
    AnalysisStatus, LegacyDiagnostic, LegacyDiagnosticKind, LegacyFunctionSummary,
    UnrestrictedLegacyResult, pin_hold, review_hold,
};

impl AnalysisStatus {
    /// The four-value verdict this status denotes.
    #[must_use]
    pub const fn verdict(self) -> Verdict {
        match self {
            Self::Safe => Verdict::Safe,
            Self::Pending => Verdict::Pending,
            Self::Unknown => Verdict::Unknown,
            Self::Unsafe => Verdict::Unsafe,
        }
    }

    /// The status denoting `verdict`.
    #[must_use]
    pub const fn from_verdict(verdict: Verdict) -> Self {
        match verdict {
            Verdict::Safe => Self::Safe,
            Verdict::Pending => Self::Pending,
            Verdict::Unknown => Self::Unknown,
            Verdict::Unsafe => Self::Unsafe,
        }
    }
}

/// `true` when `description` is a held call's refusal: a call whose declared
/// contract is withheld awaiting review or for a pin mismatch.
#[must_use]
pub fn is_held_call(description: &str) -> bool {
    review_hold::held_call(description).is_some()
        || pin_hold::held_call(description).is_some()
        // Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g): a call a
        // declared, unestablished callee frame holds for review.
        || super::cell_frames_route::frame_held_call(description).is_some()
}

/// The verdict one diagnostic contributes to its function.
#[must_use]
pub fn diagnostic_verdict(kind: &LegacyDiagnosticKind) -> Verdict {
    match kind {
        LegacyDiagnosticKind::Leak { .. }
        | LegacyDiagnosticKind::DoubleRelease { .. }
        | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
        | LegacyDiagnosticKind::CustodyConflict { .. }
        | LegacyDiagnosticKind::InvalidRelease { .. }
        | LegacyDiagnosticKind::UseAfterRelease { .. }
        | LegacyDiagnosticKind::KindMismatchedRelease { .. }
        | LegacyDiagnosticKind::UseAfterClose { .. }
        | LegacyDiagnosticKind::CountedUser { .. } => Verdict::Unsafe,
        LegacyDiagnosticKind::Unsupported { description } if is_held_call(description) => {
            Verdict::Pending
        }
        LegacyDiagnosticKind::UnknownAlias { .. }
        | LegacyDiagnosticKind::Unsupported { .. }
        | LegacyDiagnosticKind::SummaryConflict { .. } => Verdict::Unknown,
    }
}

/// A function's status: the fold of its diagnostics' verdicts.
pub fn classify(diagnostics: &[LegacyDiagnostic]) -> AnalysisStatus {
    AnalysisStatus::from_verdict(Verdict::fold(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic_verdict(&diagnostic.kind)),
    ))
}

/// A unit's status: the fold of its functions' statuses, joined with
/// `Unknown` when some selected function could not be projected.
pub fn unit_status<'a>(
    functions: impl IntoIterator<Item = &'a LegacyFunctionSummary>,
    projection_failed: bool,
) -> AnalysisStatus {
    let failures = projection_failed.then_some(Verdict::Unknown);
    AnalysisStatus::from_verdict(Verdict::fold(
        functions
            .into_iter()
            .map(|function| function.status.verdict())
            .chain(failures),
    ))
}

impl UnrestrictedLegacyResult {
    /// The unit's four-value verdict.
    ///
    /// `Safe` and `Unsafe` are their variants. The `Unknown` variant carries
    /// every other result, and it is `Pending` exactly when its partial
    /// summary's own status is `Pending`: the analysis ran, every function was
    /// projected, and nothing but held calls stood between the unit and a
    /// verdict. A result the audit withheld from `Safe` (an unestablished
    /// definition set, a refused certificate binding) keeps a `Safe` partial
    /// status and so stays `Unknown`, as does every result without a summary.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        match self {
            Self::Safe(_) => Verdict::Safe,
            Self::Unsafe(_) => Verdict::Unsafe,
            Self::Unknown {
                partial: Some(summary),
                ..
            } if summary.status == AnalysisStatus::Pending => Verdict::Pending,
            Self::Unknown { .. } => Verdict::Unknown,
        }
    }
}
