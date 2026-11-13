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

//! Rule 36b: source-checked self-writes transported into live owned storage.
//! The write set remains in the CFG. No neutral call contract is installed.
use super::*;
use crate::typed_access_path::{StatementIndex, qual};
use crate::typed_c_ast::{children, kind};
use crate::typed_place::{RootKind, StepKind, TypedPlace};
use serde_json::Value;

fn name(text: &str) -> bool {
    is_identifier(text) && text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

fn peel(mut node: &Value) -> Option<&Value> {
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("NoOp" | "LValueToRValue" | "FunctionToPointerDecay")
                ) => {}
            _ => return Some(node),
        }
        let [inner] = children(node) else { return None };
        node = inner;
    }
}

fn record_pointer(ty: &str) -> bool {
    ty.starts_with("struct ")
        && ty.ends_with(" *")
        && ty.matches('*').count() == 1
        && !ty.contains(['(', ')'])
        && !["const", "volatile", "_Atomic"]
            .iter()
            .any(|word| ty.contains(word))
}

// Iterative: arbitrary callers can inhabit the admitted deep-AST band.
fn unique_root(function: &Value, place: &TypedPlace) -> bool {
    let mut declarations = 0;
    let mut work = vec![function];
    while let Some(node) = work.pop() {
        match kind(node) {
            "ParmVarDecl" | "VarDecl" if node["name"].as_str() == Some(&place.name) => {
                if node["id"].as_str() != Some(&place.decl) || node.get("storageClass").is_some() {
                    return false;
                }
                declarations += 1;
            }
            "DeclRefExpr"
                if node["referencedDecl"]["name"].as_str() == Some(&place.name)
                    && node["referencedDecl"]["id"].as_str() != Some(&place.decl) =>
            {
                return false;
            }
            _ => {}
        }
        work.extend(children(node));
    }
    declarations == 1
}

fn actual(index: &StatementIndex, argument: &Value, ty: &str) -> Option<TypedPlace> {
    if qual(argument) != Some(ty) {
        return None;
    }
    let argument = peel(argument)?;
    let place = if kind(argument) == "DeclRefExpr" {
        let decl = &argument["referencedDecl"];
        if !matches!(decl["kind"].as_str(), Some("ParmVarDecl" | "VarDecl")) {
            return None;
        }
        TypedPlace {
            decl: decl["id"].as_str()?.to_owned(),
            name: decl["name"].as_str()?.to_owned(),
            root: RootKind::Pointer,
            decl_kind: decl["kind"].as_str()?.to_owned(),
            root_type: qual(argument)?.to_owned(),
            steps: Vec::new(),
        }
    } else if kind(argument) == "UnaryOperator" && argument["opcode"].as_str() == Some("&") {
        let [object] = children(argument) else {
            return None;
        };
        let reader = index.reader(&|_| false);
        let place = crate::typed_place::read_place(object, reader.fields()).ok()?;
        if place.root != RootKind::Pointer
            || place.steps.is_empty()
            || place.steps.len() > 3
            || place.steps.iter().enumerate().any(|(n, step)| {
                step.kind
                    != if n == 0 {
                        StepKind::Arrow
                    } else {
                        StepKind::Dot
                    }
                    || step.storage != crate::union_bitfield::MemberStorage::Ordinary
                    || !name(&step.member)
                    || !step.ty.starts_with("struct ")
                    || step.ty.contains('*')
                    || step.ty.contains("volatile")
                    || step.ty.contains("const")
                    || step.ty.contains("_Atomic")
            })
            || format!("{} *", place.steps.last()?.ty) != ty
        {
            return None;
        }
        place
    } else {
        return None;
    };
    if !record_pointer(&place.root_type)
        || !name(&place.name)
        || !unique_root(&index.function, &place)
    {
        return None;
    }
    Some(place)
}

type Declarations = BTreeMap<String, (String, String)>;

