//! Rule 36: a source-confirmed return of the original borrowed pointer formal.
//! This licenses only a derived-CFG exit. Full-body/all-exit validation remains
//! authoritative; extraction and Rust–Lean correspondence are premise 28.
//!
//! Card **RULE36-PILOT** (plan `docs/plans/pluggable-cards/README.md` seam
//! B): the rule's body is the package `rule-36-exact-formal-return`; this
//! file is its host. It reads the gateway's tokens, declarations, typed AST
//! and abstract state for the package, performs what it admits, and asks it
//! only while registry rule `36` is on (`QCORE_GATEWAY_RULES_OFF`). Off, the
//! lexical candidate is false, the return route is not asked (the return
//! keeps its pre-rule-36 lowering), and the classifier refuses, so off only
//! removes admissions.
use super::*;
use crate::typed_access_path::{StatementIndex, is_pointer_type, qual};
use crate::typed_c_ast::{children, kind};
use qcore_gateway_rule_api::Host;
use qcore_gateway_rule_api::traits::{self as package, RulePackage};
use rule_36_exact_formal_return::{
    self as rule, Binding, ExactFormalHost, ExactFormalReturnRoute, FormalReturnHost,
    FunctionReading, SourceHost,
};
use serde_json::Value;
use std::sync::Arc;

pub(super) use rule::Flow;

/// Whether rule 36's package answers in this run.
fn on() -> bool {
    crate::gateway_rules::enabled(rule::ID)
}

/// The exit classifier over the gateway's abstract state; [`Flow::Refuse`]
/// while the rule is off.
pub(super) fn classify(
    state: &AbstractState,
    source: &str,
    parameter: usize,
    bases: &BTreeMap<usize, usize>,
) -> Flow {
    if !on() {
        return Flow::Refuse;
    }
    let binding = state.pointers.get(source).map(|pointer| Binding {
        unknown: pointer.unknown,
        identity: pointer.derivation.is_identity(),
        may_be_null: pointer.may_be_null,
        targets: &pointer.targets,
    });
    rule::classify(
        binding,
        !state.unresolved_effect_sites.is_empty(),
        parameter,
        |site| bases.get(&site).copied(),
        |site| state.resources.contains_key(&site),
    )
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

/// The lexical pre-filter of the derived-result CFG and of the neutral
/// summary's refinement; `false` while the rule is off.
pub(crate) fn candidate(tokens: &[Token], function: &str) -> bool {
    if !on() {
        return false;
    }
    let Some(parameters) = function_parameters(tokens, function) else {
        return false;
    };
    let Some((start, _)) = find_function_body(tokens, function) else {
        return false;
    };
    let Some(end) = matching(tokens, start, "{", "}") else {
        return false;
    };
    rule::source::candidate::<FormalSource<'_>>(&parameters, &tokens[start + 1..end], || {
        let texts = tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();
        declared_abi::exact_pointer_formals(&texts, function)
    })
}

/// The gateway's typed index as rule 36's source host.
pub(crate) struct FormalSource<'a> {
    index: &'a StatementIndex,
    unbound: &'a dyn Fn(&str) -> bool,
}

impl Host for FormalSource<'_> {
    type Token = Token;
    type Decision = ();
    type Statement = ();
    type Condition = ();
    type Node = Value;

    fn token_text(token: &Token) -> &str {
        &token.text
    }

    fn token_line(token: &Token) -> usize {
        token.line
    }

    fn refusal((): &()) -> Option<&str> {
        None
    }
}

impl SourceHost for FormalSource<'_> {
    fn node_kind(node: &Value) -> &str {
        kind(node)
    }

    fn node_attribute<'n>(node: &'n Value, key: &str) -> Option<&'n str> {
        node[key].as_str()
    }

    fn node_field<'n>(node: &'n Value, key: &str) -> Option<&'n Value> {
        node.get(key)
    }

    fn node_type(node: &Value) -> Option<&str> {
        qual(node)
    }

    fn node_children(node: &Value) -> &[Value] {
        children(node)
    }

    fn pointer_type(text: &str) -> bool {
        is_pointer_type(text)
    }
}

impl FormalReturnHost for FormalSource<'_> {
    fn formal_unbound(&self, name: &str) -> bool {
        (self.unbound)(name)
    }

    fn node_tokens(&self, node: &Value) -> Option<Arc<Vec<String>>> {
        self.index.node_tokens_once(node)
    }

    fn node_line(&self, node: &Value) -> Option<usize> {
        self.index.node_line(node)
    }
}

