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

//! Body evidence for bounds declarations and its per-run standing (card
//! **CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS**, gateway rules 14f–14g,
//! contract specification C09.1 with C06.5's shape).
//!
//! A bounds contract v1 declaration of a function whose body a unit defines
//! (a `static inline` header definition in every unit that includes it, or a
//! main-source definition) is checked against that body in the unit's first
//! phase. This module decides what that check is evidence *of*, and folds the
//! units of one run into the declaration's standing, exactly as rules 13g–13j
//! fold custody evidence:
//!
//! * **Coverage, per clause** ([`covers`]). A body whose row is `Safe` under
//!   its in-force declaration and does not refute it (`body`) covers each quant
//!   check (every access through the formal was decided inside the admitted
//!   set) and each precondition (`bounds_extent`, `bounds_index`: the body is
//!   safe relative to them). A frame clause is covered only by a positive frame
//!   check of that parameter (`frame`). A return extent, a derived relation
//!   (its length is a fact about the caller's object, which no body
//!   establishes), `bounds_preserves_extents()` and `bounds_global_extent` are
//!   covered by nothing.
//! * **One unit** ([`unit_outcome`]). Held (a review marker or a pin out of
//!   force): no evidence is kept. Refuted: a failure. Some clause covered:
//!   validated, with its covered clauses. Otherwise unvalidated.
//! * **The run** ([`standing`]). `held` when the declaration is held;
//!   otherwise `validated` only when at least one unit examined a body, every
//!   unit that did validated it, and every clause is covered in every one;
//!   `partly-validated` when every examining unit validated it and some, not
//!   all, clauses are covered everywhere; a `declared-assumption` otherwise. A
//!   failure in any unit makes it a declared assumption.
//! * **Roll-up** ([`rollup_class`]). A bounds `Safe` row rests on no
//!   declaration, only on validated ones, on partly-validated ones and no
//!   declared assumption, or on a declared assumption (held declarations count
//!   as assumptions, as a row resting on one is never `Safe` anyway).
//!
//! No verdict reads any of it. The Lean model is `QCore.BoundsBodyStanding`,
//! bound by the shared vectors in `crates/qcore-core/tests/bounds_body_standing.rs`.

use std::collections::{BTreeMap, BTreeSet};

/// A clause of a bounds declaration, by what can cover it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClauseKind {
    /// `quant checks P, Idx #i: …;`
    QuantCheck,
    /// `bounds_extent(P, …)` or `bounds_index(i, …)`.
    Precondition,
    /// `bounds_frame_reads|writes|moves(P)`.
    Frame,
    /// `bounds_return_derived(P, …)`.
    Relation,
    /// `bounds_return_extent(…)`.
    ReturnExtent,
    /// `bounds_preserves_extents()`.
    Preserves,
    /// `bounds_global_extent(…)`.
    GlobalExtent,
}

impl ClauseKind {
    pub const ALL: [Self; 7] = [
        Self::QuantCheck,
        Self::Precondition,
        Self::Frame,
        Self::Relation,
        Self::ReturnExtent,
        Self::Preserves,
        Self::GlobalExtent,
    ];
}

/// The evidence one unit's body check produced for one declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct BodyCheck {
    /// The body's own row is `Safe` under the in-force declaration.
    pub safe: bool,
    /// The body refutes the declaration (C06.4, rules 11s(7), 11e(4), 12l(2)).
    pub refuted: bool,
}

/// The evidence names a covered clause records.
pub const BODY: &str = "bounds-body";
pub const FRAME: &str = "bounds-frame";

/// Whether one clause is covered, and by which evidence: `body` is the body
/// check, `frame_confirmed` the positive frame check of the clause's own
/// parameter (meaningful only for a frame clause).
#[must_use]
pub fn covers(kind: ClauseKind, body: BodyCheck, frame_confirmed: bool) -> Option<&'static str> {
    if body.refuted {
        return None;
    }
    match kind {
        ClauseKind::QuantCheck | ClauseKind::Precondition if body.safe => Some(BODY),
        ClauseKind::Frame if frame_confirmed => Some(FRAME),
        _ => None,
    }
}

/// One unit's outcome for one declaration whose body it examined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitOutcome {
    /// Held for a human act: no evidence kept.
    Held,
    /// The body refutes the declaration.
    Failed,
    /// Some clause is covered: the covered clause labels.
    Validated(BTreeSet<String>),
    /// Examined, nothing covered.
    Unvalidated,
}

