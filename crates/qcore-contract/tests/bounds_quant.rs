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

//! Card QUANT-INDEX-CHECKS: the quantified index check of bounds contract v1
//! (grammar G05c, contract specification C09.2) loads total-or-reject, every
//! unsupported form refused by a named diagnostic, and every other consumer
//! still rejects it.

use std::collections::BTreeMap;

use qcore_contract::bounds_contract::{BoundsClause, load_bounds_contract_sources};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_core::index_quant::{BoundTerm, QuantPredicate, Relation};
use qcore_frontend::frontend::{SourceContractClauseKind, parse_source_program};

fn module(clauses: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "quant.qv".to_owned(),
        format!(
            "module contract;
extern fn get_meta_len(map: ptr<const, u8>, len: usize, out: ptr<mut, ptr<mut, u8>>) -> u16
contract {{
    requires bounds_contract_v1;
{clauses}
    ensures normal bounds_frame_reads(map);
    ensures normal bounds_frame_writes(out);
}};
"
        ),
    )])
}

fn check(predicate: &str) -> Result<QuantPredicate, String> {
    let contracts = load_bounds_contract_sources(&module(&format!(
        "    quant checks map, Idx #i: {predicate};"
    )))?;
    let contract = contracts.contract("get_meta_len").expect("declared");
    contract
        .clauses
        .iter()
        .find_map(|entry| match &entry.clause {
            BoundsClause::QuantCheck { predicate, .. } => Some(predicate.clone()),
            _ => None,
        })
        .ok_or_else(|| "no quant check".to_owned())
}

fn rejected(predicate: &str, code: &str) {
    match check(predicate) {
        Ok(read) => panic!("`{predicate}` must be rejected ({code}), read {read:?}"),
        Err(error) => assert!(error.contains(code), "`{predicate}`: {error}"),
    }
}

fn constant(value: i128) -> BoundTerm {
    BoundTerm {
        constant: value,
        formal: None,
    }
}

fn compare(relation: Relation, bound: BoundTerm) -> QuantPredicate {
    QuantPredicate::Compare { relation, bound }
}

#[test]
fn the_motivating_range_loads_as_a_conjunction() {
    assert_eq!(
        check("0 <= #i && #i < 24").expect("loads"),
        QuantPredicate::And(
            Box::new(compare(Relation::Ge, constant(0))),
            Box::new(compare(Relation::Lt, constant(24)))
        )
    );
    let contracts =
        load_bounds_contract_sources(&module("    quant checks map, Idx #i: 0 <= #i && #i < 24;"))
            .expect("loads");
    let entry = contracts
        .contract("get_meta_len")
        .expect("declared")
        .clauses[0]
        .clone();
    assert_eq!(entry.text, "quant checks map, Idx #i: 0 <= #i && #i < 24");
    assert!(entry.clause.is_precondition(), "a caller obligation");
}

#[test]
fn every_comparison_operator_loads_with_the_variable_on_either_side() {
    let cases = [
        ("#i == 3", Relation::Eq, 3),
        ("3 == #i", Relation::Eq, 3),
        ("0 <= #i && #i < 24", Relation::Lt, 24),
        ("0 <= #i && 24 > #i", Relation::Lt, 24),
        ("0 <= #i && #i <= 23", Relation::Le, 23),
        ("0 <= #i && 23 >= #i", Relation::Le, 23),
        ("#i > 21 && #i < 24", Relation::Gt, 21),
        ("21 < #i && #i < 24", Relation::Gt, 21),
        ("#i >= 22 && #i <= 23", Relation::Ge, 22),
        ("22 <= #i && #i <= 23", Relation::Ge, 22),
    ];
    for (text, relation, bound) in cases {
        let read = check(text).unwrap_or_else(|e| panic!("`{text}`: {e}"));
        let mut found = false;
        let mut stack = vec![&read];
        while let Some(node) = stack.pop() {
            match node {
                QuantPredicate::Compare {
                    relation: r,
                    bound: b,
                } => {
                    found |= *r == relation && *b == constant(bound);
                }
                QuantPredicate::And(a, b) | QuantPredicate::Or(a, b) => {
                    stack.push(a);
                    stack.push(b);
                }
            }
        }
        assert!(found, "`{text}` read as {read:?}");
    }
}