/// The parser as the host of rule 36's return route.
impl ExactFormalHost for Parser<'_> {
    fn derived_result_cfg(&self) -> bool {
        self.admit_derived_returns
    }

    fn confirmed_formal_return(&self, name: &str, line: usize) -> Option<usize> {
        self.exact_formal_returns
            .get_or_init(|| self.read_exact_formal_returns())
            .get(&(line, name.to_owned()))
            .copied()
    }

    fn lower_exact_formal_return(
        &mut self,
        query: &package::ReturnQuery<'_, Self>,
        parameter: usize,
    ) -> Statement {
        Statement::Return {
            line: query.line,
            writes: self.expression_writes(query.expression),
            expression_supported: true,
            returned_place: None,
            returned_evaluation: Some(Box::new(LegacyInstruction::ExactFormalReturn {
                source: query.expression[0].text.clone(),
                parameter,
            })),
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        }
    }
}

impl Parser<'_> {
    /// Rule 36's return route, asked first in `return_statement`: the
    /// package's lowering of a confirmed `return P;` of the derived-result
    /// CFG, or `None` (the rule is off, or the package refuses).
    pub(super) fn exact_formal_return_route(
        &mut self,
        expression: &[Token],
        line: usize,
    ) -> Option<Statement> {
        // Nothing is decided before the first return route: the host's
        // refusal (not supported, no evaluation), which the route never reads.
        let refused = Statement::Return {
            line,
            expression_supported: false,
            writes: RefusedWrites::Unbounded,
            returned_place: None,
            returned_evaluation: None,
            returned_evaluation_escapes: false,
            returns_null_constant: false,
        };
        let query = package::ReturnQuery {
            expression,
            line,
            // `return_statement` is also the lowering of operands the
            // elaborators hand back, which carry no stream place; the rule
            // reads none (`ReturnQuery::origin` is not optional, a friction
            // recorded in docs/reports/rule36-pilot/).
            origin: usize::MAX,
            decided: &refused,
        };
        // Card ARM-OBLIGATIONS: an admission's obligations are carried
        // into the unit's rows, or it is refused.
        crate::arm_obligations::consult(
            &[ExactFormalReturnRoute],
            |route| route.id(),
            |route| crate::gateway_rules::enabled(route.id()),
            |_, route| package::ReturnRoute::ask(route, self, &query),
        )
        .map(|decided| decided.lowering)
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
        let body =
            find_function_body(self.tokens, &self.typed_path_function).and_then(|(start, _)| {
                matching(self.tokens, start, "{", "}").map(|end| &self.tokens[start + 1..end])
            });
        let unbound = |name: &str| self.position_answered(name) && self.position_never_bound(name);
        rule::source::exact_formal_returns(
            &FormalSource {
                index,
                unbound: &unbound,
            },
            &FunctionReading {
                formals: &parameters,
                pointer_formals: &indices,
                function: &index.function,
                body,
            },
        )
    }
}

