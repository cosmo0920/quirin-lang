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

//! Card **SP9-PLUGIN-GUARDED-APPLICATION-FORMULAS** (contract specification
//! C04.18.6, C04.18.9's SP9 block; user decisions of 2026-11-23: D1 A′
//! prefixed terms with the environment implicit, D2 b, D3 I2): the 14
//! guarded symbols of the frozen plugin sidecar state typed-rendering
//! formulas over the SP9 block. Each prerequisite is its typed atom's formula
//! bound to the SP9 terms, and each formula is the Lean definition
//! `QCore.PluginGuardedSymbols` proves equal to its existing definition.

#[path = "../../../tests/support/legacy_prose_premise.rs"]
mod legacy_prose_premise;

use qcore_frontend::premise_formula::{
    PROSE_GUARDED, RENDERED_TERMS, RENDERED_VALUE_SORTS, guarded_proposition,
};
use qcore_frontend::quant_proposition::lean;
use qcore_schemas::quant_proposition::{Formula, Term};
use std::collections::BTreeSet;

const SIDECAR: &str =
    include_str!("../../../docs/reports/premise-guarded-decomposition/applications/plugin.json");
const KAFKA_MEM: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/kafka_mem.qv");
const LEAN: &str = include_str!("../../../proof/QCore/PremiseFormula/PluginGuardedSymbols.lean");

/// Each symbol and the Lean `def` its formula is.
const LEAN_NAMES: [(&str, &str); 14] = [
    ("default_case", "defaultCaseSymbol"),
    ("default_lifetime_valid", "defaultLifetimeValidSymbol"),
    ("environment", "environmentSymbol"),
    ("invoke", "invokeSymbol"),
    ("plugin_artifact_symbol_abi", "artifactSymbolAbiSymbol"),
    ("plugin_callback_candidates", "callbackCandidatesSymbol"),
    ("plugin_code_context_lifetime", "codeContextLifetimeSymbol"),
    ("plugin_interceptor_custody", "interceptorCustodySymbol"),
    (
        "plugin_interceptor_registration",
        "interceptorRegistrationSymbol",
    ),
    ("plugin_lifecycle_unload", "lifecycleUnloadSymbol"),
    ("plugin_outer_bridge", "outerBridgeSymbol"),
    ("registered_case", "registeredCaseSymbol"),
    ("registered_lifetime_valid", "registeredLifetimeValidSymbol"),
    ("unregistered", "unregisteredSymbol"),
];

const PREFIX: &str = "plg_";

fn sidecar() -> serde_json::Value {
    serde_json::from_str(SIDECAR).unwrap()
}

