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

//! Rule 36a: unit-local calls to checked self-link initializer leaves.
//! CG11 owns the stores. This route publishes no custody contract.
use super::*;
use crate::typed_access_path::{StatementIndex, qual};
use crate::typed_c_ast::{children, kind};
use serde_json::Value;

#[derive(Clone, Debug)]
struct Profile {
    parameter: String,
    pointer_type: String,
    declaration: String,
    stores: BTreeSet<(usize, String)>,
    calls: BTreeMap<(usize, Vec<String>), String>,
}

fn plain_name(name: &str) -> bool {
    is_identifier(name)
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn peel(mut node: &Value) -> Option<&Value> {
    loop {
        match kind(node) {
            "ParenExpr" => {}
            "ImplicitCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("LValueToRValue" | "NoOp" | "FunctionToPointerDecay")
                ) => {}
            _ => return Some(node),
        }
        let [child] = children(node) else { return None };
        node = child;
    }
}

fn original(node: &Value, parameter: &str, id: &str, ty: &str) -> bool {
    peel(node).is_some_and(|node| {
        kind(node) == "DeclRefExpr"
            && qual(node) == Some(ty)
            && node["referencedDecl"]["kind"].as_str() == Some("ParmVarDecl")
            && node["referencedDecl"]["name"].as_str() == Some(parameter)
            && node["referencedDecl"]["id"].as_str() == Some(id)
    })
}

fn null(mut node: &Value) -> bool {
    loop {
        match kind(node) {
            "IntegerLiteral" => return node["value"].as_str() == Some("0"),
            "GNUNullExpr" => return true,
            "ParenExpr" => {}
            "ImplicitCastExpr"
                if matches!(
                    node["castKind"].as_str(),
                    Some("NullToPointer" | "NoOp" | "BitCast")
                ) => {}
            "CStyleCastExpr"
                if qual(node) == Some("void *")
                    && node["castKind"].as_str() == Some("NullToPointer") => {}
            _ => return false,
        }
        let [child] = children(node) else {
            return false;
        };
        node = child;
    }
}

fn guard(node: &Value, name: &str, id: &str, ty: &str) -> bool {
    let Some(node) = peel(node) else { return false };
    if original(node, name, id, ty) {
        return true;
    }
    match (kind(node), children(node)) {
        ("UnaryOperator", [value]) if node["opcode"].as_str() == Some("!") => {
            original(value, name, id, ty)
        }
        ("BinaryOperator", [left, right])
            if matches!(node["opcode"].as_str(), Some("==" | "!=")) =>
        {
            (original(left, name, id, ty) && null(right))
                || (null(left) && original(right, name, id, ty))
        }
        _ => false,
    }
}

fn unescaped(tokens: &[Token], name: &str) -> bool {
    tokens.iter().enumerate().all(|(at, token)| {
        token.text != name || tokens.get(at + 1).is_some_and(|next| next.text == "(")
    })
}

