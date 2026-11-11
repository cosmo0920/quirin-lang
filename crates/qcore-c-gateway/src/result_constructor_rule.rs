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

//! Card **RULE37-PILOT**: the host side of rule 37's package
//! (`rule_37_result_constructors`, `crates/gateway-rules/rule-37-result-constructors`).
//!
//! The package's body lives in its crate; this module only builds the
//! queries from the gateway's own readings and composes the answers through
//! [`crate::arm_obligations::consult`], with the package's run-time switch
//! ([`crate::gateway_rules::enabled`]). With the switch on, every decision is
//! the one the gateway made before the move (identity run in
//! `docs/reports/rule37-pilot/`); off, the package is not asked: the store
//! scan counts every mention, as before rule 37, and every checked
//! specification naming `asserted_allocation` is withdrawn.

use std::collections::{BTreeMap, BTreeSet};

use qcore_gateway_rule_api::traits::RulePackage;
use qcore_gateway_rule_api::{
    CheckedOperandArm, CheckedOperandQuery, Obligation, SpecificationPremiseArm, SpecificationQuery,
};
use qcore_schemas::contract::ResourceContracts;
use rule_37_result_constructors::{ID, PACKAGE};

use crate::typed_field_access::TypedAccessModel;

/// Arm `sizeof-operand-exemption` at `aggregate_store::store_bases`: the
/// operand spans of `body` whose mentions the store scan reads as
/// unevaluated. The host's typed, non-variably-modified operand reader
/// (`sizeof_operand`, card W44-COLLECTION-ALLOCATION) decides which spans are
/// checked, and is read only when the package is asked; a refusal, or the
/// package off, is no span, which is the scan before rule 37.
pub(crate) fn exempt_operand_spans(
    model: &TypedAccessModel,
    parameters: &[&str],
    body: &[&str],
) -> Vec<(usize, usize)> {
    crate::arm_obligations::consult(
        &[PACKAGE],
        |package| package.id(),
        |_| crate::gateway_rules::enabled(ID),
        |_, package| {
            let operand_types = crate::sizeof_operand::OperandTypes::read(model, parameters, body);
            let checked = crate::sizeof_operand::unevaluated_spans(body, &operand_types);
            CheckedOperandArm::ask(
                package,
                &CheckedOperandQuery {
                    tokens: body,
                    checked: &checked,
                },
            )
        },
    )
    .map(|decided| decided.lowering)
    .unwrap_or_default()
}

/// Arm `asserted-allocation-premise` at `legacy_checked_spec::settle`: which
/// checked specifications naming the premise rule 37 owns are withdrawn, and
/// the obligations each admitted one rests on.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct PremiseSettlement {
    /// Withdrawn: rule 37 does not admit them, or admits them with a premise
    /// their declaration does not state.
    pub(crate) withdrawn: BTreeSet<String>,
    /// Card ARM-OBLIGATIONS: every admitted specification with its
    /// admission's obligations, which the settlement carries into the unit's
    /// rows while the specification stands.
    pub(crate) admitted: BTreeMap<String, crate::arm_obligations::Carried>,
}

/// The settlement of [`PremiseSettlement`] over the unit's environment.
///
/// `None` when the environment's declared premises cannot be read: the
/// caller then withdraws every checked specification (fail-closed, as for an
/// unreadable checked set).
pub(crate) fn premise_settlement(
    contracts: &ResourceContracts,
    checked: &BTreeSet<String>,
) -> Option<PremiseSettlement> {
    if checked.is_empty() {
        return Some(PremiseSettlement::default());
    }
    let declared = qcore_contract::contract_loader::declared_premises(contracts).ok()?;
    Some(settle_premises(
        &declared_names(&declared),
        checked,
        crate::gateway_rules::enabled,
    ))
}

/// The named premises each declaration states (`requires
/// resource_premise(NAME, ...)`), literal-argument premises excluded.
fn declared_names(
    declared: &BTreeMap<String, Vec<qcore_contract::contract_loader::DeclaredPremise>>,
) -> BTreeMap<String, Vec<String>> {
    declared
        .iter()
        .map(|(function, premises)| {
            (
                function.clone(),
                premises
                    .iter()
                    .filter(|premise| {
                        premise.origin == qcore_contract::contract_loader::PremiseOrigin::Declared
                    })
                    .map(|premise| premise.name.clone())
                    .collect(),
            )
        })
        .collect()
}

