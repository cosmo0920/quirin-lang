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

//! Card CG05-ELABORATOR-CALL-FOOTPRINTS (gateway rule 11n): the elaborator's
//! event footprints and `EvalGraph::check_with_footprints`, bound to
//! `proof/QCore/ElaboratorCallFootprints.lean` by its `footprintVectors`, and
//! the footprint `nested_call_effects::call_footprint` reads for the fixture
//! contracts.

use std::collections::BTreeMap;

use crate::elaboration::{
    Cell, EvalGraph, Event, Footprint, Place, Refusal, elaborate, event_footprint,
};

const LEAN: &str = include_str!("../../../../proof/QCore/ElaboratorCallFootprints.lean");

fn lean_rows(header: &str) -> Vec<&'static str> {
    let table = LEAN.split(header).nth(1).expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .collect()
}

/// One Lean `EEv` as a Rust event and, for a call, its footprint (`None`
/// where the Lean event's footprint is `none`). `id` is the event's index.
fn lean_event(text: &str, id: usize) -> (Event, bool, Option<Footprint>) {
    let words = text
        .split(|c: char| c.is_whitespace() || c == '(' || c == ')')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let flag = |word: &str| match word {
        "true" => true,
        "false" => false,
        other => panic!("not a flag {other:?}"),
    };
    let object = |decl: &str| Place::Object {
        decl: decl.to_owned(),
        name: decl.to_owned(),
    };
    let path = |root: &str, indirect: &str| Place::Path {
        spelling: format!("p{root}"),
        root: root.to_owned(),
        indirect: flag(indirect),
    };
    match words.as_slice() {
        [".readObject", decl, scalar] => (Event::Read(object(decl)), flag(scalar), None),
        [".writeObject", decl, pointer] => (
            Event::Write {
                place: object(decl),
                pointer: flag(pointer),
            },
            false,
            None,
        ),
        [".readPath", root, indirect, scalar] => {
            (Event::Read(path(root, indirect)), flag(scalar), None)
        }
        [".writePath", root, indirect, pointer] => (
            Event::Write {
                place: path(root, indirect),
                pointer: flag(pointer),
            },
            false,
            None,
        ),
        [".call", "none"] => (Event::Call { callee: "f".into() }, false, None),
        [".call", "some", name] => {
            let footprint = match *name {
                "releaseP" => Footprint {
                    reads: vec![],
                    writes: vec![Cell::Identity("0".into())],
                },
                "borrowP" => Footprint {
                    reads: vec![Cell::Identity("0".into())],
                    writes: vec![],
                },
                "allocFresh" => Footprint {
                    reads: vec![],
                    writes: vec![Cell::Fresh(id)],
                },
                other => panic!("unknown footprint {other:?}"),
            };
            (
                Event::Call {
                    callee: (*name).into(),
                },
                false,
                Some(footprint),
            )
        }
        [".control"] => (Event::Branch, false, None),
        other => panic!("unknown Lean event {other:?}"),
    }
}

/// `check_with_footprints` on two unordered events, the elaborator's way:
/// every non-call event's footprint is `event_footprint`'s.
fn admitted(first: &str, second: &str) -> bool {
    let mut graph = EvalGraph::default();
    let mut footprints = BTreeMap::new();
    for (id, text) in [first, second].into_iter().enumerate() {
        let (event, scalar, call) = lean_event(text, id);
        let footprint = match &event {
            Event::Call { .. } => call,
            other => event_footprint(other, scalar),
        };
        if let Some(footprint) = footprint {
            footprints.insert(id, footprint);
        }
        graph.events.push(event);
    }
    graph.unsequenced.push(vec![vec![0], vec![1]]);
    let unaliased = |decl: &str| decl == "0";
    graph
        .check_with_footprints(&unaliased, &|_, _| false, &footprints)
        .is_ok()
}

/// The mechanical binding of `QCore.ElaboratorCallFootprints.footprintVectors`:
/// the Lean decision is the Rust check's, in both orders.
#[test]
fn cg05_lean_footprint_vectors_match_the_rust_check() {
    let rows = lean_rows("def footprintVectors : List (EEv × EEv × Bool) := [");
    assert_eq!(rows.len(), 16);
    for row in rows {
        let fields = row
            .trim_end_matches(',')
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split(", ")
            .map(str::trim)
            .collect::<Vec<_>>();
        let [first, second, expected] = fields.as_slice() else {
            panic!("{row}");
        };
        let expected = *expected == "true";
        assert_eq!(admitted(first, second), expected, "{row}");
        assert_eq!(admitted(second, first), expected, "{row} (reversed)");
    }
}

/// With an empty footprint map, `check_with_footprints` is `check_with`.
#[test]
fn an_empty_map_is_check_with() {
    let events = [
        Event::Read(Place::Object {
            decl: "0".into(),
            name: "x".into(),
        }),
        Event::Write {
            place: Place::Object {
                decl: "0".into(),
                name: "x".into(),
            },
            pointer: false,
        },
        Event::Call { callee: "f".into() },
        Event::Read(Place::Path {
            spelling: "s->a".into(),
            root: "1".into(),
            indirect: true,
        }),
        Event::Branch,
    ];
    let unaliased = |decl: &str| decl == "0";
    for a in &events {
        for b in &events {
            let graph = EvalGraph {
                events: vec![a.clone(), b.clone()],
                unsequenced: vec![vec![vec![0], vec![1]]],
                ..EvalGraph::default()
            };
            assert_eq!(
                graph.check_with_footprints(&unaliased, &|_, _| false, &BTreeMap::new()),
                graph.check_with(&unaliased, &|_, _| false),
                "{a:?} {b:?}"
            );
        }
    }
}

