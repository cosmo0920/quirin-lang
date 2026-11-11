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

//! Card NULL-DEREF-QTT-GROUNDING (USER DECISION 2026-10-11, "Two levels";
//! contract specification C09.3, grammar G05d): `quant deref #B;` elaborates
//! to `quant null-deref #B; quant checks B, Idx #i: #i == 0; quant live #B;`
//! before any consumer sees it. A shorthand, its decomposition and every mix
//! give one canonical clause set (and one canonical digest) while the pin
//! stays over the text as written; disagreeing parts, a repeated shorthand and
//! an unknown shorthand are refused by name; resource v1 refuses the new forms;
//! a liveness part passes an audit route only where custody's own declaration
//! states it.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::bounds_contract::{
    BoundsClause, BoundsContracts, BoundsFunctionContract, load_bounds_contract_sources,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_contract::deref_parts::{canonical_parts, check_live_backing, states_live};
use qcore_frontend::frontend::{SourceContractClauseKind, parse_source_program};

fn module(clauses: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "deref.qv".to_owned(),
        format!(
            "module contract;
extern fn take(p: ptr<mut, u8>, q: ptr<const, u8>) -> i32
contract {{
    requires bounds_contract_v1;
{clauses}
}};
"
        ),
    )])
}

fn load(clauses: &str) -> Result<BoundsFunctionContract, String> {
    let contracts = load_bounds_contract_sources(&module(clauses))?;
    Ok(contracts.own("take").expect("declared").clone())
}

fn rejected(clauses: &str, code: &str) {
    match load(clauses) {
        Ok(read) => panic!("`{clauses}` must be rejected ({code}), read {read:#?}"),
        Err(error) => assert!(error.contains(code), "`{clauses}`: {error}"),
    }
}

/// The canonical clause set as a set of clauses (order aside).
fn clause_set(contract: &BoundsFunctionContract) -> BTreeSet<String> {
    contract
        .clauses
        .iter()
        .map(|entry| format!("{:?} | {}", entry.clause, entry.text))
        .collect()
}

const SHORTHAND: &str = "    quant deref #p;";
const DECOMPOSITION: &str = "    quant null-deref #p;
    quant checks p, Idx #i: #i == 0;
    quant live #p;";

#[test]
fn the_shorthand_elaborates_to_its_three_canonical_parts() {
    let contract = load(SHORTHAND).expect("loads");
    let kinds = contract
        .clauses
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            "quant null-deref #p".to_owned(),
            "quant checks p, Idx #i: #i == 0".to_owned(),
            "quant live #p".to_owned(),
        ]
    );
    assert!(matches!(
        contract.clauses[0].clause,
        BoundsClause::NullDeref { .. }
    ));
    assert!(matches!(
        contract.clauses[1].clause,
        BoundsClause::QuantCheck { .. }
    ));
    assert!(matches!(
        contract.clauses[2].clause,
        BoundsClause::Live { .. }
    ));
}

