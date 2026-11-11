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

//! Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (rule 12q): a consultation's
//! parity decisions are its own. Two consultations driven in sequence on one
//! thread: the second's record holds exactly what it holds when it runs
//! alone, whatever the first consultation decided and whether or not the
//! first was recorded. Before the card, decisions went into a thread-local
//! list that only a matched recording consultation cleared, so the second
//! consultation's row carried the first's leftovers
//! (`docs/reports/gapo-parity-decisions-nondeterministic/`).

use super::*;
use crate::legacy_c::{CCompilerDialect, tokenize};
use crate::typed_access_path::{StatementIndex, enter_source};

/// One function whose statements make each kind of decision a typed
/// access-path reader records: rule 10t's tracked place (line 2), rule 12j's
/// record-object place (line 3) and rule 10v's pointer-store classification
/// (line 4).
const SOURCE: &str = "struct I { int x; }; struct S { struct I in; struct S *next; int v; };\n\
int f(struct S *s) { struct S o; int y = s->next->in.x;\n\
o.in.x = 3;\n\
s->next = s;\n\
return y + o.in.x; }";

fn texts(spelling: &str) -> Vec<String> {
    tokenize(spelling)
        .into_iter()
        .map(|token| token.text)
        .collect()
}

/// One recording consultation of `spelling` on `line`: its answer and the
/// place decisions and classifications its record holds.
fn consult(index: &StatementIndex, spelling: &str, line: usize) -> (bool, Vec<Value>, Vec<Value>) {
    let owned = texts(spelling);
    let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let decisions = Decisions::kept();
    let plan = index.statement_with(&texts, line, &|| None, |_| false, Some(&decisions));
    (
        plan.is_some(),
        decisions.places(),
        decisions.classifications(),
    )
}

/// Every consultation the scenario drives, in one order.
const CONSULTATIONS: &[(&str, usize)] = &[
    ("int y = s -> next -> in . x", 2),
    ("o . in . x = 3", 3),
    ("s -> next = s", 4),
    // No Clang statement carries these tokens on this line: the
    // consultation declines before its reader runs.
    ("s -> v = 9", 4),
];

#[test]
fn a_consultation_records_exactly_its_own_decisions_in_any_order() {
    if crate::clang_command().is_err() {
        return;
    }
    let _scope = enter_source(SOURCE, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    // Each consultation alone, on a fresh index each time.
    let alone = CONSULTATIONS
        .iter()
        .map(|(spelling, line)| {
            let fresh = StatementIndex::read("f").expect("typed function");
            consult(&fresh, spelling, *line)
        })
        .collect::<Vec<_>>();
    // The scenario makes every kind of decision, and the declined
    // consultation makes none.
    assert!(!alone[0].1.is_empty(), "rule 10t place: {:?}", alone[0]);
    assert!(
        alone[1].1.iter().any(|place| place["rule"] == "12j"),
        "rule 12j place: {:?}",
        alone[1]
    );
    assert!(
        !alone[2].2.is_empty(),
        "rule 10v classification: {:?}",
        alone[2]
    );
    assert_eq!(alone[3], (false, Vec::new(), Vec::new()));
    // Forward, backward and with every earlier consultation unrecorded: the
    // same records.
    for order in [[0, 1, 2, 3], [3, 2, 1, 0], [2, 0, 3, 1]] {
        for &at in &order {
            let (spelling, line) = CONSULTATIONS[at];
            assert_eq!(
                consult(&index, spelling, line),
                alone[at],
                "{spelling} in {order:?}"
            );
        }
    }
    for at in 0..CONSULTATIONS.len() {
        // Every other consultation first, through a reader that records
        // nothing (the consumers of rules 10s, 11f and the residual probes).
        for (other, (spelling, line)) in CONSULTATIONS.iter().enumerate() {
            if other != at {
                let owned = texts(spelling);
                let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
                let _ = index.statement_with(&texts, *line, &|| None, |_| false, None);
            }
        }
        let (spelling, line) = CONSULTATIONS[at];
        assert_eq!(
            consult(&index, spelling, line),
            alone[at],
            "{spelling} after the others"
        );
    }
}

/// The finding's own shape: a consultation whose node does not match, after
/// a reader whose consumer recorded nothing, writes no `typed-place` row.
#[test]
fn a_declined_consultation_after_an_unrecorded_reader_holds_nothing() {
    if crate::clang_command().is_err() {
        return;
    }
    let _scope = enter_source(SOURCE, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let owned = texts("s -> next -> in . x");
    let operand = owned.iter().map(String::as_str).collect::<Vec<_>>();
    // A position reader that records nothing decides the tracked place.
    let unrecorded = index.condition(&operand, 2, |_| false, None);
    let recorded = Decisions::kept();
    let _ = index.condition(&operand, 2, |_| false, Some(&recorded));
    assert!(unrecorded.is_some() && !recorded.places().is_empty());
    // The next consultation declines (no node) and its record is empty.
    assert_eq!(
        consult(&index, "s -> v = 9", 4),
        (false, Vec::new(), Vec::new())
    );
}

/// The position consultations (rule 10k) record into the record they are
/// handed, and keep nothing without one.
#[test]
fn position_consultations_record_into_their_own_record() {
    if crate::clang_command().is_err() {
        return;
    }
    let _scope = enter_source(SOURCE, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let owned = texts("s -> next -> in . x");
    let operand = owned.iter().map(String::as_str).collect::<Vec<_>>();
    let condition = Decisions::kept();
    assert!(
        index
            .condition(&operand, 2, |_| false, Some(&condition))
            .is_some()
    );
    let returned = Decisions::kept();
    let _ = index.returned(&operand, 2, |_| false, |_| true, Some(&returned));
    assert_eq!(condition.places(), returned.places());
    assert!(!condition.places().is_empty());
}

/// Rule 12q: the value-connective edge, edge root and deciding operand are
/// restored when the lowering they scope unwinds.
#[test]
fn edge_scopes_are_restored_on_unwind() {
    let region = serde_json::json!({"kind": "ConditionalOperator", "inner": [
        {"kind": "IntegerLiteral"}, {"kind": "IntegerLiteral"}, {"kind": "IntegerLiteral"}
    ]});
    let probe = |region: &Value| crate::value_connective::edge_node(region).is_some();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        crate::value_connective::on_edge(&[(&region, Some(1))], || {
            assert!(probe(&region));
            crate::value_connective::with_edge_root(region.clone(), || {
                assert!(crate::value_connective::edge_root().is_some());
                crate::value_connective::deciding(&region, || {
                    assert!(
                        crate::value_connective::deciding_candidate(&region, &[&region]).is_some()
                    );
                    panic!("the lowering unwinds");
                })
            })
        })
    }));
    assert!(unwound.is_err());
    assert!(!probe(&region));
    assert!(crate::value_connective::edge_root().is_none());
    assert!(crate::value_connective::deciding_candidate(&region, &[&region]).is_none());
}
