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

//! Card CG05-NESTED-CALL-EFFECTS (rule 10y): the footprint cells, the call
//! tree read from Clang, the order check, and their mechanical binding to
//! `proof/QCore/NestedCallEffects.lean`.

use std::collections::BTreeMap;

use super::*;
use crate::elaboration::cells_overlap;
use crate::legacy_c::CCompilerDialect;
use crate::typed_access_path::{StatementIndex, enter_source};

fn cell(code: &str) -> Cell {
    match code.split_once(':') {
        Some(("object", decl)) => Cell::Object(decl.to_owned()),
        Some(("identity", name)) => Cell::Identity(name.to_owned()),
        Some(("fresh", event)) => Cell::Fresh(event.parse().expect("an event id")),
        _ => panic!("unknown cell {code}"),
    }
}

/// `u` is an automatic object whose address is never taken, `a` one whose
/// address is taken.
fn unaliased(decl: &str) -> bool {
    decl == "u"
}

#[test]
fn the_overlap_vectors_hold_both_ways() {
    for (one, other, expected) in OVERLAP_VECTORS {
        assert_eq!(
            cells_overlap(&cell(one), &cell(other), &unaliased),
            *expected,
            "{one} {other}"
        );
        assert_eq!(
            cells_overlap(&cell(other), &cell(one), &unaliased),
            *expected,
            "{other} {one} (symmetric)"
        );
    }
}

/// The fixture contracts the order vectors read, as the application point
/// would apply them (the `q_*`, `r_*` and `cg_*` families of the CG00
/// fixtures, `cg_pair` added by this card).
const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *p); \
int r_release_code(void *p); int r_borrow_code(void *p); void *r_wrap(void *p); \
void cg_sink(int value); void cg_sink2(int left, int right); void cg_pair(void *p, int value); \
int opaque(void *p);";

struct Fixture<'a> {
    fields: &'a BTreeMap<String, Option<FieldFacts>>,
}

impl Facts for Fixture<'_> {
    fn contract(&self, callee: &str) -> Option<CallFacts> {
        use ResourceParameterMode::{Borrowed, Released};
        let facts = |modes: Vec<ResourceParameterMode>, result| CallFacts {
            modes,
            result,
            tail: false,
            container: false,
        };
        let owned = ResourceReturnMode::Owned { nullable: false };
        let none = ResourceReturnMode::None;
        Some(match callee {
            "q_alloc" => facts(vec![], owned),
            "q_free" | "r_release_code" => facts(vec![Released { nullable: false }], none),
            "r_borrow_code" | "cg_sink" => facts(vec![Borrowed], none),
            "r_wrap" => facts(vec![Borrowed], owned),
            "cg_sink2" | "cg_pair" => facts(vec![Borrowed, Borrowed], none),
            _ => return None,
        })
    }

    fn unaliased(&self, _decl: &str) -> bool {
        true
    }

    fn fields(&self) -> &BTreeMap<String, Option<FieldFacts>> {
        self.fields
    }
}

