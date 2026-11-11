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

//! Adapter for the checked same-item owning result. No list membership is
//! manufactured from an accessor's borrowed result or a representation premise.

use super::{AbstractPointer, AbstractState, LegacyDiagnostic, LegacyDiagnosticKind};
use crate::detach_same_item::{Decision, Facts, decide};
use qcore_schemas::contract::{ResourceFunctionContract, ResourceKind, ResourceParameterMode};
use std::collections::BTreeSet;

pub(super) fn checked(
    arguments: &[Option<String>],
    contract: &ResourceFunctionContract,
    parameter: usize,
    nullable: bool,
    state: &AbstractState,
) -> Option<AbstractPointer> {
    let ResourceParameterMode::Removed {
        owner_parameter,
        nullable: input_nullable,
    } = *contract.parameters.get(parameter)?
    else {
        return None;
    };
    if parameter == owner_parameter
        || input_nullable != nullable
        || contract.result_kind != ResourceKind::Heap
        || contract.parameter_kind(parameter) != ResourceKind::Heap
        || contract.parameter_kind(owner_parameter) != ResourceKind::Heap
        || contract.parameters.get(owner_parameter)
            != Some(&ResourceParameterMode::Owned { nullable: false })
        || contract.parameters.iter().enumerate().any(|(i, mode)| {
            i != parameter && i != owner_parameter && *mode != ResourceParameterMode::Borrowed
        })
    {
        return None;
    }
    let item_name = arguments.get(parameter)?.as_ref()?;
    let item = if item_name == "0" {
        AbstractPointer {
            may_be_null: true,
            ..Default::default()
        }
    } else {
        state.pointers.get(item_name)?.clone()
    };
    let parent = state
        .pointers
        .get(arguments.get(owner_parameter)?.as_ref()?)?;
    let parent_site = parent.targets.first().copied();
    let item_site = item.targets.first().copied();
    let owners = item_site.map(|site| {
        state
            .contained
            .iter()
            .filter_map(|(owner, children)| children.contains(&site).then_some(*owner))
            .collect::<BTreeSet<_>>()
    });
    let pending = |site| {
        state.conditional_replacements.contains_key(&site)
            || state
                .conditional_replacements
                .values()
                .any(|old| old.contains(&site))
    };
    let facts = Facts {
        parent_live_identity: !parent.unknown
            && parent.derivation.is_identity()
            && !parent.may_be_null
            && parent.targets.len() == 1
            && parent_site.is_some_and(|site| {
                state.resources.get(&site) == Some(&super::LIVE)
                    && !state.nullable_sites.contains(&site)
                    && !state.unresolved_effect_sites.contains(&site)
                    && !pending(site)
            }),
        item_identity: !item.unknown && item.derivation.is_identity() && item.targets.len() <= 1,
        nullable,
        item_null: item.targets.is_empty() && item.may_be_null,
        item_may_be_null: item.may_be_null,
        exact_membership: parent_site != item_site
            && parent_site.is_some_and(|parent| owners == Some(BTreeSet::from([parent]))),
        item_settled: item_site.is_some_and(|site| {
            state.resources.get(&site) == Some(&super::CONTAINED)
                && !state.unresolved_effect_sites.contains(&site)
                && !pending(site)
        }),
    };
    match decide(facts) {
        Decision::Nothing | Decision::Restore => Some(item),
        Decision::Refuse => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    function: &str,
    line: usize,
    called: &str,
    arguments: &[Option<String>],
    contract: &ResourceFunctionContract,
    parameter: usize,
    nullable: bool,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> Option<AbstractPointer> {
    let checked = checked(arguments, contract, parameter, nullable, state);
    if checked.is_none() {
        diagnostics.insert(LegacyDiagnostic { function: function.to_owned(), line,
            kind: LegacyDiagnosticKind::Unsupported { description: format!("call to `{called}`: same-item detach membership, identity or live parent is not established") } });
        state.unmodeled = true;
        for source in arguments.iter().flatten() {
            if let Some(pointer) = state.pointers.get(source).cloned() {
                super::mark_unresolved_effect(state, &pointer);
            }
        }
    }
    checked
}

/// Exact premise identity. It describes representation only. Call-site
/// membership and ownership must still pass `prepare` independently.
pub(super) const REPRESENTATION_NAME: &str = "detach_list_representation";
pub(super) const REPRESENTATION_STATEMENT: &str = "For this removed-result declaration, the live parent represents exactly its owned children by a finite acyclic child and next chain; child is NULL precisely when the chain is empty, each nonfirst child's prev is its predecessor, the first child's prev is the last child, chain child identities are pairwise distinct, every non-NULL next or prev target is a live child of that same parent, the last child's next is NULL, the parent is not a child, and no concurrent writer mutates these links during the call";

/// The bounded physical adapter. Parameter names are alpha-renamed, but the
/// complete statement/control/write sequence is checked, never a subset of
/// stores. Unknown syntax, extra writes/calls, qualifiers or record ambiguity
/// refuse. The model's list representation remains a declared premise.
const UNLINK_BODY: &str = r#"{
    if ((P == 0) || (I == 0) || (I != P->child && I->prev == 0)) { return 0; }
    if (I != P->child) { I->prev->next = I->next; }
    if (I->next != 0) { I->next->prev = I->prev; }
    if (I == P->child) { P->child = I->next; }
    else if (I->next == 0) { P->child->prev = I->prev; }
    I->prev = 0;
    I->next = 0;
    return I;
}"#;

fn normalize_nulls(mut words: Vec<String>) -> Vec<String> {
    for word in &mut words {
        if word == "0" {
            *word = "__null_constant".to_owned();
        }
    }
    loop {
        let mut changed = false;
        for at in 0..words.len() {
            let tail = &words[at..];
            let count = if tail.starts_with(&[
                "(".to_owned(),
                "void".to_owned(),
                "*".to_owned(),
                ")".to_owned(),
                "__null_constant".to_owned(),
            ]) {
                5
            } else if tail.starts_with(&[
                "(".to_owned(),
                "__null_constant".to_owned(),
                ")".to_owned(),
            ]) {
                3
            } else {
                continue;
            };
            words.splice(at..at + count, ["__null_constant".to_owned()]);
            changed = true;
            break;
        }
        if !changed {
            return words;
        }
    }
}

fn physical_body(
    tokens: &[super::Token],
    name: &str,
    contract: &ResourceFunctionContract,
) -> Result<(), &'static str> {
    let qcore_schemas::contract::ResourceReturnMode::Removed { parameter, .. } = contract.result
    else {
        return Err("result");
    };
    let Some(ResourceParameterMode::Removed {
        owner_parameter, ..
    }) = contract.parameters.get(parameter)
    else {
        return Err("removed-input");
    };
    let Some(parameters) = super::function_parameters(tokens, name) else {
        return Err("parameters");
    };
    if parameters.len() != 2 {
        return Err("arity");
    }
    let Some(parent) = parameters.get(*owner_parameter).and_then(Option::as_ref) else {
        return Err("parent-name");
    };
    let Some(item) = parameters.get(parameter).and_then(Option::as_ref) else {
        return Err("item-name");
    };
    if parent == item
        || matches!(parent.as_str(), "child" | "next" | "prev")
        || matches!(item.as_str(), "child" | "next" | "prev")
    {
        return Err("names");
    }
    let Some((start, _)) = super::find_function_body(tokens, name) else {
        return Err("body");
    };
    let Some(end) = super::matching(tokens, start, "{", "}") else {
        return Err("body-end");
    };
    let words = tokens[start..=end]
        .iter()
        .map(|token| {
            if &token.text == parent {
                "P".to_owned()
            } else if &token.text == item {
                "I".to_owned()
            } else {
                token.text.clone()
            }
        })
        .collect();
    let expected = super::tokenize(UNLINK_BODY)
        .into_iter()
        .map(|token| token.text)
        .collect();
    if normalize_nulls(words) != normalize_nulls(expected) {
        return Err("complete-frame");
    }
    let all = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    let model = crate::typed_field_access::TypedAccessModel::read(&all);
    let Some((open, close)) = super::function_parameter_range(tokens, name, start) else {
        return Err("parameter-range");
    };
    let parameter_words = tokens[open + 1..close]
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>();
    // A by-value formal's own `const` qualifies the pointer slot, not its
    // pointee. The exact body frame never assigns that slot. Normalize only
    // this single-star suffix for the local record-type view; do not widen
    // the shared lexical reader or remove pointee/volatile/atomic qualifiers.
    let mut normalized_parameters = Vec::new();
    for (index, group) in parameter_words.split(|word| *word == ",").enumerate() {
        if index > 0 {
            normalized_parameters.push(",");
        }
        let Some(star) = group.iter().position(|word| *word == "*") else {
            return Err("pointer-parameter");
        };
        if group[..star]
            .iter()
            .any(|word| matches!(*word, "const" | "volatile" | "_Atomic"))
        {
            return Err("qualified-pointee");
        }
        // cJSON defines its named struct and alias together. The shared
        // classifier intentionally leaves that alias opaque. Resolve only a
        // direct, unique, complete named nonunion struct for this exact frame;
        // the original source/signature remain the certificate input.
        if let [alias] = &group[..star]
            && let Some(tag) = model.defining_struct_alias(alias)
        {
            normalized_parameters.extend(["struct", tag, "*"]);
        } else {
            normalized_parameters.extend_from_slice(&group[..=star]);
        }
        match &group[star + 1..] {
            ["const", name] | [name] if super::is_identifier(name) => {
                normalized_parameters.push(name)
            }
            _ => return Err("pointer-parameter-suffix"),
        }
    }
    let body_words = tokens[start + 1..end]
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>();
    let types = crate::typed_field_access::FunctionTypes::read(
        &model,
        &normalized_parameters,
        &body_words,
        &BTreeSet::new(),
    );
    let names = types
        .record_pointer_names()
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let Some(tag) = names.get(parent.as_str()).copied() else {
        return Err("parent-record");
    };
    if names.get(item.as_str()).copied() != Some(tag) {
        return Err("item-record");
    }
    for field in ["child", "next", "prev"] {
        if model.record_pointer_field(tag, field) != Some(tag) {
            return Err(field);
        }
    }
    Ok(())
}

