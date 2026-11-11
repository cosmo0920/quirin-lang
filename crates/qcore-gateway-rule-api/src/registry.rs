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

//! The model of `crates/gateway-rules/registry.toml`: one row per rule
//! package, with the crate holding its body, its placements (position and
//! consultation order, or the host predicate it amends), its Lean modules, its
//! rule document, its reserved reason keys and what switching it off means.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::position::Position;
use crate::reason::{ReasonKey, check_reservations};

/// What `QCORE_GATEWAY_RULES_OFF` does to a package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Off {
    /// Listed, the package refuses without being asked: its answers are
    /// removed and the next package (or the host's existing refusal) decides.
    Refuses,
    /// The package cannot be switched off by refusal (a hold, whose refusal
    /// would widen); listing it is refused.
    NotSwitchable,
}

/// One placement of a package at a route-map position: either a row of the
/// position's ordered table (`order`, 1-based) or an arm amending one named
/// predicate of the host there (`arm`). Exactly one of the two is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub position: Position,
    pub order: Option<u16>,
    pub arm: Option<&'static str>,
    /// The host function the placement asks today (a `Parser` method, or the
    /// predicate an arm amends).
    pub route: &'static str,
}

/// One registry row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleEntry {
    /// The rule id (`10s`), or the card name where the code names no rule.
    pub id: &'static str,
    /// The crate holding the rule's body (`qcore-c-gateway` until the rule's
    /// migration card moves it to `crates/gateway-rules/rule-<id>-<name>`).
    pub crate_name: &'static str,
    pub placements: &'static [Placement],
    /// The Lean modules (`QCore.<Module>`) that model the rule.
    pub lean: &'static [&'static str],
    /// The rule's single authority (`docs/agents/rules/<id>.md`, or the card
    /// report where the code names no rule).
    pub document: &'static str,
    pub reason_keys: &'static [ReasonKey],
    pub off: Off,
}

/// A registry that breaks one of [`check_registry`]'s laws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistryError(pub String);

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RegistryError {}

/// A plain rule id: ASCII letters, digits, `.`, `-`, `_` (the spelling
/// `quirin-audit measure --rules-off` accepts).
pub(crate) fn plain_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// The laws every registry keeps: unique plain ids; at least one placement
/// each, with exactly one of `order` and `arm`; per ordered position, the
/// orders are `1..=n` with no gap or repeat; arms unique per position; a
/// package placed at a hold position is not switchable; every Lean module is
/// a `QCore.` name; the reason keys pass [`check_reservations`].
///
/// # Errors
///
/// The first broken law, by rule id.
pub fn check_registry(entries: &[RuleEntry]) -> Result<(), RegistryError> {
    let error = |text: String| Err(RegistryError(text));
    let mut ids = BTreeSet::new();
    let mut orders = BTreeMap::<Position, Vec<u16>>::new();
    let mut arms = BTreeSet::<(Position, &str)>::new();
    for entry in entries {
        if !plain_id(entry.id) {
            return error(format!("rule id `{}` is not a plain id", entry.id));
        }
        if !ids.insert(entry.id) {
            return error(format!("rule {} is registered twice", entry.id));
        }
        if entry.placements.is_empty() {
            return error(format!("rule {} has no placement", entry.id));
        }
        if entry.crate_name.is_empty() || entry.document.is_empty() {
            return error(format!("rule {} names no crate or document", entry.id));
        }
        for module in entry.lean {
            if !module.starts_with("QCore.") || module.len() <= "QCore.".len() {
                return error(format!(
                    "rule {}: Lean module `{module}` is not a QCore module",
                    entry.id
                ));
            }
        }
        for placement in entry.placements {
            match (placement.order, placement.arm) {
                (Some(0), _) => {
                    return error(format!("rule {}: orders start at 1", entry.id));
                }
                (Some(order), None) => orders.entry(placement.position).or_default().push(order),
                (None, Some(arm)) => {
                    if !arms.insert((placement.position, arm)) {
                        return error(format!(
                            "arm `{arm}` at {} is registered twice",
                            placement.position
                        ));
                    }
                }
                _ => {
                    return error(format!(
                        "rule {} at {}: a placement has exactly one of order and arm",
                        entry.id, placement.position
                    ));
                }
            }
            if placement.position.is_hold() && entry.off != Off::NotSwitchable {
                return error(format!(
                    "rule {} is placed at the hold position {}; its refusal would widen, so it \
                     must be registered not switchable",
                    entry.id, placement.position
                ));
            }
        }
    }
    for (position, mut list) in orders {
        list.sort_unstable();
        let expected: Vec<u16> = (1..=u16::try_from(list.len()).unwrap_or(u16::MAX)).collect();
        if list != expected {
            return error(format!(
                "position {position}: consultation orders {list:?} are not 1..={}",
                list.len()
            ));
        }
    }
    check_reservations(
        entries
            .iter()
            .flat_map(|entry| entry.reason_keys.iter().map(move |key| (entry.id, key))),
    )
    .map_err(RegistryError)
}

