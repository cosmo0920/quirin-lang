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

//! A constructor's result state (card **OP-CONSTRUCTOR-DECLARATIONS**;
//! contract spec §C04.19.4, grammar G05, gateway rules 12o, 13w and 14e).
//!
//! USER DECISIONS 2026-10-14, verbatim in the spec:
//!
//! ```text
//! ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0);
//! ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, cb);
//! quant live #result: rd_kafka_op_s.rko_op_cb |-> #absent;
//! ```
//!
//! * `resource_discriminant_set(result, TAG.FIELD, P, FLAGS)`: the returned
//!   object's member holds the actual of the integer parameter `P` OR-ed with
//!   the canonical decimal `FLAGS`, exactly; a consumer selects the arm of that
//!   value under its own `resource_discriminant` mask (rule 12o's decided arm).
//! * `resource_callback_member_registered(result, TAG.FIELD, F)`: the returned
//!   object's function-pointer member holds the callback `F` (rule 13w's
//!   per-path registration, through a member).
//! * `quant live #result: TAG.FIELD |-> #absent;`: the result is live (exactly
//!   `quant live #result`) and its member maps to `absent`, no callback.
//!   Its two levels elaborate to one canonical set (rule 14e):
//!   [`ConstructedState::live`] and a [`MemberState`] whose value is
//!   [`MemberValue::Absent`].
//!
//! A declaration carrying any of these is withheld from the function map with
//! the retaining declarations, like every callback clause.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::arm_release::{ArmClause, ArmEffect, ArmGuard, ArmJoin};
use crate::callback::CallbackMember;

/// The reserved right-hand side of a member state: no callback is stored
/// (distinct from C's `NULL`, a value; `#absent` is a state). Card
/// **SUPPLIED-OWNED-RETURN-CODE-HANDOFF** (USER DECISION 2026-10-15): a value
/// keyword carries `#`, so `#absent` is its one accepted spelling and a bare
/// `absent` rejects naming it.
pub const ABSENT: &str = "#absent";

/// The keyword's word without its `#` ([`ABSENT`]).
pub const ABSENT_WORD: &str = "absent";

/// `resource_discriminant_set(result, TAG.FIELD, P, FLAGS)`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DiscriminantSet {
    /// The member, `TAG.FIELD`.
    pub member: CallbackMember,
    /// The integer parameter whose actual is the value.
    pub parameter: usize,
    /// The bits OR-ed into the actual (`0`: none).
    pub flags: u64,
}

/// What a constructor leaves in a function-pointer member of its result.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "value")]
pub enum MemberValue {
    /// The callback parameter at this index (rule 13w's registration).
    Callback { callback: usize },
    /// `absent`: no callback (rule 14e).
    Absent,
    /// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (user decision D1): the named
    /// function, a stated default (`quant live #result: TAG.FIELD |-> f;`,
    /// or `resource_callback_member_registered(result, TAG.FIELD, f)` with
    /// `f` naming no parameter). It is a store of `f` into the member for
    /// rule 12n's run closure.
    Function { function: String },
}

/// One member's state on the result.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemberState {
    pub member: CallbackMember,
    pub value: MemberValue,
}

/// A constructor's stated result state: the canonical parts of every spelling.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstructedState {
    /// `quant live #result`: restates `resource_return_owned(nonnull)`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub live: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discriminant: Option<DiscriminantSet>,
    /// At most one per member, in clause order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<MemberState>,
}

impl ConstructedState {
    /// Whether nothing is stated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.live && self.discriminant.is_none() && self.members.is_empty()
    }

    /// Whether some member is stated `absent` (the clause rule 14e checks
    /// against the defining body).
    #[must_use]
    pub fn states_absent(&self) -> bool {
        self.members
            .iter()
            .any(|state| state.value == MemberValue::Absent)
    }
}

/// A classified rewrite of a discriminant on an existing object: inside a
/// case group of a `switch` reading the same object's member (`& ~mask` when
/// `mask` is given), whose labels are `from`, the member is set to `to`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Rewrite {
    /// The bits the switch clears before comparing (`None`: none).
    pub mask: Option<u64>,
    pub from: BTreeSet<i64>,
    pub to: i64,
}

