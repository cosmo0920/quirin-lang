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

//! Card CG06-VALUE-CONNECTIVE-RESIDUALS, sibling-effect part (rule 11m): a
//! value region composed with its siblings by CG05's footprints, read from
//! Clang over the fixture contracts, and the mechanical binding to
//! `proof/QCore/ValueConnectives.lean`.

use std::collections::BTreeMap;

use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};

use super::*;
use crate::legacy_c::CCompilerDialect;
use crate::nested_call_effects::CallFacts;
use crate::typed_access_path::{FieldFacts, StatementIndex, enter_source};
use crate::value_connective::{Reading, Refusal, regions};

const LEAN: &str = include_str!("../../../../proof/QCore/ValueConnectives.lean");

/// The fixture contracts the vectors read, as the application point would
/// apply them (the CG00 `q_*`, `r_*` and `cg_*` families; `cg_count` takes a
/// scalar's address and borrows it).
const DECLARATIONS: &str = "void *q_alloc(void); void q_free(void *p); \
int r_release_code(void *p); int r_consume_code(void *p); int r_borrow_code(void *p); void *r_wrap(void *p); \
void cg_log(const char *format, ...); int cg_count(int *n); int opaque(void);";

struct Fixture<'a> {
    fields: &'a BTreeMap<String, Option<FieldFacts>>,
}

