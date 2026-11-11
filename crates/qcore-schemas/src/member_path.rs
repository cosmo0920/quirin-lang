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
    /// Card **FB-TEARDOWN-DESTROY-DECLARATIONS** (decision 2, 2026-10-16:
    /// "Yes, allow it"): `when #PARAM OP CONST` on a plain integer
    /// parameter. The state holds at the calls whose actual there satisfies
    /// the guard; every other path of the class reads `#kept`. Omitted when
    /// absent, so every declaration before the card serializes as it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameter_guard: Option<ParameterGuard>,
}

/// The comparison of a parameter guard: rule 12w's operators and `!=`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl ParameterOp {
    /// The operator read from its spelling.
    #[must_use]
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        Some(match spelling {
            "==" => Self::Eq,
            "!=" => Self::Ne,
            "<" => Self::Lt,
            ">" => Self::Gt,
            "<=" => Self::Le,
            ">=" => Self::Ge,
            _ => return None,
        })
    }

    /// The written operator.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Le => "<=",
            Self::Ge => ">=",
        }
    }

    /// `value OP constant` over the mathematical integers.
    #[must_use]
    pub const fn holds(self, value: i128, constant: i128) -> bool {
        match self {
            Self::Eq => value == constant,
            Self::Ne => value != constant,
            Self::Lt => value < constant,
            Self::Gt => value > constant,
            Self::Le => value <= constant,
            Self::Ge => value >= constant,
        }
    }

    /// Whether the guard splits `[low, high]` into two non-empty parts.
    #[must_use]
    pub fn partitions(self, constant: i128, low: i128, high: i128) -> bool {
        (low..=high).contains(&constant)
            && match self {
                Self::Eq | Self::Ne => low < high,
                Self::Lt | Self::Ge => low < constant,
                Self::Gt | Self::Le => constant < high,
            }
    }
}

/// A parameter guard's constant: a canonical decimal, or a C enumerator
/// (`@NAME`, a module alias resolved to it) whose value Clang states per
/// unit (rule 13l).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardConstant {
    Value(i128),
    Name(String),
}

/// `when #PARAM OP CONST`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ParameterGuard {
    /// The integer parameter tested.
    pub parameter: usize,
    pub op: ParameterOp,
    pub constant: GuardConstant,
    /// The value range of the parameter's declared integer spelling: an
    /// actual outside it is converted by C, so it decides nothing.
    pub low: i128,
    pub high: i128,
}

/// What a call's actual decides about a parameter guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardDecision {
    Holds,
    Fails,
    Undecided,
}

impl ParameterGuard {
    /// The decision for an actual whose value is `actual` and a constant
    /// whose value is `constant` (`None`: not decided, never favourable).
    #[must_use]
    pub fn decide(&self, actual: Option<i128>, constant: Option<i128>) -> GuardDecision {
        match (actual, constant) {
            (Some(value), Some(constant))
                if self.low <= value
                    && value <= self.high
                    && self.low <= constant
                    && constant <= self.high =>
            {
                if self.op.holds(value, constant) {
                    GuardDecision::Holds
                } else {
                    GuardDecision::Fails
                }
            }
            _ => GuardDecision::Undecided,
        }
    }

    /// The canonical text, `#NAME OP CONST` given the parameter's name.
    #[must_use]
    pub fn spelling(&self, name: &str) -> String {
        let constant = match &self.constant {
            GuardConstant::Value(value) => value.to_string(),
            GuardConstant::Name(name) => format!("@{name}"),
        };
        format!("#{name} {} {constant}", self.op.spelling())
    }
}

/// The state a guarded clause gives its path at a call: the stated state
/// where the guard holds, `#kept` where it fails, and where the actual is
/// undecided the **cautious join** of the two (`{V, #kept}`; `#kept` itself
/// when `V` is `#kept`) (`QCore.MemberPathDirectCall.state_under`).
#[must_use]
pub fn state_under(state: PathState, decision: GuardDecision) -> PathState {
    let kept = PathState {
        fate: Fate::Kept,
        maybe: false,
    };
    match decision {
        GuardDecision::Holds => state,
        GuardDecision::Fails => kept,
        GuardDecision::Undecided if state.fate == Fate::Kept => kept,
        GuardDecision::Undecided => PathState {
            fate: state.fate,
            maybe: true,
        },
    }
}