fn local_reference(node: &Value, declarations: &Declarations) -> Option<String> {
    let node = peel(node)?;
    if kind(node) != "DeclRefExpr" {
        return None;
    }
    let declaration = &node["referencedDecl"];
    let id = declaration["id"].as_str()?;
    let variable = declaration["name"].as_str()?;
    let (expected, ty) = declarations.get(variable)?;
    (id == expected && qual(node) == Some(ty.as_str())).then(|| variable.to_owned())
}

fn null_value(mut node: &Value) -> bool {
    loop {
        match kind(node) {
            "IntegerLiteral" => return node["value"].as_str() == Some("0"),
            "ParenExpr" | "ImplicitCastExpr" | "CStyleCastExpr" => {
                let [inner] = children(node) else {
                    return false;
                };
                node = inner;
            }
            _ => return false,
        }
    }
}

fn direct_callee(node: &Value) -> Option<(&str, &str)> {
    let [callee, ..] = children(node) else {
        return None;
    };
    let callee = peel(callee)?;
    let decl = &callee["referencedDecl"];
    if kind(callee) != "DeclRefExpr" || decl["kind"].as_str() != Some("FunctionDecl") {
        return None;
    }
    Some((decl["name"].as_str()?, decl["id"].as_str()?))
}

fn assignment_value(
    mut node: &Value,
    destination_type: &str,
    declarations: &Declarations,
    contracts: &ResourceContracts,
) -> bool {
    if null_value(node) {
        return true;
    }
    if let Some(variable) = local_reference(node, declarations) {
        return declarations[&variable].1 == destination_type;
    }
    // Pointer conversions of a new logical Owned result are allowed. An
    // integer round trip or a cast of an existing identity is not.
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr" | "CStyleCastExpr"
                if qual(node).is_some_and(|ty| ty.contains('*'))
                    && matches!(node["castKind"].as_str(), Some("NoOp" | "BitCast")) => {}
            _ => break,
        }
        let [inner] = children(node) else {
            return false;
        };
        node = inner;
    }
    kind(node) == "CallExpr"
        && qual(node).is_some_and(|ty| ty.contains('*'))
        && direct_callee(node).is_some_and(|(function, _)| {
            contracts
                .contract(function)
                .is_some_and(|c| matches!(c.result, ResourceReturnMode::Owned { .. }))
        })
}

fn ordinary_member(
    index: &StatementIndex,
    node: &Value,
    declarations: &Declarations,
) -> Option<TypedPlace> {
    let reader = index.reader(&|_| false);
    let place = crate::typed_place::read_place(node, reader.fields()).ok()?;
    let [step] = place.steps.as_slice() else {
        return None;
    };
    let (id, ty) = declarations.get(&place.name)?;
    (place.root == RootKind::Pointer
        && record_pointer(&place.root_type)
        && &place.decl == id
        && &place.root_type == ty
        && step.kind == StepKind::Arrow
        && step.storage == crate::union_bitfield::MemberStorage::Ordinary
        && name(&step.member)
        && !["const", "volatile", "_Atomic"]
            .iter()
            .any(|q| step.ty.contains(q)))
    .then_some(place)
}