#[test]
fn disjunction_and_parentheses_load_literally() {
    let read = check("(0 <= #i && #i < 4) || (8 <= #i && #i < 12)").expect("loads");
    assert!(matches!(read, QuantPredicate::Or(..)), "{read:?}");
    // `&&` binds tighter than `||`, as written.
    let read = check("#i == 1 || #i >= 2 && #i <= 3").expect("loads");
    assert!(
        matches!(&read, QuantPredicate::Or(_, right) if matches!(**right, QuantPredicate::And(..))),
        "{read:?}"
    );
}

#[test]
fn formal_relative_and_signed_bounds_load() {
    assert_eq!(
        check("0 <= #i && #i < len").expect("loads"),
        QuantPredicate::And(
            Box::new(compare(Relation::Ge, constant(0))),
            Box::new(compare(
                Relation::Lt,
                BoundTerm {
                    constant: 0,
                    formal: Some("len".into())
                }
            ))
        )
    );
    assert!(check("0 <= #i && #i <= len - 1").is_ok());
    assert!(check("0 <= #i && #i < len + 2 - 1").is_ok());
    assert!(check("-2 <= #i && #i < 0").is_ok());
    assert!(check("0 - 2 <= #i && #i < 0").is_ok());
}

#[test]
fn unsupported_forms_are_rejected_by_name() {
    rejected("0 <= #i && #i != 5", "quant-check-operator");
    rejected("!(#i < 0) && #i < 4", "quant-check-operator");
    rejected("0 <= #i && #i * 2 < 4", "quant-check-operator");
    rejected("0 <= #i < 24", "quant-check-chained");
    rejected("#i < #i", "quant-check-variable");
    rejected("0 < 24", "quant-check-variable");
    rejected("0 <= #j && #j < 24", "quant-check-variable");
    rejected("0 <= #i && #i + 1 < 24", "quant-check-variable");
    rejected("0 <= #i && #i < CIO_FILE_HEADER_MIN", "quant-check-term");
    rejected("0 <= #i && #i < len + len", "quant-check-term");
    rejected("0 <= #i && #i < len - len + 4", "quant-check-term");
    rejected("0 <= #i && #i < -len + 30", "quant-check-term");
    rejected("0 <= #i && #i < map", "quant-check-term");
    rejected("0 <= #i && #i < 024", "quant-check-term");
    rejected("0 <= #i && #i < (24)", "quant-check-term");
    rejected(
        "0 <= #i && #i < 99999999999999999999999999999999999999999",
        "quant-check-overflow",
    );
    rejected("0 <= #i &&", "quant-check-shape");
    // An unbalanced group never reaches the reader: the structural parser
    // refuses the module first.
    rejected("(0 <= #i && #i < 24", "mismatched closing delimiter");
    rejected("((0 <= #i) && #i < 24", "mismatched closing delimiter");
}

#[test]
fn unbounded_and_empty_sets_are_rejected() {
    rejected("0 <= #i", "quant-check-unbounded");
    rejected("#i < 24", "quant-check-unbounded");
    // `||` is read literally: an over-wide disjunction admits every index.
    rejected("#i < 24 || 0 <= #i", "quant-check-unbounded");
    // The user's spelling of the out-of-range set reads as written.
    rejected("#i < 24 || 0 < #i", "quant-check-unbounded");
    rejected("(0 <= #i && #i < 4) || #i > 10", "quant-check-unbounded");
    rejected("#i < 0 && #i > 5", "quant-check-empty");
    rejected("#i == 3 && #i == 4", "quant-check-empty");
    // A symbolic set is bounded by its form; its emptiness is decided at use.
    assert!(check("len <= #i && #i < 4").is_ok());
}