impl UnitOutcome {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Failed => "failed",
            Self::Validated(_) => "validated",
            Self::Unvalidated => "unvalidated",
        }
    }
}

/// One unit's outcome from its clause coverage.
#[must_use]
pub fn unit_outcome(held: bool, refuted: bool, covered: &BTreeSet<String>) -> UnitOutcome {
    if held {
        UnitOutcome::Held
    } else if refuted {
        UnitOutcome::Failed
    } else if covered.is_empty() {
        UnitOutcome::Unvalidated
    } else {
        UnitOutcome::Validated(covered.clone())
    }
}

/// A declaration's standing for one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Standing {
    Held,
    Validated,
    /// The clause labels no examining unit left uncovered are validated; these
    /// are the uncovered ones.
    PartlyValidated(BTreeSet<String>),
    DeclaredAssumption,
}

impl Standing {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Validated => "validated",
            Self::PartlyValidated(_) => "partly-validated",
            Self::DeclaredAssumption => "declared-assumption",
        }
    }
}

/// Fold every examining unit's outcome for one declaration (`clauses`: every
/// clause label the declaration has; `held`: the declaration is held in this
/// run).
#[must_use]
pub fn standing(held: bool, clauses: &BTreeSet<String>, units: &[UnitOutcome]) -> Standing {
    if held || units.contains(&UnitOutcome::Held) {
        return Standing::Held;
    }
    if units.is_empty() || clauses.is_empty() {
        return Standing::DeclaredAssumption;
    }
    let mut everywhere: Option<BTreeSet<String>> = None;
    for unit in units {
        let UnitOutcome::Validated(covered) = unit else {
            return Standing::DeclaredAssumption;
        };
        everywhere = Some(match everywhere {
            None => covered.clone(),
            Some(all) => all.intersection(covered).cloned().collect(),
        });
    }
    let everywhere = everywhere.unwrap_or_default();
    let uncovered = clauses
        .difference(&everywhere)
        .cloned()
        .collect::<BTreeSet<_>>();
    if uncovered.is_empty() {
        Standing::Validated
    } else if uncovered.len() == clauses.len() {
        Standing::DeclaredAssumption
    } else {
        Standing::PartlyValidated(uncovered)
    }
}

/// Where a bounds `Safe` row's declarations leave it in the roll-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rollup {
    NoDeclaration,
    OnlyValidated,
    PartlyValidated,
    DeclaredAssumption,
}

impl Rollup {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoDeclaration => "resting_on_no_declaration",
            Self::OnlyValidated => "resting_only_on_validated_declarations",
            Self::PartlyValidated => "resting_on_a_partly_validated_declaration",
            Self::DeclaredAssumption => "resting_on_a_declared_assumption",
        }
    }
}

/// A row's roll-up class from the standings of the declarations it rests on
/// (a name with no standing is a declared assumption).
#[must_use]
pub fn rollup_class(dependencies: &[String], standings: &BTreeMap<String, Standing>) -> Rollup {
    if dependencies.is_empty() {
        return Rollup::NoDeclaration;
    }
    let mut class = Rollup::OnlyValidated;
    for name in dependencies {
        match standings.get(name) {
            Some(Standing::Validated) => {}
            Some(Standing::PartlyValidated(_)) => class = class.max(Rollup::PartlyValidated),
            _ => return Rollup::DeclaredAssumption,
        }
    }
    class
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(labels: &[&str]) -> BTreeSet<String> {
        labels.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_failure_in_one_unit_wins() {
        let clauses = set(&["q"]);
        let units = [UnitOutcome::Validated(set(&["q"])), UnitOutcome::Failed];
        assert_eq!(
            standing(false, &clauses, &units),
            Standing::DeclaredAssumption
        );
    }

    #[test]
    fn a_clause_must_be_covered_in_every_unit() {
        let clauses = set(&["q", "f"]);
        let units = [
            UnitOutcome::Validated(set(&["q", "f"])),
            UnitOutcome::Validated(set(&["q"])),
        ];
        assert_eq!(
            standing(false, &clauses, &units),
            Standing::PartlyValidated(set(&["f"]))
        );
    }
}
