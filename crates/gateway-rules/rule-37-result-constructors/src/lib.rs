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

//! Gateway rule **37**, checked result constructors (`docs/agents/rules/37.md`;
//! card W48-BU-RK-RESULT-CONSTRUCTORS; migrated by card RULE37-PILOT onto the
//! gateway rule interface, plan `docs/plans/pluggable-cards/README.md` §3 B).
//!
//! The package answers at two host predicates, both registered as arms in
//! `crates/gateway-rules/registry.toml`:
//!
//! * **The typed `sizeof` exemption** ([`CheckedOperandArm`], route
//!   `sizeof_operand::decide`, asked from `aggregate_store::store_bases`). The aggregate store scan reads a name's
//!   mentions to decide whether a local aggregate pointer escapes. A mention
//!   inside a `sizeof` operand that the host's existing typed,
//!   non-variably-modified operand reader admitted is unevaluated (C11
//!   6.5.3.4p2) and is not an escape. The package admits exactly the reader's
//!   checked spans, and only when every one is a well-formed span starting at
//!   a `sizeof` keyword; anything else is refused and the scan keeps its old
//!   reading, in which every mention counts. A syntactic `sizeof` alone never
//!   reaches this package, and the exemption supplies neither an owner
//!   identity nor non-nullness: every other premise of the store scan (unique
//!   local aggregate, complete declarations, no alias or evaluated escape, a
//!   live known non-null owner) stays the host's.
//! * **The `asserted_allocation` premise** ([`SpecificationPremiseArm`],
//!   route `asserted_allocation::decide`, asked from `legacy_checked_spec::settle`). A checked specification (contract
//!   spec §C06.3) whose declaration names [`ASSERTED_ALLOCATION`] is a
//!   wrapper whose normal return is non-null only because allocation failure
//!   terminates. The host asks the package before the specification may stand;
//!   the package admits it with the premise as an
//!   [`Obligation::Premise`], which the declaration must state, so the premise
//!   stays visible in every verdict's assumptions. The host's own settlement
//!   still requires the complete body to establish the specification; the
//!   admission never replaces that check, and the body observation never
//!   proves the deployed-build premise.
//!
//! Switched off (`QCORE_GATEWAY_RULES_OFF=37`), the package is not asked: the
//! scan counts every mention, as before rule 37, and every checked
//! specification naming `asserted_allocation` is withdrawn. Off only removes
//! answers. Rule 37's third part, the null readers' refusal to read a checked
//! custody specification as an unchecked non-null result, is a guard, not an
//! admission: it stays in the host and is never switched off, because removing
//! it would widen.
//!
//! Lean: `QCore.ResultConstructors` (the rule) and
//! `QCore.ResultConstructorSwitch` (the switch: on is the rule, off is the old
//! reading, off never widens). The vectors in this crate's tests mirror the
//! latter's decisions.

pub mod asserted_allocation;
pub mod sizeof_operand;

use qcore_gateway_rule_api::{
    Admission, Answer, CheckedOperandArm, CheckedOperandQuery, Obligation, SpecificationPremiseArm,
    SpecificationQuery, traits::RulePackage,
};

/// The registry id.
pub const ID: &str = "37";

/// The named premise rule 37 owns: the deployed build preserves the analysed
/// assertion and its declared terminating failure call, so allocation failure
/// has no normal return.
pub const ASSERTED_ALLOCATION: &str = "asserted_allocation";

/// The keyword every checked operand span starts at.
const SIZEOF: &str = "sizeof";

/// Rule 37's package.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResultConstructors;

/// The package, for the host's tables.
pub const PACKAGE: ResultConstructors = ResultConstructors;

impl RulePackage for ResultConstructors {
    fn id(&self) -> &'static str {
        ID
    }
}

/// Whether `checked` are well-formed operand spans of `tokens`: each
/// `[start, end)` lies inside `tokens`, starts at a `sizeof` keyword and
/// covers at least one operand token, and the spans are in order and
/// disjoint.
#[must_use]
pub fn well_formed_spans(tokens: &[&str], checked: &[(usize, usize)]) -> bool {
    let mut previous_end = 0;
    checked.iter().all(|&(start, end)| {
        let ok = start >= previous_end
            && start.checked_add(2).is_some_and(|least| least <= end)
            && end <= tokens.len()
            && tokens[start] == SIZEOF;
        previous_end = end;
        ok
    })
}

impl CheckedOperandArm for ResultConstructors {
    fn ask(&self, query: &CheckedOperandQuery<'_>) -> Answer<Vec<(usize, usize)>> {
        if well_formed_spans(query.tokens, query.checked) {
            Answer::admit(query.checked.to_vec())
        } else {
            Answer::Refuse
        }
    }
}