/// Every in-unit removed-result declaration needs the complete physical
/// adapter and its explicit representation premise. A failed check withdraws
/// it before any caller or inference reads it. A successful conditional check
/// does not mark the declaration or its premise as validated.
pub(super) fn settle(
    tokens: &[super::Token],
    contracts: &qcore_schemas::contract::ResourceContracts,
) -> Option<qcore_schemas::contract::ResourceContracts> {
    use qcore_contract::contract_loader::{PremiseOrigin, declared_premises};
    let premises = declared_premises(contracts).ok();
    let mut withdrawn = Vec::new();
    for (name, contract) in contracts.functions() {
        if !matches!(
            contract.result,
            qcore_schemas::contract::ResourceReturnMode::Removed { .. }
        ) || super::find_function_body(tokens, name).is_none()
        {
            continue;
        }
        let representation = premises
            .as_ref()
            .and_then(|m| m.get(name))
            .is_some_and(|ps| {
                ps.iter().any(|p| {
                    p.origin == PremiseOrigin::Declared
                        && p.name == REPRESENTATION_NAME
                        && p.statement == REPRESENTATION_STATEMENT
                })
            });
        if !representation || physical_body(tokens, name, contract).is_err() {
            withdrawn.push(name.clone());
        }
    }
    if withdrawn.is_empty() {
        return None;
    }
    let mut settled = contracts.clone();
    for name in withdrawn {
        crate::contract_dependencies::withdrawn(&name, "same-item-detach", true);
        settled.withdraw_declaration(&name);
    }
    Some(settled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qcore_schemas::contract::ResourceReturnMode;
    const BODY: &str = include_str!("../tests/fixtures/detach-same-item/detach_impl.h");
    fn contract() -> ResourceFunctionContract {
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Removed {
                    owner_parameter: 0,
                    nullable: true,
                },
            ],
            ResourceReturnMode::Removed {
                parameter: 1,
                nullable: true,
            },
        )
    }
    #[test]
    fn complete_physical_unlink_frame_accepts_renaming_and_cjson_null_spelling() {
        for source in [
            BODY.to_owned(),
            BODY.replace("DetachNode *parent,", "struct DetachNode *parent,")
                .replace(
                    "DetachNode * const item)",
                    "struct DetachNode * const item)",
                ),
            BODY.replace("parent", "owner").replace("item", "entry"),
            BODY.replace(" 0", " ((void*)0)"),
        ] {
            assert!(
                physical_body(&super::super::tokenize(&source), "dt_detach", &contract()).is_ok(),
                "{:?}\n{source}",
                physical_body(&super::super::tokenize(&source), "dt_detach", &contract())
            );
        }
    }
    #[test]
    fn exact_cjson_defining_struct_alias_frame_is_checked() {
        let fixture = include_str!("../tests/fixtures/detach-same-item/cjson_actual_frame.h");
        let actual = match std::env::var("QCORE_DETACH_PRODUCTION_INPUT") {
            Ok(path) => {
                std::fs::read_to_string(path).expect("explicit production input is readable")
            }
            Err(std::env::VarError::NotPresent) => fixture.to_owned(),
            Err(error) => panic!("invalid production input path: {error}"),
        };
        let result = physical_body(
            &super::super::tokenize(&actual),
            "cJSON_DetachItemViaPointer",
            &contract(),
        );
        assert!(result.is_ok(), "actual cJSON frame: {result:?}");
        for bad in [
            fixture.replace("typedef struct cJSON", "typedef struct"),
            fixture.replace("typedef struct cJSON", "typedef union cJSON"),
            fixture.replace("} cJSON;", "} cJSONStruct; typedef struct cJSON *cJSON;"),
            format!("{fixture}\ntypedef int cJSON;\n"),
            format!("{fixture}\nstruct cJSON {{ int other; }};\n"),
            fixture.replace("cJSON *parent", "const cJSON *parent"),
            fixture.replace("* const item", "* volatile item"),
            fixture.replace("* const item", "* _Atomic item"),
            fixture.replace("* const item", "**item"),
            fixture.replace("struct cJSON *prev;", "struct cJSON * volatile prev;"),
            fixture.replace("struct cJSON *prev;", "_Atomic(struct cJSON *) prev;"),
            fixture.replace("item->next = ((void*)0);", "parent->next = ((void*)0);"),
        ] {
            assert_ne!(
                bad, fixture,
                "negative control must mutate the exact retained spelling"
            );
            assert!(
                physical_body(
                    &super::super::tokenize(&bad),
                    "cJSON_DetachItemViaPointer",
                    &contract()
                )
                .is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn extra_missing_and_misdirected_effects_refuse() {
        for source in [
            BODY.replace("return item;", "return parent;"),
            BODY.replace("item->prev = 0;", ""),
            BODY.replace("item->next = 0;", "parent->next = 0;"),
            BODY.replace(
                "parent->child->prev = item->prev;",
                "parent->child->prev = item;",
            ),
            BODY.replace("return item;", "touch(item); return item;"),
            BODY.replace(
                "item->prev->next = item->next;",
                "item->prev->next = parent;",
            ),
            BODY.replace(
                "struct DetachNode *prev;",
                "volatile struct DetachNode *prev;",
            ),
            BODY.replace("struct DetachNode *next;", "void *next;"),
            BODY.replace("* const item)", "* volatile item)"),
            BODY.replace("* const item)", "* _Atomic item)"),
            BODY.replace("* const item)", "**item)"),
            BODY.replace("DetachNode *parent,", "const DetachNode *parent,"),
            BODY.replace("return item;", "parent->child = item; return item;"),
        ] {
            assert!(
                physical_body(&super::super::tokenize(&source), "dt_detach", &contract()).is_err(),
                "{source}"
            );
        }
    }
}