/// What a **direct call** of a declaration (its states read once, no arm
/// after it) does to a tracked object the caller linked at a stated path
/// (rule 30 at a direct call, card FB-TEARDOWN-DESTROY-DECLARATIONS).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectOutcome {
    /// It leaves the caller's custody exactly once (released or handed on),
    /// and the link is gone.
    Gone,
    /// It stays linked and live.
    Stays,
    /// It leaves the caller's custody once, and the member still designates
    /// it: a later release or hand-off of it refuses as a double release.
    Taken,
    Refused(Refusal),
}

/// A path's state (`None`: unstated) at a direct call. A set is gone on one
/// reading and kept on the other, so it refuses (`QCore.MemberPathDirectCall`).
#[must_use]
pub const fn compose_direct(state: Option<PathState>) -> DirectOutcome {
    match state {
        None => DirectOutcome::Refused(Refusal::Unstated),
        Some(PathState {
            fate: Fate::Released | Fate::Moved,
            maybe: false,
        }) => DirectOutcome::Gone,
        Some(PathState {
            fate: Fate::Kept, ..
        }) => DirectOutcome::Stays,
        Some(PathState {
            fate: Fate::Stale,
            maybe: false,
        }) => DirectOutcome::Taken,
        Some(PathState { maybe: true, .. }) => DirectOutcome::Refused(Refusal::Undecided),
    }
}

/// The spelling of a direct outcome in the vectors.
#[must_use]
pub const fn direct_word(outcome: DirectOutcome) -> &'static str {
    match outcome {
        DirectOutcome::Gone => "gone",
        DirectOutcome::Stays => "stays",
        DirectOutcome::Taken => "taken",
        DirectOutcome::Refused(refusal) => outcome_word(Outcome::Refused(refusal)),
    }
}

/// `DIRECT_VECTORS`: (fate word or `unstated`, maybe, outcome), shared with
/// `QCore.MemberPathDirectCall.directVectors`.
pub const DIRECT_VECTORS: &[(&str, bool, &str)] = &[
    ("released", false, "gone"),
    ("moved", false, "gone"),
    ("stale", false, "taken"),
    ("kept", false, "stays"),
    ("released", true, "undecided"),
    ("moved", true, "undecided"),
    ("stale", true, "undecided"),
    ("unstated", false, "unstated"),
];

