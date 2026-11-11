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

//! Exact read-only child/next indexed lookup frame. This establishes borrowed
//! provenance only; caller membership and concrete child identity are separate.
//! The original source remains the replay input. No contract is synthesized by
//! this reader and no unknown child is represented by a fresh resource site.

use super::{Token, find_function_body, function_parameter_range, function_parameters, matching};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Frame {
    pub record: String,
    pub parent: String,
    pub cursor: String,
    pub index: String,
    pub read_lines: [usize; 2],
}

const BODY: &str = "{ TYPE * C = 0; if (P == 0) { return 0; } C = P->child; while ((C != 0) && (N > 0)) { N--; C = C->next; } return C; }";

// The retained preprocessor spells NULL as ((void *)0). No other conversion
// or expression is normalized. Numeric zero also appears as the index bound.
fn nulls(mut words: Vec<String>) -> Vec<String> {
    loop {
        let mut change = None;
        for at in 0..words.len() {
            let tail = &words[at..];
            let pattern = ["(", "void", "*", ")", "0"];
            if tail.iter().map(String::as_str).take(5).eq(pattern) {
                change = Some((at, 5));
                break;
            }
            if tail.iter().map(String::as_str).take(3).eq(["(", "0", ")"]) {
                change = Some((at, 3));
                break;
            }
        }
        let Some((at, n)) = change else {
            return words;
        };
        words.splice(at..at + n, ["0".to_owned()]);
    }
}

