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

//! Card CG13-UNION-AND-BITFIELD-PLACES (rule 10w): the decision table, the
//! record facts read from Clang, the punning check, the layout reader and
//! the mechanical binding to `QCore.UnionBitfieldPlace`.

use super::*;
use crate::legacy_c::CCompilerDialect;
use crate::typed_access_path::{StatementIndex, enter_source};

fn storage(code: &str) -> MemberStorage {
    match code {
        "ordinary" => MemberStorage::Ordinary,
        "union" => MemberStorage::Union,
        "bitfield" => MemberStorage::Bitfield,
        "union-bitfield" => MemberStorage::UnionBitfield,
        "unread" => MemberStorage::Unread,
        other => panic!("unknown storage {other}"),
    }
}

fn access(code: &str) -> Access {
    match code {
        "scalar-read" => Access::ScalarRead,
        "pointer-read" => Access::PointerRead,
        "scalar-store" => Access::ScalarStore,
        "pointer-store" => Access::PointerStore,
        other => panic!("unknown access {other}"),
    }
}

#[test]
fn the_decision_vectors_hold() {
    for (member, kind, punned, free, outcome) in DECISION_VECTORS {
        let decided = decide(storage(member), access(kind), *punned, *free);
        assert_eq!(decided.code(), *outcome, "{member} {kind} {punned} {free}");
        assert_eq!(storage(member).code(), *member);
        assert_eq!(access(kind).code(), *kind);
    }
}

/// Every refusal row refuses whatever the two flags say, and a union pointer
/// store never becomes admissible.
#[test]
fn refusals_do_not_depend_on_the_flags() {
    for punned in [false, true] {
        for free in [false, true] {
            assert!(!decide(MemberStorage::Union, Access::PointerStore, punned, free).admits());
            assert!(
                !decide(
                    MemberStorage::UnionBitfield,
                    Access::ScalarRead,
                    punned,
                    free
                )
                .admits()
            );
            assert!(!decide(MemberStorage::Unread, Access::ScalarStore, punned, free).admits());
            assert!(!decide(MemberStorage::Bitfield, Access::PointerRead, punned, free).admits());
        }
    }
}

