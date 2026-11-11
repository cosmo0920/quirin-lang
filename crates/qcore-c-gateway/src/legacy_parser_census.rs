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

//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the parser's measurement-only
//! census hooks (feature `indirect-census`).
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

impl Parser<'_> {
    /// Card **C-GATEWAY-INDIRECT-CENSUS-2**: hand one already-decided
    /// `indirect memory access` refusal to the instrumented census.
    ///
    /// Called only after the classifier has committed to refusing, so it can
    /// neither admit nor refuse anything. Compiled out with the feature off.
    #[cfg(feature = "indirect-census")]
    pub(super) fn census_indirect(
        &self,
        texts: &[&str],
        exempt: &[(usize, usize)],
        refusal: &crate::indirect_census::Refusal,
    ) {
        // A derived-result validation parse re-reads a body the ordinary parse
        // already recorded; recording it again would double-count. So would the
        // call argument rule's second classification of a refused statement
        // (card W44-INDIRECT-CALL-ARGUMENT-ACCESS).
        if self.admit_derived_returns || self.argument_probe.rerun {
            return;
        }
        crate::indirect_census::record(
            &crate::indirect_census::Context {
                function: &self.census_function,
                formal_parameters: &self.formal_parameters,
                typed_model: &self.typed_model,
                typed_types: &self.typed_types,
                typed_bases: &self.typed_bases,
            },
            texts,
            exempt,
            refusal,
            &self.census_bindings,
        );
    }

    /// Card **W43-INDIRECT-ACCESS-GENERAL**: hand one already-classified
    /// statement to the statement census when the classifier refused it, or
    /// named an indirect destination that only an out-parameter contract can
    /// sanction. Compiled out with the feature off; it reads the instruction
    /// after it was produced and cannot change it.
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: record one element null test
    /// the condition parser lowered. Compiled out with the feature off.
    #[cfg(feature = "indirect-census")]
    pub(super) fn census_element_condition(&self, line: usize, condition: &[Token], base: &str) {
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        crate::indirect_census::record_statement(
            &crate::indirect_census::Context {
                function: &self.census_function,
                formal_parameters: &self.formal_parameters,
                typed_model: &self.typed_model,
                typed_types: &self.typed_types,
                typed_bases: &self.typed_bases,
            },
            line,
            &texts,
            format!(
                "element null test over `{base}`; fallback: unmodeled call, aggregate or indirect access in branch condition"
            ),
            &self.census_bindings,
        );
    }

    #[cfg(feature = "indirect-census")]
    pub(super) fn census_statement(
        &self,
        line: usize,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) {
        let outcome = match instruction {
            LegacyInstruction::Unknown { description, .. } => description.clone(),
            LegacyInstruction::Alias { destination, .. }
            | LegacyInstruction::Clear { destination }
                if is_indirect_place(destination) =>
            {
                format!("indirect destination `{destination}`")
            }
            LegacyInstruction::ContractCall {
                destination: Some(destination),
                ..
            } if is_indirect_place(destination) => {
                format!("indirect call destination `{destination}`")
            }
            // Card W43-INDEXED-RESOURCE-COLLECTIONS: an element operation the
            // classifier lowered, with the refusal it keeps as its fallback.
            LegacyInstruction::ElementStore { base, fallback, .. }
            | LegacyInstruction::ElementRelease { base, fallback, .. } => {
                let LegacyInstruction::Unknown { description, .. } = fallback.as_ref() else {
                    return;
                };
                format!("element operation over `{base}`; fallback: {description}")
            }
            _ => return,
        };
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        crate::indirect_census::record_statement(
            &crate::indirect_census::Context {
                function: &self.census_function,
                formal_parameters: &self.formal_parameters,
                typed_model: &self.typed_model,
                typed_types: &self.typed_types,
                typed_bases: &self.typed_bases,
            },
            line,
            &texts,
            outcome,
            &self.census_bindings,
        );
    }

    /// Card **W43-INDEX-TRAVERSAL**: hand one already-decided controlling
    /// expression to the traversal census when the classifier refused it.
    /// Compiled out with the feature off.
    #[cfg(feature = "indirect-census")]
    pub(super) fn census_traversal_condition(
        &self,
        line: usize,
        position: &'static str,
        condition: &[Token],
        refused: bool,
    ) {
        if !refused {
            return;
        }
        let texts = condition
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        crate::traversal_census::record_condition(
            &self.census_function,
            line,
            position,
            &texts,
            contains_call(condition) || contains_braces(condition),
            &self.typed_model,
            &self.typed_types,
        );
        // Card W44-INDIRECT-CHAIN-ACCESS: the member-chain census.
        crate::indirect_chain_census::record_condition(
            &self.chain_census_context(),
            line,
            position,
            contains_call(condition) || contains_braces(condition),
            &texts,
        );
    }

    /// Card **W45-SUB-OBJECT-STORES**: hand one already-classified statement
    /// whose left-hand side the lexical gate reads as a sub-object store to the
    /// sub-object store census. Compiled out with the feature off; it reads the
    /// instruction after it was produced and cannot change it.
    #[cfg(feature = "indirect-census")]
    pub(super) fn census_sub_object_store(
        &self,
        line: usize,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) {
        use crate::sub_object_store_census as census;
        if !census::collecting() || self.admit_derived_returns || self.argument_probe.rerun {
            return;
        }
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let Some(equal) = texts.iter().position(|text| *text == "=") else {
            return;
        };
        let left = &texts[..equal];
        let sub_object = direct_assignment_destination(left).is_some_and(|place| {
            place.contains('.') || place.contains('[') || place.contains("->")
        }) || names_a_sub_object(left);
        if !sub_object {
            return;
        }
        let (outcome, description) = match instruction {
            LegacyInstruction::Unknown { description, .. } => {
                ("Unknown".to_owned(), Some(description.clone()))
            }
            other => {
                let debug = format!("{other:?}");
                let name = debug
                    .split(|character: char| !character.is_ascii_alphanumeric())
                    .next()
                    .unwrap_or_default()
                    .to_owned();
                (name, None)
            }
        };
        let context = crate::indirect_census::Context {
            function: &self.census_function,
            formal_parameters: &self.formal_parameters,
            typed_model: &self.typed_model,
            typed_types: &self.typed_types,
            typed_bases: &self.typed_bases,
        };
        let right = &texts[equal + 1..];
        let callees = right
            .windows(2)
            .filter(|pair| is_identifier(pair[0]) && pair[1] == "(")
            .map(|pair| {
                let contract = self.contracts.contract(pair[0]);
                census::CalleeFact {
                    callee: pair[0].to_owned(),
                    result: contract.map(|contract| format!("{:?}", contract.result)),
                    result_kind: contract.map(|contract| format!("{:?}", contract.result_kind)),
                    parameters_borrowed: contract.map(|contract| {
                        contract
                            .parameters
                            .iter()
                            .all(|mode| matches!(mode, ResourceParameterMode::Borrowed))
                    }),
                }
            })
            .collect();
        let reader = match sub_object_store::read_destination(
            left,
            &self.typed_model,
            &self.typed_types,
            &self.sub_object_roots,
        ) {
            Err(refusal) => format!("destination:{refusal}"),
            Ok(destination) => match sub_object_store::read_value(
                right,
                destination.member,
                &self.typed_model,
                &self.typed_types,
            ) {
                Err(refusal) => format!("value:{refusal}"),
                Ok(value) => format!(
                    "read:{:?}:{}",
                    destination.member,
                    match value {
                        sub_object_store::ValueSyntax::Scalar(_) => "scalar",
                        sub_object_store::ValueSyntax::Untracked => "untracked",
                        sub_object_store::ValueSyntax::Name(_) => "name",
                        sub_object_store::ValueSyntax::Call(_) => "call",
                    },
                ),
            },
        };
        census::record(&census::StoreRecord {
            unit_seq: crate::indirect_census::unit_seq(),
            function: self.census_function.clone(),
            line,
            texts: texts.iter().map(|text| (*text).to_owned()).collect(),
            equal,
            comparison: texts.get(equal + 1) == Some(&"="),
            outcome,
            description,
            admitted: !matches!(instruction, LegacyInstruction::Unknown { .. }),
            formals: census::names_where(&texts, |name| self.formal_parameters.contains(name)),
            declared: texts
                .iter()
                .filter(|text| crate::typed_field_access::is_name(text))
                .filter_map(|name| {
                    crate::indirect_census::declared_label(&context, name)
                        .map(|label| ((*name).to_owned(), label))
                })
                .collect(),
            may_bind: census::names_where(&texts, |name| self.typed_types.may_bind(name)),
            never_bound: census::names_where(&texts, |name| self.typed_bases.never_bound(name)),
            owned_names: census::names_where(&texts, |name| {
                self.census_bindings.owned.contains(name)
            }),
            derived_names: census::names_where(&texts, |name| {
                self.census_bindings.derived.contains(name)
            }),
            descriptors: census::names_where(&texts, |name| self.descriptor_names.contains(name)),
            destination_chains: crate::indirect_census::chains(
                &context,
                &texts,
                &(0..equal).collect::<Vec<_>>(),
            ),
            callees,
            reader,
        });
    }

    /// Card **W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE**: record one clause
    /// whose left-hand side's first place-spelling suffix starts after index
    /// 0. Compiled out with the feature off; it reads the instruction after it
    /// was produced and cannot change it.
    #[cfg(feature = "indirect-census")]
    pub(super) fn census_destination_suffix(
        &self,
        line: usize,
        clause: &'static str,
        tokens: &[Token],
        texts: &[&str],
        instruction: &LegacyInstruction,
    ) {
        use crate::destination_suffix_census as census;
        if !census::collecting() {
            return;
        }
        let Some((equal, start, place)) = census::strict_suffix(texts) else {
            return;
        };
        let prefix = &texts[..start];
        let declarator_prefix = is_declarator_prefix(prefix);
        let (outcome, description) = match instruction {
            LegacyInstruction::Unknown { description, .. } => {
                ("Unknown".to_owned(), Some(description.clone()))
            }
            other => {
                let debug = format!("{other:?}");
                let name = debug
                    .split(|character: char| !character.is_ascii_alphanumeric())
                    .next()
                    .unwrap_or_default()
                    .to_owned();
                (name, None)
            }
        };
        let (named_root, forecast) = if declarator_prefix {
            (None, None)
        } else {
            let root = census::place_root(&place).to_owned();
            let body = find_function_body(self.tokens, &self.census_function).and_then(
                |(body_start, _)| {
                    matching(self.tokens, body_start, "{", "}")
                        .map(|body_end| &self.tokens[body_start + 1..body_end])
                },
            );
            let body_texts = body
                .map(|body| {
                    body.iter()
                        .map(|token| (token.text.as_str(), token.line))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let plain = body_texts.iter().map(|(text, _)| *text).collect::<Vec<_>>();
            let kind = if self.formal_parameters.contains(&root) {
                "formal"
            } else if crate::publication::body_declarations(&plain)
                .0
                .contains(&root)
            {
                "local"
            } else {
                let unit = self
                    .tokens
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect::<Vec<_>>();
                if crate::publication::file_scope_objects(&unit).contains(&root) {
                    "file-scope"
                } else {
                    "undeclared"
                }
            };
            let (later_mentions, later_call_argument) =
                census::later_mentions(&body_texts, &root, line);
            let contains_call = contains_call(tokens) || contains_braces(tokens);
            let refusal = LegacyInstruction::Unknown {
                description: "indirect memory access is outside the resource projection".into(),
                writes: self.refused_writes(texts),
            };
            let forecast = census::Forecast {
                contains_call,
                scalar_field_noop: !contains_call
                    && crate::typed_field_access::scalar_field_noop_pointer_bases(
                        texts,
                        &self.typed_model,
                        &self.typed_types,
                    )
                    .is_some(),
                scalar_member_store: !contains_call
                    && crate::typed_field_access::scalar_member_store_base(
                        texts,
                        &self.typed_model,
                        &self.typed_types,
                    )
                    .is_some(),
                typed_scalar: !contains_call
                    && crate::typed_field_access::typed_scalar_statement(
                        texts,
                        &self.typed_model,
                        &self.typed_types,
                    )
                    .is_some(),
                chain: !contains_call
                    && crate::indirect_chain_access::chain_statement(
                        texts,
                        &self.typed_model,
                        &self.typed_types,
                        &self.typed_bases,
                    )
                    .is_some(),
                stepped: clause == "statement" && self.stepped_access(tokens, &refusal).is_some(),
            };
            (
                Some(census::NamedRoot {
                    root,
                    kind,
                    later_mentions,
                    later_call_argument,
                }),
                Some(forecast),
            )
        };
        census::record(census::SuffixRecord {
            unit_seq: crate::indirect_census::unit_seq(),
            function: self.census_function.clone(),
            line,
            clause,
            texts: texts.iter().map(|text| (*text).to_owned()).collect(),
            equal,
            start,
            place,
            prefix: prefix.iter().map(|text| (*text).to_owned()).collect(),
            declarator_prefix,
            admitted: !matches!(instruction, LegacyInstruction::Unknown { .. }),
            outcome,
            description,
            named_root,
            forecast,
        });
    }

    /// Card **W44-INDIRECT-CHAIN-ACCESS**: the declaration facts the
    /// member-chain census reads. Compiled out with the feature off.
    #[cfg(feature = "indirect-census")]
    pub(super) fn chain_census_context(&self) -> crate::indirect_chain_census::ChainContext<'_> {
        crate::indirect_chain_census::ChainContext {
            function: &self.census_function,
            formal_parameters: &self.formal_parameters,
            model: &self.typed_model,
            types: &self.typed_types,
            bases: &self.typed_bases,
        }
    }

    /// Card **W43-INDEX-TRAVERSAL**: hand one already-classified statement or
    /// `for` clause to the traversal census. Compiled out with the feature off;
    /// it reads the instruction after it was produced and cannot change it.
    #[cfg(feature = "indirect-census")]
    pub(super) fn census_traversal_statement(
        &self,
        line: usize,
        clause: &'static str,
        tokens: &[Token],
        instruction: &LegacyInstruction,
    ) {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let outcome = match instruction {
            LegacyInstruction::Unknown { description, .. } => Some(description.clone()),
            LegacyInstruction::Alias { destination, .. }
            | LegacyInstruction::Clear { destination }
                if is_indirect_place(destination) =>
            {
                Some(format!("indirect destination `{destination}`"))
            }
            _ => None,
        };
        // Card W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: the destination-suffix
        // census, for every clause whatever its outcome.
        self.census_destination_suffix(line, clause, tokens, &texts, instruction);
        // Card W44-INDIRECT-CHAIN-ACCESS: the member-chain census, for the
        // refusals the typed route is consulted for and every other one.
        if let Some(outcome) = &outcome {
            let consulted = !matches!(instruction, LegacyInstruction::Unknown { .. })
                || TYPED_SCALAR_CONSULTED.contains(&outcome.as_str());
            crate::indirect_chain_census::record_statement(
                &self.chain_census_context(),
                line,
                clause,
                outcome.clone(),
                consulted,
                contains_call(tokens) || contains_braces(tokens),
                &texts,
            );
        }
        let copy = match instruction {
            LegacyInstruction::Alias {
                destination,
                source,
            } if is_plain_name_place(destination) && is_plain_name_place(source) => {
                if self.typed_types.pointer_kind(destination).is_some()
                    && self.typed_types.pointer_kind(source).is_some()
                {
                    Some(("pointer-copy", destination.clone(), source.clone()))
                } else if self.typed_types.declared_is_scalar(destination)
                    && self.typed_types.declared_is_scalar(source)
                {
                    Some(("scalar-copy", destination.clone(), source.clone()))
                } else {
                    None
                }
            }
            _ => None,
        };
        let typed_with_subscript_write =
            matches!(instruction, LegacyInstruction::IndirectScalarAccess { .. }) && {
                let mut depth = 0usize;
                let mut written = false;
                for (index, text) in texts.iter().enumerate() {
                    match *text {
                        "[" => depth += 1,
                        "]" => depth = depth.saturating_sub(1),
                        "+" | "-" if depth > 0 && texts.get(index + 1) == Some(text) => {
                            written = true;
                        }
                        "=" if depth > 0
                            && texts.get(index + 1) != Some(&"=")
                            && !matches!(
                                index.checked_sub(1).map(|at| texts[at]),
                                Some("=" | "!" | "<" | ">")
                            ) =>
                        {
                            written = true;
                        }
                        _ => {}
                    }
                }
                written
            };
        crate::traversal_census::record_statement(
            crate::traversal_census::StatementView {
                function: &self.census_function,
                line,
                clause,
                outcome,
                copy,
                typed_with_subscript_write,
                contains_call: contains_call(tokens) || contains_braces(tokens),
            },
            &texts,
            &self.typed_model,
            &self.typed_types,
        );
    }

    /// Card **W43-RETURN-EXPRESSION**: hand one already-decided return
    /// expression refusal to the instrumented census. Compiled out with the
    /// feature off; called only after every return route has refused.
    #[cfg(feature = "indirect-census")]
    pub(super) fn census_return(&self, line: usize, texts: &[&str]) {
        if self.admit_derived_returns {
            return;
        }
        let calls = texts
            .iter()
            .enumerate()
            .filter(|(index, text)| {
                is_identifier(text)
                    && texts.get(index + 1) == Some(&"(")
                    && !matches!(**text, "sizeof" | "_Alignof" | "__alignof__")
            })
            .map(|(_, callee)| {
                let contract = self.contracts.contract(callee);
                crate::indirect_census::ReturnCall {
                    callee: (*callee).to_owned(),
                    contracted: contract.is_some(),
                    result_mode: contract.map(|contract| format!("{:?}", contract.result)),
                    defined_in_unit: find_function_body(self.tokens, callee).is_some(),
                }
            })
            .collect();
        crate::indirect_census::record_return(
            &crate::indirect_census::Context {
                function: &self.census_function,
                formal_parameters: &self.formal_parameters,
                typed_model: &self.typed_model,
                typed_types: &self.typed_types,
                typed_bases: &self.typed_bases,
            },
            line,
            texts,
            calls,
        );
    }
}
