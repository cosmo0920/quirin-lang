//! Rule 36: a source-confirmed return of the original borrowed pointer formal.
//! This licenses only a derived-CFG exit. Full-body/all-exit validation remains
//! authoritative; extraction and Rust–Lean correspondence are premise 28.
use super::*;
use crate::typed_access_path::{StatementIndex, is_pointer_type, qual};
use crate::typed_c_ast::{children, kind};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flow {
    Null,
    Exact(usize),
    Refuse,
}

pub(super) fn classify(
    state: &AbstractState,
    source: &str,
    parameter: usize,
    bases: &BTreeMap<usize, usize>,
) -> Flow {
    let Some(pointer) = state.pointers.get(source) else {
        return Flow::Refuse;
    };
    if pointer.unknown
        || !pointer.derivation.is_identity()
        || !state.unresolved_effect_sites.is_empty()
    {
        return Flow::Refuse;
    }
    if pointer.targets.is_empty() {
        return if pointer.may_be_null {
            Flow::Null
        } else {
            Flow::Refuse
        };
    }
    if pointer.targets.len() == 1 {
        let site = *pointer.targets.first().expect("one target");
        if bases.get(&site) == Some(&parameter) && !state.resources.contains_key(&site) {
            return Flow::Exact(site);
        }
    }
    Flow::Refuse
}

pub(super) fn apply(
    state: &mut AbstractState,
    source: &str,
    parameter: usize,
    bases: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    function: &str,
    line: usize,
) {
    if classify(state, source, parameter, bases) == Flow::Refuse {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        let pointer = state
            .pointers
            .get(source)
            .cloned()
            .unwrap_or(AbstractPointer {
                unknown: true,
                may_be_null: true,
                ..Default::default()
            });
        transfer::mark_unresolved_effect(state, &pointer);
        state.unmodeled = true;
    }
}

// Only top-level const is removed. Volatile/atomic and function-pointer types
// still fail the existing positive pointer test; pointee const is preserved.
fn object_pointer(node: &Value) -> bool {
    qual(node).is_some_and(|text| {
        let text = text.trim_end();
        let text = text.strip_suffix("const").map_or(text, str::trim_end);
        is_pointer_type(text)
    })
}

pub(super) fn candidate(tokens: &[Token], function: &str) -> bool {
    let Some(parameters) = function_parameters(tokens, function) else {
        return false;
    };
    let Some((start, _)) = find_function_body(tokens, function) else {
        return false;
    };
    let Some(end) = matching(tokens, start, "{", "}") else {
        return false;
    };
    let returned = tokens[start + 1..end]
        .windows(3)
        .filter_map(|run| {
            (run[0].text == "return" && run[2].text == ";")
                .then(|| {
                    parameters
                        .iter()
                        .position(|name| name.as_deref() == Some(run[1].text.as_str()))
                })
                .flatten()
        })
        .collect::<BTreeSet<_>>();
    if returned.is_empty() {
        return false;
    }
    let texts = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>();
    !returned.is_disjoint(&declared_abi::exact_pointer_formals(&texts, function))
}

impl Parser<'_> {
    pub(super) fn exact_formal_return(&self, source: &str, line: usize) -> Option<usize> {
        self.exact_formal_returns
            .get_or_init(|| self.read_exact_formal_returns())
            .get(&(line, source.to_owned()))
            .copied()
    }

    fn read_exact_formal_returns(&self) -> BTreeMap<(usize, String), usize> {
        self.exact_formal_return_facts().unwrap_or_default()
    }

    fn exact_formal_return_facts(&self) -> Option<BTreeMap<(usize, String), usize>> {
        let parameters = function_parameters(self.tokens, &self.typed_path_function)?;
        let texts = self
            .tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        let indices = declared_abi::exact_pointer_formals(&texts, &self.typed_path_function);
        if indices.is_empty() {
            return None;
        }
        let index = self.typed_positions_index()?;
        let ast_parameters = children(&index.function)
            .iter()
            .filter(|node| kind(node) == "ParmVarDecl")
            .collect::<Vec<_>>();
        if ast_parameters.len() != parameters.len() {
            return None;
        }
        let body = children(&index.function)
            .iter()
            .find(|node| kind(node) == "CompoundStmt")?;
        let mut names = BTreeMap::new();
        for node in &ast_parameters {
            value_name(node, &mut names)?;
        }
        let mut eligible = BTreeMap::new();
        for parameter in indices {
            let name = parameters.get(parameter)?.as_ref()?;
            let node = *ast_parameters.get(parameter)?;
            if node["name"].as_str() == Some(name)
                && object_pointer(node)
                && self.position_answered(name)
                && self.position_never_bound(name)
            {
                eligible.insert(name.clone(), (parameter, node["id"].as_str()?.to_owned()));
            }
        }
        if eligible.is_empty() {
            return None;
        }
        let mut writes = BTreeSet::new();
        inspect_body(body, &mut names, &mut writes)?;
        eligible.retain(|_, (_, id)| !writes.contains(id));
        let (start, _) = find_function_body(self.tokens, &self.typed_path_function)?;
        let end = matching(self.tokens, start, "{", "}")?;
        let mut lexical = BTreeMap::<(usize, String), usize>::new();
        for run in self.tokens[start + 1..end].windows(3) {
            if run[0].text == "return" && run[2].text == ";" && eligible.contains_key(&run[1].text)
            {
                *lexical
                    .entry((run[0].line, run[1].text.clone()))
                    .or_default() += 1;
            }
        }
        let mut ast = BTreeMap::new();
        inspect_returns(body, index, &eligible, &mut ast)?;
        let mut answer = BTreeMap::new();
        for (key, count) in lexical {
            if ast.get(&key) == Some(&(count, true)) {
                answer.insert(key.clone(), eligible.get(&key.1)?.0);
            }
        }
        Some(answer)
    }
}

