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

//! Type-switched releases (card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES**;
//! contract spec §C04.19.2, grammar G05, gateway rule 12o).
//!
//! USER DECISION 2026-10-14 (clause 2's form): one discriminant per
//! declaration, then one clause per arm guarded with `when` or `otherwise`,
//! decimal constants citing their source:
//!
//! ```text
//! requires resource_discriminant(rko, rko_type, 1610612736);
//! ensures normal resource_arm_released(rko, rko_u.offset_commit.partitions) when rko_type == 6;
//! ensures normal resource_arm_empty(rko) when rko_type == 2;
//! ensures normal resource_arm_empty(rko) otherwise;
//! ```
//!
//! A declaration carrying one is withheld from the function map with the
//! retaining declarations, like every callback clause.

use serde::{Deserialize, Serialize};

/// `requires resource_discriminant(P, FIELD, MASK)`: the release is chosen by
/// `P->FIELD & ~MASK`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Discriminant {
    /// The parameter whose member is the discriminant.
    pub parameter: usize,
    /// The member path, segments joined by `.`.
    pub field: String,
    /// The bits the body clears before switching (`0`: none).
    pub mask: u64,
}

/// Which discriminant values an arm covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "guard")]
pub enum ArmGuard {
    /// `when FIELD == VALUE`.
    Equals { value: i64 },
    /// `otherwise`: every value no `when` arm names.
    Otherwise,
}

/// What one arm does.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "effect")]
pub enum ArmEffect {
    /// `resource_arm_released(P, PATH)`: the arm releases the object stored at
    /// the member path `PATH` of `P`'s object.
    Released { path: String },
    /// `resource_arm_empty(P)`: the arm releases no member.
    Empty,
}

/// One arm clause.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArmClause {
    pub guard: ArmGuard,
    pub effect: ArmEffect,
}

/// How the arms of one declaration treat one member path, joined over every
/// arm (the cautious join an undecided discriminant takes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmJoin {
    /// Every arm releases it.
    Every,
    /// Some arms release it and some do not: undecided.
    Some,
    /// No arm releases it.
    None,
}

/// The cautious join of `arms` at `path`.
#[must_use]
pub fn join_at(arms: &[ArmClause], path: &str) -> ArmJoin {
    let guards = guards(arms);
    let releasing = guards
        .iter()
        .filter(|guard| {
            arms.iter().any(|arm| {
                arm.guard == **guard
                    && matches!(&arm.effect, ArmEffect::Released { path: released } if released == path)
            })
        })
        .count();
    if releasing == 0 {
        ArmJoin::None
    } else if releasing == guards.len() && has_otherwise(arms) {
        ArmJoin::Every
    } else {
        ArmJoin::Some
    }
}

/// The distinct guards of `arms`, in order.
#[must_use]
pub fn guards(arms: &[ArmClause]) -> Vec<ArmGuard> {
    let mut out: Vec<ArmGuard> = Vec::new();
    for arm in arms {
        if !out.contains(&arm.guard) {
            out.push(arm.guard);
        }
    }
    out
}

/// Whether the arms name the default arm.
#[must_use]
pub fn has_otherwise(arms: &[ArmClause]) -> bool {
    arms.iter().any(|arm| arm.guard == ArmGuard::Otherwise)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ARM_JOIN_VECTORS` (`QCore.OpCallbacks.armJoinVectors`): arms
    /// (`None` guard: `otherwise`; `None` path: an empty arm), a path and
    /// the join.
    #[allow(clippy::type_complexity)]
    const ARM_JOIN_VECTORS: &[(&[(Option<i64>, Option<&str>)], &str, ArmJoin)] = &[
        (
            &[
                (Some(6), Some("u.c.p")),
                (Some(6), Some("u.c.r")),
                (None, None),
            ],
            "u.c.p",
            ArmJoin::Some,
        ),
        (
            &[
                (Some(1), Some("u.a")),
                (Some(2), Some("u.a")),
                (None, Some("u.a")),
            ],
            "u.a",
            ArmJoin::Every,
        ),
        (
            &[(Some(1), Some("u.a")), (Some(2), Some("u.a"))],
            "u.a",
            ArmJoin::Some,
        ),
        (
            &[(Some(1), Some("u.a")), (None, None)],
            "u.b",
            ArmJoin::None,
        ),
    ];

    const LEAN: &str = include_str!("../../../proof/QCore/OpCallbacks.lean");

    #[test]
    fn arm_join_vectors_hold_and_are_the_lean_vectors() {
        let header = "def armJoinVectors : List (List ArmClause × String × Join) := [";
        let start = LEAN.find(header).expect("the Lean list") + header.len();
        let body = &LEAN[start..];
        let rows = body[..body.find("\n]").expect("the list's end")]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_end_matches(',').to_owned())
            .collect::<Vec<_>>();
        let mut rendered = Vec::new();
        for (arms, path, join) in ARM_JOIN_VECTORS {
            let clauses = arms
                .iter()
                .map(|(value, path)| ArmClause {
                    guard: value.map_or(ArmGuard::Otherwise, |value| ArmGuard::Equals { value }),
                    effect: path.map_or(ArmEffect::Empty, |path| ArmEffect::Released {
                        path: path.to_owned(),
                    }),
                })
                .collect::<Vec<_>>();
            assert_eq!(join_at(&clauses, path), *join, "{arms:?} {path}");
            let spell = |value: Option<String>| {
                value.map_or("Option.none".to_owned(), |v| format!("some {v}"))
            };
            rendered.push(format!(
                "([{}], \"{path}\", .{})",
                arms.iter()
                    .map(|(value, path)| format!(
                        "({}, {})",
                        spell(value.map(|value| value.to_string())),
                        spell(path.map(|path| format!("\"{path}\"")))
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
                match join {
                    ArmJoin::Every => "every",
                    ArmJoin::Some => "some",
                    ArmJoin::None => "none",
                }
            ));
        }
        assert_eq!(rows, rendered);
    }
}