fn profile(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
    leaf: bool,
) -> Option<Profile> {
    let parameters = function_parameters(tokens, function)?;
    let [Some(parameter)] = parameters.as_slice() else {
        return None;
    };
    if !unescaped(tokens, function)
        || contracts.has_guarded_interface(function)
        || contracts.contract(function).is_some()
    {
        return None;
    }
    let index = StatementIndex::read(function)?;
    let function_type = qual(&index.function)?;
    if !function_type.starts_with("void (") || function_type.contains("...") {
        return None;
    }
    if children(&index.function)
        .iter()
        .any(|n| !matches!(kind(n), "ParmVarDecl" | "CompoundStmt"))
    {
        return None;
    }
    let formals = children(&index.function)
        .iter()
        .filter(|n| kind(n) == "ParmVarDecl")
        .collect::<Vec<_>>();
    let [formal] = formals.as_slice() else {
        return None;
    };
    if formal["name"].as_str() != Some(parameter) {
        return None;
    }
    let id = formal["id"].as_str().filter(|id| !id.is_empty())?;
    let ty = qual(formal)?;
    if !ty.starts_with("struct ")
        || !ty.ends_with(" *")
        || ty.matches('*').count() != 1
        || ty.contains(['(', ')'])
        || ty.contains("const")
        || ty.contains("volatile")
        || ty.contains("_Atomic")
    {
        return None;
    }
    let body = children(&index.function)
        .iter()
        .find(|n| kind(n) == "CompoundStmt")?;
    let (start, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, start, "{", "}")?;
    let body_tokens = index.node_tokens(body)?;
    if !body_tokens
        .iter()
        .map(String::as_str)
        .eq(tokens[start..=end].iter().map(|t| t.text.as_str()))
    {
        return None;
    }
    let mut result = Profile {
        parameter: parameter.clone(),
        pointer_type: ty.to_owned(),
        declaration: index.function["id"]
            .as_str()
            .filter(|id| !id.is_empty())?
            .to_owned(),
        stores: BTreeSet::new(),
        calls: BTreeMap::new(),
    };
    let reader = index.reader(&|_| false);
    let mut work = vec![body];
    while let Some(node) = work.pop() {
        match (kind(node), children(node)) {
            ("CompoundStmt", inner) => work.extend(inner),
            ("ReturnStmt", []) => {}
            ("IfStmt", [condition, then]) if guard(condition, parameter, id, ty) => work.push(then),
            ("IfStmt", [condition, then, otherwise]) if guard(condition, parameter, id, ty) => {
                work.push(then);
                work.push(otherwise);
            }
            ("BinaryOperator", [left, right]) if node["opcode"].as_str() == Some("=") => {
                if kind(left) != "MemberExpr"
                    || left["isArrow"].as_bool() != Some(true)
                    || qual(left) != Some(ty)
                    || !original(right, parameter, id, ty)
                {
                    return None;
                }
                let [base] = children(left) else { return None };
                if !original(base, parameter, id, ty) {
                    return None;
                }
                let field = left["name"].as_str().filter(|s| plain_name(s))?;
                let facts = reader
                    .fields()
                    .get(left["referencedMemberDecl"].as_str()?)?
                    .as_ref()?;
                if facts.union_owner || facts.bitfield {
                    return None;
                }
                let texts = index.node_tokens(node)?;
                if texts.iter().map(String::as_str).collect::<Vec<_>>()
                    != [parameter, "-", ">", field, "=", parameter]
                {
                    return None;
                }
                if !result
                    .stores
                    .insert((index.node_line(node)?, format!("{parameter}->{field}")))
                {
                    return None;
                }
            }
            ("CallExpr", [callee, argument]) if !leaf && original(argument, parameter, id, ty) => {
                let callee = peel(callee)?;
                if kind(callee) != "DeclRefExpr"
                    || callee["referencedDecl"]["kind"].as_str() != Some("FunctionDecl")
                {
                    return None;
                }
                let name = callee["referencedDecl"]["name"].as_str()?;
                let target = profile(tokens, name, contracts, true)?;
                if target.pointer_type != ty
                    || callee["referencedDecl"]["id"].as_str() != Some(target.declaration.as_str())
                {
                    return None;
                }
                let texts = index.node_tokens(node)?;
                if texts.iter().map(String::as_str).collect::<Vec<_>>()
                    != [name, "(", parameter, ")"]
                {
                    return None;
                }
                if result
                    .calls
                    .insert((index.node_line(node)?, texts), name.to_owned())
                    .is_some()
                {
                    return None;
                }
            }
            _ => return None,
        }
    }
    if (leaf && result.stores.is_empty()) || (!leaf && result.calls.is_empty()) {
        return None;
    }
    Some(result)
}

fn empty_exits(summary: &LegacyFunctionSummary, exits: &[ExitObservation]) -> bool {
    summary.status == AnalysisStatus::Safe
        && summary.diagnostics.is_empty()
        && summary.lifetime_effects.is_empty()
        && !exits.is_empty()
        && !summary
            .cfg
            .nodes
            .iter()
            .any(|n| matches!(n.instruction, LegacyInstruction::Unknown { .. }))
        && exits.iter().all(|e| {
            e.parameters.iter().all(Option::is_none)
                && e.out_slots.iter().all(Option::is_none)
                && e.returned == ReturnedValue::None
                && e.caller_containment.is_empty()
                && e.pending_replacements.is_empty()
                && !e.derived_base_touched
                && e.selected.is_none()
        })
}

