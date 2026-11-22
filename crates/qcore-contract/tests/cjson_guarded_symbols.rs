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

//! Card **SP6-GUARDED-SYMBOL-PROSE** (contract specification C04.18.6,
//! C04.18.9's SP7 block): the 17 guarded symbols of
//! `cJSON_DetachItemViaPointer` state formulas, the guarded document its
//! unquoted clauses, and the string forms are refused (`QF003`) on every
//! route.

#[path = "../../../tests/support/legacy_prose_premise.rs"]
mod legacy_prose_premise;

use qcore_contract::{
    guarded_decomposition::{self as guarded, Inline},
    guarded_metadata_bridge as bridge, hypothesis,
};
use qcore_frontend::premise_formula::{
    GUARDED_DECISION_VECTORS, PROSE_GUARDED, RENDERED_TERMS, guarded_proposition,
};
use qcore_frontend::quant_proposition::lean;
use qcore_schemas::premise_flow::View;
use std::collections::BTreeMap;

const MODULE: &str = "providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv";
const CJSON: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
const LEAN: &str = include_str!("../../../proof/QCore/PremiseFormula/CjsonGuardedSymbols.lean");
const TYPED_LEAN: &str = include_str!("../../../proof/QCore/PremiseFormula/CjsonTypedAtoms.lean");
const QGD: &str =
    include_str!("../../../docs/reports/premise-guarded-decomposition/applications/cjson.qgd");
const DECLARATION: &str = "cJSON_DetachItemViaPointer";

/// Each symbol and the Lean `def` its formula is: the nine prerequisites
/// are SP6's typed atoms (`QCore.CjsonTypedAtoms`), the eight others this
/// card's (`QCore.CjsonGuardedSymbols`).
const LEAN_NAMES: [(&str, &str, bool); 17] = [
    ("detach", "detachSymbol", false),
    ("detach_chain", "chainAtom", true),
    ("detach_distinct", "distinctAtom", true),
    ("detach_empty", "emptyAtom", true),
    ("detach_first_prev", "firstPrevAtom", true),
    ("detach_last_next", "lastNextAtom", true),
    ("detach_link_targets", "linkTargetsAtom", true),
    ("detach_no_writer", "noWriterAtom", true),
    ("detach_parent_outside", "parentOutsideAtom", true),
    ("detach_predecessor", "predecessorAtom", true),
    ("environment", "environmentSymbol", false),
    ("failure_case", "failureCaseSymbol", false),
    ("invalid_membership", "invalidMembershipSymbol", false),
    ("membership", "membershipSymbol", false),
    (
        "original_list_unchanged",
        "originalListUnchangedSymbol",
        false,
    ),
    ("remaining_list_valid", "remainingListValidSymbol", false),
    ("success_case", "successCaseSymbol", false),
];

fn sources(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(MODULE.to_owned(), text.to_owned())])
}

fn graph() -> qcore_schemas::hypothesis::Graph {
    hypothesis::replay(&sources(CJSON))
        .unwrap()
        .remove(DECLARATION)
        .unwrap()
}

fn inline() -> Inline {
    let parsed = qcore_frontend::frontend::parse_source_program(CJSON).unwrap();
    let decl = parsed
        .declarations
        .iter()
        .find(|d| d.value.name == DECLARATION)
        .unwrap();
    guarded::declaration(&decl.value, &graph())
        .unwrap()
        .unwrap()
}

#[test]
fn every_guarded_symbol_states_a_formula_of_the_typed_rendering() {
    let inline = inline();
    assert_eq!(inline.bindings.len(), 17);
    let recorded = legacy_prose_premise::guarded_symbols();
    assert_eq!(recorded.len(), 17);
    for (id, binding) in &inline.bindings {
        let d = guarded_proposition(id, &binding.proposition).unwrap();
        for p in &d.parameters {
            assert!(
                qcore_frontend::premise_formula::term(&p.name).is_some()
                    || RENDERED_TERMS.iter().any(|t| t.name == p.name),
                "{id}: #{}",
                p.name
            );
        }
        let r = recorded.iter().find(|r| r.id == *id).unwrap();
        assert_eq!(binding.proposition, r.formula, "{id}");
        assert_eq!(format!("{:?}", binding.role).to_lowercase(), r.role);
    }
    // No string text, no string document.
    for line in CJSON.lines() {
        let line = line.trim_start();
        if line.starts_with("requires resource_guarded_") {
            assert!(!line.contains('"'), "{line}");
        }
    }
    // The eight symbols that were English, now formulas.
    let changed = recorded.iter().filter(|r| r.prose != r.formula).count();
    assert_eq!(changed, 8);
}

