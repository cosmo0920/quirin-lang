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

//! Native policy inputs for the bounded source importers. Structural decoding
//! and operational semantics stay in Rust; QCore owns these acceptance rules.
#[cfg(test)]
pub(crate) use qcore_c_gateway::typed_source::DEFAULT_SOURCE_RULES as RULES;

#[cfg(test)]
fn check(name: &str, facts: &[(&str, i128)]) -> Result<(), String> {
    crate::contract_observation::check_contract_facts(RULES, name, facts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract_observation::check_contract_facts;

    #[test]
    fn every_extracted_rule_accepts_valid_facts_and_rejects_mutations() {
        let cases: &[(&str, &[(&str, i128)])] = &[
            (
                "source_enum_value",
                &[("evaluated_constant", 1), ("type_matches", 1)],
            ),
            (
                "source_scan_effects",
                &[("header_calls", 0), ("match_calls", 0), ("return_calls", 0)],
            ),
            (
                "source_scan_initial",
                &[("remaining_paths", 0), ("index_escaped", 0)],
            ),
            ("source_type_match", &[("type_matches", 1)]),
            ("source_symbol", &[("producer_type_matches", 1)]),
            ("source_loop_index", &[("scope_matches", 1)]),
            ("source_scan_region", &[("read_only", 1)]),
            (
                "frame_write",
                &[("lifecycle_field", 0), ("ordinary_int_field", 1)],
            ),
            (
                "frame_declaration",
                &[("declaration_count", 1), ("all_external_prototypes", 1)],
            ),
            (
                "frame_dependency",
                &[("layout_matches", 1), ("duplicate", 0)],
            ),
        ];
        let parsed = crate::frontend::parse_source_program(RULES).unwrap();
        assert_eq!(parsed.declarations.len(), cases.len());
        for (function, facts) in cases {
            check(function, facts).unwrap();
            for index in 0..facts.len() {
                let mut invalid = facts.to_vec();
                invalid[index].1 = 1 - invalid[index].1;
                assert!(
                    check(function, &invalid).is_err(),
                    "accepted bad {function} fact {index}"
                );
            }
            let start = RULES.find(&format!("extern fn {function}(")).unwrap();
            let start = start + RULES[start..].find("ensures normal ").unwrap();
            let end = start + RULES[start..].find(';').unwrap();
            for predicate in ["false", "unsupported_predicate"] {
                let changed = format!(
                    "{}ensures normal {predicate}{}",
                    &RULES[..start],
                    &RULES[end..]
                );
                assert!(
                    check_contract_facts(&changed, function, facts).is_err(),
                    "accepted mutated {function}"
                );
            }
        }
    }
}
