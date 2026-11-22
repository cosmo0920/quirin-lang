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

//! Card **SP6-SYMBOLIC-HYPOTHESIS-ATOMS** (contract specification C04.18.7,
//! C04.18.9's SP6 block): the nine `detach_*` hypotheses of
//! `cJSON_DetachItemViaPointer` are typed atoms, each the typed rendering of
//! its part of the declaration's premise formula over registered terms; the
//! string hypothesis atom is refused (`QF002`) on every route.

#[path = "../../../tests/support/legacy_prose_premise.rs"]
mod legacy_prose_premise;

use qcore_contract::hypothesis;
use qcore_schemas::hypothesis::{Atom, Conjunction, ErasedPremise, Graph, Proposition};
use qcore_schemas::quant_proposition::{Definition, Formula};
use std::collections::BTreeMap;

const CJSON: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
const LEAN: &str = include_str!("../../../proof/QCore/PremiseFormula/CjsonTypedAtoms.lean");
const DECLARATION: &str = "cJSON_DetachItemViaPointer";

/// The Lean `def` each atom is bound to.
const LEAN_NAMES: [(&str, &str); 9] = [
    ("detach_chain", "chainAtom"),
    ("detach_distinct", "distinctAtom"),
    ("detach_empty", "emptyAtom"),
    ("detach_first_prev", "firstPrevAtom"),
    ("detach_last_next", "lastNextAtom"),
    ("detach_link_targets", "linkTargetsAtom"),
    ("detach_no_writer", "noWriterAtom"),
    ("detach_parent_outside", "parentOutsideAtom"),
    ("detach_predecessor", "predecessorAtom"),
];

fn graph() -> Graph {
    let sources = BTreeMap::from([("cjson.qv".to_owned(), CJSON.to_owned())]);
    hypothesis::replay(&sources)
        .unwrap()
        .remove(DECLARATION)
        .unwrap()
}

fn definition(graph: &Graph, id: &str) -> Definition {
    match &graph.atoms[id].proposition {
        Proposition::Formula { definition } => definition.clone(),
        other => panic!("{id} is not a typed atom: {other:?}"),
    }
}

use qcore_frontend::quant_proposition::lean;

#[test]
fn the_nine_hypotheses_are_typed_atoms_of_the_refinement() {
    let graph = graph();
    assert_eq!(graph.version, 3);
    assert!(graph.requirements.is_empty(), "every atom is grouped");
    let refinement = &graph.refinements["detach_list_representation"];
    assert_eq!(
        refinement.children,
        LEAN_NAMES
            .iter()
            .map(|(id, _)| (*id).to_owned())
            .collect::<Vec<_>>()
    );
    for (id, _) in LEAN_NAMES {
        definition(&graph, id);
    }
    assert!(
        graph
            .atoms
            .iter()
            .all(|(id, a)| id.starts_with("legacy_atom_")
                || matches!(a.proposition, Proposition::Formula { .. })),
        "no opaque declared atom"
    );
    assert!(!CJSON.contains("resource_hypothesis("));
}

#[test]
fn the_nine_atoms_are_the_lean_definitions() {
    let graph = graph();
    for (id, name) in LEAN_NAMES {
        let rendered = format!(
            "def {name} : Expr := {}",
            lean(&definition(&graph, id).formula)
        );
        assert!(
            LEAN.lines().any(|line| line == rendered),
            "{id}: the Lean module has no `{rendered}`"
        );
    }
}

#[test]
fn the_nine_atoms_render_registered_terms() {
    let graph = graph();
    for (id, _) in LEAN_NAMES {
        let d = definition(&graph, id);
        qcore_frontend::premise_formula::check_rendering(&d).unwrap();
        // Every parameter a registered C04.18.9 term or the SP6 `#runs`.
        for p in &d.parameters {
            assert!(
                qcore_frontend::premise_formula::term(&p.name).is_some() || p.name == "runs",
                "{id}: #{}",
                p.name
            );
        }
    }
    // A parameter in another sort, or an unregistered one, is not a rendering.
    let wrong = qcore_frontend::quant_proposition::parse(
        "#a(#children: predicate<obj>, #parent: obj) = #children(#parent)",
    )
    .unwrap();
    assert!(qcore_frontend::premise_formula::check_rendering(&wrong).is_err());
    let unknown =
        qcore_frontend::quant_proposition::parse("#a(#holds: predicate<>) = #holds()").unwrap();
    assert!(qcore_frontend::premise_formula::check_rendering(&unknown).is_err());
}