#[test]
fn the_shorthand_the_decomposition_and_every_mix_are_one_canonical_set() {
    let shorthand = load(SHORTHAND).expect("shorthand");
    let mixes = [
        DECOMPOSITION,
        "    quant live #p;
    quant checks p, Idx #i: #i == 0;
    quant null-deref #p;",
        "    quant deref #p;
    quant null-deref #p;",
        "    quant null-deref #p;
    quant deref #p;",
        "    quant deref #p;
    quant checks p, Idx #i: #i == 0;
    quant live #p;",
        "    quant live #p;
    quant deref #p;
    quant null-deref #p;
    quant checks p, Idx #i: #i == 0;",
        // The bound variable's spelling is not meaning: merged into the
        // shorthand's canonical spelling.
        "    quant checks p, Idx #k: #k == 0;
    quant deref #p;",
    ];
    for mix in mixes {
        let contract = load(mix).unwrap_or_else(|error| panic!("`{mix}`: {error}"));
        assert_eq!(clause_set(&contract), clause_set(&shorthand), "`{mix}`");
        assert_eq!(
            canonical_parts(&contract.clauses),
            canonical_parts(&shorthand.clauses),
            "`{mix}`"
        );
        assert_eq!(
            contract.canonical_sha256, shorthand.canonical_sha256,
            "`{mix}`"
        );
        // The pin stays over the text as written: a different text needs a
        // re-pin even when it means the same.
        assert_ne!(contract.text_sha256, shorthand.text_sha256, "`{mix}`");
    }
    // Every consumer reads the same: the bounds view (the extent part only),
    // the null property's clauses (the existence part only).
    let one = load_bounds_contract_sources(&module(SHORTHAND)).expect("shorthand");
    let two = load_bounds_contract_sources(&module(DECOMPOSITION)).expect("decomposition");
    let view = |contracts: &BoundsContracts| {
        clause_set(contracts.for_bounds().own("take").expect("a bounds view"))
    };
    assert_eq!(view(&one), view(&two));
    assert_eq!(
        view(&one),
        BTreeSet::from([format!(
            "{:?} | quant checks p, Idx #i: #i == 0",
            load(SHORTHAND).expect("loads").clauses[1].clause
        )])
    );
    let nulls = |contracts: &BoundsContracts| {
        contracts
            .null_derefs()
            .map(|(name, contract)| {
                (
                    name.clone(),
                    contract
                        .clauses
                        .iter()
                        .filter(|e| matches!(e.clause, BoundsClause::NullDeref { .. }))
                        .map(|e| e.text.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(nulls(&one), nulls(&two));
}

#[test]
fn the_existing_null_deref_form_keeps_its_meaning_and_its_digest() {
    // `quant null-deref #B;` is the existence part alone, its pinned text line
    // unchanged by this card (the 13 corpus declarations keep their pins).
    let contract = load("    quant null-deref #p;").expect("loads");
    assert_eq!(contract.clauses.len(), 1);
    assert!(matches!(
        contract.clauses[0].clause,
        BoundsClause::NullDeref { .. }
    ));
    assert!(!states_live(
        &load_bounds_contract_sources(&module("    quant null-deref #p;")).unwrap()
    ));
}

#[test]
fn contradictory_or_repeated_parts_are_refused_by_name() {
    rejected(
        "    quant deref #p;
    quant checks p, Idx #i: #i >= 0 && #i < 4;",
        "quant-deref-conflict",
    );
    rejected(
        "    quant checks p, Idx #i: #i == 1;
    quant deref #p;",
        "quant-deref-conflict",
    );
    rejected(
        "    quant deref #p;
    requires bounds_extent(p, 4, elements);",
        "quant-deref-conflict",
    );
    rejected(
        "    quant deref #p;
    quant deref #p;",
        "duplicate bounds clause for deref p",
    );
    // Two written parts with one key keep the loader's duplicate rule.
    rejected(
        "    quant live #p;
    quant live #p;",
        "duplicate bounds clause for live p",
    );
    rejected(
        "    quant null-deref #p;
    quant null-deref #p;",
        "duplicate bounds clause for null-deref p",
    );
    // Shapes and subjects.
    rejected("    quant deref p;", "quant-deref-shape");
    rejected("    quant live #p, #q;", "quant-live-shape");
    rejected("    quant deref #r;", "quant-deref-parameter");
    rejected("    quant live #r;", "quant-live-parameter");
    // Different pointers do not overlap.
    let both = load(
        "    quant deref #p;
    quant null-deref #q;",
    )
    .expect("loads");
    assert_eq!(both.clauses.len(), 4);
}

#[test]
fn an_unknown_shorthand_is_refused() {
    let source = "module contract;
extern fn take(p: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    quant nonnull #p;
};
";
    let error = parse_source_program(source).expect_err("unknown shorthand");
    assert!(
        error.to_string().contains("quant-shorthand-unknown"),
        "{error}"
    );
    let error = load_bounds_contract_sources(&BTreeMap::from([(
        "unknown.qv".to_owned(),
        source.to_owned(),
    )]))
    .expect_err("unknown shorthand");
    assert!(error.contains("quant-shorthand-unknown"), "{error}");
}

#[test]
fn the_new_forms_parse_as_their_own_clause_kinds() {
    let program = parse_source_program(
        "module contract;
extern fn take(p: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    quant deref #p;
    quant live #p;
};
",
    )
    .expect("parses");
    let kinds = program.declarations[0]
        .value
        .function_contract
        .as_ref()
        .expect("contract")
        .clauses
        .iter()
        .map(|clause| clause.value.kind.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            SourceContractClauseKind::Requires,
            SourceContractClauseKind::QuantDeref,
            SourceContractClauseKind::QuantLive,
        ]
    );
}

#[test]
fn resource_v1_refuses_the_new_forms() {
    for clause in ["quant deref #p;", "quant live #p;"] {
        let source = format!(
            "module contract;
extern fn take(p: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    {clause}
}};
"
        );
        let error = load_resource_contract_sources(&BTreeMap::from([("r.qv".to_owned(), source)]))
            .expect_err("resource v1 refuses a part it cannot read");
        assert!(error.contains("quant"), "{clause}: {error}");
    }
}

fn resource(mode: &str) -> qcore_schemas::contract::ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "r.qv".to_owned(),
        format!(
            "module contract;
extern fn take(p: ptr<mut, u8>, q: ptr<const, u8>) -> i32
contract {{
    requires resource_contract_v1;
    ensures normal {mode};
    ensures normal resource_borrowed(q);
    ensures normal resource_return_none();
}};
"
        ),
    )]))
    .expect("resource fixture")
}

#[test]
fn a_liveness_part_passes_only_where_custody_states_it() {
    let bounds = load_bounds_contract_sources(&module(SHORTHAND)).expect("loads");
    assert!(states_live(&bounds));
    let backed = check_live_backing(&bounds, &resource("resource_owned(p, nonnull)"))
        .expect("backed by resource_owned");
    assert_eq!(backed.len(), 1, "{backed:?}");
    let error = check_live_backing(&bounds, &resource("resource_borrowed(p)"))
        .expect_err("a borrow is not a liveness obligation");
    assert!(error.contains("quant-live-contradicts-custody"), "{error}");
    let error = check_live_backing(
        &bounds,
        &qcore_schemas::contract::ResourceContracts::default(),
    )
    .expect_err("no custody declaration");
    assert!(error.contains("quant-live-unbacked"), "{error}");
    // An environment without a liveness part needs no backing.
    let plain = load_bounds_contract_sources(&module("    quant null-deref #p;")).unwrap();
    assert_eq!(
        check_live_backing(
            &plain,
            &qcore_schemas::contract::ResourceContracts::default()
        ),
        Ok(Vec::new())
    );
}
