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

//! Card **GATEWAY-RULE-API** (plan `docs/plans/pluggable-cards/README.md`,
//! seam B; user decision 3 of 2026-10-24): the gateway's rule registry and
//! its run-time switches.
//!
//! [`REGISTRY`] is generated from `crates/gateway-rules/registry.toml` (`python3
//! scripts/gen_gateway_registry.py`). Every registered rule reads
//! `QCORE_GATEWAY_RULES_OFF` through [`enabled`]: a listed rule's package is
//! not asked (a table row falls through to the next row, an arm keeps its
//! reader's old refusal), so off only removes answers and is fail-closed by
//! construction. The list is read once per process.
//!
//! A list naming an id the registry does not hold, a hold, or a malformed
//! element is refused, never ignored: [`check`] returns the refusal, which
//! `quirin-audit` turns into a failed run before any analysis, and a library
//! caller that does not stop gets every switchable rule off (the analysis
//! only refuses more) with one line on standard error.
//!
//! A hold (`off = "not-switchable"`, rule 32a's caller-frame hold; card
//! RULE32A-PILOT) is on under every list, a refused one included: its
//! admission is a refusal, so removing it would widen. A list naming it is
//! refused, never applied.

use std::collections::BTreeSet;
use std::sync::{Once, OnceLock};

use qcore_gateway_rule_api::{Off, RulesOff, SwitchError};

pub use qcore_gateway_rule_api::RULES_OFF_VARIABLE;
pub use registry::REGISTRY;

#[rustfmt::skip]
#[path = "gateway_rule_registry.rs"]
mod registry;

static SWITCHES: OnceLock<Result<RulesOff, SwitchError>> = OnceLock::new();
static REFUSAL_REPORTED: Once = Once::new();

fn switches() -> &'static Result<RulesOff, SwitchError> {
    SWITCHES.get_or_init(|| RulesOff::from_environment(REGISTRY))
}

/// This run's switch list, or the refusal to run under it.
///
/// # Errors
///
/// The variable names an id the registry does not hold, a rule that is not
/// switchable, or a malformed element.
pub fn check() -> Result<&'static RulesOff, &'static SwitchError> {
    switches().as_ref()
}

/// The rules this run has off, sorted: the list, or every switchable rule
/// when the list was refused. This is what every report component records
/// (`rule_set.off`), so a switch the analyzer did not apply is never recorded
/// as applied, nor the reverse.
#[must_use]
pub fn effective_off() -> BTreeSet<String> {
    match switches() {
        Ok(off) => off.ids().clone(),
        Err(_) => RulesOff::all(REGISTRY).ids().clone(),
    }
}

/// Card RULE-SWITCH-ENTRY-POINTS: the admission of an entry point whose
/// output does not record the rule set it ran under (`rule_set.off`).
///
/// Such an entry point runs only with nothing switched off: a non-empty list
/// would change its answers without its output saying so, and ignoring the
/// list would let a reader believe rules were off that were not. A refused
/// list (unknown id, hold, malformed) is refused here as everywhere.
///
/// # Errors
///
/// The list is refused, or it names any rule: the message names
/// `entry_point`, the list and the route that records it.
pub fn refuse_unrecorded(entry_point: &str) -> Result<(), String> {
    let off = check().map_err(|refusal| format!("{entry_point}: {refusal}"))?;
    if off.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{entry_point} does not record the gateway rule set it runs under, so it refuses \
         {RULES_OFF_VARIABLE}={}; unset it, or run a route that records it \
         (quirin-audit current, pinned or contracts with a report, or a configuration run, \
         each writing rule_set.off; quirin-audit measure --rules-off)",
        off.ids().iter().cloned().collect::<Vec<_>>().join(",")
    ))
}

/// [`refuse_unrecorded`] for a `main` with nothing to return: the refusal is
/// printed on standard error and the process exits with status 1 before
/// anything is analysed.
pub fn exit_if_unrecorded(entry_point: &str) {
    if let Err(refusal) = refuse_unrecorded(entry_point) {
        eprintln!("{refusal}");
        std::process::exit(1);
    }
}

/// Whether `id` is registered as a hold, which no list switches off.
fn is_hold(id: &str) -> bool {
    REGISTRY
        .iter()
        .any(|entry| entry.id == id && entry.off == Off::NotSwitchable)
}