/// `GUARD_VECTORS`: (operator, actual or `None`, constant, the stated fate
/// word, the state's spelling at the call), shared with
/// `QCore.MemberPathDirectCall.guardVectors`.
pub const GUARD_VECTORS: &[(&str, Option<i128>, i128, &str, &str)] = &[
    ("!=", Some(1), 0, "released", "#released"),
    ("!=", Some(0), 0, "released", "#kept"),
    ("!=", None, 0, "released", "{#released, #kept}"),
    ("==", Some(0), 0, "stale", "#stale"),
    ("==", Some(2), 0, "stale", "#kept"),
    ("<", Some(-1), 0, "moved", "#moved"),
    (">=", None, 3, "moved", "{#moved, #kept}"),
    ("==", None, 1, "kept", "#kept"),
];

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

    /// The Lean vectors (`QCore.MemberPathRelease.composeVectors`) are these.
    #[test]
    fn compose_vectors_are_the_lean_vectors() {
        let lean = include_str!("../../../proof/QCore/MemberPathRelease.lean");
        let start = lean.find("def composeVectors").expect("the Lean vectors");
        let body = &lean[start..];
        let body = &body[..body.find("\n]").expect("the vector list ends")];
        let rows = body
            .lines()
            .filter_map(|line| {
                let line = line.trim().trim_end_matches(',');
                let inner = line.strip_prefix('(')?.strip_suffix(')')?;
                let parts = inner
                    .split(',')
                    .map(|part| part.trim().trim_matches('"').to_owned())
                    .collect::<Vec<_>>();
                Some(parts)
            })
            .collect::<Vec<_>>();
        let rust = COMPOSE_VECTORS
            .iter()
            .map(|&(fate, maybe, arm, outcome)| {
                vec![
                    fate.to_owned(),
                    maybe.to_string(),
                    arm.to_owned(),
                    outcome.to_owned(),
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, rust);
    }

    fn lean_rows(name: &str) -> Vec<Vec<String>> {
        let lean = include_str!("../../../proof/QCore/MemberPathDirectCall.lean");
        let start = lean.find(&format!("def {name}")).expect("the Lean vectors");
        let body = &lean[start..];
        let body = &body[..body.find("\n]").expect("the vector list ends")];
        body.lines()
            .filter_map(|line| {
                let line = line.trim().trim_end_matches(',');
                let inner = line.strip_prefix('(')?.strip_suffix(')')?;
                Some(
                    inner
                        .split(',')
                        .map(|part| part.trim().trim_matches('"').to_owned())
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    #[test]
    fn direct_vectors_hold_and_are_the_lean_vectors() {
        for &(fate, maybe, expected) in DIRECT_VECTORS {
            let state = Fate::from_word(fate).map(|fate| PathState { fate, maybe });
            assert_eq!(
                direct_word(compose_direct(state)),
                expected,
                "{fate} {maybe}"
            );
        }
        let rust = DIRECT_VECTORS
            .iter()
            .map(|&(fate, maybe, outcome)| {
                vec![fate.to_owned(), maybe.to_string(), outcome.to_owned()]
            })
            .collect::<Vec<_>>();
        assert_eq!(lean_rows("directVectors"), rust);
    }

    #[test]
    fn guard_vectors_hold_and_are_the_lean_vectors() {
        for &(op, actual, constant, fate, expected) in GUARD_VECTORS {
            let guard = ParameterGuard {
                parameter: 1,
                op: ParameterOp::from_spelling(op).expect("an operator"),
                constant: GuardConstant::Value(constant),
                low: -(1 << 31),
                high: (1 << 31) - 1,
            };
            let state = PathState {
                fate: Fate::from_word(fate).expect("a fate"),
                maybe: false,
            };
            let decision = guard.decide(actual, Some(constant));
            assert_eq!(
                state_under(state, decision).spelling(),
                expected,
                "{op} {actual:?} {constant} {fate}"
            );
        }
        let rust = GUARD_VECTORS
            .iter()
            .map(|&(op, actual, constant, fate, spelled)| {
                vec![
                    op.to_owned(),
                    actual.map_or_else(
                        || "none".to_owned(),
                        |value| {
                            if value < 0 {
                                format!("some ({value})")
                            } else {
                                format!("some {value}")
                            }
                        },
                    ),
                    constant.to_string(),
                    fate.to_owned(),
                    spelled.to_owned(),
                ]
            })
            .collect::<Vec<_>>();
        // The Lean rows spell the set with a comma; rejoin the split cell.
        let lean = lean_rows("guardVectors")
            .into_iter()
            .map(|mut row| {
                if row.len() == 6 {
                    let tail = row.pop().expect("six cells");
                    let last = row.pop().expect("five cells");
                    row.push(format!("{last}, {tail}"));
                }
                row
            })
            .collect::<Vec<_>>();
        assert_eq!(lean, rust);
    }

    #[test]
    fn an_out_of_range_or_unresolved_value_is_undecided() {
        let guard = ParameterGuard {
            parameter: 0,
            op: ParameterOp::Ne,
            constant: GuardConstant::Value(0),
            low: 0,
            high: 255,
        };
        assert_eq!(guard.decide(Some(256), Some(0)), GuardDecision::Undecided);
        assert_eq!(guard.decide(Some(-1), Some(0)), GuardDecision::Undecided);
        assert_eq!(guard.decide(Some(1), None), GuardDecision::Undecided);
        assert_eq!(guard.decide(None, Some(0)), GuardDecision::Undecided);
        assert_eq!(guard.decide(Some(1), Some(0)), GuardDecision::Holds);
        assert!(!ParameterOp::Lt.partitions(0, 0, 255));
        assert!(ParameterOp::Ne.partitions(0, 0, 255));
        assert!(!ParameterOp::Eq.partitions(300, 0, 255));
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
            parameter_guard: None,
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