#[test]
fn the_canonical_printing_is_the_typed_atoms_spelling_and_reads_back() {
    use qcore_frontend::quant_proposition::{parse_formula_tokens, print};
    let tokens = |text: &str| {
        qcore_frontend::lexer::lex(text)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect::<Vec<_>>()
    };
    // Each typed atom's source formula is its own canonical printing.
    let graph = graph();
    let parsed = qcore_frontend::frontend::parse_source_program(CJSON).unwrap();
    let decl = parsed
        .declarations
        .iter()
        .find(|d| d.value.name == DECLARATION)
        .unwrap();
    let mut atoms = 0;
    for clause in decl.value.function_contract.iter().flat_map(|c| &c.clauses) {
        if clause.value.kind == qcore_frontend::frontend::SourceContractClauseKind::QuantProposition
        {
            let formula = clause.value.expression_source.split_once(") = ").unwrap().1;
            let id = clause.value.expression_source[1..]
                .split_once('(')
                .unwrap()
                .0;
            let qcore_schemas::hypothesis::Proposition::Formula { definition } =
                &graph.atoms[id].proposition
            else {
                panic!("{id}");
            };
            assert_eq!(print(&definition.formula), formula, "{id}");
            atoms += 1;
        }
    }
    assert_eq!(atoms, 9);
    // Every guarded proposition reads back as itself; another spelling of
    // the same tree prints the same.
    for binding in inline().bindings.values() {
        let f = parse_formula_tokens(&tokens(&binding.proposition)).unwrap();
        assert_eq!(print(&f), binding.proposition);
    }
    let spaced = parse_formula_tokens(&tokens("( #item_null( )  &&  ! #parent_null() )")).unwrap();
    assert_eq!(print(&spaced), "(#item_null() && !#parent_null())");
}

#[test]
fn the_symbols_are_the_lean_definitions() {
    let inline = inline();
    for (id, name, typed) in LEAN_NAMES {
        let d = guarded_proposition(id, &inline.bindings[id].proposition).unwrap();
        let line = format!("def {name} : Expr := {}", lean(&d.formula));
        let module = if typed { TYPED_LEAN } else { LEAN };
        assert!(
            module.lines().any(|l| l == line),
            "{id}: the Lean module has no `{line}`"
        );
    }
}

/// Prints the Lean `def` lines of the eight new symbols (`--ignored
/// --nocapture`), the text `the_symbols_are_the_lean_definitions` binds.
#[test]
#[ignore]
fn print_the_lean_definitions() {
    let inline = inline();
    for (id, name, typed) in LEAN_NAMES {
        if !typed {
            let d = guarded_proposition(id, &inline.bindings[id].proposition).unwrap();
            println!("def {name} : Expr := {}", lean(&d.formula));
        }
    }
}