/// The binding of a typed plugin atom's formula to the SP9 terms (user
/// decision D1 A′): every free name `#X` is read as `#plg_X` with its
/// environment argument `#env` dropped (a function left with no argument is
/// a value), and every quantifier sort `S` as `plg_S`. Bound variables keep
/// their names.
fn bind(f: &Formula, bound: &mut BTreeSet<String>) -> Formula {
    fn term(t: &Term, bound: &BTreeSet<String>) -> Term {
        match t {
            Term::Variable { name } if bound.contains(name) => t.clone(),
            Term::Variable { name } => Term::Variable {
                name: format!("{PREFIX}{name}"),
            },
            Term::Integer { .. } => t.clone(),
            Term::Apply {
                function,
                arguments,
            } => {
                let arguments = arguments_of(arguments, bound);
                if arguments.is_empty() {
                    Term::Variable {
                        name: format!("{PREFIX}{function}"),
                    }
                } else {
                    Term::Apply {
                        function: format!("{PREFIX}{function}"),
                        arguments,
                    }
                }
            }
            other => panic!("no arithmetic in a plugin atom: {other:?}"),
        }
    }
    fn arguments_of(arguments: &[Term], bound: &BTreeSet<String>) -> Vec<Term> {
        let rest = match arguments.first() {
            Some(Term::Variable { name }) if name == "env" && !bound.contains(name) => {
                &arguments[1..]
            }
            _ => panic!("the environment is every atom application's first argument"),
        };
        rest.iter().map(|t| term(t, bound)).collect()
    }
    let boxed = |f: &Formula, bound: &mut BTreeSet<String>| Box::new(bind(f, bound));
    match f {
        Formula::Predicate { name, arguments } => Formula::Predicate {
            name: format!("{PREFIX}{name}"),
            arguments: arguments_of(arguments, bound),
        },
        Formula::Equal { left, right } => Formula::Equal {
            left: term(left, bound),
            right: term(right, bound),
        },
        Formula::Compare {
            operator,
            left,
            right,
        } => Formula::Compare {
            operator: operator.clone(),
            left: term(left, bound),
            right: term(right, bound),
        },
        Formula::Not { body } => Formula::Not {
            body: boxed(body, bound),
        },
        Formula::And { left, right } => Formula::And {
            left: boxed(left, bound),
            right: boxed(right, bound),
        },
        Formula::Or { left, right } => Formula::Or {
            left: boxed(left, bound),
            right: boxed(right, bound),
        },
        Formula::Implies { left, right } => Formula::Implies {
            left: boxed(left, bound),
            right: boxed(right, bound),
        },
        Formula::Forall {
            variable,
            sort,
            body,
        }
        | Formula::Exists {
            variable,
            sort,
            body,
        } => {
            let fresh = bound.insert(variable.clone());
            let body = boxed(body, bound);
            if fresh {
                bound.remove(variable);
            }
            let sort = format!("{PREFIX}{sort}");
            if matches!(f, Formula::Forall { .. }) {
                Formula::Forall {
                    variable: variable.clone(),
                    sort,
                    body,
                }
            } else {
                Formula::Exists {
                    variable: variable.clone(),
                    sort,
                    body,
                }
            }
        }
    }
}

fn parse(text: &str) -> Formula {
    let tokens = qcore_frontend::lexer::lex(text)
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();
    qcore_frontend::quant_proposition::parse_formula_tokens(&tokens).unwrap()
}

#[test]
fn every_plugin_symbol_is_a_formula_over_the_sp9_block() {
    let sidecar = sidecar();
    let bindings = sidecar["bindings"].as_object().unwrap();
    let recorded = legacy_prose_premise::plugin_guarded_symbols();
    assert_eq!(recorded.len(), 14);
    assert_eq!(bindings.len(), 14);
    let sp9: BTreeSet<_> = RENDERED_TERMS
        .iter()
        .filter(|t| t.block == "SP9")
        .map(|t| t.name)
        .collect();
    for r in &recorded {
        // The frozen sidecar's English text and role, recorded exactly.
        let b = &bindings[&r.id];
        assert_eq!(
            b["proposition"].as_str(),
            Some(r.prose.as_str()),
            "{}",
            r.id
        );
        assert_eq!(b["role"].as_str(), Some(r.role.as_str()), "{}", r.id);
        // The formula is a canonical typed rendering, every free name an SP9
        // term.
        let d = guarded_proposition(&r.id, &r.formula).unwrap();
        assert!(!d.parameters.is_empty());
        for p in &d.parameters {
            assert!(sp9.contains(p.name.as_str()), "{}: #{}", r.id, p.name);
        }
        // The English text itself is refused by name.
        let error = guarded_proposition(&r.id, &r.prose).unwrap_err();
        assert!(error.starts_with(PROSE_GUARDED), "{}: {error}", r.id);
        assert_eq!(r.atom.is_some(), r.role == "prerequisite", "{}", r.id);
    }
    // The legacy reader lifts all 14, and only from the exact recorded text.
    let mut lifted = sidecar["bindings"].clone();
    assert_eq!(legacy_prose_premise::lift_bindings(&mut lifted), 14);
    for r in &recorded {
        assert_eq!(
            lifted[&r.id]["proposition"].as_str(),
            Some(r.formula.as_str())
        );
    }
    let mut changed = sidecar["bindings"].clone();
    changed["invoke"]["proposition"] = "Admission of every reached callback.".into();
    assert_eq!(legacy_prose_premise::lift_bindings(&mut changed), 13);
    assert!(
        guarded_proposition("invoke", changed["invoke"]["proposition"].as_str().unwrap())
            .unwrap_err()
            .starts_with(PROSE_GUARDED)
    );
}

