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

fn actual(index: &StatementIndex, argument: &Value, ty: &str) -> Option<(String, Vec<String>)> {
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
    Some((
        place.name,
        place.steps.into_iter().map(|step| step.member).collect(),
    ))
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
            let index = self.typed_positions_index()?;
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
            let (base, path) = actual(index, argument, &ty)?;
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

#[cfg(test)]
pub(crate) fn exercise_source_readers(source: &str) -> Option<bool> {
    let index = StatementIndex::read("chain")?;
    let (ty, _, writes) =
        self_link_init::checked_effect(&tokenize(source), "reset", &ResourceContracts::default())?;
    let mut work = vec![&index.function];
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
#[path = "../tests/unit/tracked_self_init.rs"]
mod tests;