#[test]
fn the_sp7_block_is_new_typed_and_bound_to_lean() {
    let sp7: Vec<_> = RENDERED_TERMS.iter().filter(|t| t.block == "SP7").collect();
    assert_eq!(sp7.len(), 20);
    for t in &sp7 {
        assert!(
            qcore_frontend::premise_formula::term(t.name).is_none(),
            "#{} is already a registered term",
            t.name
        );
        assert!(!t.meaning.is_empty());
    }
    let names = sp7
        .iter()
        .map(|t| format!("\"{}\"", t.name))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def sp7Terms : List String := [{names}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
}

#[test]
fn the_readable_document_is_rendered_from_the_clauses_unchanged() {
    let inline = inline();
    assert_eq!(inline.entry, "detach_guarded");
    assert_eq!(inline.readable, QGD);
    // The bridge renders the module's guarded block exactly.
    let addition = bridge::render(&inline).unwrap();
    assert_eq!(CJSON.matches(&addition).count(), 1);
    assert!(addition.contains("(environment && detach_chain"));
    assert!(!addition.contains('∧'));
}

#[test]
fn the_guarded_decision_vectors_are_the_lean_table() {
    let rows = GUARDED_DECISION_VECTORS
        .iter()
        .map(|(clause, accepted)| {
            let decision =
                qcore_frontend::guarded_decomposition::parse_source_clause(clause).is_ok();
            assert_eq!(decision, *accepted, "{clause}");
            let statement = if clause.contains('"') {
                ".str".to_owned()
            } else {
                format!(".formula {accepted}")
            };
            format!("({statement}, {accepted})")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def guardedVectors : List (GuardedStatement × Bool) := [{rows}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    for (clause, accepted) in GUARDED_DECISION_VECTORS {
        if clause.contains('"') {
            let error =
                qcore_frontend::guarded_decomposition::parse_source_clause(clause).unwrap_err();
            assert!(error.starts_with(PROSE_GUARDED), "{clause}: {error}");
            assert!(!accepted);
        }
    }
}

#[test]
fn a_string_guarded_symbol_or_document_is_refused_by_name_on_every_route() {
    assert_eq!(PROSE_GUARDED, "QF003");
    let recorded = legacy_prose_premise::guarded_symbols();
    let mut cases = Vec::new();
    for r in recorded.iter().filter(|r| r.prose != r.formula) {
        // The retired English text, where the formula now stands.
        cases.push((
            true,
            CJSON.replacen(
                &format!(
                    "resource_guarded_symbol_v1({}, {}, {},",
                    r.id, r.role, r.formula
                ),
                &format!(
                    "resource_guarded_symbol_v1({}, {}, \"{}\",",
                    r.id, r.role, r.prose
                ),
                1,
            ),
        ));
    }
    // A quoted formula is a string.
    cases.push((
        false,
        CJSON.replacen("#source_corresponds()", "\"#source_corresponds()\"", 1),
    ));
    // The retired string document.
    let inline = inline();
    let source = qcore_frontend::guarded_decomposition::source_clauses(
        &qcore_frontend::guarded_decomposition::parse(&inline.readable).unwrap(),
    );
    cases.push((
        true,
        CJSON.replacen(&source, &format!("\"{}\"", inline.readable), 1),
    ));
    // Under another introducer too.
    cases.push((
        false,
        CJSON.replacen(
            "    requires resource_guarded_symbol_v1(detach, operation, #admits_removed(),",
            "    ensures normal resource_guarded_symbol_v1(detach, operation, \"Admission\",",
            1,
        ),
    ));
    assert_eq!(cases.len(), 11);
    for (liftable, text) in &cases {
        assert_ne!(text, CJSON);
        let parse = qcore_frontend::frontend::parse_source_program(text)
            .unwrap_err()
            .to_string();
        assert!(parse.contains("QF003"), "{parse}");
        let load = qcore_contract::contract_loader::load_resource_contract_sources(&sources(text))
            .unwrap_err()
            .to_string();
        assert!(load.contains("QF003"), "{load}");
        assert!(
            hypothesis::replay(&sources(text))
                .unwrap_err()
                .contains("QF003")
        );
        // ... and lifted only by the test-side legacy reader, which knows only
        // the recorded texts.
        if *liftable {
            assert_eq!(legacy_prose_premise::lift(text), CJSON);
        } else {
            assert_ne!(legacy_prose_premise::lift(text), CJSON);
        }
    }
    // A standalone binding holding English, or a noncanonical spelling, is
    // refused by the lowering and has no source form.
    let graph = graph();
    for (id, text) in [
        (
            "detach",
            "Admission of the selected call under its Removed contract",
        ),
        ("environment", "#source_corresponds( )"),
        ("environment", "\"#source_corresponds()\""),
    ] {
        let mut bindings = inline.bindings.clone();
        bindings.get_mut(id).unwrap().proposition = text.into();
        let error = guarded::replay(
            View::Resource,
            &sources(CJSON),
            MODULE,
            DECLARATION,
            &graph,
            &bindings,
            &inline.readable,
        )
        .unwrap_err();
        assert!(error.starts_with("QF003"), "{text}: {error}");
        let mut metadata = inline.clone();
        metadata.bindings = bindings;
        assert!(bridge::render(&metadata).unwrap_err().starts_with("QF003"));
    }
}

#[test]
fn the_symbols_stay_proposed_metadata() {
    let sources = sources(CJSON);
    let inline = inline();
    let lowered = guarded::replay_inline(
        View::Resource,
        &sources,
        MODULE,
        DECLARATION,
        &graph(),
        &inline,
    )
    .unwrap();
    let context = qcore_schemas::hypothesis::Context {
        artifact: "a".into(),
        property: "resource".into(),
        semantic_profile: "p".into(),
        scope: "s".into(),
        bounds: "b".into(),
        assumptions: "x".into(),
        source_identity: "i".into(),
        build_identity: "j".into(),
        evidence_version: "e".into(),
    };
    let status = guarded::evaluate_proposed(&lowered, &context, &context, &[]).unwrap();
    assert!(
        status
            .guards
            .iter()
            .chain(&status.alternatives)
            .chain(&status.terminal_postconditions)
            .all(|s| *s == guarded::Status::Open)
    );
}