#[test]
fn each_prerequisite_is_its_production_atom_bound_to_the_sp9_terms() {
    let hypotheses = legacy_prose_premise::hypotheses();
    let mut prerequisites = 0;
    for r in legacy_prose_premise::plugin_guarded_symbols() {
        let Some(atom) = &r.atom else { continue };
        prerequisites += 1;
        // The atom is the SP6 record's, and the production typed atom of
        // `rd_kafka_dump0`'s module states it.
        let h = hypotheses.iter().find(|h| h.id == r.id).unwrap();
        assert_eq!(&h.formula, atom, "{}", r.id);
        assert_eq!(h.prose, r.prose, "{}", r.id);
        assert!(KAFKA_MEM.contains(&format!(") = {atom};")), "{}", r.id);
        // The symbol's formula is the atom's, bound to the SP9 terms.
        let bound = bind(&parse(atom), &mut BTreeSet::new());
        assert_eq!(bound, parse(&r.formula), "{}", r.id);
        assert_eq!(
            qcore_frontend::quant_proposition::print(&bound),
            r.formula,
            "{}",
            r.id
        );
        // Every atom parameter but the environment is an SP9 term of the
        // prefixed name, in the sort the binding gives it.
        let typed = h.typed.split_once(") = ").unwrap().0;
        for parameter in typed
            .split_once('(')
            .unwrap()
            .1
            .split(", ")
            .map(|p| p.split_once(": ").unwrap())
        {
            let (name, sort) = (parameter.0.trim_start_matches('#'), parameter.1);
            if name == "env" {
                assert_eq!(sort, "environment");
                continue;
            }
            let t = RENDERED_TERMS
                .iter()
                .find(|t| t.block == "SP9" && t.name == format!("{PREFIX}{name}"))
                .unwrap_or_else(|| panic!("#{name} has no SP9 term"));
            let expected = bound_sort(sort);
            assert_eq!(t.sort.sort(), expected, "#{name}");
        }
    }
    assert_eq!(prerequisites, 7);
}

/// The SP9 sort of an atom parameter sort: the environment argument dropped,
/// every value sort prefixed (`integer` kept), a function of no argument a
/// value.
fn bound_sort(sort: &str) -> qcore_schemas::quant_proposition::Sort {
    use qcore_schemas::quant_proposition::Sort as G;
    let value = |s: &str| {
        if s == "integer" {
            s.to_owned()
        } else {
            format!("{PREFIX}{s}")
        }
    };
    let list = |inner: &str| -> Vec<String> {
        inner
            .split(',')
            .filter(|s| *s != "environment")
            .map(value)
            .collect()
    };
    if let Some(inner) = sort
        .strip_prefix("predicate<")
        .and_then(|s| s.strip_suffix('>'))
    {
        G::Predicate {
            arguments: list(inner),
        }
    } else if let Some(inner) = sort
        .strip_prefix("function<")
        .and_then(|s| s.strip_suffix('>'))
    {
        let mut arguments = list(inner);
        let result = arguments.pop().unwrap();
        if arguments.is_empty() {
            G::Value { name: result }
        } else {
            G::Function { arguments, result }
        }
    } else {
        G::Value { name: value(sort) }
    }
}

