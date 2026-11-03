//! Card **CELL-GUARD-RESULT-RANGE-CHECKED** (contract specification C09.4,
//! gateway rule 38e): a pin-derived integer result set as the resource
//! environment carries it to rule 38's result-guard reading.

use serde::{Deserialize, Serialize};

/// How a derived result set stands on the route that installed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultRangeStanding {
    /// Pinned in force, its derivation current: the only standing read.
    InForce,
    /// Held: a review marker, a pin not in force, or a derivation whose C
    /// library sets changed.
    Held,
    /// Declared and pinned but never derived (no derivation in its lock): an
    /// unchecked range, never read by rule 38.
    Unchecked,
}

/// One function's derived result set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultRangeFact {
    /// Sorted, disjoint, non-adjacent closed intervals.
    pub intervals: Vec<(i64, i64)>,
    /// The universe of the declared result type.
    pub universe: (i64, i64),
    pub standing: ResultRangeStanding,
    /// Why it is not in force, or the derivation it rests on.
    pub detail: String,
}
