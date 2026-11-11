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

//! The null-candidate triage's pure decisions (card **NULL-CANDIDATE-TRIAGE**,
//! contract specification C09.3, gateway rules 14r–14w).
//!
//! The null property ([`crate::null_deref`]) leaves a dereference whose
//! pointer may be null as a *candidate* (`Unknown`). This module decides, on
//! top of it and without changing any of its decisions:
//!
//! * a **path decision** ([`admit`]): whether a witness path may take one arm
//!   of a branch. The arm is taken when the consumer decides the condition
//!   (a constant, a NULL test it reads, a repetition of a condition already
//!   decided on the path over unchanged values) and agrees, or when the
//!   condition reads only values no earlier decision on the path constrained
//!   (each such value may be chosen freely, so both arms are feasible). A
//!   condition over a value an earlier decision constrained is *correlated*:
//!   the path is unresolved, never taken;
//! * a **finding**'s class ([`FindingClass`]) and the row verdict with its
//!   findings ([`triaged_verdict`]): a validated path makes the row `Unsafe`,
//!   provisional, never an upstream defect; triage never makes a row `Safe`;
//! * the **standing of a declared non-null member** ([`member_standing`]):
//!   withdrawn when a store some unit made is not proved non-null; never
//!   `validated`, since zero-initialised construction is a listed premise;
//! * the **standing of a declared non-null result** ([`result_standing`]),
//!   rule 13g's fold over the units that examined a body;
//! * the **demotion** of a row that relied on a withdrawn declaration
//!   ([`demote`]) and the verdict after cross-unit travel adds obligations
//!   ([`with_travelled_checks`]): neither improves a verdict.
//!
//! The Lean model is `QCore.NullTriage`, bound by shared vectors
//! (`crates/qcore-core/tests/null_triage.rs`).

use crate::null_deref::{NullVerdict, Outcome, null_verdict};

/// What the consumer can say of a branch condition on one witness path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// The condition's value is decided (a constant, a NULL test of a pointer
    /// whose null-ness the path fixed, a repetition over unchanged values).
    Decided(bool),
    /// Every value the condition reads is unconstrained on the path.
    Free,
    /// The condition reads a value an earlier decision constrained.
    Correlated,
}

/// Whether a witness path may take one arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    Taken,
    /// The arm contradicts a decided condition: no execution takes it.
    Infeasible,
    /// The consumer cannot tell: the path is not a witness.
    Unresolved,
}

impl Admission {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Taken => "taken",
            Self::Infeasible => "infeasible",
            Self::Unresolved => "unresolved",
        }
    }
}

/// The path decision for taking `arm` of a branch whose condition is
/// `condition`.
#[must_use]
pub fn admit(condition: Condition, arm: bool) -> Admission {
    match condition {
        Condition::Decided(value) if value == arm => Admission::Taken,
        Condition::Decided(_) => Admission::Infeasible,
        Condition::Free => Admission::Taken,
        Condition::Correlated => Admission::Unresolved,
    }
}

/// The class of a validated finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FindingClass {
    /// The only NULL condition on the path is an allocation that fails.
    AllocationFailure,
    /// NULL on a path that needs no allocation failure (a null constant, a
    /// lookup that misses, the null arm of the function's own test).
    NormalPath,
}

impl FindingClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AllocationFailure => "null-on-allocation-failure",
            Self::NormalPath => "null-on-normal-path",
        }
    }
}

/// The reason a finding contributes to its row.
pub const FINDING: &str = "null-deref-finding";

/// The triage status of one strong-source candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Triage {
    /// A validated path: a finding of this class.
    Finding(FindingClass),
    /// Every path explored, none reaches the dereference with the pointer
    /// NULL under a named condition.
    Infeasible,
    /// A correlated condition, an unread form or the search budget.
    Unresolved,
    /// A path reaches the dereference with the pointer uninitialised: an
    /// indeterminate value, not a NULL dereference.
    Indeterminate,
}

impl Triage {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Finding(_) => "finding",
            Self::Infeasible => "infeasible",
            Self::Unresolved => "unresolved",
            Self::Indeterminate => "indeterminate",
        }
    }
}

/// The row's verdict with its findings: the null verdict of its checks, made
/// `Unsafe` (provisional) when a candidate's path was validated. A row with
/// no finding keeps its verdict, so triage never makes a row `Safe`.
#[must_use]
pub fn triaged_verdict(base: NullVerdict, findings: usize) -> NullVerdict {
    if findings > 0 {
        base.join(NullVerdict::Unsafe)
    } else {
        base
    }
}

/// The standing of one declared non-null member over a run: `withdrawn` when
/// a store some unit made was not proved non-null; `declared-assumption`
/// when no analysed unit stores it; otherwise `partly-validated` (the
/// `stores` clause covered; the `construction` premise never is).
#[must_use]
pub fn member_standing(examined: usize, failed: usize) -> &'static str {
    if failed > 0 {
        "withdrawn"
    } else if examined == 0 {
        "declared-assumption"
    } else {
        "partly-validated"
    }
}

/// One unit's evidence about a declared non-null result's body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultEvidence {
    /// Every return of the body is non-null.
    Validated,
    /// Some return is not proved non-null (and none is null).
    Unvalidated,
    /// Some return is null on a path: the declaration is refuted.
    Refuted,
}

/// Rule 13g's fold for a declared non-null result: `withdrawn` when a unit
/// refutes it; `validated` when every unit that examined a body validated it
/// and at least one did; otherwise a `declared-assumption`.
#[must_use]
pub fn result_standing(units: &[ResultEvidence]) -> &'static str {
    if units.contains(&ResultEvidence::Refuted) {
        "withdrawn"
    } else if !units.is_empty() && units.iter().all(|u| *u == ResultEvidence::Validated) {
        "validated"
    } else {
        "declared-assumption"
    }
}

/// The reason a withdrawn declaration contributes to a row that relied on it.
pub const WITHDRAWN: &str = "null-run-withdrawn-declaration";

/// A row that relied on a withdrawn declaration is never `Safe` or `Pending`:
/// it becomes `Unknown`. Any other verdict stands.
#[must_use]
pub fn demote(verdict: NullVerdict, relied_on_withdrawn: bool) -> NullVerdict {
    if relied_on_withdrawn {
        verdict.join(NullVerdict::Unknown)
    } else {
        verdict
    }
}

/// The verdict after cross-unit travel adds `added` caller checks to a row
/// whose verdict was `base`: the join with the verdict of the added checks
/// alone, so travel never improves a row.
#[must_use]
pub fn with_travelled_checks(base: NullVerdict, added: &[Outcome]) -> NullVerdict {
    base.join(null_verdict(added, &[]).0)
}