impl SpecificationPremiseArm for ResultConstructors {
    fn premise(&self) -> &'static str {
        ASSERTED_ALLOCATION
    }

    fn ask(&self, query: &SpecificationQuery<'_>) -> Answer<()> {
        if query.premises.contains(&ASSERTED_ALLOCATION) {
            Answer::Admit(Admission {
                lowering: (),
                obligations: vec![Obligation::Premise(ASSERTED_ALLOCATION.to_owned())],
            })
        } else {
            Answer::Refuse
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ASSERTED_ALLOCATION, ID, PACKAGE, well_formed_spans};
    use qcore_gateway_rule_api::{
        Answer, CheckedOperandArm, CheckedOperandQuery, Obligation, SpecificationPremiseArm,
        SpecificationQuery, consult, traits::RulePackage,
    };

    fn tokens(text: &str) -> Vec<&str> {
        text.split_whitespace().collect()
    }

    /// The host's composition: one arm, asked only while enabled.
    fn exempt(on: bool, body: &[&str], checked: &[(usize, usize)]) -> Vec<(usize, usize)> {
        let query = CheckedOperandQuery {
            tokens: body,
            checked,
        };
        consult(&[PACKAGE], |_| on, |_, p| CheckedOperandArm::ask(p, &query))
            .map(|c| c.admission.lowering)
            .unwrap_or_default()
    }

    /// `QCore.ResultConstructorSwitch.escapesUnder`, read over a token index:
    /// a mention escapes unless it lies in an exempt span.
    fn escapes(on: bool, body: &[&str], checked: &[(usize, usize)], at: usize) -> bool {
        !exempt(on, body, checked)
            .iter()
            .any(|(start, end)| (*start..*end).contains(&at))
    }

    #[test]
    fn the_package_answers_as_rule_37() {
        assert_eq!(PACKAGE.id(), ID);
        assert_eq!(
            SpecificationPremiseArm::premise(&PACKAGE),
            ASSERTED_ALLOCATION
        );
    }

    /// Lean `the_switch_on_reads_rule_37` / `the_switch_off_reads_the_old_scan`
    /// and `only_a_checked_operand_is_newly_exempt`: on, a mention inside a
    /// checked span is no escape; off, and outside every span, it is.
    #[test]
    fn a_checked_operand_mention_is_exempt_only_with_the_switch_on() {
        let body = tokens("e = allocate ( sizeof ( * e ) ) ; e -> name = 0 ;");
        let checked = [(4, 9)];
        assert!(!escapes(true, &body, &checked, 7), "checked sizeof(*e)");
        assert!(escapes(false, &body, &checked, 7), "off: the old scan");
        assert!(escapes(true, &body, &checked, 0), "the binding is outside");
        assert!(escapes(true, &body, &checked, 11), "the store is outside");
        assert!(
            escapes(true, &body, &[], 7),
            "an unchecked sizeof is no exemption"
        );
    }

    /// Lean `the_switch_off_never_widens_the_scan`: every mention that escapes
    /// on escapes off.
    #[test]
    fn the_switch_off_never_widens_the_scan() {
        let body = tokens("p = f ( sizeof * p , sizeof ( q [ 2 ] ) ) ;");
        let checked = [(4, 7), (8, 15)];
        for at in 0..body.len() {
            assert!(
                !escapes(true, &body, &checked, at) || escapes(false, &body, &checked, at),
                "{at}"
            );
        }
    }

    /// Fail-closed: spans that do not start at `sizeof`, run past the body,
    /// cover no operand, overlap or are out of order are refused whole.
    #[test]
    fn malformed_spans_are_refused_whole() {
        let body = tokens("x = sizeof ( * p ) + sizeof * q ;");
        assert!(well_formed_spans(&body, &[(2, 7), (8, 11)]));
        assert!(well_formed_spans(&body, &[]));
        for bad in [
            vec![(3, 7)],
            vec![(2, 3)],
            vec![(2, 99)],
            vec![(8, 11), (2, 7)],
            vec![(2, 9), (8, 11)],
        ] {
            assert!(!well_formed_spans(&body, &bad), "{bad:?}");
            assert!(exempt(true, &body, &bad).is_empty(), "{bad:?}");
        }
    }

    fn standing(on: bool, premises: &[&str]) -> Option<Vec<Obligation>> {
        let query = SpecificationQuery {
            function: "rd_calloc",
            premises,
        };
        consult(
            &[PACKAGE],
            |_| on,
            |_, p| SpecificationPremiseArm::ask(p, &query),
        )
        .map(|c| c.admission.obligations)
    }

    /// Lean `the_switch_off_withdraws_a_premised_specification`,
    /// `an_admitted_premised_specification_carries_the_premise` and
    /// `the_switch_off_never_widens_the_specifications`.
    #[test]
    fn the_premise_is_admitted_and_carried_only_with_the_switch_on() {
        let carried = vec![Obligation::Premise(ASSERTED_ALLOCATION.into())];
        assert_eq!(
            standing(true, &[ASSERTED_ALLOCATION]),
            Some(carried.clone())
        );
        assert_eq!(
            standing(true, &["rk_no_plugin_library", ASSERTED_ALLOCATION]),
            Some(carried)
        );
        assert_eq!(standing(false, &[ASSERTED_ALLOCATION]), None);
        assert_eq!(standing(true, &["rk_no_plugin_library"]), None);
        assert_eq!(standing(true, &[]), None);
        assert_eq!(
            SpecificationPremiseArm::ask(
                &PACKAGE,
                &SpecificationQuery {
                    function: "f",
                    premises: &["asserted_allocation_extra"],
                }
            ),
            Answer::Refuse,
            "a premise is named exactly"
        );
    }

    /// Plan seam B: a rule package depends on the rule interface only.
    #[test]
    fn the_package_depends_on_the_rule_interface_only() {
        let manifest = include_str!("../Cargo.toml");
        let mut section = "";
        let mut dependencies = Vec::new();
        for line in manifest.lines().map(str::trim) {
            if line.starts_with('[') {
                section = line;
                continue;
            }
            if section.contains("dependencies") && !line.is_empty() && !line.starts_with('#') {
                dependencies.push(line.split('=').next().unwrap_or("").trim().to_owned());
            }
        }
        assert_eq!(dependencies, ["qcore-gateway-rule-api"]);
    }
}