pub(super) fn read(tokens: &[Token], function: &str) -> Option<Frame> {
    let parameters = function_parameters(tokens, function)?;
    let [Some(parent), Some(index)] = parameters.as_slice() else {
        return None;
    };
    let (start, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, start, "{", "}")?;
    let (open, close) = function_parameter_range(tokens, function, start)?;
    let parameter_words = tokens[open + 1..close]
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>();
    let groups = parameter_words.split(|w| *w == ",").collect::<Vec<_>>();
    let [first, second] = groups.as_slice() else {
        return None;
    };
    let first = first.strip_prefix(&["const"]).unwrap_or(first);
    let type_words = match *first {
        [alias, "*", name] if name == parent => vec![alias],
        ["struct", tag, "*", name] if name == parent => vec!["struct", tag],
        _ => return None,
    };
    let (name, specs) = second.split_last()?;
    if name != index {
        return None;
    }
    let body = tokens[start..=end]
        .iter()
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>();
    let mut declaration = vec!["{"];
    declaration.extend_from_slice(&type_words);
    declaration.push("*");
    if !body.starts_with(&declaration) {
        return None;
    }
    let cursor = *body.get(declaration.len())?;
    if !super::is_identifier(cursor)
        || [parent.as_str(), index.as_str(), "child", "next"].contains(&cursor)
        || parent == index
        || ["child", "next"].contains(&parent.as_str())
        || ["child", "next"].contains(&index.as_str())
    {
        return None;
    }
    // The return type is the same mutable record pointer, with optional static.
    let function_at = open.checked_sub(1)?;
    let mut ret = type_words.clone();
    ret.push("*");
    let before = function_at.checked_sub(ret.len())?;
    if !tokens[before..function_at]
        .iter()
        .map(|t| t.text.as_str())
        .eq(ret)
    {
        return None;
    }
    if before > 0 && !matches!(tokens[before - 1].text.as_str(), ";" | "}" | "static") {
        return None;
    }
    let mut normalized = vec!["{".to_owned(), "TYPE".to_owned()];
    normalized.extend(body[1 + type_words.len()..].iter().map(|word| {
        if *word == parent {
            "P"
        } else if *word == index {
            "N"
        } else if *word == cursor {
            "C"
        } else {
            word
        }
        .to_string()
    }));
    let expected = super::tokenize(BODY).into_iter().map(|t| t.text).collect();
    if nulls(normalized) != nulls(expected) {
        return None;
    }
    // Build the unit type model only after the cheap complete-frame match.
    // Noncandidate functions do not acquire a new whole-unit reader cost.
    let words = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
    let model = crate::typed_field_access::TypedAccessModel::read(&words);
    let tag = match type_words.as_slice() {
        [alias] => model.defining_struct_alias(alias)?,
        ["struct", tag] if model.record_definition_is_complete(tag) => tag,
        _ => return None,
    };
    if model.integer_class(specs)?.signed {
        return None;
    }
    for member in ["child", "next"] {
        if model.record_pointer_field(tag, member) != Some(tag) {
            return None;
        }
    }
    // The shared continuation view deliberately ignores const. This local
    // frame requires mutable, unqualified link declarations as written.
    let definitions = words
        .windows(3)
        .enumerate()
        .filter_map(|(at, w)| (w == ["struct", tag, "{"]).then_some(at + 2))
        .collect::<Vec<_>>();
    let [record_open] = definitions.as_slice() else {
        return None;
    };
    let record_close = matching(tokens, *record_open, "{", "}")?;
    for member in ["child", "next"] {
        let declarations = words[record_open + 1..record_close]
            .split(|word| *word == ";")
            .filter(|group| group.last() == Some(&member))
            .collect::<Vec<_>>();
        if declarations.as_slice() != [&["struct", tag, "*", member][..]] {
            return None;
        }
    }
    let read_lines = tokens[start..=end]
        .windows(7)
        .filter_map(|w| {
            (w[0].text == cursor
                && w[1].text == "="
                && w[3].text == "-"
                && w[4].text == ">"
                && matches!(w[5].text.as_str(), "child" | "next")
                && w[6].text == ";")
                .then_some(w[0].line)
        })
        .collect::<Vec<_>>();
    let [head_line, next_line] = read_lines.as_slice() else {
        return None;
    };
    Some(Frame {
        record: tag.to_owned(),
        parent: parent.clone(),
        cursor: cursor.to_owned(),
        index: index.clone(),
        read_lines: [*head_line, *next_line],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const ACTUAL: &str =
        include_str!("../tests/fixtures/detach-lookup-membership/cjson_lookup_frame.h");
    #[test]
    fn actual_complete_readonly_frame_and_mutation_controls() {
        let actual = ACTUAL.replace("\r\n", "\n");
        assert!(read(&super::super::tokenize(&actual), "get_array_item").is_some());
        for (from, to) in [
            ("index--;", "index++;"),
            ("current_child->next", "array->next"),
            (
                "return current_child;",
                "array->child = current_child; return current_child;",
            ),
            ("size_t index", "double index"),
            ("const cJSON *array", "volatile cJSON *array"),
            ("const cJSON *array", "const cJSON **array"),
            ("struct cJSON *next;", "const struct cJSON *next;"),
            ("struct cJSON *next;", "volatile struct cJSON *next;"),
            ("struct cJSON *next;", "struct cJSON *volatile next;"),
            ("struct cJSON *next;", "_Atomic(struct cJSON *) next;"),
            ("struct cJSON *child;", "struct cJSON **child;"),
            ("typedef struct cJSON", "typedef union cJSON"),
            ("typedef struct cJSON\n{", "typedef struct\n{"),
            (
                "current_child = array->child;",
                "current_child = array->next;",
            ),
            ("(index > 0)", "(index >= 0)"),
            ("(current_child != ((void*)0))", "(array != ((void*)0))"),
        ] {
            let changed = actual.replace(from, to);
            assert_ne!(changed, actual, "control must edit source: {from}");
            assert!(
                read(&super::super::tokenize(&changed), "get_array_item").is_none(),
                "{from} -> {to}"
            );
        }
        let conflicting = format!("{actual}\ntypedef struct Other {{ int x; }} cJSON;");
        assert!(read(&super::super::tokenize(&conflicting), "get_array_item").is_none());
        let alpha = actual
            .replace("get_array_item", "checked_lookup")
            .replace("current_child", "cursor")
            .replace("array", "parent")
            .replace("index", "position")
            .replace("cJSON", "LookupNode");
        assert!(read(&super::super::tokenize(&alpha), "checked_lookup").is_some());
    }
}

/// Replace only the two pointer reads of the independently checked complete
/// frame. The ordinary CFG, NULL branch, loop and return remain intact.
pub(super) fn settle(cfg: &mut super::LegacyCfg, tokens: &[Token], function: &str) {
    let Some(frame) = read(tokens, function) else {
        return;
    };
    let candidates = cfg
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(id, node)| {
            let super::LegacyInstruction::IndirectScalarAccess {
                destination: Some(destination),
                borrowed_destination: true,
                frame: None,
                readback: None,
                bases,
                chains,
                ..
            } = &node.instruction
            else {
                return None;
            };
            let [base] = bases.as_slice() else {
                return None;
            };
            let [chain] = chains.as_slice() else {
                return None;
            };
            if destination != &frame.cursor
                || !chain.dereferenced_root
                || !chain.links.is_empty()
                || chain.root != *base
            {
                return None;
            }
            let guarded_cursor = if base == &frame.parent
                && chain.member == "child"
                && node.line == frame.read_lines[0]
            {
                false
            } else if base == &frame.cursor
                && chain.member == "next"
                && node.line == frame.read_lines[1]
            {
                true
            } else {
                return None;
            };
            Some((id, guarded_cursor))
        })
        .collect::<Vec<_>>();
    if candidates.len() != 2 || candidates.iter().filter(|(_, guarded)| *guarded).count() != 1 {
        return;
    }
    for (id, guarded_cursor) in candidates {
        cfg.nodes[id].lookup_read = Some(super::CheckedLookupRead {
            parent: frame.parent.clone(),
            cursor: frame.cursor.clone(),
            guarded_cursor,
        });
    }
}

/// A name-only borrowed formal is not a tracked object. Present that virtual
/// relation as untracked only at this exact read point; never hide a resource,
/// invalidation, conflicting member binding, or unresolved/nullable parent.
/// The cursor's non-NULL fact comes from the checked while-body guard.
pub(super) fn read_view(
    node: &super::LegacyCfgNode,
    state: &super::AbstractState,
    virtual_sites: &std::collections::BTreeSet<usize>,
) -> Option<super::AbstractState> {
    let description = node.lookup_read.as_ref()?;
    let super::LegacyInstruction::IndirectScalarAccess {
        typed_alternative: None,
        destination: Some(destination),
        bases,
        chains,
        borrowed_destination: true,
        frame: None,
        readback: None,
        ..
    } = &node.instruction
    else {
        return None;
    };
    let [base] = bases.as_slice() else {
        return None;
    };
    let [chain] = chains.as_slice() else {
        return None;
    };
    let expected = if description.guarded_cursor {
        &description.cursor
    } else {
        &description.parent
    };
    let member = if description.guarded_cursor {
        "next"
    } else {
        "child"
    };
    if base != expected
        || chain.root != *base
        || !chain.dereferenced_root
        || !chain.links.is_empty()
        || chain.member != member
        || destination != &description.cursor
        || state.unmodeled
        || state.widened
    {
        return None;
    }
    let parent = state.pointers.get(&description.parent)?;
    let site = identity(parent)?;
    let cursor_relation = !description.guarded_cursor
        || state
            .pointers
            .get(&description.cursor)
            .is_some_and(|cursor| {
                !cursor.unknown
                    && cursor.targets == parent.targets
                    && matches!(
                        cursor.derivation,
                        super::Derivation::Derived {
                            extent: super::DerivedExtent::Borrowed,
                            ..
                        }
                    )
                    && !state.invalidated.contains_key(&description.cursor)
            });
    let facts = crate::list_lookup::ReadViewFacts {
        virtual_site: virtual_sites.contains(&site),
        tracked_status: state.resources.contains_key(&site),
        unresolved: state.nullable_sites.contains(&site)
            || state.unresolved_effect_sites.contains(&site)
            || state.conditional_replacements.contains_key(&site)
            || state
                .conditional_replacements
                .values()
                .any(|sites| sites.contains(&site)),
        conflict: state.field_links.contains_key(&site)
            || state
                .field_links
                .values()
                .flat_map(|fields| fields.values())
                .any(|children| children.contains(&site))
            || state
                .lookup_links
                .iter()
                .any(|((owner, _, _), child)| *owner == site || *child == Some(site))
            || state.borrows.contains_key(&description.parent)
            || state.borrows.contains_key(&description.cursor)
            || state
                .contained
                .iter()
                .any(|(owner, children)| *owner == site || children.contains(&site))
            || state.invalidated.contains_key(&description.parent)
            || state.pointers.keys().any(|place| {
                crate::aggregate_store::place_owner(place)
                    .is_some_and(|owner| owner == description.parent || owner == description.cursor)
            }),
        cursor_relation,
    };
    if !crate::list_lookup::allow_read_view(facts) {
        return None;
    }
    let mut view = state.clone();
    let pointer = view.pointers.get_mut(expected)?;
    pointer.targets.clear();
    pointer.unknown = true;
    pointer.may_be_null = false;
    Some(view)
}

/// Re-derive assigned direct constant-index calls. A nested call, whole return,
/// cast, unknown index, multiple matching calls on one line, or richer lifetime
/// declaration receives no identity-refinement descriptor.
pub(super) fn settle_calls(
    cfg: &mut super::LegacyCfg,
    tokens: &[Token],
    contracts: &super::ResourceContracts,
) {
    use super::{
        CheckedLookupCall, DerivedExtent, LegacyInstruction, ResourceParameterMode,
        ResourceReturnMode,
    };
    let Some((start, _)) = find_function_body(tokens, &cfg.function) else {
        return;
    };
    let Some(end) = matching(tokens, start, "{", "}") else {
        return;
    };
    let mut pointer_records = None;
    for node in &mut cfg.nodes {
        let LegacyInstruction::ContractCall {
            function,
            destination: Some(destination),
            arguments,
            contract,
            nested,
            container,
            retention,
            produced_sites,
            ..
        } = &node.instruction
        else {
            continue;
        };
        if !nested.is_empty()
            || container.is_some()
            || retention.is_some()
            || !produced_sites.is_empty()
            || contracts.lifetime_contract(function).is_some()
            || contracts.lifetime_summary(function).is_some()
            || contract.parameters
                != [
                    ResourceParameterMode::Borrowed,
                    ResourceParameterMode::Borrowed,
                ]
            || !matches!(
                contract.result,
                ResourceReturnMode::Derived {
                    parameter: 0,
                    extent: DerivedExtent::Borrowed,
                    nullable: true,
                    const_dropped: true
                }
            )
        {
            continue;
        }
        let Some(frame) = read(tokens, function) else {
            continue;
        };
        let Some(Some(parent)) = arguments.first() else {
            continue;
        };
        let records = pointer_records.get_or_insert_with(|| {
            let words = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
            let model = crate::typed_field_access::TypedAccessModel::read(&words);
            let Some((open, close)) = function_parameter_range(tokens, &cfg.function, start) else {
                return std::collections::BTreeMap::new();
            };
            let types = crate::typed_field_access::FunctionTypes::read(
                &model,
                &words[open + 1..close],
                &words[start + 1..end],
                &BTreeSet::new(),
            );
            types
                .record_pointer_names()
                .into_iter()
                .filter(|(_, tag)| model.record_definition_is_complete(tag))
                .map(|(name, tag)| (name.to_owned(), tag.to_owned()))
                .collect::<std::collections::BTreeMap<_, _>>()
        });
        if !crate::list_lookup::record_eligible(
            records.contains_key(parent),
            records.get(parent) == Some(&frame.record),
        ) {
            continue;
        }
        // A same-spelled local/formal function pointer must not be confused
        // with the retained defining function. Address-taking or any other
        // non-call occurrence conservatively withholds this descriptor too.
        if function_parameters(tokens, &cfg.function)
            .is_some_and(|parameters| parameters.iter().flatten().any(|name| name == function))
            || tokens[start + 1..end]
                .iter()
                .enumerate()
                .any(|(at, token)| {
                    token.text == *function
                        && tokens
                            .get(start + at + 2)
                            .is_none_or(|next| next.text != "(")
                })
        {
            continue;
        }
        let candidates = tokens[start + 1..end]
            .windows(9)
            .filter_map(|w| {
                let texts = w.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
                match texts.as_slice() {
                    [d, "=", f, "(", p, ",", n, ")", ";"]
                        if *d == destination
                            && *f == function
                            && *p == parent
                            && w[2].line == node.line
                            && !n.is_empty()
                            && n.bytes().all(|b| b.is_ascii_digit())
                            && (*n == "0" || !n.starts_with('0')) =>
                    {
                        // Fits every admitted C unsigned parameter, even
                        // unsigned char, without target-width conversion.
                        n.parse::<usize>().ok().filter(|index| *index <= 255)
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        if let [index] = candidates.as_slice() {
            node.lookup = Some(CheckedLookupCall {
                callee: function.clone(),
                parent: parent.clone(),
                index: *index,
                record: frame.record.clone(),
                pointer_records: records.clone(),
            });
        }
    }
}

use super::{AbstractPointer, AbstractState, LegacyCfgNode, LegacyInstruction};
use std::collections::BTreeSet;

fn settled(state: &AbstractState, site: usize, status: u8) -> bool {
    state.resources.get(&site) == Some(&status)
        && !state.nullable_sites.contains(&site)
        && !state.unresolved_effect_sites.contains(&site)
        && !state.conditional_replacements.contains_key(&site)
        && !state
            .conditional_replacements
            .values()
            .any(|old| old.contains(&site))
}
fn identity(pointer: &AbstractPointer) -> Option<usize> {
    (!pointer.unknown
        && !pointer.may_be_null
        && pointer.derivation.is_identity()
        && pointer.targets.len() == 1)
        .then(|| *pointer.targets.first().unwrap())
}

/// An exact physical prefix and independent custody partition are both needed.
/// Missing links (including unknown closure) never mean NULL.
pub(super) fn resolve(
    node: &LegacyCfgNode,
    state: &AbstractState,
    kinds: &std::collections::BTreeMap<usize, super::ResourceKind>,
) -> Option<AbstractPointer> {
    let description = node.lookup.as_ref()?;
    if !crate::list_lookup::record_eligible(
        description
            .pointer_records
            .contains_key(&description.parent),
        description.pointer_records.get(&description.parent) == Some(&description.record),
    ) {
        return None;
    }
    if state.unmodeled {
        return None;
    }
    let parent = identity(state.pointers.get(&description.parent)?)?;
    if super::kind_of(kinds, parent) != super::ResourceKind::Heap
        || !settled(state, parent, super::LIVE)
    {
        return None;
    }
    let resolved = crate::list_lookup::resolve_prefix(
        description.index,
        state
            .lookup_links
            .get(&(parent, description.record.clone(), "child".to_owned()))
            .copied(),
        |item| {
            state
                .lookup_links
                .get(&(item, description.record.clone(), "next".to_owned()))
                .copied()
        },
        |item| {
            super::kind_of(kinds, item) == super::ResourceKind::Heap
                && item != parent
                && settled(state, item, super::CONTAINED)
                && state
                    .contained
                    .iter()
                    .filter(|(_, children)| children.contains(&item))
                    .map(|(owner, _)| *owner)
                    .collect::<BTreeSet<_>>()
                    == BTreeSet::from([parent])
        },
    )?;
    Some(match resolved {
        None => AbstractPointer {
            may_be_null: true,
            ..Default::default()
        },
        Some(item) => AbstractPointer {
            targets: BTreeSet::from([item]),
            ..Default::default()
        },
    })
}

pub(super) fn stored_link(
    node: &LegacyCfgNode,
    state: &AbstractState,
    caller_sites: &BTreeSet<usize>,
    pointer_records: &std::collections::BTreeMap<String, String>,
) -> Option<(usize, String, String, String)> {
    let place = super::written_field_place(&node.instruction)?;
    let owner = crate::aggregate_store::place_owner(place)?;
    let field = crate::aggregate_store::place_field(place)?;
    if !matches!(field, "child" | "next") {
        return None;
    }
    Some((
        super::resolved_aggregate_owner(state, owner, caller_sites)?,
        pointer_records.get(owner)?.clone(),
        field.to_owned(),
        place.to_owned(),
    ))
}

/// Only a checked same-item transfer can detach an identity-refined alias from
/// its old parent lifetime. Other borrows still end under the existing rule.
pub(super) fn prepare_removed(node: &LegacyCfgNode, state: &mut AbstractState) {
    let LegacyInstruction::ContractCall {
        arguments,
        contract,
        nested,
        ..
    } = &node.instruction
    else {
        return;
    };
    let super::ResourceReturnMode::Removed {
        parameter,
        nullable,
    } = contract.result
    else {
        return;
    };
    if !nested.is_empty() {
        return;
    }
    let Some(item) = super::detach_route::checked(arguments, contract, parameter, nullable, state)
    else {
        return;
    };
    if item.targets.len() != 1 || item.may_be_null {
        return;
    }
    for name in state.lookup_aliases.clone() {
        if state.pointers.get(&name).is_some_and(|p| p == &item) {
            state.borrows.remove(&name);
            state.lookup_aliases.remove(&name);
        }
    }
}

pub(super) fn after(
    node: &LegacyCfgNode,
    state: &mut AbstractState,
    result: Option<AbstractPointer>,
    stored: Option<(usize, String, String, String)>,
) {
    // Clear facts on every call except this exact complete read-only frame;
    // any unbounded or indirect write also destroys the physical prefix.
    let physical_write = match &node.instruction {
        LegacyInstruction::ContractCall { .. } => node.lookup.is_none(),
        LegacyInstruction::Alias { destination, .. } | LegacyInstruction::Clear { destination } => {
            crate::may_alias::place_root(destination) != Some(destination.as_str())
                && stored.is_none()
        }
        LegacyInstruction::OpaqueValue { destination, .. }
        | LegacyInstruction::VaArgPointer { destination }
        | LegacyInstruction::ForgetScalar { destination }
        | LegacyInstruction::PointerStep { destination, .. }
        | LegacyInstruction::Acquire { destination, .. } => {
            crate::may_alias::place_root(destination) != Some(destination.as_str())
        }
        LegacyInstruction::AggregateStore { .. } => stored.is_none(),
        LegacyInstruction::NoOp
        | LegacyInstruction::BranchOnNull { .. }
        | LegacyInstruction::BranchOnDistinct { .. }
        | LegacyInstruction::BranchOnValidity { .. }
        | LegacyInstruction::BranchOnResult { .. }
        | LegacyInstruction::ArgumentReads { .. } => false,
        // New/unrecognized operations, wrapper instructions and indirect
        // stores fail closed; no black-list default can retain stale links.
        _ => true,
    };
    if physical_write || state.unmodeled {
        state.lookup_links.clear();
    }
    if let Some((owner, record, field, place)) = stored {
        // Different record views may overlap. A typed store invalidates every
        // prior link of this owner before establishing its own exact field.
        state.lookup_links.retain(|(site, _, _), _| *site != owner);
        if !state.unmodeled
            && let Some(pointer) = state.pointers.get(&place)
        {
            let value = if !pointer.unknown && pointer.derivation.is_identity() {
                if pointer.targets.is_empty() && pointer.may_be_null {
                    Some(None)
                } else {
                    identity(pointer).map(Some)
                }
            } else {
                None
            };
            if let Some(value) = value {
                state.lookup_links.insert((owner, record, field), value);
            }
        }
    }
    let copied = match &node.instruction {
        LegacyInstruction::Alias {
            destination,
            source,
        } if state.lookup_aliases.contains(source) => Some(destination.clone()),
        _ => None,
    };
    for name in super::rebound_names(&node.instruction) {
        state.lookup_aliases.remove(name);
    }
    if let Some(destination) = copied {
        state.lookup_aliases.insert(destination);
    }
    if !state.unmodeled
        && let Some(pointer) = result
        && let (
            Some(description),
            LegacyInstruction::ContractCall {
                destination: Some(destination),
                ..
            },
        ) = (&node.lookup, &node.instruction)
    {
        // The ordinary call has already enforced all its effects/conditions.
        // Only replace its borrowed binding if that binding survived the gate.
        if state.pointers.get(destination).is_some_and(|p| {
            matches!(
                p.derivation,
                super::Derivation::Derived {
                    extent: super::DerivedExtent::Borrowed,
                    ..
                }
            )
        }) {
            let has_identity = !pointer.targets.is_empty();
            state.pointers.insert(destination.clone(), pointer);
            if has_identity {
                state.lookup_aliases.insert(destination.clone());
                state.borrows.insert(
                    destination.clone(),
                    BTreeSet::from([super::slot_move_route::Borrow {
                        key: (node.line, destination.clone()),
                        parent: description.parent.clone(),
                    }]),
                );
            } else {
                state.borrows.remove(destination);
            }
        }
    }
}

#[cfg(test)]
mod flow_controls {
    use super::*;
    #[test]
    fn opaque_member_write_cannot_preserve_an_old_physical_link() {
        let mut state = AbstractState::default();
        state
            .lookup_links
            .insert((1, "Node".to_owned(), "child".to_owned()), Some(2));
        let node = LegacyCfgNode {
            lookup: None,
            lookup_read: None,
            instruction: LegacyInstruction::OpaqueValue {
                destination: "p->child".to_owned(),
                alias_read: None,
            },
            successors: vec![],
            line: 1,
            is_exit: false,
            loop_exit_edge: None,
        };
        after(&node, &mut state, None, None);
        assert!(state.lookup_links.is_empty());
    }
    #[test]
    fn physical_links_are_must_facts_and_null_is_not_absence() {
        let key = (1, "Node".to_owned(), "child".to_owned());
        let mut left = AbstractState::default();
        left.lookup_links.insert(key.clone(), Some(2));
        let right = left.clone();
        left.join(&right, &BTreeSet::new());
        assert_eq!(left.lookup_links.get(&key), Some(&Some(2)));
        let mut other = right.clone();
        other.lookup_links.insert(key.clone(), None);
        left.join(&other, &BTreeSet::new());
        assert!(!left.lookup_links.contains_key(&key));
        let mut null = AbstractState::default();
        null.lookup_links.insert(key.clone(), None);
        null.join(&AbstractState::default(), &BTreeSet::new());
        assert!(!null.lookup_links.contains_key(&key));
    }
    #[test]
    fn same_site_and_field_with_a_different_record_cannot_supply_a_prefix() {
        let mut state = AbstractState::default();
        state
            .resources
            .extend([(1, super::super::LIVE), (2, super::super::CONTAINED)]);
        state.contained.insert(1, BTreeSet::from([2]));
        let pointer = |site| AbstractPointer {
            targets: BTreeSet::from([site]),
            ..Default::default()
        };
        state.pointers.insert("p".to_owned(), pointer(1));
        state.pointers.insert("q->child".to_owned(), pointer(2));
        let node = LegacyCfgNode {
            lookup: Some(super::super::CheckedLookupCall {
                callee: "lookup".to_owned(),
                parent: "p".to_owned(),
                index: 0,
                record: "Node".to_owned(),
                pointer_records: [("p".to_owned(), "Node".to_owned())].into(),
            }),
            lookup_read: None,
            instruction: LegacyInstruction::NoOp,
            successors: vec![],
            line: 1,
            is_exit: false,
            loop_exit_edge: None,
        };
        let kinds = [
            (1, super::super::ResourceKind::Heap),
            (2, super::super::ResourceKind::Heap),
        ]
        .into();
        state
            .lookup_links
            .insert((1, "Other".to_owned(), "child".to_owned()), Some(2));
        assert!(resolve(&node, &state, &kinds).is_none());
        state
            .lookup_links
            .insert((1, "Node".to_owned(), "child".to_owned()), Some(2));
        assert_eq!(resolve(&node, &state, &kinds), Some(pointer(2)));
        let mut unknown = node.clone();
        unknown.lookup.as_mut().unwrap().pointer_records.clear();
        assert!(resolve(&unknown, &state, &kinds).is_none());
        unknown
            .lookup
            .as_mut()
            .unwrap()
            .pointer_records
            .insert("p".to_owned(), "Other".to_owned());
        assert!(resolve(&unknown, &state, &kinds).is_none());
        after(
            &node,
            &mut state,
            None,
            Some((
                1,
                "Other".to_owned(),
                "child".to_owned(),
                "q->child".to_owned(),
            )),
        );
        assert!(
            resolve(&node, &state, &kinds).is_none(),
            "an overlapping differently typed store invalidates the old record view"
        );
    }
}

#[cfg(test)]
mod read_obligation_controls {
    use super::super::{
        AliasPeers, CheckedLookupRead, Derivation, DerivedExtent, NodeAliasFacts, TransferContext,
    };
    use super::*;
    fn node() -> LegacyCfgNode {
        LegacyCfgNode {
            lookup: None,
            lookup_read: Some(CheckedLookupRead {
                parent: "array".to_owned(),
                cursor: "current_child".to_owned(),
                guarded_cursor: false,
            }),
            instruction: LegacyInstruction::IndirectScalarAccess {
                typed_alternative: None,
                bases: vec!["array".to_owned()],
                alias_reads: vec![AliasPeers {
                    place: "array".to_owned(),
                    peers: vec!["peer".to_owned()],
                }],
                destination: Some("current_child".to_owned()),
                frame: None,
                chains: vec![crate::indirect_chain_access::AccessChain {
                    root: "array".to_owned(),
                    dereferenced_root: true,
                    root_never_bound: true,
                    root_bound_untracked: false,
                    root_untracked_only: false,
                    links: vec![],
                    member: "child".to_owned(),
                }],
                borrowed_destination: true,
                readback: None,
            },
            successors: vec![],
            line: 1,
            is_exit: false,
            loop_exit_edge: None,
        }
    }
    fn pointer() -> AbstractPointer {
        AbstractPointer {
            targets: BTreeSet::from([1]),
            ..Default::default()
        }
    }
    fn run(
        node: &LegacyCfgNode,
        state: AbstractState,
        alias: &NodeAliasFacts,
    ) -> (AbstractState, BTreeSet<super::super::LegacyDiagnostic>) {
        let context = TransferContext {
            derived_bases: BTreeSet::from([1]),
            caller_sites: BTreeSet::from([1]),
            ..Default::default()
        };
        let mut diagnostics = BTreeSet::new();
        let output = super::super::transfer_base(
            "get_array_item",
            node,
            state,
            &Default::default(),
            &context,
            alias,
            &mut diagnostics,
        );
        (output, diagnostics)
    }
    #[test]
    fn real_null_unresolved_and_released_peer_reads_match_original_diagnostics_and_state() {
        for guarded_cursor in [false, true] {
            let mut enhanced = node();
            if guarded_cursor {
                enhanced.lookup_read.as_mut().unwrap().guarded_cursor = true;
                if let LegacyInstruction::IndirectScalarAccess {
                    bases,
                    chains,
                    alias_reads,
                    ..
                } = &mut enhanced.instruction
                {
                    bases[0] = "current_child".to_owned();
                    chains[0].root = "current_child".to_owned();
                    chains[0].member = "next".to_owned();
                    alias_reads[0].place = "current_child".to_owned();
                }
            }
            let mut original = enhanced.clone();
            original.lookup_read = None;
            for case in 0..10 {
                let mut state = AbstractState::default();
                let mut parent = pointer();
                let mut aliases = NodeAliasFacts::default();
                match case {
                    0 => {
                        state.resources.insert(1, super::super::LIVE);
                    }
                    1 => {
                        state.resources.insert(1, super::super::RELEASED);
                    }
                    2 => {
                        parent.targets.clear();
                        parent.may_be_null = true;
                    }
                    3 => {
                        parent.unknown = true;
                    }
                    4 => {
                        parent.may_be_null = true;
                    }
                    5 => {
                        state.unresolved_effect_sites.insert(1);
                    }
                    6 => {
                        state.nullable_sites.insert(1);
                    }
                    7 => {
                        parent.derivation = Derivation::Mixed;
                    }
                    8 => {
                        state.pointers.insert("array->child".to_owned(), pointer());
                    }
                    9 => {
                        aliases.released.insert("peer".to_owned());
                        state.resources.insert(2, super::super::RELEASED);
                        state.pointers.insert(
                            "peer".to_owned(),
                            AbstractPointer {
                                targets: BTreeSet::from([2]),
                                ..Default::default()
                            },
                        );
                    }
                    _ => unreachable!(),
                }
                if guarded_cursor {
                    let mut cursor = parent.clone();
                    cursor.may_be_null = true;
                    cursor.derivation = Derivation::Derived {
                        extent: DerivedExtent::Borrowed,
                        const_dropped: true,
                        nullable: true,
                        stepped: super::super::Stepped::No,
                    };
                    state.pointers.insert("current_child".to_owned(), cursor);
                }
                state.pointers.insert("array".to_owned(), parent);
                let baseline = run(&original, state.clone(), &aliases);
                let current = run(&enhanced, state, &aliases);
                assert_eq!(current, baseline, "negative read case {case}");
                if case != 0 {
                    assert!(!current.1.is_empty(), "negative case {case} must refuse");
                }
                if case == 9 {
                    assert!(current.1.iter().any(|d| matches!(&d.kind,
                super::super::LegacyDiagnosticKind::UnknownAlias { variable } if variable == if guarded_cursor { "current_child/peer" } else { "array/peer" })));
                }
            }
        }
    }

    #[test]
    fn only_a_guarded_name_only_site_gains_nonowning_provenance() {
        for guarded_cursor in [false, true] {
            let mut read = node();
            let mut state = AbstractState::default();
            state.pointers.insert("array".to_owned(), pointer());
            if guarded_cursor {
                read.lookup_read.as_mut().unwrap().guarded_cursor = true;
                if let LegacyInstruction::IndirectScalarAccess {
                    bases,
                    chains,
                    alias_reads,
                    ..
                } = &mut read.instruction
                {
                    bases[0] = "current_child".to_owned();
                    chains[0].root = "current_child".to_owned();
                    chains[0].member = "next".to_owned();
                    alias_reads[0].place = "current_child".to_owned();
                }
                let mut cursor = super::super::derive_pointer(
                    state.pointers.get("array"),
                    DerivedExtent::Borrowed,
                    true,
                    true,
                );
                cursor.may_be_null = true;
                state.pointers.insert("current_child".to_owned(), cursor);
            }
            let before = state.clone();
            let (after, diagnostics) = run(&read, state, &NodeAliasFacts::default());
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
            assert_eq!(after.resources, before.resources);
            assert_eq!(after.contained, before.contained);
            assert_eq!(after.pointers["array"], before.pointers["array"]);
            let cursor = &after.pointers["current_child"];
            assert_eq!(cursor.targets, BTreeSet::from([1]));
            assert!(matches!(
                cursor.derivation,
                Derivation::Derived {
                    extent: DerivedExtent::Borrowed,
                    ..
                }
            ));
            assert!(!cursor.unknown);
        }
    }
}