impl Facts for Fixture<'_> {
    fn contract(&self, callee: &str) -> Option<CallFacts> {
        use ResourceParameterMode::{Borrowed, Consumed, Released};
        let facts = |modes: Vec<ResourceParameterMode>, result, tail| CallFacts {
            modes,
            result,
            tail,
            container: false,
        };
        let owned = ResourceReturnMode::Owned { nullable: false };
        let none = ResourceReturnMode::None;
        Some(match callee {
            "q_alloc" => facts(vec![], owned, false),
            "q_free" | "r_release_code" => facts(vec![Released { nullable: false }], none, false),
            "r_consume_code" => facts(vec![Consumed { nullable: false }], none, false),
            "r_borrow_code" | "cg_count" => facts(vec![Borrowed], none, false),
            "r_wrap" => facts(vec![Borrowed], owned, false),
            "cg_log" => facts(vec![Borrowed], none, true),
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

/// The value regions of `subject`'s last statement, read with the
/// composition over the fixture contracts.
fn read(body: &str) -> Result<usize, Refusal> {
    let source = format!("{DECLARATIONS}\nvoid subject(void *p) {{ {body} }}\n");
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("subject").expect("typed function");
    let fixture = Fixture {
        fields: &index.fields,
    };
    let body = children(&index.function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
        .expect("a body");
    let last = children(body).last().expect("a statement");
    let compose = |region: &Value, siblings: &[&Value]| compose(region, siblings, &fixture);
    regions(
        last,
        &Reading {
            locate: &|_| Some((0, 0)),
            unaliased: &|_| false,
            compose: Some(&compose),
        },
    )
    .map(|regions| regions.len())
}

fn admitted(body: &str) -> bool {
    read(body).is_ok()
}

#[test]
fn every_footprint_vector_is_decided_as_pinned() {
    if crate::clang_command().is_err() {
        return;
    }
    for (body, expected) in FOOTPRINT_VECTORS {
        assert_eq!(admitted(body), *expected, "{body}: {:?}", read(body));
    }
}

#[test]
fn each_refusal_names_its_class() {
    if crate::clang_command().is_err() {
        return;
    }
    let composed = |body: &str| match read(body) {
        Err(Refusal::Composition(premise, composed)) => {
            assert_eq!(*premise, Refusal::SiblingEffect, "{body}");
            composed
        }
        other => panic!("{body}: {other:?}"),
    };
    assert!(matches!(
        composed("cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, r_release_code(p));"),
        Composed::Conflict(_)
    ));
    assert_eq!(
        composed("cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, opaque());"),
        Composed::UncontractedSibling("opaque".into())
    );
    assert_eq!(
        composed("int n = 0; cg_log(\"%d %d\", p ? 1 : 0, n++);"),
        Composed::NonCallSibling
    );
    assert_eq!(
        composed("int n = 0; cg_log(\"%d %d\", n ? 1 : 0, cg_count(&n));"),
        Composed::AddressedDecision("n".into())
    );
    // An arm CG05's reading refuses (an uncontracted callee in the region).
    assert!(matches!(
        composed("cg_log(\"%d %d\", p ? opaque() : 0, r_borrow_code(p));"),
        Composed::UnreadRegion(_)
    ));
    // An effect that is not a call inside the region.
    assert_eq!(
        composed("int n = 0; cg_log(\"%d %d\", p ? n++ : 0, r_borrow_code(p));"),
        Composed::NonCallRegion
    );
}

/// Rule 10z's own admissions are untouched: a region whose siblings are free
/// of effects is never composed.
#[test]
fn an_effect_free_sibling_is_never_composed() {
    if crate::clang_command().is_err() {
        return;
    }
    assert_eq!(
        read("cg_log(\"%d %d\", p ? r_borrow_code(p) : 0, 1);"),
        Ok(1)
    );
    // A deciding call beside an inert sibling is rule 10z's.
    assert_eq!(read("int n = 0; n = r_borrow_code(p) ? 1 : 2;"), Ok(1));
}

/// The two arms are composed in two graphs: a release on one arm and a
/// consumption of the same identity on the other never pair (in one graph
/// they would not commute), and each arm is checked against the siblings.
#[test]
fn the_arms_are_composed_apart() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = format!(
        "{DECLARATIONS}\nvoid subject(void *p, int c) {{ int n = 0; cg_log(\"%d %d\", c ? r_release_code(p) : r_consume_code(p), cg_count(&n)); }}\n"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("subject").expect("typed function");
    let fixture = Fixture {
        fields: &index.fields,
    };
    let body = children(&index.function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
        .expect("a body");
    let call = children(body).last().expect("a statement");
    // `cg_log`'s actuals: the format, the region, the sibling.
    let actuals = &children(call)[1..];
    let region = strip_parens(&actuals[1]);
    let region = if kind(region) == "ConditionalOperator" {
        region
    } else {
        children(region).first().expect("the region")
    };
    let siblings = [&children(call)[0], &actuals[0], &actuals[2]];
    assert_eq!(compose(region, &siblings, &fixture), Ok(()));
    // One arm's graph: the deciding read, `Branch`, the arm's call, `Merge`,
    // and the sibling's read and call; the arm sequenced after the branch.
    let mut operands = Operands::new(&fixture);
    let (decision, [Some(arm), _]) = parts(region).expect("parts") else {
        panic!("a select");
    };
    let decided = operands.operand(decision).expect("read");
    let branch = operands.control(Event::Branch);
    operands.sequence(&decided, branch);
    let evaluated = operands.operand(arm).expect("read");
    for &event in &evaluated {
        operands.sequence(&[branch], event);
    }
    let sibling = operands.operand(siblings[2]).expect("read");
    let graph = operands.graph();
    assert!(
        evaluated
            .iter()
            .all(|&event| graph.sequenced(branch, event))
    );
    assert!(
        sibling
            .iter()
            .all(|&event| !graph.sequenced(branch, event) && !graph.sequenced(event, branch))
    );
    assert_eq!(operands.check(), Ok(()));
}

/// The mechanical binding: `footprintVectors` = `FOOTPRINT_VECTORS`.
#[test]
fn the_lean_footprint_vectors_match() {
    let body = LEAN
        .split("def footprintVectors : List (String × Bool) := [")
        .nth(1)
        .expect("footprintVectors");
    let body = &body[..body.find("\n]").expect("end")];
    let rows = body
        .split("\n  (\"")
        .skip(1)
        .map(|row| {
            let (source, rest) = row.rsplit_once("\",").expect("a row");
            (source.replace("\\\"", "\""), rest.contains("true"))
        })
        .collect::<Vec<_>>();
    let rust = FOOTPRINT_VECTORS
        .iter()
        .map(|(source, admitted)| ((*source).to_owned(), *admitted))
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}
