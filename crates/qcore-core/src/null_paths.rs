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

//! Card **NULL-TRIAGE-UNRESOLVED-PATHS** (gateway rules 15q–15u): the pure
//! decisions of the null path search's exact and inexact readings.
//!
//! The path search ([`crate::null_triage`], rule 14r) looks for one path to a
//! dereference with the pointer NULL. This module adds, without changing a
//! decision [`crate::null_triage::admit`] makes:
//!
//! * a **reading** of each branch condition ([`Reading`]): decided (the
//!   consumer knows its value on the path), **exact** (it reads only
//!   unconstrained entry values, so either arm has a real execution), or
//!   **inexact** (anything else: a call's result, a correlated value, a value
//!   the search does not follow). An inexact condition may still be explored
//!   on both arms, which over-approximates the executions, but a path that
//!   took an inexact arm is never a witness ([`explore`]);
//! * the **conclusion** of a search ([`conclude`]): a finding only for an
//!   exact path, `infeasible` only when every path was explored and none
//!   reaches the dereference NULL, otherwise unresolved;
//! * **difference and sum arms** ([`relation_arms`]): the arms of `x OP y`
//!   (`x - y` or `x + y` against a constant) for two independent values over
//!   hole-free intervals: an arm is feasible exactly when the difference (or
//!   sum) interval meets the arm's set;
//! * the **counter range** of a body ([`counter_lower_bound`]): a value that
//!   starts at constants and only grows by non-negative constants never goes
//!   below its least start (a defined execution has no signed overflow).
//!
//! The Lean model is `QCore.NullTriagePaths`, bound by shared vectors
//! (`crates/qcore-core/tests/null_paths.rs`).

use crate::null_triage::Triage;

/// The consumer's reading of one branch condition on one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// The condition's value is decided on the path.
    Decided(bool),
    /// Every value it reads is an unconstrained entry value (a formal, a
    /// memory value read since the last call or store), so both arms have an
    /// execution.
    Exact,
    /// A call's result without an established achievable range, a
    /// correlated value, a value the search does not follow: the arm may be
    /// explored, but it is no evidence that an execution takes it.
    Inexact,
}

/// What one exploration step does with an arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Taken; the path stays exact.
    Taken,
    /// No execution takes it.
    Infeasible,
    /// Taken for exploration only; the path is no longer a witness.
    TakenInexact,
}

impl Step {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Taken => "taken",
            Self::Infeasible => "infeasible",
            Self::TakenInexact => "taken-inexact",
        }
    }
}

/// The exploration step for `arm` of a condition read as `reading`.
#[must_use]
pub fn explore(reading: Reading, arm: bool) -> Step {
    match reading {
        Reading::Decided(value) if value == arm => Step::Taken,
        Reading::Decided(_) => Step::Infeasible,
        Reading::Exact => Step::Taken,
        Reading::Inexact => Step::TakenInexact,
    }
}

/// What the exploration of one candidate saw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Explored {
    /// An exact path reached the dereference with the pointer NULL under a
    /// named condition.
    pub exact_reach: bool,
    /// An inexact path did.
    pub inexact_reach: bool,
    /// Every path was explored to its end (no budget, loop bound, unread
    /// form or label the search cannot enter cut one).
    pub complete: bool,
}

/// The search's conclusion. (A finding's class and the indeterminate case
/// are the caller's; this decides only which of the three it may be.)
#[must_use]
pub fn conclude(explored: Explored) -> &'static str {
    if explored.exact_reach {
        "finding"
    } else if explored.inexact_reach || !explored.complete {
        "unresolved"
    } else {
        "infeasible"
    }
}

/// Whether a triage removes its candidate: only a proved-infeasible one
/// (every path explored, none reaches the dereference NULL).
#[must_use]
pub fn removes(triage: &Triage) -> bool {
    matches!(triage, Triage::Infeasible)
}

/// The arms of `x - y OP c` (`sum` false) or `x + y OP c` (`sum` true) for
/// two independent values over the hole-free intervals `x` and `y`:
/// `(true arm feasible, false arm feasible)`. `None` for an operator other
/// than a comparison, or an interval that is empty or overflows.
#[must_use]
pub fn relation_arms(
    x: (i128, i128),
    y: (i128, i128),
    sum: bool,
    op: &str,
    c: i128,
) -> Option<(bool, bool)> {
    if x.0 > x.1 || y.0 > y.1 {
        return None;
    }
    let (lo, hi) = if sum {
        (x.0.checked_add(y.0)?, x.1.checked_add(y.1)?)
    } else {
        (x.0.checked_sub(y.1)?, x.1.checked_sub(y.0)?)
    };
    let meets = |a: i128, b: i128| a.max(lo) <= b.min(hi);
    let below = |k: i128| lo < k; // some value < k
    let above = |k: i128| hi > k; // some value > k
    let equal = meets(c, c);
    let other = lo != hi || lo != c; // some value != c
    Some(match op {
        "==" => (equal, other),
        "!=" => (other, equal),
        "<" => (below(c), hi >= c),
        ">=" => (hi >= c, below(c)),
        "<=" => (lo <= c, above(c)),
        ">" => (above(c), lo <= c),
        _ => return None,
    })
}

/// The least value a counter can hold: `starts` are the constants it is set
/// to, `steps` the constants it is increased by. `Some(min(starts))` when
/// there is a start and every step is non-negative (a defined execution never
/// overflows, so the counter never goes below its least start); `None`
/// otherwise.
#[must_use]
pub fn counter_lower_bound(starts: &[i128], steps: &[i128]) -> Option<i128> {
    if steps.iter().any(|k| *k < 0) {
        return None;
    }
    starts.iter().copied().min()
}
