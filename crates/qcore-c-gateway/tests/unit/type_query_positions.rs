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

//! Card K2-NO-TYPED-ACCESS (rule 79): the formulas, the decision, the token
//! fact and their binding to `QCore.TypeQueryPositions`.

use crate::type_query_positions::{
    CONDITIONS, DECISION_VECTORS, EventClass, Facts, Query, RULE, Reason, code, decide, formulas,
    product_run, validated, vector_facts,
};

const LEAN: &str = include_str!("../../../../proof/QCore/TypeQueryPositions.lean");

#[test]
fn every_formula_is_a_checked_typed_rendering_formula() {
    for (id, formula) in formulas() {
        qcore_contract::premise_formula::guarded_proposition(id, formula).unwrap_or_else(|e| {
            let tokens = qcore_contract::formula_syntax::lexer::lex(formula).expect("lexes");
            let kinds = tokens.into_iter().map(|t| t.kind).collect::<Vec<_>>();
            let canonical =
                qcore_contract::formula_syntax::quant_proposition::parse_formula_tokens(&kinds)
                    .map(|f| qcore_contract::formula_syntax::quant_proposition::print(&f));
            panic!("{id}: {e}\ncanonical: {canonical:?}")
        });
    }
    validated().unwrap();
    assert_eq!(RULE, "79");
}

#[test]
fn every_formula_names_only_k2_terms_and_sorts() {
    for (id, formula) in formulas() {
        assert!(id.starts_with("k2_"), "{id}");
        for word in formula.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '#')) {
            if let Some(name) = word.strip_prefix('#') {
                assert!(
                    name.starts_with("k2_") || name == "q" || name == "e",
                    "{id}: #{name}"
                );
            }
        }
    }
}