fn value_name(node: &Value, names: &mut BTreeMap<String, String>) -> Option<()> {
    let name = node["name"].as_str()?;
    let id = node["id"].as_str()?;
    if name.is_empty() || id.is_empty() {
        return None;
    }
    if names.get(name).is_some_and(|old| old != id) {
        return None;
    }
    names.insert(name.to_owned(), id.to_owned());
    Some(())
}

fn references(node: &Value, written: &mut BTreeSet<String>) -> Option<()> {
    if kind(node) == "DeclRefExpr" {
        written.insert(node["referencedDecl"]["id"].as_str()?.to_owned());
    }
    for child in children(node) {
        references(child, written)?;
    }
    Some(())
}

fn inspect_body(
    node: &Value,
    names: &mut BTreeMap<String, String>,
    written: &mut BTreeSet<String>,
) -> Option<()> {
    match kind(node) {
        "VarDecl" | "EnumConstantDecl" | "FunctionDecl" => value_name(node, names)?,
        "DeclRefExpr" => {
            value_name(&node["referencedDecl"], names)?;
        }
        "BinaryOperator" => {
            if node["opcode"].as_str()? == "=" {
                references(children(node).first()?, written)?;
            }
        }
        "CompoundAssignOperator" => references(children(node).first()?, written)?,
        "UnaryOperator" => {
            if matches!(node["opcode"].as_str()?, "&" | "++" | "--") {
                references(children(node).first()?, written)?;
            }
        }
        _ => {}
    }
    for child in children(node) {
        inspect_body(child, names, written)?;
    }
    Some(())
}

fn original_formal(mut node: &Value, name: &str, id: &str) -> bool {
    loop {
        if !object_pointer(node) {
            return false;
        }
        if kind(node) != "ImplicitCastExpr" {
            break;
        }
        if !matches!(
            node["castKind"].as_str(),
            Some("LValueToRValue" | "NoOp" | "BitCast")
        ) {
            return false;
        }
        let [child] = children(node) else {
            return false;
        };
        node = child;
    }
    kind(node) == "DeclRefExpr"
        && node["referencedDecl"]["kind"].as_str() == Some("ParmVarDecl")
        && node["referencedDecl"]["name"].as_str() == Some(name)
        && node["referencedDecl"]["id"].as_str() == Some(id)
}

