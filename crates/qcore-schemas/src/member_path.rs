//! A callback's member-path release or move-out (card
//! **OP-CALLBACK-MEMBER-PATH-RELEASE**; contract spec §C04.19.6, grammar
//! G05d, gateway rule 30).
//!
//! USER DECISION 2026-10-15 (verbatim in the spec and the card's report):
//! "B: state + set" and "Build it now".
//!
//! ```text
//! quant live #rko: rd_kafka_op_s.rko_u.leaders.eonce |-> #released when #result != #KEEP;
//! quant live #rko: rd_kafka_op_s.rko_u.leaders.partitions |-> {#moved, #kept} when #result != #KEEP;
//! quant live #reply: rd_kafka_op_s.rko_u.leaders.opaque |-> #stale;
//! ```
//!
//! The right-hand side is the member's state after the call: `#released` /
//! `#moved` (the object it held was released / handed to another holder, not
//! the caller, and the member reads cleared: `NULL`, or an emptied list),
//! `#stale` (the object was taken but the member still designates it),
//! `#kept` (untouched: the state of every path the declaration does not
//! name), or a two-element set `{V, #kept}` (one of the two on each path).
//! No move destination is stated.
//!
//! A consumer composes each candidate's state at a path the caller links a
//! tracked object at with the release the invoking declaration then applies
//! there (an arm, C04.19.2), and joins over the candidates ([`compose`],
//! [`join`]): the table of the card's prediction, proved in
//! `QCore.MemberPathRelease`.

use serde::{Deserialize, Serialize};

use crate::arm_release::ArmJoin;
use crate::callback::CallbackMember;
use crate::handoff::HandOffGuard;

/// `#released`.
pub const RELEASED: &str = "released";
/// `#moved`.
pub const MOVED: &str = "moved";
/// `#stale`.
pub const STALE: &str = "stale";
/// `#kept`.
pub const KEPT: &str = "kept";

/// What a call leaves at a member path of a formal's object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fate {
    /// The object is released; the member reads cleared.
    Released,
    /// The object is handed to another holder (not the caller); the member
    /// reads cleared.
    Moved,
    /// The object is taken (released or handed on); the member still
    /// designates it.
    Stale,
    /// Untouched.
    Kept,
}

impl Fate {
    /// The value keyword, without its `#`.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Released => RELEASED,
            Self::Moved => MOVED,
            Self::Stale => STALE,
            Self::Kept => KEPT,
        }
    }

    /// The keyword's fate, `None` for any other word.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            RELEASED => Some(Self::Released),
            MOVED => Some(Self::Moved),
            STALE => Some(Self::Stale),
            KEPT => Some(Self::Kept),
            _ => None,
        }
    }
}

/// A path's state: one fate, or (`maybe`) the set `{fate, #kept}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PathState {
    pub fate: Fate,
    /// The set `{fate, #kept}`: one of the two on each path. Never with
    /// [`Fate::Kept`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub maybe: bool,
}

impl PathState {
    /// The canonical spelling of the right-hand side.
    #[must_use]
    pub fn spelling(self) -> String {
        if self.maybe {
            format!("{{#{}, #{KEPT}}}", self.fate.word())
        } else {
            format!("#{}", self.fate.word())
        }
    }
}

/// `quant live #P: TAG.PATH |-> STATE [when GUARD];`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemberPathState {
    /// The formal whose object the path is below.
    pub parameter: usize,
    pub member: CallbackMember,
    pub state: PathState,
    /// Absent (every return), or the complement of the declaration's
    /// hand-off guard.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<HandOffGuard>,
}

/// Why a composition refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// A stale path some arm releases: a second release.
    DoubleRelease,
    /// A stale path no arm releases: a second invocation would take it again.
    Stale,
    /// Gone on one reading and kept on another (a may-set at a path no arm
    /// releases, two candidates that disagree, an arm join that is not
    /// decided).
    Undecided,
    /// No declaration in force states the path.
    Unstated,
}

/// What happens to a tracked object the caller linked at a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// It leaves the caller's custody exactly once (released by the call or
    /// the arm, or handed on), and the link is gone.
    Gone,
    /// It stays linked and live (no arm releases it).
    Stays,
    Refused(Refusal),
}

/// One fate composed with the arm join at the path.
#[must_use]
pub const fn compose_fate(fate: Fate, arm: ArmJoin) -> Outcome {
    match (fate, arm) {
        (Fate::Kept, ArmJoin::Every) | (Fate::Released | Fate::Moved, _) => Outcome::Gone,
        (Fate::Kept, ArmJoin::None) => Outcome::Stays,
        (Fate::Kept, ArmJoin::Some) => Outcome::Refused(Refusal::Undecided),
        (Fate::Stale, ArmJoin::None) => Outcome::Refused(Refusal::Stale),
        (Fate::Stale, ArmJoin::Every | ArmJoin::Some) => Outcome::Refused(Refusal::DoubleRelease),
    }
}

/// The join of two outcomes: equal ones stand; a refusal stands over
/// anything; `Gone` against `Stays` is undecided.
#[must_use]
pub fn join(left: Outcome, right: Outcome) -> Outcome {
    match (left, right) {
        (left, right) if left == right => left,
        (Outcome::Refused(reason), _) | (_, Outcome::Refused(reason)) => Outcome::Refused(reason),
        _ => Outcome::Refused(Refusal::Undecided),
    }
}