/// The ordered table of `position`: every placement with an order, by order.
#[must_use]
pub fn ordered_table(
    entries: &'static [RuleEntry],
    position: Position,
) -> Vec<(&'static RuleEntry, &'static Placement)> {
    let mut table: Vec<_> = entries
        .iter()
        .flat_map(|entry| entry.placements.iter().map(move |p| (entry, p)))
        .filter(|(_, placement)| placement.position == position && placement.order.is_some())
        .collect();
    table.sort_by_key(|(_, placement)| placement.order);
    table
}

#[cfg(test)]
mod tests {
    use super::{Off, Placement, RuleEntry, check_registry, ordered_table};
    use crate::position::Position;

    const fn row(order: u16, route: &'static str) -> Placement {
        Placement {
            position: Position::StatementFallback,
            order: Some(order),
            arm: None,
            route,
        }
    }

    const fn entry(id: &'static str, placements: &'static [Placement]) -> RuleEntry {
        RuleEntry {
            id,
            crate_name: "qcore-c-gateway",
            placements,
            lean: &["QCore.Example"],
            document: "docs/agents/rules/x.md",
            reason_keys: &[],
            off: Off::Refuses,
        }
    }

    static GOOD: [RuleEntry; 2] = [
        entry("10s", &[row(2, "elaborated_statement")]),
        entry("W43-INDEX-TRAVERSAL", &[row(1, "stepped_access")]),
    ];

    #[test]
    fn a_well_formed_registry_passes_and_its_table_is_in_order() {
        assert_eq!(check_registry(&GOOD), Ok(()));
        let table: Vec<_> = ordered_table(&GOOD, Position::StatementFallback)
            .into_iter()
            .map(|(entry, _)| entry.id)
            .collect();
        assert_eq!(table, ["W43-INDEX-TRAVERSAL", "10s"]);
        assert!(ordered_table(&GOOD, Position::ReturnFallback).is_empty());
    }

    const A1: &[Placement] = &[row(1, "a")];
    const B1: &[Placement] = &[row(1, "b")];
    const B2: &[Placement] = &[row(2, "b")];
    const B3: &[Placement] = &[row(3, "b")];
    const BOTH: &[Placement] = &[Placement {
        position: Position::Classification,
        order: Some(1),
        arm: Some("x"),
        route: "r",
    }];

    #[test]
    fn broken_registries_are_refused() {
        assert!(check_registry(&[entry("10s", A1), entry("10s", B2)]).is_err());
        assert!(check_registry(&[entry("10s", A1), entry("10u", B3)]).is_err());
        assert!(check_registry(&[entry("10s", A1), entry("10u", B1)]).is_err());
        assert!(check_registry(&[entry("10s", BOTH)]).is_err());
        assert!(check_registry(&[entry("10s", &[])]).is_err());
        assert!(check_registry(&[entry("10 s", A1)]).is_err());
        let mut lean = entry("10s", A1);
        lean.lean = &["Elaboration"];
        assert!(check_registry(&[lean]).is_err());
    }

    #[test]
    fn a_hold_must_not_be_switchable() {
        const HOLD: &[Placement] = &[Placement {
            position: Position::TokenHold,
            order: Some(1),
            arm: None,
            route: "caller_frame_hold",
        }];
        let mut hold = entry("32a", HOLD);
        assert!(check_registry(&[hold]).is_err());
        hold.off = Off::NotSwitchable;
        assert_eq!(check_registry(&[hold]), Ok(()));
    }
}