impl Rewrite {
    /// Whether an object holding `value` satisfies one of the labels.
    #[must_use]
    pub fn applies(&self, value: i64) -> bool {
        let read = match self.mask {
            Some(mask) => value & !(mask as i64),
            None => value,
        };
        self.from.contains(&read)
    }
}

/// The values a decided discriminant may hold: the least set containing
/// `value` and closed under every classified rewrite.
#[must_use]
pub fn reachable_values(value: i64, rewrites: &[Rewrite]) -> BTreeSet<i64> {
    let mut out = BTreeSet::from([value]);
    loop {
        let more = rewrites
            .iter()
            .filter(|rewrite| out.iter().any(|value| rewrite.applies(*value)))
            .map(|rewrite| rewrite.to)
            .filter(|to| !out.contains(to))
            .collect::<Vec<_>>();
        if more.is_empty() {
            return out;
        }
        out.extend(more);
    }
}

/// The arm a value selects under `mask`: its `when` clauses, else the
/// `otherwise` clauses; `None` when neither exists (no arm, never neutral).
#[must_use]
pub fn arm_of(arms: &[ArmClause], mask: u64, value: i64) -> Option<Vec<&ArmClause>> {
    let read = value & !(mask as i64);
    let named = arms
        .iter()
        .filter(|arm| arm.guard == ArmGuard::Equals { value: read })
        .collect::<Vec<_>>();
    if !named.is_empty() {
        return Some(named);
    }
    let otherwise = arms
        .iter()
        .filter(|arm| arm.guard == ArmGuard::Otherwise)
        .collect::<Vec<_>>();
    (!otherwise.is_empty()).then_some(otherwise)
}