#[test]
fn the_header_subject_and_binder_are_checked() {
    let load = |clause: &str| load_bounds_contract_sources(&module(clause));
    for (clause, code) in [
        (
            "    quant checks len, Idx #i: 0 <= #i && #i < 4;",
            "quant-check-subject",
        ),
        (
            "    quant checks out, Idx #i: 0 <= #i && #i < 4;",
            "quant-check-subject",
        ),
        (
            "    quant checks nothing, Idx #i: 0 <= #i && #i < 4;",
            "quant-check-subject",
        ),
        (
            "    quant checks map, Index #i: 0 <= #i && #i < 4;",
            "quant-check-binder",
        ),
        (
            "    quant checks map, Idx i: 0 <= #i && #i < 4;",
            "quant-check-shape",
        ),
        (
            "    quant checks map Idx #i: 0 <= #i && #i < 4;",
            "quant-check-shape",
        ),
        (
            "    quant checks map, Idx #i 0 <= #i && #i < 4;",
            "quant-check-shape",
        ),
        ("    quant checks map, Idx #i:;", "quant-check-shape"),
        (
            "    quant checks map, Idx #i: 0 <= #i && #i < 4;\n    quant checks map, Idx #i: 0 <= #i && #i < 8;",
            "duplicate bounds clause for quant map",
        ),
        (
            "    requires bounds_extent(map, 24, elements);\n    quant checks map, Idx #i: 0 <= #i && #i < 24;",
            "quant-check-with-extent",
        ),
    ] {
        let error = load(clause).expect_err(clause);
        assert!(error.contains(code), "{clause}: {error}");
    }
}

#[test]
fn an_exploding_disjunctive_form_is_rejected() {
    let pair = "(#i == 0 || #i == 1)";
    let text = [pair; 7].join(" && ");
    rejected(&text, "quant-check-size");
}

#[test]
fn a_contract_level_review_marker_holds_the_check() {
    let contracts = load_bounds_contract_sources(&module(
        "    requires bounds_review_pending(\"chunkio header layout\");\n    quant checks map, Idx #i: 0 <= #i && #i < 24;",
    ))
    .expect("loads");
    let contract = contracts.contract("get_meta_len").expect("declared");
    assert!(contract.held());
    assert!(contract.holds()[0].contains("chunkio header layout"));
}

#[test]
fn the_clause_text_is_part_of_the_declaration_digest() {
    let digest = |predicate: &str| {
        load_bounds_contract_sources(&module(&format!(
            "    quant checks map, Idx #i: {predicate};"
        )))
        .expect("loads")
        .contract("get_meta_len")
        .expect("declared")
        .text_sha256
        .clone()
    };
    assert_ne!(digest("0 <= #i && #i < 24"), digest("0 <= #i && #i < 25"));
    assert_eq!(digest("0 <= #i && #i < 24"), digest("0<=#i&&#i<24"));
}

#[test]
fn the_structural_parser_retains_the_clause_and_other_consumers_reject_it() {
    let text = "module contract;
extern fn get_meta_len(map: ptr<const, u8>) -> u16
contract {
    requires resource_contract_v1;
    quant checks map, Idx #i: 0 <= #i && #i < 24;
    ensures normal resource_borrowed(map);
    ensures normal resource_return_none();
};
";
    let parsed = parse_source_program(text).expect("structural parse");
    let clauses = &parsed.declarations[0]
        .value
        .function_contract
        .as_ref()
        .expect("contract")
        .clauses;
    assert_eq!(clauses[1].value.kind, SourceContractClauseKind::QuantCheck);
    assert_eq!(
        clauses[1].value.expression_source,
        "map, Idx #i: 0 <= #i && #i < 24"
    );
    // Resource v1 rejects the module: its vocabulary has no quant check.
    let error =
        load_resource_contract_sources(&BTreeMap::from([("r.qv".to_owned(), text.to_owned())]))
            .expect_err("resource v1 rejects a quant check");
    assert!(error.to_string().contains("quantitative"), "{error}");
    // The selector-based `quant normal` envelope keeps its meaning.
    let parsed = parse_source_program(
        "module contract;\nextern fn f() -> unit\ncontract {\n    quant normal credit >= 0;\n};\n",
    )
    .expect("structural parse");
    assert!(matches!(
        parsed.declarations[0]
            .value
            .function_contract
            .as_ref()
            .unwrap()
            .clauses[0]
            .value
            .kind,
        SourceContractClauseKind::Quant(_)
    ));
}