/// A call footprint read through Clang from the fixture contracts, and the
/// elaborator's use of it: `x += r_release_code(p)` is admitted with a
/// footprint context and refused without one.
#[test]
fn the_elaborator_reads_call_footprints_from_the_context() {
    use crate::test_support::clang_available;
    if !clang_available() {
        return;
    }
    let source = "int r_release_code(void *p); int r_borrow_code(void *p);\n\
                  void subject(void *p, int *q) { int x = 0; x += r_release_code(p); \
                  *q += r_release_code(q); x += r_release_code(p) + r_borrow_code(p); }";
    let unit = crate::legacy_c::C11TranslationUnit {
        source,
        ..Default::default()
    };
    let function =
        crate::typed_c_ast::function_ast(source, &unit, "subject").expect("subject's AST");
    let statements = crate::typed_c_ast::children(
        crate::typed_c_ast::children(&function)
            .iter()
            .find(|node| crate::typed_c_ast::kind(node) == "CompoundStmt")
            .expect("a body"),
    )
    .to_vec();
    let with = context::Footprints::new(&function, true);
    let without = context::Footprints::new(&function, false);
    // `x += r_release_code(p)`.
    assert!(elaborate(&statements[1], &with).is_ok());
    assert!(matches!(
        elaborate(&statements[1], &without),
        Err(Refusal::UnorderedCall(_))
    ));
    // `*q += r_release_code(q)`.
    assert!(matches!(
        elaborate(&statements[2], &with),
        Err(Refusal::CallFootprintConflict(..))
    ));
    // A release and a borrow of one identity, unordered.
    assert!(matches!(
        elaborate(&statements[3], &with),
        Err(Refusal::CallFootprintConflict(..))
    ));
}

mod context {
    use std::collections::{BTreeMap, BTreeSet};

    use qcore_schemas::contract::{ResourceParameterMode, ResourceReturnMode};
    use serde_json::Value;

    use crate::elaboration::{Context, Footprint, Refusal, strip_parens};
    use crate::nested_call_effects::{CallFacts, Facts, call_footprint};
    use crate::typed_access_path::FieldFacts;
    use crate::typed_c_ast::{children, kind};

    /// A route-free context whose two callees release and borrow their one
    /// pointer formal.
    pub(super) struct Footprints {
        automatic: BTreeSet<String>,
        taken: BTreeSet<String>,
        fields: BTreeMap<String, Option<FieldFacts>>,
        calls: bool,
    }

    impl Footprints {
        pub(super) fn new(function: &Value, calls: bool) -> Self {
            fn walk(node: &Value, automatic: &mut BTreeSet<String>, taken: &mut BTreeSet<String>) {
                match kind(node) {
                    "ParmVarDecl" | "VarDecl" if node["storageClass"].as_str().is_none() => {
                        automatic.insert(node["id"].as_str().unwrap().to_owned());
                    }
                    "UnaryOperator" if node["opcode"].as_str() == Some("&") => {
                        let operand = strip_parens(&children(node)[0]);
                        if let Some(id) = operand["referencedDecl"]["id"].as_str() {
                            taken.insert(id.to_owned());
                        }
                    }
                    _ => {}
                }
                for child in children(node) {
                    walk(child, automatic, taken);
                }
            }
            let mut automatic = BTreeSet::new();
            let mut taken = BTreeSet::new();
            walk(function, &mut automatic, &mut taken);
            Footprints {
                automatic,
                taken,
                fields: BTreeMap::new(),
                calls,
            }
        }
    }

    impl Facts for Footprints {
        fn contract(&self, callee: &str) -> Option<CallFacts> {
            let mode = match callee {
                "r_release_code" => ResourceParameterMode::Released { nullable: false },
                "r_borrow_code" => ResourceParameterMode::Borrowed,
                _ => return None,
            };
            Some(CallFacts {
                modes: vec![mode],
                result: ResourceReturnMode::None,
                tail: false,
                container: false,
            })
        }
        fn unaliased(&self, decl: &str) -> bool {
            self.automatic.contains(decl) && !self.taken.contains(decl)
        }
        fn fields(&self) -> &BTreeMap<String, Option<FieldFacts>> {
            &self.fields
        }
    }

    impl Context for Footprints {
        type Facts = ();
        fn scalar(&self, node: &Value) -> bool {
            crate::typed_access_path::is_scalar(node)
        }
        fn pointer(&self, node: &Value) -> bool {
            crate::typed_access_path::is_pointer(node)
        }
        fn place(&self, _node: &Value, _store: bool) -> Result<(), Refusal> {
            Ok(())
        }
        fn unaliased(&self, decl: &str) -> bool {
            Facts::unaliased(self, decl)
        }
        fn call_footprint(&self, call: &Value) -> Option<Footprint> {
            self.calls
                .then(|| call_footprint(call, self).ok())
                .flatten()
        }
    }
}