/// The mechanical binding of `proof/QCore/UnionBitfieldPlace.lean`: its
/// decision vectors are the Rust ones, row by row.
#[test]
fn cg13_lean_vectors_match_the_rust_tables() {
    let lean = include_str!("../../../../proof/QCore/UnionBitfieldPlace.lean");
    let table = lean
        .split("def decisionVectors : List (Storage × Access × Bool × Bool × Outcome) := [")
        .nth(1)
        .expect("decisionVectors");
    let table = &table[..table.find("\n]").expect("end of vectors")];
    let code = |name: &str| -> String {
        let name = name.trim().trim_start_matches('.');
        let mut out = String::new();
        for character in name.chars() {
            if character.is_ascii_uppercase() {
                out.push('-');
                out.push(character.to_ascii_lowercase());
            } else {
                out.push(character);
            }
        }
        out
    };
    let rows = table
        .lines()
        .filter(|line| line.trim_start().starts_with("(."))
        .map(|line| {
            let line = line.trim().trim_end_matches(',');
            let inner = &line[1..line.len() - 1];
            let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
            assert_eq!(parts.len(), 5, "{line}");
            (
                code(parts[0]),
                code(parts[1]),
                parts[2] == "true",
                parts[3] == "true",
                code(parts[4]),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), DECISION_VECTORS.len());
    for (row, vector) in rows.iter().zip(DECISION_VECTORS) {
        assert_eq!(row.0, vector.0);
        assert_eq!(row.1, vector.1, "{row:?}");
        assert_eq!(row.2, vector.2, "{row:?}");
        assert_eq!(row.3, vector.3, "{row:?}");
        assert_eq!(row.4, vector.4, "{row:?}");
    }
}

const RECORDS: &str = "struct B { unsigned a : 3; unsigned b : 5; int x; unsigned c : 4; \
unsigned : 0; unsigned d : 2; unsigned long long e : 40; unsigned f : 1; }; \
union N { int i; long l; unsigned char bytes[8]; struct { short s; enum E { E0 } e; } in; }; \
union P { int i; void *p; }; union Q { int i; struct { int a; char *s; } in; }; \
typedef union { long n; double d; } TU; \
struct V { int kind; union { char *s; long n; }; TU t; };";

fn facts(source: &str) -> (StatementIndex, RecordFacts) {
    let _scope = enter_source(source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let records = (*index.records).clone();
    (index, records)
}

fn field_id(records: &RecordFacts, name: &str) -> String {
    let ids = records
        .field_names
        .iter()
        .filter(|(id, field)| *field == name && records.units.contains_key(*id))
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 1, "{name}");
    ids[0].clone()
}

/// Units follow C11's memory-location rule: a non-bit-field member or a
/// zero-width bit-field ends a unit; adjacent bit-fields share one whatever
/// their declared types.
#[test]
fn bitfield_units_follow_the_memory_location_rule() {
    if crate::clang_command().is_err() {
        return;
    }
    let (_index, records) = facts(&format!("{RECORDS} int f(void) {{ return 0; }}"));
    let unit = |name: &str| {
        records.units[&field_id(&records, name)]
            .clone()
            .expect("a unit")
            .ordinal
    };
    assert_eq!(unit("a"), unit("b"));
    assert_ne!(unit("b"), unit("c"));
    assert_ne!(unit("c"), unit("d"));
    assert_eq!(unit("d"), unit("e"));
    assert_eq!(unit("e"), unit("f"));
}

/// The union record owning a member spelled `member`, when exactly one does.
fn union_of(records: &RecordFacts, member: &str) -> String {
    let owners = records
        .union_owner
        .iter()
        .filter(|(field, _)| records.field_names.get(*field).map(String::as_str) == Some(member))
        .map(|(_, record)| record.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(owners.len(), 1, "{member}");
    owners.into_iter().next().unwrap()
}

fn union_member(records: &RecordFacts, record: &str, member: &str) -> String {
    records
        .union_owner
        .iter()
        .find(|(field, owner)| {
            *owner == record && records.field_names.get(*field).map(String::as_str) == Some(member)
        })
        .map(|(field, _)| field.clone())
        .expect("member")
}

#[test]
fn custody_free_unions_are_exactly_the_scalar_ones() {
    if crate::clang_command().is_err() {
        return;
    }
    let (_index, records) = facts(&format!("{RECORDS} int f(void) {{ return 0; }}"));
    let free = |member: &str| records.custody_free.contains(&union_of(&records, member));
    assert!(
        free("bytes"),
        "union N: scalars, an array, an inline struct of scalars and an enum"
    );
    assert!(!free("p"), "union P: a pointer member");
    assert!(free("d"), "TU: a typedef of scalars");
    assert!(!free("s"), "the anonymous union of V: a pointer member");
    // union Q: an inline struct with a pointer member. Its `in` is shared
    // with N, so it is found as the union declaring `in` but not `bytes`.
    let n = union_of(&records, "bytes");
    let q = records
        .union_owner
        .iter()
        .filter(|(field, owner)| {
            **owner != n && records.field_names.get(*field).map(String::as_str) == Some("in")
        })
        .map(|(_, owner)| owner.clone())
        .next()
        .expect("union Q");
    assert!(!records.custody_free.contains(&q));
}

#[test]
fn a_store_through_another_member_puns_a_pointer_read() {
    if crate::clang_command().is_err() {
        return;
    }
    for (body, expected) in [
        ("void *f(union P *u) { return u->p; }", false),
        ("void *f(union P *u) { u->p = 0; return u->p; }", false),
        ("void *f(union P *u) { u->i = 1; return u->p; }", true),
        ("void *f(union P *u) { u->i++; return u->p; }", true),
        (
            "void g(int *); void *f(union P *u) { g(&u->i); return u->p; }",
            true,
        ),
        (
            "void *f(union P *u, union P w) { *u = w; return u->p; }",
            true,
        ),
        (
            "void *f(union P *u, union P *w) { w->i = 1; return u->p; }",
            true,
        ),
    ] {
        let (index, records) = facts(&format!("{RECORDS} {body}"));
        let record = union_of(&records, "p");
        let pointer = union_member(&records, &record, "p");
        assert_eq!(
            punned(&index.function, &records, &record, &pointer, "union P"),
            expected,
            "{body}"
        );
    }
}

const DUMP: &str = "
*** Dumping AST Record Layout
         0 | union S::(unnamed at bf.c:1:84)
         0 |   int i
         0 |   void * q
           | [sizeof=8, align=8]

*** Dumping AST Record Layout
         0 | struct S
         0 |   int a
     4:0-2 |   unsigned int bf
     4:3-7 |   unsigned int bg
    8:0-39 |   unsigned long c
        16 |   void * p
        24 |   union S::(unnamed at bf.c:1:84) u
        24 |     int i
        24 |     void * q
           | [sizeof=32, align=8]
";

#[test]
fn the_layout_reader_takes_direct_bitfields_of_the_named_record() {
    let layout = parse_layout(DUMP, "S").expect("layout");
    assert_eq!(layout.size, 32);
    assert_eq!(
        layout.bitfields.into_iter().collect::<Vec<_>>(),
        [
            ("bf".to_owned(), (4, 0, 2)),
            ("bg".to_owned(), (4, 3, 7)),
            ("c".to_owned(), (8, 0, 39)),
        ]
    );
    assert!(parse_layout(DUMP, "T").is_none());
    let twice = format!("{DUMP}{DUMP}");
    assert!(
        parse_layout(&twice, "S").is_none(),
        "an ambiguous dump refuses"
    );
}

/// Example E: two bit-fields with disjoint bit ranges in one unit are one
/// storage unit to the core model; bit-fields of two units are not.
#[test]
fn the_core_model_keeps_one_unit_one_memory_location() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = "struct S { int a; unsigned bf : 3; unsigned bg : 5; int x; unsigned bh : 2; };\n\
                  int f(struct S *s) { return s->bf + s->bg + s->bh; }";
    let (_index, records) = facts(source);
    let bf = field_id(&records, "bf");
    let bg = field_id(&records, "bg");
    let bh = field_id(&records, "bh");
    let record = records.units[&bf].as_ref().unwrap().record.clone();
    let layout = ClangLayout {
        size: 16,
        bitfields: BTreeMap::from([
            ("bf".to_owned(), (4, 0, 2)),
            ("bg".to_owned(), (4, 3, 7)),
            ("bh".to_owned(), (12, 0, 1)),
        ]),
    };
    assert_eq!(
        core_shared_unit(&records, &record, &layout, &bf, &bg),
        Some(0)
    );
    assert_eq!(core_shared_unit(&records, &record, &layout, &bf, &bh), None);
    // A layout missing a member refuses the question.
    let partial = ClangLayout {
        size: 16,
        bitfields: BTreeMap::from([("bf".to_owned(), (4, 0, 2))]),
    };
    assert_eq!(
        core_shared_unit(&records, &record, &partial, &bf, &bg),
        None
    );
}

/// A route-free elaborator context that admits every place and names a
/// bit-field pair by the core model, over Clang's real layout dump.
struct Units<'a> {
    records: &'a RecordFacts,
}