/// The decision of [`premise_settlement`] over already-read premise names,
/// with the switch as a parameter (`QCore.ResultConstructorSwitch.inForce`
/// with `established` left to the settlement). An admitted specification
/// keeps its admission's obligations (`QCore.ArmObligations`): its premises
/// for the rows' assumptions, an open obligation for their denominator.
fn settle_premises(
    premises: &BTreeMap<String, Vec<String>>,
    checked: &BTreeSet<String>,
    enabled: impl Fn(&str) -> bool,
) -> PremiseSettlement {
    let owned = SpecificationPremiseArm::premise(&PACKAGE);
    let mut settlement = PremiseSettlement::default();
    for function in checked {
        let names = premises
            .get(function.as_str())
            .map(|names| names.iter().map(String::as_str).collect::<Vec<_>>())
            .unwrap_or_default();
        if !names.contains(&owned) {
            continue;
        }
        let held = crate::arm_obligations::consult_held(
            &[PACKAGE],
            |package| package.id(),
            |_| enabled(ID),
            |_, package| {
                SpecificationPremiseArm::ask(
                    package,
                    &SpecificationQuery {
                        function,
                        premises: &names,
                    },
                )
            },
        );
        match held {
            // The lowering is the specification standing; its obligations
            // go with it.
            Some(crate::arm_obligations::Held {
                lowering: (),
                obligations,
                carried,
            }) if obligations.iter().all(|obligation| match obligation {
                Obligation::Premise(name) => names.contains(&name.as_str()),
                Obligation::Open(_) => true,
            }) =>
            {
                settlement.admitted.insert(function.clone(), carried);
            }
            _ => {
                settlement.withdrawn.insert(function.clone());
            }
        }
    }
    settlement
}

#[cfg(test)]
mod tests {
    use super::settle_premises;
    use crate::arm_obligations::{Carriage, Carried};
    use std::collections::{BTreeMap, BTreeSet};

    fn withdrawn_by_premise(
        premises: &BTreeMap<String, Vec<String>>,
        checked: &BTreeSet<String>,
        enabled: impl Fn(&str) -> bool,
    ) -> BTreeSet<String> {
        settle_premises(premises, checked, enabled).withdrawn
    }

    fn names(entries: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(f, p)| ((*f).to_owned(), p.iter().map(|s| (*s).to_owned()).collect()))
            .collect()
    }

    /// `QCore.ResultConstructorSwitch`: off withdraws exactly the checked
    /// specifications naming `asserted_allocation`; on withdraws none; a
    /// specification without the premise, or not checked, ignores the switch.
    #[test]
    fn the_switch_withdraws_exactly_the_premised_specifications() {
        let premises = names(&[
            ("rd_calloc", &["asserted_allocation"]),
            (
                "rd_strdup",
                &["rk_no_plugin_library", "asserted_allocation"],
            ),
            ("other", &["rk_no_plugin_library"]),
            ("unchecked", &["asserted_allocation"]),
        ]);
        let checked: BTreeSet<String> = ["rd_calloc", "rd_strdup", "other", "bare"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert!(withdrawn_by_premise(&premises, &checked, |_| true).is_empty());
        assert_eq!(
            withdrawn_by_premise(&premises, &checked, |id| id != "37"),
            BTreeSet::from(["rd_calloc".to_owned(), "rd_strdup".to_owned()])
        );
        assert!(
            withdrawn_by_premise(&premises, &BTreeSet::new(), |_| false).is_empty(),
            "nothing checked, nothing withdrawn"
        );
    }

    /// Card ARM-OBLIGATIONS: an admitted specification keeps its admission's
    /// premise for the rows resting on it; a withdrawn one keeps none.
    #[test]
    fn an_admitted_specification_keeps_its_premise() {
        let premises = names(&[
            ("rd_calloc", &["asserted_allocation"]),
            ("other", &["rk_no_plugin_library"]),
        ]);
        let checked: BTreeSet<String> = ["rd_calloc", "other"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let on = settle_premises(&premises, &checked, |_| true);
        assert!(on.withdrawn.is_empty());
        assert_eq!(
            on.admitted,
            BTreeMap::from([(
                "rd_calloc".to_owned(),
                Carried {
                    premises: BTreeSet::from([Carriage {
                        rule: "37".into(),
                        name: "asserted_allocation".into(),
                    }]),
                    open: BTreeSet::new(),
                }
            )])
        );
        let off = settle_premises(&premises, &checked, |id| id != "37");
        assert!(off.admitted.is_empty(), "withdrawn: nothing rests on it");
        assert_eq!(off.withdrawn, BTreeSet::from(["rd_calloc".to_owned()]));
    }
}
