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

//! The run-time rule switch (user decision 3 of 2026-10-24):
//! `QCORE_GATEWAY_RULES_OFF`, a comma list of registry ids.

use std::collections::BTreeSet;
use std::fmt;

use crate::registry::{Off, RuleEntry, plain_id};

/// The variable every registry entry reads. `quirin-audit measure
/// --rules-off` sets it and checks that the analyzer binary names it.
pub const RULES_OFF_VARIABLE: &str = "QCORE_GATEWAY_RULES_OFF";

/// A switch list the analyzer refuses to run under. Never ignored: an id the
/// registry does not hold could be a typo for a rule the run would then
/// silently keep on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SwitchError {
    /// Not valid Unicode.
    NotUnicode,
    /// An empty element or one that is not a plain id.
    Malformed(String),
    /// Ids the registry does not hold.
    Unknown(Vec<String>),
    /// Ids whose packages cannot be switched off by refusal (holds).
    NotSwitchable(Vec<String>),
}

impl fmt::Display for SwitchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotUnicode => write!(formatter, "{RULES_OFF_VARIABLE} is not valid Unicode"),
            Self::Malformed(text) => write!(
                formatter,
                "{RULES_OFF_VARIABLE}=`{text}`: expected a comma list of rule ids (`35,10s`)"
            ),
            Self::Unknown(ids) => write!(
                formatter,
                "{RULES_OFF_VARIABLE} names {} the gateway rule registry does not hold \
                 (crates/gateway-rules/registry.toml); an unknown id is refused, never ignored",
                ids.join(", ")
            ),
            Self::NotSwitchable(ids) => write!(
                formatter,
                "{RULES_OFF_VARIABLE} names {}, which cannot be switched off: a hold's refusal \
                 would widen the analysis",
                ids.join(", ")
            ),
        }
    }
}

impl std::error::Error for SwitchError {}

/// The rules a run switches off. Empty when the variable is unset or empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RulesOff {
    ids: BTreeSet<String>,
}

impl RulesOff {
    /// No rule off.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Every switchable rule of `registry` off: what a run under a refused
    /// switch list falls back to, so a library caller that does not stop on
    /// the error still gets only refusals from the packages.
    #[must_use]
    pub fn all(registry: &[RuleEntry]) -> Self {
        Self {
            ids: registry
                .iter()
                .filter(|entry| entry.off == Off::Refuses)
                .map(|entry| entry.id.to_owned())
                .collect(),
        }
    }

    /// Parse a switch list against `registry`. Surrounding spaces of each
    /// element are trimmed; an empty text is the empty list.
    ///
    /// # Errors
    ///
    /// A malformed element, an id the registry does not hold, or an id that
    /// is not switchable.
    pub fn parse(text: &str, registry: &[RuleEntry]) -> Result<Self, SwitchError> {
        if text.trim().is_empty() {
            return Ok(Self::none());
        }
        let mut ids = BTreeSet::new();
        for element in text.split(',').map(str::trim) {
            if !plain_id(element) {
                return Err(SwitchError::Malformed(text.to_owned()));
            }
            ids.insert(element.to_owned());
        }
        let unknown: Vec<String> = ids
            .iter()
            .filter(|id| !registry.iter().any(|entry| entry.id == id.as_str()))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(SwitchError::Unknown(unknown));
        }
        let fixed: Vec<String> = ids
            .iter()
            .filter(|id| {
                registry
                    .iter()
                    .any(|entry| entry.id == id.as_str() && entry.off == Off::NotSwitchable)
            })
            .cloned()
            .collect();
        if !fixed.is_empty() {
            return Err(SwitchError::NotSwitchable(fixed));
        }
        Ok(Self { ids })
    }

    /// Read [`RULES_OFF_VARIABLE`] from the process environment.
    ///
    /// # Errors
    ///
    /// As [`Self::parse`], or a value that is not Unicode.
    pub fn from_environment(registry: &[RuleEntry]) -> Result<Self, SwitchError> {
        match std::env::var_os(RULES_OFF_VARIABLE) {
            None => Ok(Self::none()),
            Some(value) => Self::parse(value.to_str().ok_or(SwitchError::NotUnicode)?, registry),
        }
    }

    /// Whether `id` is switched off.
    #[must_use]
    pub fn is_off(&self, id: &str) -> bool {
        self.ids.contains(id)
    }

    /// The switched-off ids, sorted (what `run.json`, the invocation record
    /// and every report component record).
    #[must_use]
    pub fn ids(&self) -> &BTreeSet<String> {
        &self.ids
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{RulesOff, SwitchError};
    use crate::position::Position;
    use crate::registry::{Off, Placement, RuleEntry};

    const ROW: &[Placement] = &[Placement {
        position: Position::StatementFallback,
        order: Some(1),
        arm: None,
        route: "elaborated_statement",
    }];
    const HOLD: &[Placement] = &[Placement {
        position: Position::TokenHold,
        order: Some(1),
        arm: None,
        route: "caller_frame_hold",
    }];

    const fn entry(id: &'static str, placements: &'static [Placement], off: Off) -> RuleEntry {
        RuleEntry {
            id,
            crate_name: "qcore-c-gateway",
            placements,
            lean: &[],
            document: "d",
            reason_keys: &[],
            off,
        }
    }

    static REGISTRY: [RuleEntry; 3] = [
        entry("10s", ROW, Off::Refuses),
        entry("35", ROW, Off::Refuses),
        entry("32a", HOLD, Off::NotSwitchable),
    ];

    #[test]
    fn a_list_of_registered_ids_parses_and_is_recorded_sorted() {
        let off = RulesOff::parse(" 35 ,10s,35", &REGISTRY).expect("valid");
        assert!(off.is_off("35") && off.is_off("10s") && !off.is_off("32a"));
        assert_eq!(off.ids().iter().collect::<Vec<_>>(), ["10s", "35"]);
        assert_eq!(RulesOff::parse("", &REGISTRY), Ok(RulesOff::none()));
        assert_eq!(RulesOff::parse("  ", &REGISTRY), Ok(RulesOff::none()));
    }

    #[test]
    fn an_unknown_id_is_refused_never_ignored() {
        assert_eq!(
            RulesOff::parse("35,53", &REGISTRY),
            Err(SwitchError::Unknown(vec!["53".into()]))
        );
        assert!(
            RulesOff::parse("53", &REGISTRY)
                .unwrap_err()
                .to_string()
                .contains("never ignored")
        );
    }

    #[test]
    fn malformed_lists_and_holds_are_refused() {
        for text in ["35,", ",35", "35,,10s", "35;10s", "3 5"] {
            assert_eq!(
                RulesOff::parse(text, &REGISTRY),
                Err(SwitchError::Malformed(text.into())),
                "{text}"
            );
        }
        assert_eq!(
            RulesOff::parse("32a", &REGISTRY),
            Err(SwitchError::NotSwitchable(vec!["32a".into()]))
        );
    }

    #[test]
    fn all_switches_off_every_switchable_rule_only() {
        let all = RulesOff::all(&REGISTRY);
        assert_eq!(all.ids().iter().collect::<Vec<_>>(), ["10s", "35"]);
    }
}