/// Optional, source-sealed slot proof. No statement receives call permission
/// from this scan. Every walk over the caller is iterative.
fn source_frame(
    index: &StatementIndex,
    actual: &TypedPlace,
    declaration: &str,
    writes: &BTreeSet<String>,
    contracts: &ResourceContracts,
) -> Option<(Vec<String>, Vec<String>)> {
    let mut declarations = Declarations::new();
    let mut work = vec![index.function.as_ref()];
    while let Some(node) = work.pop() {
        if matches!(kind(node), "ParmVarDecl" | "VarDecl") {
            let variable = node["name"].as_str()?;
            if !name(variable) || node.get("storageClass").is_some() {
                return None;
            }
            if declarations
                .insert(
                    variable.to_owned(),
                    (node["id"].as_str()?.to_owned(), qual(node)?.to_owned()),
                )
                .is_some()
            {
                return None;
            }
        }
        work.extend(children(node));
    }
    let root_tag = actual
        .root_type
        .strip_prefix("struct ")?
        .strip_suffix(" *")?;
    let records = index
        .records
        .names
        .iter()
        .filter(|(_, tag)| tag.as_str() == root_tag)
        .collect::<Vec<_>>();
    let [(record, _)] = records.as_slice() else {
        return None;
    };
    if let Some(first) = actual.steps.first()
        && index
            .records
            .struct_members
            .get(&first.field)
            .map(|(owner, _)| owner)
            != Some(*record)
    {
        return None;
    }
    let compatible = declarations
        .iter()
        .filter_map(|(n, (_, ty))| (ty == &actual.root_type).then_some(n.clone()))
        .collect::<Vec<_>>();
    if !compatible.contains(&actual.name) {
        return None;
    }
    let mut candidates = BTreeSet::new();
    let mut work = vec![index.function.as_ref()];
    while let Some(node) = work.pop() {
        match kind(node) {
            "VarDecl" if qual(node)?.contains('*') => match children(node) {
                [] => {}
                [value] if assignment_value(value, qual(node)?, &declarations, contracts) => {}
                _ => return None,
            },
            "BinaryOperator" if node["opcode"].as_str() == Some("=") => {
                let [left, right] = children(node) else {
                    return None;
                };
                let ty = qual(left)?;
                if ty.contains('*') {
                    if local_reference(left, &declarations).is_none()
                        && ordinary_member(index, left, &declarations).is_none()
                    {
                        return None;
                    }
                    if !assignment_value(right, ty, &declarations, contracts) {
                        return None;
                    }
                }
            }
            "CompoundAssignOperator" if qual(node)?.contains('*') => return None,
            "UnaryOperator" => {
                if node["opcode"].as_str() == Some("&") {
                    let [object] = children(node) else {
                        return None;
                    };
                    if qual(object)?.contains('*') {
                        return None;
                    }
                }
                if matches!(node["opcode"].as_str(), Some("++" | "--")) && qual(node)?.contains('*')
                {
                    return None;
                }
            }
            "CallExpr" => {
                let (function, id) = direct_callee(node)?;
                if id != declaration {
                    let contract = contracts.contract(function)?;
                    if contracts.relational_contract(function).is_some()
                        || contracts.retaining_contract(function).is_some()
                        || contracts.lifetime_contract(function).is_some()
                        || contracts.lifetime_summary(function).is_some()
                        || !contract.container_effects.is_empty()
                        || !contract.counted_users.is_empty()
                        || contract.parameters.iter().any(|mode| {
                            matches!(
                                mode,
                                ResourceParameterMode::Out { .. }
                                    | ResourceParameterMode::Stored { .. }
                                    | ResourceParameterMode::Removed { .. }
                            )
                        })
                    {
                        return None;
                    }
                }
            }
            "MemberExpr" if qual(node)?.contains('*') => {
                if let Some(place) = ordinary_member(index, node, &declarations) {
                    let step = &place.steps[0];
                    if place.root_type == actual.root_type
                        && index
                            .records
                            .struct_members
                            .get(&step.field)
                            .map(|(owner, _)| owner)
                            == Some(*record)
                    {
                        candidates.insert(step.member.clone());
                    }
                }
            }
            _ => {}
        }
        work.extend(children(node));
    }
    let touched = actual.steps.first().map_or_else(
        || writes.clone(),
        |first| BTreeSet::from([first.member.clone()]),
    );
    candidates.retain(|field| !touched.contains(field));
    if candidates.is_empty() || touched.is_empty() {
        return None;
    }
    let dump = crate::typed_access_path::record_layout_dump()?;
    let layout = crate::footprint_connection::parse_member_layout(&dump, root_tag)?;
    candidates.retain(|field| {
        touched.iter().all(|written| {
            crate::footprint_connection::core_members_disjoint(&layout, field, written, false)
        })
    });
    (!candidates.is_empty()).then(|| (candidates.into_iter().collect(), compatible))
}