/// The join of the arms the decided `values` select at `path`: `None` when
/// some value has no arm.
#[must_use]
pub fn decided_join(
    arms: &[ArmClause],
    mask: u64,
    values: &BTreeSet<i64>,
    path: &str,
) -> Option<ArmJoin> {
    let mut releasing = 0usize;
    for value in values {
        let arm = arm_of(arms, mask, *value)?;
        if arm
            .iter()
            .any(|clause| matches!(&clause.effect, ArmEffect::Released { path: p } if p == path))
        {
            releasing += 1;
        }
    }
    Some(if releasing == 0 {
        ArmJoin::None
    } else if releasing == values.len() {
        ArmJoin::Every
    } else {
        ArmJoin::Some
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arms(list: &[(Option<i64>, Option<&str>)]) -> Vec<ArmClause> {
        list.iter()
            .map(|(value, path)| ArmClause {
                guard: value.map_or(ArmGuard::Otherwise, |value| ArmGuard::Equals { value }),
                effect: path.map_or(ArmEffect::Empty, |path| ArmEffect::Released {
                    path: path.to_owned(),
                }),
            })
            .collect()
    }

    /// `DECIDED_JOIN_VECTORS` (`QCore.OpCallbacks.decidedJoinVectors`): arms,
    /// a decided value, the rewrites `(from, to)` (unmasked), a path and the
    /// join (`None`: some reachable value has no arm).
    #[allow(clippy::type_complexity)]
    pub(crate) const DECIDED_JOIN_VECTORS: &[(
        &[(Option<i64>, Option<&str>)],
        i64,
        &[(i64, i64)],
        &str,
        Option<ArmJoin>,
    )] = &[
        // A decided value selects exactly its arm.
        (
            &[
                (Some(6), Some("u.c.p")),
                (Some(9), Some("u.x")),
                (None, None),
            ],
            6,
            &[],
            "u.c.p",
            Some(ArmJoin::Every),
        ),
        (
            &[
                (Some(6), Some("u.c.p")),
                (Some(9), Some("u.x")),
                (None, None),
            ],
            9,
            &[],
            "u.c.p",
            Some(ArmJoin::None),
        ),
        // A value no `when` names takes `otherwise`.
        (
            &[(Some(6), Some("u.c.p")), (None, None)],
            17,
            &[],
            "u.c.p",
            Some(ArmJoin::None),
        ),
        // A rewrite inside one arm keeps it (17 -> 16, both `otherwise`).
        (
            &[(Some(6), Some("u.c.p")), (None, None)],
            17,
            &[(17, 16)],
            "u.c.p",
            Some(ArmJoin::None),
        ),
        // A rewrite into another arm joins the two.
        (
            &[(Some(6), Some("u.c.p")), (None, None)],
            17,
            &[(17, 6)],
            "u.c.p",
            Some(ArmJoin::Some),
        ),
        // No arm and no `otherwise`: undecided, never neutral.
        (&[(Some(6), Some("u.c.p"))], 7, &[], "u.c.p", None),
    ];

    const LEAN: &str = include_str!("../../../proof/QCore/OpCallbacks.lean");

    #[test]
    fn decided_join_vectors_are_the_lean_vectors() {
        let header =
            "def decidedJoinVectors : List (List ArmClause × List Int × String × Option Join) := [";
        let start = LEAN.find(header).expect("the Lean list") + header.len();
        let body = &LEAN[start..];
        let rows = body[..body.find("\n]").expect("the list's end")]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_end_matches(',').to_owned())
            .collect::<Vec<_>>();
        let spell =
            |value: Option<String>| value.map_or("Option.none".to_owned(), |v| format!("some {v}"));
        let rendered = DECIDED_JOIN_VECTORS
            .iter()
            .map(|(list, value, rewrites, path, join)| {
                let rewrites = rewrites
                    .iter()
                    .map(|(from, to)| Rewrite {
                        mask: None,
                        from: BTreeSet::from([*from]),
                        to: *to,
                    })
                    .collect::<Vec<_>>();
                let values = reachable_values(*value, &rewrites)
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "([{}], [{values}], \"{path}\", {})",
                    list.iter()
                        .map(|(value, path)| format!(
                            "({}, {})",
                            spell(value.map(|value| value.to_string())),
                            spell(path.map(|path| format!("\"{path}\"")))
                        ))
                        .collect::<Vec<_>>()
                        .join(", "),
                    match join {
                        Some(ArmJoin::Every) => "some .every",
                        Some(ArmJoin::Some) => "some .some",
                        Some(ArmJoin::None) => "some .none",
                        None => "Option.none",
                    }
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, rendered);
    }

    #[test]
    fn decided_join_vectors_hold() {
        for (list, value, rewrites, path, join) in DECIDED_JOIN_VECTORS {
            let rewrites = rewrites
                .iter()
                .map(|(from, to)| Rewrite {
                    mask: None,
                    from: BTreeSet::from([*from]),
                    to: *to,
                })
                .collect::<Vec<_>>();
            let values = reachable_values(*value, &rewrites);
            assert_eq!(
                decided_join(&arms(list), 0, &values, path),
                *join,
                "{list:?} {value} {path}"
            );
        }
    }

    #[test]
    fn the_mask_clears_flags_before_the_arm_is_chosen() {
        let list = arms(&[(Some(6), Some("u.c.p")), (None, None)]);
        // RD_KAFKA_OP_CB (1 << 29) is cleared by RD_KAFKA_OP_FLAGMASK.
        let values = BTreeSet::from([6 | (1 << 29)]);
        assert_eq!(
            decided_join(&list, 1_610_612_736, &values, "u.c.p"),
            Some(ArmJoin::Every)
        );
        assert_eq!(
            decided_join(&list, 0, &values, "u.c.p"),
            Some(ArmJoin::None)
        );
    }

    #[test]
    fn a_masked_rewrite_reads_the_cleared_value() {
        let rewrite = Rewrite {
            mask: Some(1 << 29),
            from: BTreeSet::from([17]),
            to: 16,
        };
        assert_eq!(
            reachable_values(17 | (1 << 29), std::slice::from_ref(&rewrite)),
            BTreeSet::from([16, 17 | (1 << 29)])
        );
        assert_eq!(reachable_values(18, &[rewrite]), BTreeSet::from([18]));
    }
}