/// A candidate's state (`None`: unstated, no declaration in force) composed
/// with the arm join at the path. The set `{V, #kept}` is the join of its two
/// members (`QCore.MemberPathRelease.the_set_is_the_join_of_its_members`).
#[must_use]
pub fn compose(state: Option<PathState>, arm: ArmJoin) -> Outcome {
    match state {
        None => Outcome::Refused(Refusal::Unstated),
        Some(PathState { fate, maybe: false }) => compose_fate(fate, arm),
        Some(PathState { fate, maybe: true }) => {
            join(compose_fate(fate, arm), compose_fate(Fate::Kept, arm))
        }
    }
}

/// The join over every candidate of an invocation at one linked path.
#[must_use]
pub fn join_all(outcomes: impl IntoIterator<Item = Outcome>) -> Option<Outcome> {
    outcomes.into_iter().reduce(join)
}

/// The state a declaration's clauses give `member` below the formal at
/// `parameter`, read in the class its guard admits (`Some(Kept)` for an
/// unnamed path). `None` when a clause names a path that strictly contains or
/// is contained in `path` (the declaration does not state it as one path).
#[must_use]
pub fn state_at(states: &[MemberPathState], parameter: usize, path: &str) -> Option<PathState> {
    let mut found = PathState {
        fate: Fate::Kept,
        maybe: false,
    };
    for state in states.iter().filter(|state| state.parameter == parameter) {
        let stated = state.member.path.as_str();
        if stated == path {
            found = state.state;
        } else if stated.starts_with(&format!("{path}.")) || path.starts_with(&format!("{stated}."))
        {
            return None;
        }
    }
    Some(found)
}

/// Whether `guard` is the complement of the hand-off guard `handoff` (the
/// same constant, the other operator): every result is in exactly one of the
/// two classes (`QCore.MemberPathRelease.the_complement_partitions_the_results`).
#[must_use]
pub fn complements(guard: &HandOffGuard, handoff: &HandOffGuard) -> bool {
    guard.constant == handoff.constant && guard.op != handoff.op
}

/// `COMPOSE_VECTORS`: (fate word, maybe, arm join `every` / `some` / `none`,
/// outcome `gone` / `stays` / the refusal), shared with
/// `QCore.MemberPathRelease.composeVectors`.
pub const COMPOSE_VECTORS: &[(&str, bool, &str, &str)] = &[
    ("kept", false, "every", "gone"),
    ("kept", false, "none", "stays"),
    ("kept", false, "some", "undecided"),
    ("released", false, "every", "gone"),
    ("released", false, "none", "gone"),
    ("moved", false, "some", "gone"),
    ("stale", false, "every", "double_release"),
    ("stale", false, "some", "double_release"),
    ("stale", false, "none", "stale"),
    ("moved", true, "every", "gone"),
    ("moved", true, "none", "undecided"),
    ("released", true, "some", "undecided"),
    ("stale", true, "every", "double_release"),
    ("stale", true, "none", "stale"),
    ("unstated", false, "every", "unstated"),
    ("unstated", false, "none", "unstated"),
];

/// The spelling of an arm join in the vectors.
#[must_use]
pub const fn arm_word(arm: ArmJoin) -> &'static str {
    match arm {
        ArmJoin::Every => "every",
        ArmJoin::Some => "some",
        ArmJoin::None => "none",
    }
}

/// The spelling of an outcome in the vectors.
#[must_use]
pub const fn outcome_word(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Gone => "gone",
        Outcome::Stays => "stays",
        Outcome::Refused(Refusal::DoubleRelease) => "double_release",
        Outcome::Refused(Refusal::Stale) => "stale",
        Outcome::Refused(Refusal::Undecided) => "undecided",
        Outcome::Refused(Refusal::Unstated) => "unstated",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm(word: &str) -> ArmJoin {
        match word {
            "every" => ArmJoin::Every,
            "some" => ArmJoin::Some,
            _ => ArmJoin::None,
        }
    }

    #[test]
    fn compose_vectors_hold() {
        for &(fate, maybe, join_word, expected) in COMPOSE_VECTORS {
            let state = Fate::from_word(fate).map(|fate| PathState { fate, maybe });
            assert_eq!(
                outcome_word(compose(state, arm(join_word))),
                expected,
                "{fate} {maybe} {join_word}"
            );
        }
    }

    #[test]
    fn candidates_that_disagree_refuse() {
        assert_eq!(
            join_all([Outcome::Gone, Outcome::Stays]),
            Some(Outcome::Refused(Refusal::Undecided))
        );
        assert_eq!(
            join_all([Outcome::Gone, Outcome::Gone]),
            Some(Outcome::Gone)
        );
        assert_eq!(join_all([]), None);
    }

    #[test]
    fn an_unnamed_path_is_kept_and_an_overlapping_one_is_unstated() {
        let states = [MemberPathState {
            parameter: 2,
            member: CallbackMember {
                tag: "t".into(),
                path: "u.l.p".into(),
            },
            state: PathState {
                fate: Fate::Moved,
                maybe: false,
            },
            guard: None,
        }];
        assert_eq!(
            state_at(&states, 2, "u.l.p").map(|s| s.fate),
            Some(Fate::Moved)
        );
        assert_eq!(
            state_at(&states, 2, "u.l.q").map(|s| s.fate),
            Some(Fate::Kept)
        );
        assert_eq!(
            state_at(&states, 1, "u.l.p").map(|s| s.fate),
            Some(Fate::Kept)
        );
        assert_eq!(state_at(&states, 2, "u.l"), None);
        assert_eq!(state_at(&states, 2, "u.l.p.x"), None);
    }
}