impl Parser<'_> {
    pub(super) fn tracked_self_init_call(
        &mut self,
        tokens: &[Token],
        fallback: LegacyInstruction,
    ) -> LegacyInstruction {
        if !matches!(fallback, LegacyInstruction::Unknown { .. }) {
            return fallback;
        }
        let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
        if texts.len() < 4 || !name(texts[0]) || texts[1] != "(" || texts.last() != Some(&")") {
            return fallback;
        }
        let LegacyInstruction::Unknown { description, .. } = &fallback else {
            return fallback;
        };
        if !(description == "indirect memory access is outside the resource projection"
            || *description == format!("call to uncontracted function `{}`", texts[0]))
            || descriptor_custody::statement_reads_descriptor(&texts, &self.descriptor_names)
                .is_some()
            || texts.iter().any(|text| self.in_place_names.contains(text))
        {
            return fallback;
        }
        // Cheap lexical shape filter. The AST and its exact occurrence decide below.
        let arg = &texts[2..texts.len() - 1];
        if !(matches!(arg, [root] if name(root))
            || matches!(arg, ["&", root, "-", ">", field, rest @ ..]
                if name(root) && name(field) && rest.len() <= 4 && rest.chunks(2).all(|p| matches!(p, [".", f] if name(f)))))
        {
            return fallback;
        }
        let saved = self.probe_state();
        let answer = (|| {
            let (ty, declaration, writes) =
                self_link_init::checked_effect(self.tokens, texts[0], self.contracts)?;
            let index = self.typed_positions_index()?.clone();
            let occurrence = || {
                crate::typed_place::occurrence(self.tokens, self.typed_place_origin.get(), tokens)
            };
            let node = index.statement_node(&texts, tokens.first()?.line, &occurrence)?;
            let ("CallExpr", [callee, argument]) = (kind(node), children(node)) else {
                return None;
            };
            let callee = peel(callee)?;
            if kind(callee) != "DeclRefExpr"
                || callee["referencedDecl"]["kind"].as_str() != Some("FunctionDecl")
                || callee["referencedDecl"]["name"].as_str() != Some(texts[0])
                || callee["referencedDecl"]["id"].as_str() != Some(&declaration)
            {
                return None;
            }
            let place = actual(&index, argument, &ty)?;
            let frame_saved = self.probe_state();
            let frame = source_frame(&index, &place, &declaration, &writes, self.contracts);
            if frame.is_none() {
                self.restore_probe_state(&frame_saved);
            }
            let (protected_fields, compatible_roots) = frame.unwrap_or_default();
            let base = place.name;
            let path = place
                .steps
                .into_iter()
                .map(|step| step.member)
                .collect::<Vec<_>>();
            let access = LegacyInstruction::IndirectScalarAccess {
                alias_reads: self.read_alias(Some(&base)).into_iter().collect(),
                bases: vec![base.clone()],
                destination: None,
                frame: None,
                chains: vec![crate::indirect_chain_access::AccessChain {
                    root: base.clone(),
                    dereferenced_root: true,
                    root_never_bound: false,
                    root_bound_untracked: false,
                    root_untracked_only: false,
                    links: Vec::new(),
                    member: path.join("."),
                }],
                borrowed_destination: false,
                readback: None,
                typed_alternative: None,
            };
            Some(LegacyInstruction::TrackedSelfInit {
                function: texts[0].to_owned(),
                base,
                path,
                writes: writes.into_iter().collect(),
                protected_fields,
                compatible_roots,
                access: Box::new(access),
                fallback: Box::new(fallback.clone()),
            })
        })();
        match answer {
            Some(instruction) => instruction,
            None => {
                self.restore_probe_state(&saved);
                fallback
            }
        }
    }
}