#[test]
fn the_sp9_block_is_new_prefixed_and_bound_to_lean() {
    let sp9: Vec<_> = RENDERED_TERMS.iter().filter(|t| t.block == "SP9").collect();
    assert_eq!(sp9.len(), 63);
    // Appended after the SP7 block, never reordering it; contiguous, and
    // followed only by later blocks (card SP10-FORMAL-BINDING-DESCRIPTIONS
    // appends SP10 after it, card MEMBER-EFFECT-STORES its MES block after
    // that).
    let blocks: Vec<_> = RENDERED_TERMS.iter().map(|t| t.block).collect();
    let first = blocks.iter().position(|b| *b == "SP9").unwrap();
    assert!(blocks[first..first + sp9.len()].iter().all(|b| *b == "SP9"));
    assert!(
        blocks[first + sp9.len()..]
            .iter()
            .all(|b| *b == "SP10" || *b == "MES")
    );
    assert_eq!(blocks[first - 1], "SP7");
    for t in &sp9 {
        assert!(t.name.starts_with(PREFIX), "#{}", t.name);
        assert!(
            qcore_frontend::premise_formula::term(t.name).is_none(),
            "#{} is already a registered term",
            t.name
        );
        assert_eq!(
            RENDERED_TERMS.iter().filter(|u| u.name == t.name).count(),
            1,
            "#{}",
            t.name
        );
        assert!(!t.meaning.is_empty());
    }
    let sorts: Vec<_> = RENDERED_VALUE_SORTS
        .iter()
        .filter(|(s, _)| s.starts_with(PREFIX))
        .collect();
    assert_eq!(sorts.len(), 13);
    let quoted = |names: &mut dyn Iterator<Item = &str>| {
        names
            .map(|n| format!("\"{n}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let terms = format!(
        "def sp9Terms : List String := [{}]",
        quoted(&mut sp9.iter().map(|t| t.name))
    );
    assert!(LEAN.lines().any(|l| l == terms), "no `{terms}`");
    let sorts = format!(
        "def sp9Sorts : List String := [{}]",
        quoted(&mut sorts.iter().map(|(s, _)| *s))
    );
    assert!(LEAN.lines().any(|l| l == sorts), "no `{sorts}`");
    // No generic name is a rendered term: the typed atoms' unprefixed
    // parameter names stay unregistered.
    for generic in [
        "time", "action", "member", "in_trace", "env", "ev", "invoke",
    ] {
        assert!(qcore_frontend::premise_formula::rendered_parameter_sort(generic).is_none());
        let error = guarded_proposition("environment", &format!("#{generic}()")).unwrap_err();
        assert!(error.starts_with(PROSE_GUARDED), "{error}");
    }
}

#[test]
fn the_symbols_are_the_lean_definitions() {
    let recorded = legacy_prose_premise::plugin_guarded_symbols();
    for (id, name) in LEAN_NAMES {
        let r = recorded.iter().find(|r| r.id == id).unwrap();
        let d = guarded_proposition(id, &r.formula).unwrap();
        let line = format!("def {name} : Expr := {}", lean(&d.formula));
        assert!(
            LEAN.lines().any(|l| l == line),
            "{id}: the Lean module has no `{line}`"
        );
    }
}

/// Prints each recorded symbol's canonical printing as `ID\tFORMULA`
/// (`--ignored --nocapture`): the text the record holds.
#[test]
#[ignore]
fn print_the_canonical_formulas() {
    for r in legacy_prose_premise::plugin_guarded_symbols() {
        println!(
            "CANONICAL\t{}\t{}",
            r.id,
            qcore_frontend::quant_proposition::print(&parse(&r.formula))
        );
    }
}

/// Prints the Lean `def` lines of the 14 symbols (`--ignored --nocapture`),
/// the text `the_symbols_are_the_lean_definitions` binds.
#[test]
#[ignore]
fn print_the_lean_definitions() {
    let recorded = legacy_prose_premise::plugin_guarded_symbols();
    for (id, name) in LEAN_NAMES {
        let r = recorded.iter().find(|r| r.id == id).unwrap();
        let d = guarded_proposition(id, &r.formula).unwrap();
        println!("def {name} : Expr := {}", lean(&d.formula));
    }
}