#[test]
fn the_k2_block_is_new_prefixed_and_lean_s() {
    use qcore_contract::premise_formula::{RENDERED_TERMS, RENDERED_VALUE_SORTS};
    let k2: Vec<_> = RENDERED_TERMS.iter().filter(|t| t.block == "K2").collect();
    assert_eq!(k2.len(), 14);
    for t in &k2 {
        assert!(t.name.starts_with("k2_"), "#{}", t.name);
        assert!(qcore_contract::premise_formula::term(t.name).is_none());
        assert_eq!(
            RENDERED_TERMS.iter().filter(|u| u.name == t.name).count(),
            1
        );
        assert!(!t.meaning.is_empty());
    }
    let names = k2
        .iter()
        .map(|t| format!("\"{}\"", t.name))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def k2Terms : List String := [{names}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    let sorts: Vec<_> = RENDERED_VALUE_SORTS
        .iter()
        .filter(|(s, _)| s.starts_with("k2_"))
        .map(|(s, _)| format!("\"{s}\""))
        .collect();
    assert_eq!(sorts.len(), 4);
    let line = format!("def k2Sorts : List String := [{}]", sorts.join(", "));
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    // Appended after MES: the K2 block is contiguous and followed only by
    // the K6 block (card K6-DIRECT-POSITION-ACCESS, rule 81), the CX block
    // (card QUIRIN-AUDIT-CONNECT) and the G1 block (card
    // P0-G1-CLOSURE-RECORD), never reordered.
    let blocks: Vec<_> = RENDERED_TERMS.iter().map(|t| t.block).collect();
    let first = blocks.iter().position(|b| *b == "K2").unwrap();
    let count = blocks.iter().filter(|b| **b == "K2").count();
    assert!(blocks[first..first + count].iter().all(|b| *b == "K2"));
    assert!(
        blocks[first + count..]
            .iter()
            .all(|b| *b == "K6" || *b == "CX" || *b == "G1")
    );
}

#[test]
fn the_formula_table_is_lean_s() {
    for (id, formula) in formulas() {
        let line = format!("  (\"{id}\", \"{}\"),", formula.replace('\\', "\\\\"));
        let last = line.trim_end_matches(',').to_owned();
        assert!(
            LEAN.lines().any(|l| l == line || l == last),
            "no Lean row `{line}`"
        );
    }
}

#[test]
fn each_refusal_negates_its_condition() {
    for reason in Reason::ALL {
        assert!(CONDITIONS.iter().any(|(id, _)| *id == reason.condition()));
        let facts = vector_facts(match reason {
            Reason::NoTypeQuery => "no type query",
            Reason::EvaluatedQuery => "a variably modified operand",
            Reason::Event => "a call event",
        });
        assert_eq!(decide(&facts), Err(reason));
    }
}

#[test]
fn the_decision_vectors_hold_and_are_lean_s() {
    for (name, differs, answer) in DECISION_VECTORS {
        assert_eq!(code(decide(&vector_facts(differs))), *answer, "{name}");
    }
    let rows = DECISION_VECTORS
        .iter()
        .map(|(n, d, a)| format!("(\"{n}\", \"{d}\", \"{a}\")"))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def decisionVectors : List (String × String × String) := [{rows}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
}

#[test]
fn a_region_with_a_type_query_and_inert_events_is_admitted() {
    let facts = Facts {
        queries: vec![Query {
            type_operand: true,
            variably_modified: false,
        }],
        events: vec![
            EventClass::Branch,
            EventClass::ObjectRead { unaliased: true },
            EventClass::Branch,
        ],
    };
    assert_eq!(decide(&facts), Ok(()));
    let mut store = facts.clone();
    store.events.push(EventClass::Other);
    assert_eq!(decide(&store), Err(Reason::Event));
}

#[test]
fn a_product_run_is_never_a_declaration_candidate() {
    use crate::typed_field_access::is_declaration_candidate;
    let run = ["(", "capacity", "*", "sizeof", "(", "T", "*", ")", ")"];
    assert!(product_run(&run, 1));
    assert!(!is_declaration_candidate(&run, 1));
    // Near misses keep the older candidate reading.
    let pointer = ["(", "T", "*", "p", ")"];
    assert!(!product_run(&pointer, 1));
    assert!(is_declaration_candidate(&pointer, 1));
    let keyword = ["(", "long", "*", "sizeof", "x", ")"];
    assert!(!product_run(&keyword, 1));
    let other_keyword = ["(", "n", "*", "alignof", "(", "T", ")", ")"];
    assert!(!product_run(&other_keyword, 1));
    assert!(is_declaration_candidate(&other_keyword, 1));
    let named = ["(", "T", "*", "sizeofx", ")"];
    assert!(!product_run(&named, 1));
    assert!(is_declaration_candidate(&named, 1));
}

#[test]
fn a_query_is_read_from_the_clang_node() {
    use serde_json::{Value, json};
    let query = |arg: Value, inner: Vec<Value>| {
        let mut node =
            json!({"kind": "UnaryExprOrTypeTraitExpr", "name": "sizeof", "argType": arg});
        if !inner.is_empty() {
            node["inner"] = Value::Array(inner);
        }
        json!({"kind": "BinaryOperator", "inner": [node, {"kind": "IntegerLiteral"}]})
    };
    let read = |node: &Value| {
        let mut queries = Vec::new();
        crate::type_query_positions::collect_queries(node, &mut queries);
        queries
    };
    let unevaluated = Query {
        type_operand: true,
        variably_modified: false,
    };
    let plain = query(
        json!({"qualType": "reg_t", "desugaredQualType": "unsigned long"}),
        vec![],
    );
    assert_eq!(read(&plain), vec![unevaluated]);
    let vla = query(json!({"qualType": "int[n]"}), vec![]);
    assert!(read(&vla)[0].variably_modified);
    let hidden = query(
        json!({"qualType": "row_t", "desugaredQualType": "int[n]"}),
        vec![],
    );
    assert!(read(&hidden)[0].variably_modified);
    let expression = query(
        json!({"qualType": "int"}),
        vec![json!({"kind": "DeclRefExpr"})],
    );
    assert!(!read(&expression)[0].type_operand);
    let decision = |queries: Vec<Query>| {
        decide(&Facts {
            queries,
            events: vec![EventClass::Branch],
        })
    };
    assert_eq!(decision(read(&plain)), Ok(()));
    assert_eq!(decision(read(&vla)), Err(Reason::EvaluatedQuery));
    assert_eq!(decision(read(&expression)), Err(Reason::EvaluatedQuery));
}