pub(crate) fn has_call(cfg: &LegacyCfg) -> bool {
    cfg.nodes
        .iter()
        .any(|n| matches!(n.instruction, LegacyInstruction::TrackedSelfInit { .. }))
}

/// Preserve the complete custody frame. Relations outside this bounded frame
/// refuse; they are never cleared to manufacture admission. Future fields join
/// this check automatically.
pub(super) fn apply(state: &AbstractState, base: &str) -> Option<AbstractState> {
    let pointer = state.pointers.get(base)?;
    if pointer.unknown
        || pointer.may_be_null
        || !pointer.derivation.is_identity()
        || pointer.targets.len() != 1
    {
        return None;
    }
    let site = *pointer.targets.first()?;
    if state.resources.get(&site) != Some(&LIVE) || state.nullable_sites.contains(&site) {
        return None;
    }
    let mut frame = state.clone();
    frame.pointers.clear();
    frame.resources.clear();
    frame.nullable_sites.clear();
    // Card LERI-HELD-AND-LINK-MEMBER-PREMISES (gateway rule 46): a link
    // record an earlier initialisation established is a fact about those
    // writes, not a custody relation, and this route's own output.
    frame.link_ready.clear();
    if frame != AbstractState::default() {
        return None;
    }
    let roots = state
        .pointers
        .iter()
        .filter_map(|(name, p)| {
            (p.targets.contains(&site)
                && super::is_identifier(name)
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            .then_some(name)
        })
        .collect::<Vec<_>>();
    let affected = |place: &str| {
        roots
            .iter()
            .any(|root| pointer_store_flow::rooted_at(place, root))
    };
    if state.pointers.iter().any(|(place, old)| {
        affected(place) && (old.unknown || old.targets.iter().any(|target| *target != site))
    }) {
        return None;
    }
    let mut output = state.clone();
    output.pointers.retain(|place, _| !affected(place));
    Some(output)
}

/// The additive nonempty frame. Owning relations are checked and preserved,
/// never erased. No pair of distinct resource IDs is assumed spatially disjoint.
pub(super) fn apply_frame(
    state: &AbstractState,
    base: &str,
    protected_fields: &[String],
    compatible_roots: &[String],
    context: &TransferContext<'_>,
) -> Option<AbstractState> {
    if protected_fields.is_empty()
        || !protected_fields.iter().all(|field| name(field))
        || !compatible_roots.iter().all(|root| name(root))
        || !context.caller_sites.is_empty()
        || !context.sanctioned.is_empty()
    {
        return None;
    }
    let pointer = state.pointers.get(base)?;
    if pointer.unknown
        || pointer.may_be_null
        || !pointer.derivation.is_identity()
        || pointer.targets.len() != 1
    {
        return None;
    }
    let site = *pointer.targets.first()?;
    let precise_heap = |id| {
        crate::loop_generations::generation(id) != crate::loop_generations::Generation::Older
            && kind_of(&context.site_kinds, id).is_heap()
    };
    if !precise_heap(site)
        || state.resources.get(&site) != Some(&LIVE)
        || state.nullable_sites.contains(&site)
    {
        return None;
    }
    let mut residue = state.clone();
    residue.pointers.clear();
    residue.resources.clear();
    residue.nullable_sites.clear();
    residue.field_links.clear();
    residue.partial_links.clear();
    // Card LERI-HELD-AND-LINK-MEMBER-PREMISES (gateway rule 46): as in
    // [`apply`], a link record is not a custody relation.
    residue.link_ready.clear();
    if residue != AbstractState::default() {
        return None;
    }
    if state.field_links.len() != 1 {
        return None;
    }
    let fields = state.field_links.get(&site)?;
    if fields.is_empty()
        || fields.iter().any(|(field, children)| {
            !protected_fields.contains(field)
                || children.is_empty()
                || children.iter().any(|child| {
                    *child == site
                        || !precise_heap(*child)
                        || state.resources.get(child) != Some(&LIVE)
                        || state.nullable_sites.contains(child)
                })
        })
    {
        return None;
    }
    if state.partial_links.iter().any(|(owner, partial)| {
        *owner != site
            || partial.is_empty()
            || partial.iter().any(|(field, children)| {
                children.is_empty() || !fields.get(field).is_some_and(|all| children.is_subset(all))
            })
    }) {
        return None;
    }
    // Recognize plain bindings and member-place facts only. Pointer/out-slot
    // keys and synthetic records need a separate frame rule.
    let member_root = |place: &str| -> Option<String> {
        let end = place.find(['-', '.', '['])?;
        let (root, rest) = place.split_at(end);
        (name(root) && (rest.starts_with("->") || rest.starts_with('.') || rest.starts_with('[')))
            .then(|| root.to_owned())
    };
    let roots = state
        .pointers
        .iter()
        .filter_map(|(root, value)| (name(root) && value.targets.contains(&site)).then_some(root))
        .collect::<Vec<_>>();
    if roots.iter().any(|root| {
        let value = &state.pointers[*root];
        value.derivation.is_identity()
            && (value.unknown
                || value.may_be_null
                || value.targets.len() != 1
                || !compatible_roots.contains(root))
    }) {
        return None;
    }
    let protected = |place: &str| {
        let (Some(root), Some(field)) = (
            crate::aggregate_store::place_owner(place),
            crate::aggregate_store::place_field(place),
        ) else {
            return false;
        };
        let Some(value) = state.pointers.get(root) else {
            return false;
        };
        protected_fields.iter().any(|f| f == field)
            && compatible_roots.iter().any(|r| r == root)
            && !value.unknown
            && !value.may_be_null
            && value.derivation.is_identity()
            && value.targets == BTreeSet::from([site])
    };
    for (place, old) in &state.pointers {
        if name(place) {
            continue;
        }
        member_root(place)?;
        if !protected(place)
            && roots
                .iter()
                .any(|root| pointer_store_flow::rooted_at(place, root))
            && (old.unknown || old.targets.iter().any(|target| *target != site))
        {
            return None;
        }
    }
    let mut output = state.clone();
    output
        .pointers
        .retain(|place, _| name(place) || protected(place));
    Some(output)
}

#[cfg(test)]
pub(crate) fn exercise_source_readers(source: &str) -> Option<bool> {
    let index = StatementIndex::read("chain")?;
    let (ty, _, writes) =
        self_link_init::checked_effect(&tokenize(source), "reset", &ResourceContracts::default())?;
    let mut work = vec![index.function.as_ref()];
    while let Some(node) = work.pop() {
        if kind(node) == "CallExpr"
            && let [_, arg] = children(node)
        {
            return Some(
                actual(&index, arg, &ty).is_some() && writes == BTreeSet::from(["next".to_owned()]),
            );
        }
        work.extend(children(node));
    }
    Some(false)
}

#[cfg(test)]
pub(crate) fn exercise_frame_source_readers(source: &str) -> Option<bool> {
    let index = StatementIndex::read("chain")?;
    let contracts = ResourceContracts::default();
    let (ty, declaration, writes) =
        self_link_init::checked_effect(&tokenize(source), "reset", &contracts)?;
    let mut work = vec![index.function.as_ref()];
    while let Some(node) = work.pop() {
        if kind(node) == "CallExpr" && direct_callee(node).is_some_and(|(_, id)| id == declaration)
        {
            let [_, arg] = children(node) else {
                return None;
            };
            let place = actual(&index, arg, &ty)?;
            return Some(
                source_frame(&index, &place, &declaration, &writes, &contracts)
                    .is_some_and(|(fields, roots)| fields == ["payload"] && roots == ["p"]),
            );
        }
        work.extend(children(node));
    }
    Some(false)
}

#[cfg(test)]
#[path = "../tests/unit/tracked_self_init.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/unit/tracked_self_init_frames.rs"]
mod frame_tests;