fn entry(cfg: &LegacyCfg, profile: &Profile, enabled: bool) -> Option<EntryCustody> {
    let contract = ResourceFunctionContract::new(
        vec![ResourceParameterMode::Borrowed],
        ResourceReturnMode::None,
    );
    let mut entry = EntryCustody::build(cfg, &[Some(profile.parameter.clone())], &contract)?;
    entry.allow_self_init_frame = enabled;
    Some(entry)
}

fn stores_match(cfg: &LegacyCfg, profile: &Profile) -> bool {
    let mut stores = BTreeSet::new();
    for node in &cfg.nodes {
        if let LegacyInstruction::PointerStore { store, .. } = &node.instruction
            && (store.root != profile.parameter
                || store.value != "self-address"
                || store.name.is_some()
                || !stores.insert((node.line, store.place.clone())))
        {
            return false;
        }
    }
    stores == profile.stores
}

fn checked_leaf(tokens: &[Token], function: &str, contracts: &ResourceContracts) -> Option<()> {
    let profile = profile(tokens, function, contracts, true)?;
    let cfg = build_function_cfg(tokens, function, contracts).ok()?;
    if !stores_match(&cfg, &profile) || has_call(&cfg) {
        return None;
    }
    let entry = entry(&cfg, &profile, false)?;
    let (summary, exits) = analyze_cfg_with_entry(cfg, &entry);
    empty_exits(&summary, &exits).then_some(())
}

/// Source and full-CFG checked write set; never a custody summary.
pub(super) fn checked_effect(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
) -> Option<(String, String, BTreeSet<String>)> {
    // A leaf contains no calls. Decline obvious calls before loading a
    // potentially large callee AST; the complete typed check still decides.
    let (start, _) = find_function_body(tokens, function)?;
    let end = matching(tokens, start, "{", "}")?;
    if tokens[start..=end]
        .windows(2)
        .any(|pair| is_identifier(&pair[0].text) && pair[1].text == "(" && pair[0].text != "if")
    {
        return None;
    }
    checked_leaf(tokens, function, contracts)?;
    let leaf = profile(tokens, function, contracts, true)?;
    let fields = leaf
        .stores
        .iter()
        .map(|(_, place)| {
            place
                .strip_prefix(&format!("{}->", leaf.parameter))
                .map(str::to_owned)
        })
        .collect::<Option<BTreeSet<_>>>()?;
    Some((leaf.pointer_type, leaf.declaration, fields))
}

