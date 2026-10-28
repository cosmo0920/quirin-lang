//! Card **RULE37-PILOT**: the host side of rule 37's package
//! (`rule_37_result_constructors`, `crates/gateway-rules/rule-37-result-constructors`).
//!
//! The package's body lives in its crate; this module only builds the
//! queries from the gateway's own readings and composes the answers through
//! [`qcore_gateway_rule_api::consult`], with the package's run-time switch
//! ([`crate::gateway_rules::enabled`]). With the switch on, every decision is
//! the one the gateway made before the move (identity run in
//! `docs/reports/rule37-pilot/`); off, the package is not asked: the store
//! scan counts every mention, as before rule 37, and every checked
//! specification naming `asserted_allocation` is withdrawn.

use std::collections::{BTreeMap, BTreeSet};

use qcore_gateway_rule_api::{
    CheckedOperandArm, CheckedOperandQuery, Obligation, SpecificationPremiseArm,
    SpecificationQuery, consult,
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
    consult(
        &[PACKAGE],
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
    .map(|consulted| consulted.admission.lowering)
    .unwrap_or_default()
}

/// Arm `asserted-allocation-premise` at `legacy_checked_spec::settle`: the
/// checked specifications among `checked` that must be withdrawn because they
/// name the premise rule 37 owns and rule 37 does not admit them, or admits
/// them with an obligation their declaration does not state.
///
/// `None` when the environment's declared premises cannot be read: the
/// caller then withdraws every checked specification (fail-closed, as for an
/// unreadable checked set).
pub(crate) fn premise_withdrawals(
    contracts: &ResourceContracts,
    checked: &BTreeSet<String>,
) -> Option<BTreeSet<String>> {
    if checked.is_empty() {
        return Some(BTreeSet::new());
    }
    let declared = qcore_contract::contract_loader::declared_premises(contracts).ok()?;
    Some(withdrawn_by_premise(
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

/// The decision of [`premise_withdrawals`] over already-read premise names,
/// with the switch as a parameter (`QCore.ResultConstructorSwitch.inForce`
/// with `established` left to the settlement).
fn withdrawn_by_premise(
    premises: &BTreeMap<String, Vec<String>>,
    checked: &BTreeSet<String>,
    enabled: impl Fn(&str) -> bool,
) -> BTreeSet<String> {
    let owned = SpecificationPremiseArm::premise(&PACKAGE);
    checked
        .iter()
        .filter(|function| {
            let names = premises
                .get(function.as_str())
                .map(|names| names.iter().map(String::as_str).collect::<Vec<_>>())
                .unwrap_or_default();
            if !names.contains(&owned) {
                return false;
            }
            let admitted = consult(
                &[PACKAGE],
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
            !admitted.is_some_and(|consulted| {
                consulted
                    .admission
                    .obligations
                    .iter()
                    .all(|obligation| match obligation {
                        Obligation::Premise(name) => names.contains(&name.as_str()),
                        Obligation::Open(_) => false,
                    })
            })
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::withdrawn_by_premise;
    use std::collections::{BTreeMap, BTreeSet};

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
}