/// Whether rule `id`'s package answers in this run. A hold answers under
/// every list, including a refused one (card RULE32A-PILOT).
pub(crate) fn enabled(id: &str) -> bool {
    debug_assert!(
        REGISTRY.iter().any(|entry| entry.id == id),
        "gateway rule {id} reads its switch but is not registered"
    );
    if is_hold(id) {
        return true;
    }
    #[cfg(test)]
    if let Some(on) = test_override::enabled(id) {
        return on;
    }
    match switches() {
        Ok(off) => !off.is_off(id),
        Err(refusal) => {
            REFUSAL_REPORTED.call_once(|| {
                eprintln!("qcore-c-gateway: {refusal}; every switchable gateway rule is off");
            });
            false
        }
    }
}

/// Unit tests only: run `body` with a switch list in place of the process's,
/// on this thread.
#[cfg(test)]
pub(crate) mod test_override {
    use std::cell::RefCell;

    use qcore_gateway_rule_api::RulesOff;

    thread_local! {
        static OFF: RefCell<Option<RulesOff>> = const { RefCell::new(None) };
    }

    pub(crate) fn enabled(id: &str) -> Option<bool> {
        OFF.with(|off| off.borrow().as_ref().map(|off| !off.is_off(id)))
    }

    pub(crate) fn with_rules_off<R>(list: &str, body: impl FnOnce() -> R) -> R {
        let parsed = RulesOff::parse(list, super::REGISTRY)
            .unwrap_or_else(|error| panic!("test switch list refused: {error}"));
        let previous = OFF.with(|off| off.replace(Some(parsed)));
        let result = body();
        OFF.with(|off| *off.borrow_mut() = previous);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::test_override::with_rules_off;
    use super::{REGISTRY, enabled, is_hold};
    use qcore_gateway_rule_api::{Off, Position, RulesOff, SwitchError, check_registry};

    #[test]
    fn the_registry_keeps_the_api_laws_and_every_rule_but_the_hold_is_switchable() {
        assert_eq!(check_registry(REGISTRY), Ok(()));
        // Card RULE32A-PILOT: rule 32a's caller-frame hold is the one
        // registered hold, and the one rule that is not switchable.
        for entry in REGISTRY {
            let hold = entry
                .placements
                .iter()
                .any(|placement| placement.position.is_hold());
            assert_eq!(
                entry.off,
                if hold {
                    Off::NotSwitchable
                } else {
                    Off::Refuses
                },
                "rule {}",
                entry.id
            );
        }
        let holds: Vec<&str> = REGISTRY
            .iter()
            .filter(|entry| entry.off == Off::NotSwitchable)
            .map(|entry| entry.id)
            .collect();
        assert_eq!(holds, ["32a"]);
        // Card RULE35-PILOT: every rule's body is in the gateway or in a
        // rule package the gateway hosts (`rule_hosts`).
        for entry in REGISTRY {
            assert!(
                HOSTED.contains(&(entry.id, entry.crate_name))
                    || entry.crate_name == "qcore-c-gateway",
                "rule {} names crate {}, which the gateway does not host",
                entry.id,
                entry.crate_name
            );
        }
    }

    /// The rule packages the gateway hosts: (registry id, crate). Rule 35 is
    /// asked through `rule_hosts`, rule 37 through `result_constructor_rule`.
    const HOSTED: &[(&str, &str)] = &[
        (rule_33_removed_selection::ID, "rule-33-removed-selection"),
        (rule_34_element_destroyer::ID, "rule-34-element-destroyer"),
        (rule_32a_caller_frame_hold::ID, "rule-32a-caller-frame-hold"),
        (rule_35_literal_decay::ID, "rule-35-literal-decay"),
        (
            rule_36_exact_formal_return::ID,
            "rule-36-exact-formal-return",
        ),
        (
            rule_37_result_constructors::ID,
            "rule-37-result-constructors",
        ),
        (rule_38_descriptor_cells::ID, "rule-38-descriptor-cells"),
    ];

    #[test]
    fn every_hosted_package_answers_under_its_registry_id() {
        use qcore_gateway_rule_api::traits::RulePackage;
        assert_eq!(rule_35_literal_decay::LiteralRead.id(), "35");
        assert_eq!(rule_35_literal_decay::ArrayMemberDecay.id(), "35");
        assert_eq!(
            rule_36_exact_formal_return::ExactFormalReturnRoute.id(),
            "36"
        );
        for (id, crate_name) in HOSTED {
            let entry = REGISTRY
                .iter()
                .find(|entry| entry.id == *id)
                .expect("a hosted package is registered");
            assert_eq!(entry.crate_name, *crate_name);
        }
    }

    /// Card RULE32A-PILOT (GATEWAY-RULE-API-HOLD-OFF): the hold is fixed.
    /// Its row is the package's, at the token hold, reserving the package's
    /// reason key; a list naming it is refused, alone or beside a switchable
    /// rule; it stays on under every accepted list; and the fallback of a
    /// refused list (every switchable rule off) leaves it on.
    #[test]
    fn rule_32a_is_a_fixed_hold() {
        use qcore_gateway_rule_api::traits::RulePackage;
        let entry = REGISTRY
            .iter()
            .find(|entry| entry.id == "32a")
            .expect("rule 32a is registered");
        assert_eq!(entry.crate_name, "rule-32a-caller-frame-hold");
        assert_eq!(entry.off, Off::NotSwitchable);
        assert_eq!(
            entry
                .placements
                .iter()
                .map(|placement| (placement.position, placement.arm, placement.route))
                .collect::<Vec<_>>(),
            [(
                Position::TokenHold,
                Some("caller-frame-hold"),
                "hold::decide"
            )]
        );
        assert_eq!(
            entry.reason_keys,
            [rule_32a_caller_frame_hold::SELF_RELIANCE]
        );
        assert_eq!(rule_32a_caller_frame_hold::CallerFrameHold.id(), entry.id);
        assert_eq!(
            crate::legacy_c::caller_frame_hold::CALLER_FRAME_SELF_RELIANCE,
            entry.reason_keys[0].key
        );
        for list in ["32a", " 32a ", "35,32a", "32a,10s"] {
            assert_eq!(
                RulesOff::parse(list, REGISTRY),
                Err(SwitchError::NotSwitchable(vec!["32a".into()])),
                "{list}"
            );
        }
        assert!(is_hold("32a") && !is_hold("35"));
        assert!(enabled("32a"));
        with_rules_off("35,10s,33,34,36,37", || assert!(enabled("32a")));
        assert!(!RulesOff::all(REGISTRY).is_off("32a"));
    }

    #[test]
    fn a_listed_rule_is_off_and_the_others_stay_on() {
        with_rules_off("35,10s", || {
            assert!(!enabled("35") && !enabled("10s"));
            assert!(enabled("10u") && enabled("12f"));
        });
        assert!(enabled("35"), "the override ends with its body");
    }

    #[test]
    fn an_unknown_or_malformed_list_is_refused() {
        assert_eq!(
            RulesOff::parse("35,99z", REGISTRY),
            Err(SwitchError::Unknown(vec!["99z".into()]))
        );
        assert!(matches!(
            RulesOff::parse("35,,10s", REGISTRY),
            Err(SwitchError::Malformed(_))
        ));
    }

    /// Each replaced compile-time arm constant is now its rule's switch: on,
    /// the arm admits; off, its reader's old refusal stands.
    #[test]
    fn the_arm_switches_restore_the_old_refusals() {
        use crate::rule_hosts::{decay_reads, literal_read};
        use crate::scalar_element_address::shape_admitted;
        use crate::typed_const_pointer_value::pointer_value;
        let decayed = serde_json::json!({
            "kind": "ImplicitCastExpr", "castKind": "ArrayToPointerDecay",
            "inner": [{ "kind": "MemberExpr" }]
        });
        let const_value = serde_json::json!({
            "kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
            "type": { "qualType": "struct p *const" }
        });
        let literal_star = ["*", "__qcore_literal__"];
        assert!(literal_read(&literal_star, 0));
        assert!(decay_reads(&decayed));
        assert!(shape_admitted("ArraySubscriptExpr", true));
        assert!(pointer_value(&const_value));
        with_rules_off("35", || {
            assert!(!literal_read(&literal_star, 0));
            assert!(!decay_reads(&decayed));
            assert!(shape_admitted("ArraySubscriptExpr", true), "14c stays on");
        });
        with_rules_off("14c", || {
            assert!(!shape_admitted("ArraySubscriptExpr", true));
            assert!(
                shape_admitted("MemberExpr", true),
                "rule 10n(1)'s member stands"
            );
        });
        with_rules_off("10m", || {
            assert!(!pointer_value(&const_value));
            assert!(literal_read(&literal_star, 0), "35 stays on");
        });
    }
}