#[test]
fn each_prerequisite_symbol_repeats_its_atom_formula() {
    let graph = graph();
    let parsed = qcore_frontend::frontend::parse_source_program(CJSON).unwrap();
    let decl = parsed
        .declarations
        .iter()
        .find(|d| d.value.name == DECLARATION)
        .unwrap();
    let inline = qcore_contract::guarded_decomposition::declaration(&decl.value, &graph)
        .unwrap()
        .unwrap();
    for (id, _) in LEAN_NAMES {
        let binding = &inline.bindings[id];
        let source = decl
            .value
            .function_contract
            .iter()
            .flat_map(|c| &c.clauses)
            .find(|c| {
                c.value.kind == qcore_frontend::frontend::SourceContractClauseKind::QuantProposition
                    && c.value.expression_source.starts_with(&format!("#{id}("))
            })
            .unwrap()
            .value
            .expression_source
            .clone();
        let formula = source.split_once(") = ").unwrap().1;
        assert_eq!(binding.proposition, formula, "{id}");
        assert_eq!(binding.dependencies, vec![id.to_owned()]);
    }
}

/// A one-atom graph: `id` stating `statement` (a string literal: opaque;
/// otherwise a typed definition), with what makes it otherwise well formed.
fn one_atom_graph(legacy: bool, statement: &str) -> Graph {
    let id = if legacy {
        "legacy_atom_00000000"
    } else {
        "detach_distinct"
    };
    let proposition = if let Some(s) = statement.strip_prefix('"') {
        Proposition::Opaque {
            statement: s.strip_suffix('"').unwrap().to_owned(),
        }
    } else {
        Proposition::Formula {
            definition: Definition {
                id: id.to_owned(),
                ..qcore_frontend::quant_proposition::parse(statement).unwrap_or_else(|_| {
                    Definition {
                        id: id.to_owned(),
                        parameters: Vec::new(),
                        formula: Formula::Predicate {
                            name: "p".into(),
                            arguments: Vec::new(),
                        },
                    }
                })
            },
        }
    };
    let mut graph = Graph {
        version: 1,
        atoms: BTreeMap::from([(
            id.to_owned(),
            Atom {
                id: id.to_owned(),
                proposition: proposition.clone(),
            },
        )]),
        premises: BTreeMap::new(),
        refinements: BTreeMap::new(),
        requirements: Vec::new(),
    };
    if legacy {
        let statement = match &proposition {
            Proposition::Opaque { statement } => statement.clone(),
            _ => "typed".into(),
        };
        graph.premises.insert(
            "legacy_premise_00000000".into(),
            Conjunction {
                id: "legacy_premise_00000000".into(),
                children: vec![id.into()],
                erasure: ErasedPremise {
                    name: "p".into(),
                    statement,
                    origin: "declared".into(),
                },
            },
        );
    } else {
        graph.premises.insert(
            "p".into(),
            Conjunction {
                id: "p".into(),
                children: vec![id.into()],
                erasure: ErasedPremise {
                    name: "p".into(),
                    statement: format!("all({id})"),
                    origin: "declared".into(),
                },
            },
        );
    }
    graph.version = hypothesis::expected_version(&graph);
    graph
}

#[test]
fn the_atom_decision_vectors_are_the_validator() {
    for (legacy, statement, accepted) in hypothesis::ATOM_DECISION_VECTORS {
        let graph = one_atom_graph(*legacy, statement);
        assert_eq!(
            hypothesis::validate(&graph).is_ok(),
            *accepted,
            "{legacy} {statement}: {:?}",
            hypothesis::validate(&graph)
        );
    }
}

#[test]
fn the_atom_decision_vectors_are_the_lean_table() {
    let block = LEAN
        .split("def hypothesisVectors : List (Bool × AtomStatement × Bool) := [")
        .nth(1)
        .unwrap()
        .split("\n]")
        .next()
        .unwrap();
    let rows = block
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| l.trim_end_matches(',').to_owned())
        .collect::<Vec<_>>();
    let expected = hypothesis::ATOM_DECISION_VECTORS
        .iter()
        .map(|(legacy, statement, accepted)| {
            let statement = if let Some(s) = statement.strip_prefix('"') {
                format!(".str \"{}\"", s.strip_suffix('"').unwrap())
            } else {
                // A legacy ID is refused by the reader itself; the formula is
                // read under a declared ID.
                let source = statement.replacen("#legacy_atom_00000000(", "#legacy(", 1);
                let d = qcore_frontend::quant_proposition::parse(&source).unwrap();
                format!(".typed {}", lean(&d.formula))
            };
            format!("({legacy}, {statement}, {accepted})")
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, expected);
}

