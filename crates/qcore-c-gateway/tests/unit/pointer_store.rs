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

//! Card CG11-POINTER-VALUED-STORES (rule 10v): the value classifier against
//! Clang, the decision table, and its mechanical binding to
//! `QCore.PointerStore`.

use super::*;
use crate::legacy_c::CCompilerDialect;
use crate::typed_access_path::{StatementIndex, enter_source};

fn value(code: &str) -> ValueClass {
    [
        ValueClass::Null,
        ValueClass::StringLiteral,
        ValueClass::Function,
        ValueClass::FunctionPointerName,
        ValueClass::SelfAddress,
        ValueClass::InteriorAddress,
        ValueClass::FormalPointer,
        ValueClass::LocalPointer,
        ValueClass::PointerRead,
        ValueClass::Call,
        ValueClass::Arithmetic,
        ValueClass::OtherAddress,
        ValueClass::Other,
    ]
    .into_iter()
    .find(|class| class.code() == code)
    .unwrap_or_else(|| panic!("unknown value {code}"))
}

fn owner(code: &str) -> OwnerClass {
    [
        OwnerClass::Untracked,
        OwnerClass::Tracked,
        OwnerClass::Unresolved,
    ]
    .into_iter()
    .find(|class| class.code() == code)
    .unwrap_or_else(|| panic!("unknown owner {code}"))
}

#[test]
fn the_decision_vectors_hold() {
    for (spelling, value_code, carries, owner_code, decision) in DECISION_VECTORS {
        assert_eq!(
            decide(value(value_code), *carries, owner(owner_code)).code(),
            *decision,
            "{spelling}"
        );
    }
}

/// Only a neutral value, carrying no identity, into an untracked owner is
/// admitted (`QCore.PointerStore.admitted_exactly_when`), over every input.
#[test]
fn admitted_exactly_when_neutral_identity_free_and_untracked() {
    let values = [
        "null",
        "string",
        "function",
        "function-pointer-name",
        "self-address",
        "interior-address",
        "formal-pointer",
        "local-pointer",
        "pointer-read",
        "call",
        "arithmetic",
        "other-address",
        "other",
    ];
    for code in values {
        for carries in [false, true] {
            for owner_code in ["untracked", "tracked", "unresolved"] {
                let class = value(code);
                let admitted = decide(class, carries, owner(owner_code)) == Decision::Admit;
                assert_eq!(
                    admitted,
                    class.neutral() && !carries && owner_code == "untracked",
                    "{code} {carries} {owner_code}"
                );
            }
        }
    }
}