impl Parser<'_> {
    pub(super) fn self_init_call(
        &mut self,
        tokens: &[Token],
        fallback: LegacyInstruction,
    ) -> LegacyInstruction {
        if !matches!(fallback, LegacyInstruction::Unknown { .. }) {
            return fallback;
        }
        let texts = tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>();
        let [callee, "(", actual, ")"] = texts.as_slice() else {
            return fallback;
        };
        if !plain_name(callee) || !plain_name(actual) {
            return fallback;
        }
        let saved = self.probe_state();
        let answer = (|| {
            let profile = profile(
                self.tokens,
                &self.typed_path_function,
                self.contracts,
                false,
            )?;
            if profile.parameter != *actual
                || profile
                    .calls
                    .get(&(
                        tokens.first()?.line,
                        texts.iter().map(|s| (*s).to_owned()).collect(),
                    ))?
                    .as_str()
                    != *callee
            {
                return None;
            }
            checked_leaf(self.tokens, callee, self.contracts)?;
            Some(LegacyInstruction::SelfInitCall {
                function: (*callee).to_owned(),
                base: (*actual).to_owned(),
                fallback: Box::new(fallback.clone()),
            })
        })();
        match answer {
            Some(call) => call,
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
        .any(|n| matches!(n.instruction, LegacyInstruction::SelfInitCall { .. }))
}

pub(crate) fn candidate(tokens: &[Token], function: &str, contracts: &ResourceContracts) -> bool {
    let Some(parameters) = function_parameters(tokens, function) else {
        return false;
    };
    let [Some(parameter)] = parameters.as_slice() else {
        return false;
    };
    let Some((start, _)) = find_function_body(tokens, function) else {
        return false;
    };
    let Some(end) = matching(tokens, start, "{", "}") else {
        return false;
    };
    tokens[start..end]
        .windows(4)
        .any(|run| run[1].text == "(" && run[2].text == *parameter && run[3].text == ")")
        && profile(tokens, function, contracts, false).is_some()
}

pub(crate) fn restore_unreachable(cfg: &mut LegacyCfg) {
    let mut reachable = BTreeSet::new();
    let mut work = vec![cfg.entry];
    while let Some(id) = work.pop() {
        if reachable.insert(id) {
            work.extend(&cfg.nodes[id].successors);
        }
    }
    for (id, node) in cfg.nodes.iter_mut().enumerate() {
        if !reachable.contains(&id)
            && let LegacyInstruction::SelfInitCall { fallback, .. }
            | LegacyInstruction::TrackedSelfInit { fallback, .. } = &node.instruction
        {
            node.instruction = *fallback.clone();
        }
    }
}

pub(crate) fn row(
    tokens: &[Token],
    function: &str,
    contracts: &ResourceContracts,
    cfg: LegacyCfg,
) -> Option<LegacyFunctionSummary> {
    let profile = profile(tokens, function, contracts, false)?;
    if !has_call(&cfg) || !stores_match(&cfg, &profile) {
        return None;
    }
    let entry = entry(&cfg, &profile, true)?;
    let (summary, exits) = analyze_cfg_with_entry(cfg, &entry);
    empty_exits(&summary, &exits).then_some(summary)
}

pub(super) fn admits(state: &AbstractState, base: &str, enabled: bool) -> bool {
    if !enabled || state.pointers.keys().any(|name| !plain_name(name)) {
        return false;
    }
    let mut frame = state.clone();
    frame.pointers.clear();
    if frame != AbstractState::default() {
        return false;
    }
    state
        .pointers
        .get(base)
        .is_none_or(|p| !p.unknown && p.derivation.is_identity() && p.targets.len() == 1)
}

#[cfg(test)]
pub(crate) fn exercise_source_readers(source: &str) -> Option<bool> {
    let _index = StatementIndex::read("chain")?;
    Some(
        profile(
            &tokenize(source),
            "chain",
            &ResourceContracts::default(),
            false,
        )
        .is_some(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEAF: &str = "struct Ring { struct Ring *next, *prev; };\nstatic inline void reset(struct Ring *p) {\n if (p == (void *)0) return;\n p->next = p;\n p->prev = p;\n}\n";

    fn clang() -> bool {
        match crate::clang_command() {
            Ok(_) => true,
            Err(reason) => {
                assert!(
                    std::env::var_os("QCORE_CLANG").is_none(),
                    "invalid explicit Clang: {reason}"
                );
                eprintln!("skipping Clang-dependent checked-call control: {reason}");
                false
            }
        }
    }

    fn run(source: &str, contracts: &ResourceContracts) -> UnrestrictedLegacyResult {
        analyze_unrestricted_c11(
            &C11TranslationUnit {
                source,
                ..Default::default()
            },
            "subject",
            contracts,
        )
        .expect("valid fixture")
    }

    fn summary(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
        match result {
            UnrestrictedLegacyResult::Safe(c) => &c.summary,
            UnrestrictedLegacyResult::Unsafe(s) => s,
            UnrestrictedLegacyResult::Unknown {
                partial: Some(s), ..
            } => s,
            other => panic!("missing summary: {other:?}"),
        }
    }

    #[test]
    fn checked_call_replays_without_publishing_an_ordinary_summary() {
        if !clang() {
            return;
        }
        let source = format!("{LEAF}void subject(struct Ring *p) {{\n reset(p);\n reset(p);\n}}\n");
        let UnrestrictedLegacyResult::Safe(certificate) =
            run(&source, &ResourceContracts::default())
        else {
            panic!(
                "checked call must be Safe: {:?}",
                run(&source, &ResourceContracts::default())
            );
        };
        assert!(certificate.summaries.is_empty());
        assert!(has_call(&certificate.summary.functions["subject"].cfg));
        check_legacy_safety_certificate(&certificate).expect("fresh source replay");
        let mut forged = certificate.clone();
        let node = forged
            .summary
            .functions
            .get_mut("subject")
            .unwrap()
            .cfg
            .nodes
            .iter_mut()
            .find(|n| matches!(n.instruction, LegacyInstruction::SelfInitCall { .. }))
            .unwrap();
        node.instruction = LegacyInstruction::NoOp;
        assert!(check_legacy_safety_certificate(&forged).is_err());
    }

    #[test]
    fn larger_callers_aliases_callbacks_and_second_level_wrappers_refuse() {
        if !clang() {
            return;
        }
        for tail in [
            "void subject(struct Ring *p) { struct Ring *q = p; reset(q); }",
            "void subject(struct Ring *p) { reset((struct Ring *)p); }",
            "void subject(struct Ring *p) { reset(&p->next[0]); }",
            "void subject(struct Ring *p) { reset(p); p = 0; }",
            "void subject(struct Ring *p) { reset(p); } void (*cb)(struct Ring *) = subject;",
            "void subject(struct Ring *p) { reset(p); } void (*cb)(struct Ring *) = reset;",
            "void middle(struct Ring *p) { reset(p); } void subject(struct Ring *p) { middle(p); }",
            "void subject(struct Ring *p) { reset(p); subject(p); }",
        ] {
            let source = format!("{LEAF}{tail}\n");
            let result = run(&source, &ResourceContracts::default());
            let row = &summary(&result).functions["subject"];
            assert_ne!(row.status, AnalysisStatus::Safe, "{tail}: {result:?}");
            assert!(
                !has_call(&row.cfg),
                "general callers retain their old calls: {tail}"
            );
        }
    }

    #[test]
    fn source_shape_and_existing_declared_interface_keep_the_boundary() {
        if !clang() {
            return;
        }
        for body in [
            "struct Ring *q = p; p->next = q;",
            "p->next = (struct Ring *)p;",
            "p->next = p; p = 0;",
            "if (p->next) p->next = p;",
        ] {
            let source = format!(
                "struct Ring {{ struct Ring *next; }}; void reset(struct Ring *p) {{ {body} }} void subject(struct Ring *p) {{ reset(p); }}"
            );
            let result = run(&source, &ResourceContracts::default());
            assert!(
                !has_call(&summary(&result).functions["subject"].cfg),
                "{body}"
            );
            assert_ne!(
                summary(&result).functions["subject"].status,
                AnalysisStatus::Safe
            );
        }
        let source = format!("{LEAF}void subject(struct Ring *p) {{ reset(p); }}");
        let mut contracts = ResourceContracts::default();
        contracts.insert_contract(
            "reset",
            ResourceFunctionContract::new(
                vec![ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            ),
        );
        let result = run(&source, &contracts);
        assert!(!has_call(&summary(&result).functions["subject"].cfg));
        assert!(summary(&result).functions["subject"].cfg.nodes.iter().any(|n| matches!(&n.instruction, LegacyInstruction::ContractCall { function, .. } if function == "reset")));
        let mut caller_contract = ResourceContracts::default();
        caller_contract.insert_contract(
            "subject",
            ResourceFunctionContract::new(
                vec![ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            ),
        );
        let result = run(&source, &caller_contract);
        assert!(!has_call(&summary(&result).functions["subject"].cfg));
        assert_ne!(
            summary(&result).functions["subject"].status,
            AnalysisStatus::Safe
        );
    }

    #[test]
    fn tracked_factories_and_destructors_do_not_gain_a_hidden_self_alias() {
        if !clang() {
            return;
        }
        let mut contracts = ResourceContracts::default();
        contracts.insert_contract(
            "owned",
            ResourceFunctionContract::new(vec![], ResourceReturnMode::Owned { nullable: false }),
        );
        contracts.insert_contract(
            "drop_owned",
            ResourceFunctionContract::new(
                vec![ResourceParameterMode::Released { nullable: true }],
                ResourceReturnMode::None,
            ),
        );
        let prefix = format!("{LEAF}struct Ring *owned(void); void drop_owned(void *p);\n");
        for tail in [
            "void subject(struct Ring *p) { struct Ring *q = owned(); reset(p); }",
            "void subject(struct Ring *p) { struct Ring *q = owned(); reset(p); drop_owned(q); }",
            "struct Ring *factory(void) { struct Ring *p = owned(); reset(p); return p; } void destroy(struct Ring *p) { drop_owned(p->next); drop_owned(p); } void subject(struct Ring *p) { destroy(factory()); }",
        ] {
            let result = run(&format!("{prefix}{tail}"), &contracts);
            assert_ne!(
                summary(&result).functions["subject"].status,
                AnalysisStatus::Safe,
                "{tail}"
            );
            assert!(
                summary(&result)
                    .functions
                    .values()
                    .all(|row| !has_call(&row.cfg)),
                "{tail}"
            );
        }
    }

    fn source_checked(source: &str, contracts: &ResourceContracts) -> bool {
        let _scope = crate::typed_access_path::enter_source(source, CCompilerDialect::Gnu17, None);
        let tokens = tokenize(source);
        candidate(&tokens, "subject", contracts)
            && checked_leaf(&tokens, "reset", contracts).is_some()
    }

    #[test]
    fn source_and_leaf_vectors_match_lean() {
        if !clang() {
            return;
        }
        let good = format!("{LEAF}void subject(struct Ring *p) {{ reset(p); }}");
        let contracts = ResourceContracts::default();
        let mut vectors = vec![source_checked(&good, &contracts)];
        for tail in [
            "int subject(struct Ring *p) { reset(p); return 0; }",
            "void subject(struct Ring *p) { int local = 1; reset(p); }",
        ] {
            vectors.push(source_checked(&format!("{LEAF}{tail}"), &contracts));
        }
        vectors.push(source_checked("struct Ring { void *next; }; void reset(struct Ring *p) { p->next = p; } void subject(struct Ring *p) { reset(p); }", &contracts));
        vectors.push(source_checked(
            &format!("{LEAF}void subject(struct Ring *p) {{ reset(p->next); }}"),
            &contracts,
        ));
        vectors.push(source_checked(
            &format!("{good} void (*escaped)(struct Ring *) = subject;"),
            &contracts,
        ));
        {
            let _scope =
                crate::typed_access_path::enter_source(&good, CCompilerDialect::Gnu17, None);
            let tokens = tokenize(&good);
            let profile = profile(&tokens, "reset", &contracts, true).expect("positive leaf");
            let cfg = build_function_cfg(&tokens, "reset", &contracts).unwrap();
            assert!(stores_match(&cfg, &profile));
            let mut invalid = cfg.clone();
            let node = invalid
                .nodes
                .iter_mut()
                .find(|n| matches!(n.instruction, LegacyInstruction::PointerStore { .. }))
                .unwrap();
            node.instruction = LegacyInstruction::Unknown {
                description: "failed leaf CFG control".into(),
                writes: RefusedWrites::Unbounded,
            };
            let entry = entry(&invalid, &profile, false).unwrap();
            let (summary, exits) = analyze_cfg_with_entry(invalid, &entry);
            vectors.push(empty_exits(&summary, &exits));
            let mut invalid = cfg;
            if let LegacyInstruction::PointerStore { store, .. } = &mut invalid
                .nodes
                .iter_mut()
                .find(|n| matches!(n.instruction, LegacyInstruction::PointerStore { .. }))
                .unwrap()
                .instruction
            {
                store.place.push_str("_other");
            }
            vectors.push(stores_match(&invalid, &profile));
        }
        let mut declared = contracts;
        declared.insert_contract(
            "reset",
            ResourceFunctionContract::new(
                vec![ResourceParameterMode::Borrowed],
                ResourceReturnMode::None,
            ),
        );
        vectors.push(source_checked(&good, &declared));
        assert_eq!(
            vectors,
            vec![true, false, false, false, false, false, false, false, false]
        );
        for declaration in [
            "struct Ring { struct Ring *volatile next; };",
            "struct Ring { struct Ring *next; };",
        ] {
            let formal = if declaration.contains("volatile") {
                "struct Ring *p"
            } else {
                "struct Ring *volatile p"
            };
            let source = format!(
                "{declaration} void reset({formal}) {{ p->next = p; }} void subject(struct Ring *p) {{ reset(p); }}"
            );
            assert!(!source_checked(&source, &ResourceContracts::default()));
        }
        let source = "union Ring { union Ring *next; }; void reset(union Ring *p) { p->next=p; } void subject(union Ring *p) { reset(p); }";
        assert!(!source_checked(source, &ResourceContracts::default()));
    }

    #[test]
    fn missing_reference_facts_refuse() {
        let good = serde_json::json!({"kind":"DeclRefExpr", "type":{"qualType":"struct Ring *"},
            "referencedDecl":{"kind":"ParmVarDecl","name":"p","id":"formal"}});
        assert!(original(&good, "p", "formal", "struct Ring *"));
        for field in ["kind", "name", "id"] {
            let mut bad = good.clone();
            bad["referencedDecl"].as_object_mut().unwrap().remove(field);
            assert!(!original(&bad, "p", "formal", "struct Ring *"));
        }
        let mut bad = good;
        bad.as_object_mut().unwrap().remove("type");
        assert!(!original(&bad, "p", "formal", "struct Ring *"));
    }

    #[test]
    fn unreachable_calls_retain_the_older_lowering_and_inference() {
        if !clang() {
            return;
        }
        let source = format!("{LEAF}void subject(struct Ring *p) {{ return; reset(p); }}");
        let _scope = crate::typed_access_path::enter_source(&source, CCompilerDialect::Gnu17, None);
        let tokens = tokenize(&source);
        let contracts = ResourceContracts::default();
        let cfg = build_function_cfg(&tokens, "subject", &contracts).unwrap();
        assert!(!has_call(&cfg));
        assert!(cfg.nodes.iter().any(|n| matches!(&n.instruction, LegacyInstruction::Unknown { description, .. } if description.contains("reset"))));
        let derivation = crate::legacy_summary::derive_custody_summaries(
            &tokens,
            &["reset".into(), "subject".into()],
            &contracts,
        );
        assert!(derivation.records().iter().any(|r| r.function == "subject"
            && r.contract
                == ResourceFunctionContract::new(
                    vec![ResourceParameterMode::Borrowed],
                    ResourceReturnMode::None
                )));
        assert!(!derivation.self_init_frame("subject"));
    }

    #[test]
    fn checked_rows_have_no_export_and_a_forged_neutral_import_fails_replay() {
        if !clang() {
            return;
        }
        let source = format!("{LEAF}void subject(struct Ring *p) {{ reset(p); }}");
        let unit = prepare_translation_unit(&C11TranslationUnit {
            source: &source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let wanted = BTreeSet::from(["subject".to_owned()]);
        let digest = "a".repeat(64);
        assert!(
            export_cross_unit_homes(&unit, &ResourceContracts::default(), &wanted, &digest)
                .unwrap()
                .is_empty()
        );
        let neutral_source =
            "struct Ring { struct Ring *next; }; void subject(struct Ring *p) { return; }";
        let neutral = prepare_translation_unit(&C11TranslationUnit {
            source: neutral_source,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            ..Default::default()
        });
        let mut exported =
            export_cross_unit_homes(&neutral, &ResourceContracts::default(), &wanted, &digest)
                .unwrap()
                .remove(0);
        let request = LegacyCertificateRequest {
            include_paths: unit.include_paths().to_vec(),
            definitions: unit.definitions().to_vec(),
            preprocessor_options: unit.preprocessor_options().to_vec(),
            input: unit.input().clone(),
        };
        let scope = build_certificate_scope_from_evidence(
            unit.dialect(),
            LegacyFunctionScope::MainSourceDefinitions,
            unit.preprocessed_source().unwrap(),
            Some(unit.raw_source()),
            unit.line_marked_source(),
            request.clone(),
        )
        .unwrap();
        exported.home.preprocessed_source = unit.preprocessed_source().unwrap().to_owned();
        exported.home.raw_source = unit.raw_source().to_owned();
        exported.home.line_marked_source = unit.line_marked_source().unwrap().to_owned();
        exported.home.request = request;
        exported.home.input_identity = scope.input_identity;
        let mut contracts = ResourceContracts::default();
        contracts.set_inferred_imports(BTreeMap::from([("subject".to_owned(), exported.contract)]));
        contracts.set_cross_unit_import_identities(BTreeMap::from([(
            "subject".to_owned(),
            exported.home.import_identity(),
        )]));
        contracts.set_link_resolution_sha256(Some(digest));
        let error = cross_unit::check_imports(
            "struct Ring; void subject(struct Ring *p); void caller(void) {}",
            &contracts,
            &[exported.home],
        )
        .unwrap_err();
        assert!(error.contains("no validated W4 summary"), "{error}");
    }

    #[test]
    fn empty_frame_vectors_match_lean_and_never_discharge_resources() {
        let initial = AbstractState::default();
        let mut cases = vec![
            (initial.clone(), true, true),
            (initial.clone(), false, false),
        ];
        for status in [LIVE, RELEASED, ESCAPED] {
            let mut state = initial.clone();
            state.resources.insert(7, status);
            cases.push((state, true, false));
        }
        let mut state = initial.clone();
        state.field_links.insert(7, BTreeMap::new());
        cases.push((state, true, false));
        let mut state = initial.clone();
        state.unmodeled = true;
        cases.push((state, true, false));
        let mut state = initial.clone();
        state.widened = true;
        cases.push((state, true, false));
        let mut state = initial.clone();
        state
            .pointers
            .insert("p->next".into(), AbstractPointer::default());
        cases.push((state, true, false));
        let mut state = initial.clone();
        state.pointers.insert(
            "p".into(),
            AbstractPointer {
                targets: BTreeSet::from([7]),
                ..Default::default()
            },
        );
        cases.push((state.clone(), true, true));
        state.pointers.get_mut("p").unwrap().unknown = true;
        cases.push((state, true, false));
        let mut state = initial.clone();
        state.pointers.insert(
            "p".into(),
            AbstractPointer {
                unknown: false,
                may_be_null: true,
                ..Default::default()
            },
        );
        cases.push((state, true, false));
        let mut state = initial.clone();
        state.pointers.insert(
            "p".into(),
            AbstractPointer {
                targets: BTreeSet::from([7, 8]),
                ..Default::default()
            },
        );
        cases.push((state, true, false));
        let mut state = initial;
        state.pointers.insert(
            "p".into(),
            AbstractPointer {
                targets: BTreeSet::from([7]),
                derivation: Derivation::Mixed,
                ..Default::default()
            },
        );
        cases.push((state, true, false));
        assert_eq!(
            cases
                .iter()
                .map(|(s, e, _)| admits(s, "p", *e))
                .collect::<Vec<_>>(),
            vec![
                true, false, false, false, false, false, false, false, false, true, false, false,
                false, false
            ]
        );
        let node = LegacyCfgNode {
            instruction: LegacyInstruction::SelfInitCall {
                function: "reset".into(),
                base: "p".into(),
                fallback: Box::new(LegacyInstruction::Unknown {
                    description: "call to uncontracted function `reset`".into(),
                    writes: RefusedWrites::Unbounded,
                }),
            },
            line: 1,
            successors: vec![],
            is_exit: true,
            lookup: None,
            lookup_read: None,
            loop_exit_edge: None,
        };
        for (state, enabled, expected) in cases {
            assert_eq!(admits(&state, "p", enabled), expected);
            let mut diagnostics = BTreeSet::new();
            let context = transfer::TransferContext {
                allow_self_init_frame: enabled,
                ..Default::default()
            };
            let output = transfer::transfer_dispatch(
                "subject",
                &node,
                state.clone(),
                &BTreeMap::new(),
                &context,
                &transfer::NodeAliasFacts::default(),
                &mut diagnostics,
            );
            assert_eq!(output.resources, state.resources);
            if expected {
                assert_eq!(output, state);
                assert!(diagnostics.is_empty());
            } else {
                assert!(!diagnostics.is_empty());
            }
        }
    }
}