/// Rule 13q's stack control exercises the actual new walks even when its
/// synthetic deep function returns a scalar and would not license this rule.
#[cfg(test)]
pub(crate) fn exercise_source_readers() -> Option<bool> {
    use rule::source::{inspect_body, inspect_returns, object_pointer, references, value_name};
    let index = StatementIndex::read("chain")?;
    let host = FormalSource {
        index: &index,
        unbound: &|_| true,
    };
    let body = children(&index.function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")?;
    let mut names = BTreeMap::new();
    let mut eligible = BTreeMap::new();
    for (parameter, node) in children(&index.function)
        .iter()
        .filter(|node| kind(node) == "ParmVarDecl")
        .enumerate()
    {
        value_name::<FormalSource<'_>>(node, &mut names)?;
        if object_pointer::<FormalSource<'_>>(node) {
            eligible.insert(
                node["name"].as_str()?.to_owned(),
                (parameter, node["id"].as_str()?.to_owned()),
            );
        }
    }
    inspect_body::<FormalSource<'_>>(body, &mut names, &mut BTreeSet::new())?;
    references::<FormalSource<'_>>(body, &mut BTreeSet::new())?;
    inspect_returns(&host, body, &eligible, &mut BTreeMap::new())?;
    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway_rules::test_override::with_rules_off;
    use rule::source::{inspect_body, original_formal};
    use serde_json::json;

    // Shared with QCore.ExactFormalReturn.flowVectors, in this order, through
    // the host's reading of the abstract state; the package holds the same
    // vectors as data (`rule::flow::FLOW_VECTORS`).
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
        assert_eq!(
            expected,
            rule::flow::FLOW_VECTORS
                .iter()
                .map(|vector| vector.4)
                .collect::<Vec<_>>()
        );
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

    /// Card RULE36-PILOT: off, the classifier refuses every vector, so the
    /// transfer only adds the refusal's UnknownAlias and exposure; custody,
    /// bindings and containment are unchanged.
    #[test]
    fn off_the_classifier_refuses_and_the_transfer_keeps_custody() {
        let bases = BTreeMap::from([(7, 1)]);
        let mut state = AbstractState::default();
        state.pointers.insert(
            "p".into(),
            AbstractPointer {
                targets: BTreeSet::from([7]),
                may_be_null: true,
                ..Default::default()
            },
        );
        assert_eq!(classify(&state, "p", 1, &bases), Flow::Exact(7));
        with_rules_off("36", || {
            assert_eq!(classify(&state, "p", 1, &bases), Flow::Refuse);
            let before = state.clone();
            let mut refused = state.clone();
            let mut diagnostics = BTreeSet::new();
            apply(&mut refused, "p", 1, &bases, &mut diagnostics, "f", 3);
            assert_eq!(refused.resources, before.resources);
            assert_eq!(refused.pointers, before.pointers);
            assert!(refused.unmodeled && !diagnostics.is_empty());
            assert!(!candidate(
                &tokenize("char *identity(char *p) { return p; }"),
                "identity"
            ));
        });
        with_rules_off("35", || {
            assert_eq!(classify(&state, "p", 1, &bases), Flow::Exact(7));
        });
    }

    fn formal() -> Value {
        json!({"kind":"DeclRefExpr","type":{"qualType":"char *"},
            "referencedDecl":{"kind":"ParmVarDecl","name":"p","id":"formal-1"}})
    }

    fn original(node: &Value) -> bool {
        original_formal::<FormalSource<'_>>(node, "p", "formal-1")
    }

    #[test]
    fn actual_formal_reference_and_positive_type_are_required() {
        let good = formal();
        assert!(original(&good));
        for key in ["kind", "name", "id"] {
            let mut bad = good.clone();
            bad["referencedDecl"].as_object_mut().unwrap().remove(key);
            assert!(!original(&bad));
        }
        let mut bad = good.clone();
        bad["referencedDecl"]["id"] = json!("another");
        assert!(!original(&bad));
        let mut bad = good.clone();
        bad["referencedDecl"]["kind"] = json!("EnumConstantDecl");
        assert!(!original(&bad));
        for ty in ["int", "char *volatile", "_Atomic(char *)", "char (*)(void)"] {
            let mut bad = good.clone();
            bad["type"]["qualType"] = json!(ty);
            assert!(!original(&bad));
        }
        let mut bad = good.clone();
        bad.as_object_mut().unwrap().remove("type");
        assert!(!original(&bad));
        let mut top_const = good.clone();
        top_const["type"]["qualType"] = json!("char *const");
        assert!(original(&top_const));
        for cast in [
            "FunctionToPointerDecay",
            "NullToPointer",
            "IntegralToPointer",
        ] {
            let bad = json!({"kind":"ImplicitCastExpr","type":{"qualType":"char *"},"castKind":cast,"inner":[good]});
            assert!(!original(&bad));
        }
    }

    #[test]
    fn unrelated_value_shadowing_and_formal_writes_cannot_license_an_exit() {
        let mut names = BTreeMap::from([("q".into(), "q-formal".into())]);
        let shadow = json!({"kind":"EnumConstantDecl","name":"q","id":"q-enum"});
        assert!(
            inspect_body::<FormalSource<'_>>(&shadow, &mut names, &mut BTreeSet::new()).is_none()
        );
        let write = json!({"kind":"BinaryOperator","opcode":"=","inner":[formal(),formal()]});
        let mut written = BTreeSet::new();
        inspect_body::<FormalSource<'_>>(&write, &mut BTreeMap::new(), &mut written).unwrap();
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