impl crate::elaboration::Context for Units<'_> {
    type Facts = ();
    fn scalar(&self, node: &Value) -> bool {
        crate::typed_access_path::is_scalar(node)
    }
    fn pointer(&self, node: &Value) -> bool {
        crate::typed_access_path::is_pointer(node)
    }
    fn place(&self, _node: &Value, _store: bool) -> Result<(), crate::elaboration::Refusal> {
        Ok(())
    }
    fn unaliased(&self, _decl: &str) -> bool {
        // Every root of the fixture is a formal whose address is never taken.
        true
    }
    fn shared_bitfield_unit(&self, node: &Value, one: &str, other: &str) -> Option<u32> {
        shared_unit_refusal(
            node,
            self.records,
            one,
            other,
            &crate::typed_access_path::record_layout_dump,
        )
    }
}

/// The refusal control for concurrent stores to two bit-fields of one unit:
/// the unsequenced pair is refused, and the core model names it one storage
/// unit (Q661) from Clang's own layout. Two units are refused without that
/// name (field disjointness is not admitted here).
#[test]
fn unsequenced_stores_to_one_unit_are_named_by_the_core_model() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = "struct S { int a; unsigned bf : 3; unsigned bg : 5; int x; unsigned bh : 2; };\n\
                  void f(struct S *s) { int y; y = (s->bf = 1) + (s->bg = 2); y = (s->bf = 1) + (s->bh = 2); }";
    let _scope = enter_source(source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("f").expect("typed function");
    let body = children(&index.function)
        .iter()
        .find(|node| kind(node) == "CompoundStmt")
        .expect("body");
    let statements = children(body)
        .iter()
        .filter(|node| kind(node) == "BinaryOperator")
        .collect::<Vec<_>>();
    assert_eq!(statements.len(), 2);
    let context = Units {
        records: &index.records,
    };
    match crate::elaboration::elaborate(statements[0], &context) {
        Err(crate::elaboration::Refusal::SharedBitfieldUnit(one, other, 0)) => {
            assert_eq!((one.as_str(), other.as_str()), ("s->bf", "s->bg"));
        }
        other => panic!("one unit: {other:?}"),
    }
    match crate::elaboration::elaborate(statements[1], &context) {
        Err(crate::elaboration::Refusal::NotDisjoint(..)) => {}
        other => panic!("two units: {other:?}"),
    }
    // Clang's layout dump reads the record exactly.
    let layout = parse_layout(
        &crate::typed_access_path::record_layout_dump().expect("dump"),
        "S",
    )
    .expect("layout");
    assert_eq!(layout.bitfields.len(), 3);
}