#[test]
fn a_string_hypothesis_is_refused_by_name_on_every_route() {
    let module = |clause: &str| {
        format!(
            "module contract; extern fn f(p: ptr<mut,u8>) -> unit contract {{ requires resource_contract_v1; {clause} ensures normal resource_borrowed(p); ensures normal resource_return_none(); }};"
        )
    };
    for clause in [
        "requires resource_hypothesis(a, \"A holds\");",
        "requires resource_hypothesis(a, \"#nodup(#parent)\");",
        "requires resource_hypothesis(a, \"\");",
        "ensures normal resource_hypothesis(a, \"A holds\");",
        "requires resource_hypothesis(a, typed(unknown));",
        "requires resource_hypothesis(a);",
    ] {
        let source = module(clause);
        let parse = qcore_frontend::frontend::parse_source_program(&source).unwrap_err();
        assert!(parse.to_string().contains("QF002"), "{clause}: {parse}");
        let sources = BTreeMap::from([("m.qv".to_owned(), source.clone())]);
        let load = qcore_contract::contract_loader::load_resource_contract_sources(&sources)
            .unwrap_err()
            .to_string();
        assert!(load.contains("QF002"), "{clause}: {load}");
        assert!(hypothesis::replay(&sources).unwrap_err().contains("QF002"));
        let bounds = source.replace("resource_contract_v1", "bounds_contract_v1");
        let parse = qcore_frontend::frontend::parse_source_program(&bounds).unwrap_err();
        assert!(parse.to_string().contains("QF002"), "{clause}: {parse}");
    }
    // The same module with a typed atom loads.
    let typed = module(
        "quant proposition #a(#nodup: predicate<obj>, #parent: obj) = #nodup(#parent); requires resource_premise_all(q, a);",
    );
    let sources = BTreeMap::from([("m.qv".to_owned(), typed)]);
    qcore_contract::contract_loader::load_resource_contract_sources(&sources).unwrap();
    // On the wire: a declared opaque atom is refused by name.
    let mut wire = graph();
    wire.atoms.get_mut("detach_distinct").unwrap().proposition = Proposition::Opaque {
        statement:
            "For this removed-result declaration, chain child identities are pairwise distinct"
                .into(),
    };
    assert!(hypothesis::validate(&wire).unwrap_err().contains("QF002"));
    let bytes = serde_json::to_vec(&wire).unwrap();
    assert!(hypothesis::decode(&bytes).is_err());
}

#[test]
fn the_recorded_hypotheses_are_their_typed_clauses() {
    let recorded = legacy_prose_premise::hypotheses();
    assert_eq!(recorded.len(), 24);
    let graph = graph();
    for h in &recorded {
        let source = h.typed.strip_prefix("quant proposition ").unwrap();
        let d = qcore_frontend::quant_proposition::parse(source).unwrap();
        assert_eq!(d.id, h.id);
        let atom = Atom {
            id: h.id.clone(),
            proposition: Proposition::Formula { definition: d },
        };
        assert_eq!(serde_json::to_value(&atom).unwrap(), h.atom, "{}", h.id);
        assert_eq!(source.split_once(") =").unwrap().1.trim(), h.formula);
        if let Some(current) = graph.atoms.get(&h.id) {
            assert_eq!(current, &atom, "{}", h.id);
        }
        // The retired sentence is refused where it is stated.
        let text = format!(
            "module contract; extern fn f() -> unit contract {{ requires resource_contract_v1; requires resource_hypothesis({}, \"{}\"); ensures normal resource_return_none(); }};",
            h.id, h.prose
        );
        assert!(
            qcore_frontend::frontend::parse_source_program(&text)
                .unwrap_err()
                .to_string()
                .contains("QF002")
        );
        // ... and lifted only by the test-side legacy reader.
        assert!(legacy_prose_premise::lift(&text).contains(&h.typed));
    }
}