/// The last statement of `subject`'s body.
fn last_statement(index: &StatementIndex) -> &Value {
    let body = children(&index.function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
        .expect("a body");
    children(body).last().expect("a statement")
}

fn admitted(body: &str) -> Result<usize, Refusal> {
    let source = format!("{DECLARATIONS}\nvoid subject(void) {{ {body} }}\n");
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("subject").expect("typed function");
    let fixture = Fixture {
        fields: &index.fields,
    };
    read(last_statement(&index), &fixture, ResultUse::Discarded).map(|reading| reading.root.depth())
}

#[test]
fn every_order_vector_is_decided_as_pinned() {
    if crate::clang_command().is_err() {
        return;
    }
    for (body, expected) in ORDER_VECTORS {
        let outcome = admitted(body);
        assert_eq!(outcome.is_ok(), *expected, "{body}: {outcome:?}");
    }
}

#[test]
fn the_refusal_controls_name_their_reason() {
    if crate::clang_command().is_err() {
        return;
    }
    // `f(free_and_status(p), use(p))`: refused whichever order C picks.
    let refusal = admitted("void *p = q_alloc(); cg_sink2(r_release_code(p), r_borrow_code(p));")
        .expect_err("refused");
    assert!(
        matches!(
            refusal,
            Refusal::Order(OrderRefusal::CallFootprintConflict(..))
        ),
        "{refusal:?}"
    );
    // `h(p, g(p))` with `g` releasing `p`: the read of `p` is unordered with
    // the release.
    let refusal =
        admitted("void *p = q_alloc(); cg_pair(p, r_release_code(p));").expect_err("refused");
    assert!(
        matches!(
            refusal,
            Refusal::Order(OrderRefusal::CallFootprintConflict(..))
        ),
        "{refusal:?}"
    );
    // An uncontracted inner callee.
    assert_eq!(
        admitted("void *p = q_alloc(); cg_sink(opaque(p));"),
        Err(Refusal::Uncontracted("opaque".into()))
    );
    // Data at a custody formal.
    assert_eq!(
        admitted("void *p = q_alloc(); q_free((void *)(long)r_borrow_code(p));"),
        Err(Refusal::DataAtCustody("q_free".into()))
    );
    // No nested call.
    assert_eq!(
        admitted("void *p = q_alloc(); q_free(p);"),
        Err(Refusal::NotNested)
    );
}

#[test]
fn depth_is_read_inside_out() {
    if crate::clang_command().is_err() {
        return;
    }
    assert_eq!(
        admitted("void *p = q_alloc(); cg_sink(r_borrow_code(r_wrap(r_wrap(p)))); q_free(p);")
            .map_err(|refusal| refusal.reason()),
        Err("no nested call".to_owned()),
        "the last statement is `q_free(p)`"
    );
    assert_eq!(
        admitted("void *p = q_alloc(); cg_sink(r_borrow_code(r_wrap(r_wrap(p))));"),
        Ok(4)
    );
    let source = format!(
        "{DECLARATIONS}\nvoid subject(void) {{ void *p = q_alloc(); cg_sink(r_borrow_code(r_wrap(p)) + 1); }}\n"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("subject").expect("typed function");
    let fixture = Fixture {
        fields: &index.fields,
    };
    let reading = read(last_statement(&index), &fixture, ResultUse::Discarded).expect("admitted");
    let order = reading
        .root
        .post_order()
        .iter()
        .map(|call| (call.callee.clone(), call.use_))
        .collect::<Vec<_>>();
    assert_eq!(
        order,
        [
            ("r_wrap".to_owned(), Some(ResultUse::Passed)),
            ("r_borrow_code".to_owned(), Some(ResultUse::Tested)),
            ("cg_sink".to_owned(), Some(ResultUse::Discarded)),
        ]
    );
    // Each call is sequenced after every event of its actuals.
    for call in reading.root.post_order() {
        for inner in call.post_order() {
            if inner.event != call.event {
                assert!(reading.graph.sequenced(inner.event, call.event));
            }
        }
    }
    assert_eq!(reading.places.len(), 1);
}

const LEAN: &str = include_str!("../../../../proof/QCore/NestedCallEffects.lean");

fn lean_rows(header: &str) -> Vec<&'static str> {
    let table = LEAN.split(header).nth(1).expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .collect()
}

fn lean_cell(text: &str) -> String {
    let words = text.split_whitespace().collect::<Vec<_>>();
    match words.as_slice() {
        [".object", "0"] => "object:u".into(),
        [".object", "1"] => "object:a".into(),
        [".identity", "0"] => "identity:p".into(),
        [".identity", "1"] => "identity:q".into(),
        [".fresh", k] => format!("fresh:{k}"),
        other => panic!("unknown Lean cell {other:?}"),
    }
}

/// The mechanical binding of `QCore.NestedCallEffects.overlapVectors`.
#[test]
fn cg05_lean_overlap_vectors_match_the_rust_table() {
    let rows = lean_rows("def overlapVectors : List (Cell × Cell × Bool) := [");
    assert_eq!(rows.len(), OVERLAP_VECTORS.len());
    for (row, (one, other, expected)) in rows.iter().zip(OVERLAP_VECTORS) {
        let fields = row
            .trim_end_matches(',')
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        assert_eq!(lean_cell(fields[0]), *one, "{row}");
        assert_eq!(lean_cell(fields[1]), *other, "{row}");
        assert_eq!(fields[2], expected.to_string(), "{row}");
    }
}

/// The mechanical binding of `QCore.NestedCallEffects.orderVectors`: each C
/// statement and its admission.
#[test]
fn cg05_lean_order_vectors_match_the_rust_table() {
    let rows = lean_rows("def orderVectors : List (String × Bool) := [");
    assert_eq!(rows.len(), ORDER_VECTORS.len());
    for (row, (body, expected)) in rows.iter().zip(ORDER_VECTORS) {
        let rest = row.strip_prefix("(\"").expect("a source");
        let end = rest.find('"').expect("closing quote");
        assert_eq!(&rest[..end], *body, "{row}");
        let decision = row
            .trim_end_matches(',')
            .trim_end_matches(')')
            .rsplit(',')
            .next()
            .expect("a decision")
            .trim();
        assert_eq!(decision, expected.to_string(), "{row}");
    }
}