fn inspect_returns(
    node: &Value,
    index: &StatementIndex,
    eligible: &BTreeMap<String, (usize, String)>,
    counts: &mut BTreeMap<(usize, String), (usize, bool)>,
) -> Option<()> {
    if kind(node) == "ReturnStmt" {
        let tokens = index.node_tokens_once(node)?;
        let line = index.node_line(node)?;
        if let [keyword, name] = tokens.as_slice()
            && keyword == "return"
            && let Some((_, id)) = eligible.get(name)
        {
            let valid = match children(node) {
                [value] => original_formal(value, name, id),
                _ => false,
            };
            let count = counts.entry((line, name.clone())).or_insert((0, true));
            count.0 += 1;
            count.1 &= valid;
        }
    }
    for child in children(node) {
        inspect_returns(child, index, eligible, counts)?;
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Shared with QCore.ExactFormalReturn.flowVectors, in this order.
    #[test]
    fn exact_formal_flow_vectors_preserve_custody_and_refuse_missing_facts() {
        let bases = BTreeMap::from([(7, 1)]);
        let mut original = AbstractState::default();
        original.pointers.insert(
            "p".into(),
            AbstractPointer {
                targets: BTreeSet::from([7]),
                may_be_null: true,
                ..Default::default()
            },
        );
        original.resources.insert(20, LIVE);
        let mut cases = vec![(original.clone(), 1, Flow::Exact(7))];
        let mut null = original.clone();
        null.pointers.get_mut("p").unwrap().targets.clear();
        cases.push((null.clone(), 1, Flow::Null));
        null.pointers.get_mut("p").unwrap().may_be_null = false;
        cases.push((null, 1, Flow::Refuse));
        for mode in 0..10 {
            let mut state = original.clone();
            match mode {
                0 => {
                    state.pointers.remove("p");
                }
                1 => state.pointers.get_mut("p").unwrap().unknown = true,
                2 => {
                    state.pointers.get_mut("p").unwrap().targets.insert(8);
                }
                3 => {
                    state.pointers.get_mut("p").unwrap().targets = BTreeSet::from([8]);
                }
                4 => state.pointers.get_mut("p").unwrap().derivation = Derivation::Mixed,
                5 => {
                    state.pointers.get_mut("p").unwrap().derivation = Derivation::Derived {
                        extent: DerivedExtent::Exact,
                        const_dropped: false,
                        nullable: false,
                        stepped: Stepped::No,
                    }
                }
                6 => {
                    state.resources.insert(7, LIVE);
                }
                7 => {
                    state.resources.insert(7, RELEASED);
                }
                8 => {
                    state.resources.insert(7, ESCAPED);
                }
                9 => {
                    state.unresolved_effect_sites.insert(20);
                }
                _ => unreachable!(),
            }
            cases.push((state, 1, Flow::Refuse));
        }
        cases.push((original, 0, Flow::Refuse));
        let expected = cases
            .iter()
            .map(|(_, _, result)| *result)
            .collect::<Vec<_>>();
        assert_eq!(expected.len(), 14);
        for (mut state, parameter, expected) in cases {
            assert_eq!(classify(&state, "p", parameter, &bases), expected);
            let before = state.clone();
            let mut diagnostics = BTreeSet::new();
            apply(&mut state, "p", parameter, &bases, &mut diagnostics, "f", 3);
            assert_eq!(state.resources, before.resources);
            assert_eq!(state.pointers, before.pointers);
            assert_eq!(state.contained, before.contained);
            assert_eq!(state.field_links, before.field_links);
            let mut restored = state.clone();
            restored.unresolved_effect_sites = before.unresolved_effect_sites.clone();
            restored.unmodeled = before.unmodeled;
            assert_eq!(restored, before);
            if expected != Flow::Refuse {
                assert_eq!(state, before);
                assert!(diagnostics.is_empty());
            } else {
                assert!(!diagnostics.is_empty());
                assert!(state.unmodeled);
            }
        }
    }

    fn formal() -> Value {
        json!({"kind":"DeclRefExpr","type":{"qualType":"char *"},
            "referencedDecl":{"kind":"ParmVarDecl","name":"p","id":"formal-1"}})
    }

    #[test]
    fn actual_formal_reference_and_positive_type_are_required() {
        let good = formal();
        assert!(original_formal(&good, "p", "formal-1"));
        for key in ["kind", "name", "id"] {
            let mut bad = good.clone();
            bad["referencedDecl"].as_object_mut().unwrap().remove(key);
            assert!(!original_formal(&bad, "p", "formal-1"));
        }
        let mut bad = good.clone();
        bad["referencedDecl"]["id"] = json!("another");
        assert!(!original_formal(&bad, "p", "formal-1"));
        let mut bad = good.clone();
        bad["referencedDecl"]["kind"] = json!("EnumConstantDecl");
        assert!(!original_formal(&bad, "p", "formal-1"));
        for ty in ["int", "char *volatile", "_Atomic(char *)", "char (*)(void)"] {
            let mut bad = good.clone();
            bad["type"]["qualType"] = json!(ty);
            assert!(!original_formal(&bad, "p", "formal-1"));
        }
        let mut bad = good.clone();
        bad.as_object_mut().unwrap().remove("type");
        assert!(!original_formal(&bad, "p", "formal-1"));
        let mut top_const = good.clone();
        top_const["type"]["qualType"] = json!("char *const");
        assert!(original_formal(&top_const, "p", "formal-1"));
        for cast in [
            "FunctionToPointerDecay",
            "NullToPointer",
            "IntegralToPointer",
        ] {
            let bad = json!({"kind":"ImplicitCastExpr","type":{"qualType":"char *"},"castKind":cast,"inner":[good]});
            assert!(!original_formal(&bad, "p", "formal-1"));
        }
    }

    #[test]
    fn unrelated_value_shadowing_and_formal_writes_cannot_license_an_exit() {
        let mut names = BTreeMap::from([("q".into(), "q-formal".into())]);
        let shadow = json!({"kind":"EnumConstantDecl","name":"q","id":"q-enum"});
        assert!(inspect_body(&shadow, &mut names, &mut BTreeSet::new()).is_none());
        let write = json!({"kind":"BinaryOperator","opcode":"=","inner":[formal(),formal()]});
        let mut written = BTreeSet::new();
        inspect_body(&write, &mut BTreeMap::new(), &mut written).unwrap();
        assert!(written.contains("formal-1"));
    }
    #[test]
    fn new_candidate_requires_positive_matching_depth_one_pointer_abi() {
        for source in [
            "int identity(int p) { return p; }",
            "char **identity(char **p) { return p; }",
            "char *identity(const char *p) { return p; }",
            "unknown identity(unknown p) { return p; }",
            "void (*identity(void (*p)(void)))(void) { return p; }",
        ] {
            assert!(!candidate(&tokenize(source), "identity"), "{source}");
        }
        assert!(candidate(
            &tokenize("char *identity(char *p) { return p; }"),
            "identity"
        ));
        assert!(candidate(
            &tokenize("char *identity(int x,char *p) { return p; }"),
            "identity"
        ));
    }
}