/// The mechanical binding of `proof/QCore/PointerStore.lean`: its decision
/// vectors are the Rust ones, row by row.
#[test]
fn cg11_lean_vectors_match_the_rust_tables() {
    let lean = include_str!("../../../../proof/QCore/PointerStore.lean");
    let table = lean
        .split("def decisionVectors : List (String × Value × Bool × OwnerClass × Decision) := [")
        .nth(1)
        .expect("decisionVectors");
    let table = &table[..table.find("\n]").expect("end of vectors")];
    let lean_code = |name: &str| -> String {
        let name = name.trim().trim_start_matches('.');
        let mut code = String::new();
        for character in name.chars() {
            if character.is_ascii_uppercase() {
                code.push('-');
                code.push(character.to_ascii_lowercase());
            } else {
                code.push(character);
            }
        }
        code
    };
    let rows = table
        .lines()
        .filter(|line| line.trim_start().starts_with("(\""))
        .map(|line| {
            let line = line.trim().trim_end_matches(',');
            let line = &line[2..line.len() - 1];
            let (spelling, rest) = line.rsplit_once("\", ").expect("spelling");
            let fields = rest.split(',').map(str::trim).collect::<Vec<_>>();
            (
                spelling.replace("\\\"", "\""),
                lean_code(fields[0]),
                fields[1] == "true",
                lean_code(fields[2]),
                lean_code(fields[3]),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), DECISION_VECTORS.len());
    for (row, vector) in rows.iter().zip(DECISION_VECTORS) {
        assert_eq!(row.0, vector.0);
        assert_eq!(row.1, vector.1, "{}", vector.0);
        assert_eq!(row.2, vector.2, "{}", vector.0);
        assert_eq!(row.3, vector.3, "{}", vector.0);
        assert_eq!(row.4, vector.4, "{}", vector.0);
    }
}

const RECORDS: &str = "struct I { void *p; int x; };\n\
struct S { int a; void *p; char *name; struct I in; void (*cb)(void); struct S *self; \
void **pp; };\nvoid f(void); void *cg_get(void); void (*g)(void);\n";

/// Every `=` of `function`'s body, in source order.
fn assignments(node: &Value, out: &mut Vec<Value>) {
    if kind(node) == "BinaryOperator" && node["opcode"].as_str() == Some("=") {
        out.push(node.clone());
    }
    for child in children(node) {
        assignments(child, out);
    }
}

/// Every vector's spelling, read by Clang in a function where `s` and `t` are
/// `struct S *` formals, `q` a `void *` formal, `l` a `void *` local, `o` and
/// `d` `struct S *` locals, is exactly the vector's value class.
#[test]
fn every_vector_spelling_classifies_as_its_value() {
    if crate::clang_command().is_err() {
        return;
    }
    let body = DECISION_VECTORS
        .iter()
        .map(|(spelling, ..)| format!("\n {spelling};"))
        .collect::<String>();
    let source = format!(
        "{RECORDS} void h(struct S *s, void *q, struct S *t) {{ void *l = 0; struct S *o = s; \
         struct S *d = t;{body}\n }}"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("h").expect("typed function");
    let mut found = Vec::new();
    assignments(&index.function, &mut found);
    assert_eq!(found.len(), DECISION_VECTORS.len());
    for (node, (spelling, value_code, ..)) in found.iter().zip(DECISION_VECTORS) {
        let pair = children(node);
        assert!(is_pointer_place(&pair[0]), "{spelling}");
        let place = crate::typed_place::read_place(&pair[0], &index.fields)
            .unwrap_or_else(|refusal| panic!("{spelling}: {refusal:?}"));
        let reading = classify_value(&pair[1], &place, &index.fields);
        assert_eq!(reading.class.code(), *value_code, "{spelling}");
    }
}

#[test]
fn an_address_is_the_same_object_only_from_the_root_and_through_no_other_pointer() {
    if crate::clang_command().is_err() {
        return;
    }
    let source = format!(
        "{RECORDS} struct L {{ struct L *next; struct S s; }};\n\
         void h(struct S *s, struct L *n) {{ struct L head;\n\
         s->pp = &s->in.p;\n\
         s->pp = &s->self->p;\n\
         n->next->s.pp = &n->next->s.p;\n\
         head.next = &head;\n\
         s->self = (struct S *) s;\n\
         s->p = (void *) s;\n }}"
    );
    let _scope = enter_source(&source, CCompilerDialect::C11, None);
    let index = StatementIndex::read("h").expect("typed function");
    let mut found = Vec::new();
    assignments(&index.function, &mut found);
    let classes = found
        .iter()
        .map(|node| {
            let pair = children(node);
            let place = crate::typed_place::read_place(&pair[0], &index.fields).expect("place");
            classify_value(&pair[1], &place, &index.fields).class
        })
        .collect::<Vec<_>>();
    assert_eq!(
        classes,
        [
            // An embedded member of the root's own object.
            ValueClass::InteriorAddress,
            // A member of the object `s->self` points to: another object.
            ValueClass::OtherAddress,
            // Both places read through `n->next`: not the root's object.
            ValueClass::OtherAddress,
            // A record root's own address.
            ValueClass::SelfAddress,
            // A same-type cast is a qualification (`NoOp`) conversion.
            ValueClass::SelfAddress,
            // Any other cast is not a form this rule reads.
            ValueClass::Other,
        ]
    );
}

#[test]
fn function_pointer_types_are_read_positively() {
    assert!(pointer_type("void (*)(void)"));
    assert!(pointer_type("int (*)(struct x *, int)"));
    assert!(pointer_type("void *"));
    assert!(!pointer_type("void (**)(void)"));
    assert!(!pointer_type("void (*[4])(void)"));
    assert!(!pointer_type("void (*volatile)(void)"));
    assert!(!pointer_type("int"));
}
