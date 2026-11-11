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

//! Unit tests of card CG15-STORAGE-DURATION-AND-VOLATILE (gateway rule 11h):
//! the decision table, the qualifier reader, the trace, and the mechanical
//! binding to `proof/QCore/StorageDuration.lean`.

use super::*;

#[test]
fn top_level_volatile_reads_the_object_qualifier() {
    assert!(top_level_volatile("volatile int"));
    assert!(top_level_volatile("const volatile unsigned int"));
    assert!(top_level_volatile("void *volatile"));
    assert!(top_level_volatile("char *const volatile"));
    assert!(top_level_volatile("volatile int[4]"));
    assert!(!top_level_volatile("volatile int *"));
    assert!(!top_level_volatile("int"));
    assert!(!top_level_volatile("struct volatility *"));
    assert!(!top_level_volatile("_Atomic(int)"));
}

#[test]
fn storage_of_reads_clangs_storage_class() {
    let decl = |text: &str| serde_json::from_str::<Value>(text).unwrap();
    let automatic = decl(r#"{"kind":"VarDecl","name":"a"}"#);
    let register = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"register"}"#);
    let local_static = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"static"}"#);
    let local_extern = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"extern"}"#);
    let tls = decl(r#"{"kind":"VarDecl","name":"a","storageClass":"static","tls":"static"}"#);
    assert_eq!(storage_of(&automatic, true), Some(Storage::Automatic));
    assert_eq!(storage_of(&register, true), Some(Storage::Automatic));
    assert_eq!(storage_of(&automatic, false), Some(Storage::Static));
    assert_eq!(storage_of(&local_static, true), Some(Storage::Static));
    assert_eq!(storage_of(&local_extern, true), Some(Storage::Static));
    assert_eq!(storage_of(&tls, true), Some(Storage::ThreadLocal));
    assert_eq!(storage_of(&decl(r#"{"kind":"ParmVarDecl"}"#), true), None);
    assert!(!Storage::Automatic.outlives_frame());
    assert!(Storage::Static.outlives_frame());
    assert!(Storage::ThreadLocal.outlives_frame());
}

#[test]
fn an_unlisted_form_is_refused() {
    assert_eq!(disposition("volatile gadget"), Disposition::Refused);
    assert_eq!(disposition("static declaration"), Disposition::NoEvent);
    assert_eq!(disposition("atomic access"), Disposition::Owned("CG19"));
    for (form, name, row) in FORM_TABLE {
        assert_eq!(disposition(name), row);
        assert_eq!(decide(form), row);
    }
}

#[test]
fn root_name_strips_the_place() {
    assert_eq!(root_name("p"), "p");
    assert_eq!(root_name("p->f"), "p");
    assert_eq!(root_name("*out"), "out");
    assert_eq!(root_name("s.f"), "s");
    assert_eq!(root_name("slots[0]"), "slots");
}

const LEAN: &str = include_str!("../../../../proof/QCore/StorageDuration.lean");

/// The body of the Lean list `start … \n]`.
fn block(start: &str) -> String {
    let body = LEAN.split(start).nth(1).expect(start);
    body[..body.find("\n]").expect("list end")].to_owned()
}

fn lean_disposition(text: &str) -> Disposition {
    match text.trim() {
        ".noEvent" => Disposition::NoEvent,
        ".noIdentity" => Disposition::NoIdentity,
        ".neutral" => Disposition::Neutral,
        ".forgotten" => Disposition::Forgotten,
        ".unrefined" => Disposition::Unrefined,
        ".refused" => Disposition::Refused,
        ".owned \"CG19\"" => Disposition::Owned("CG19"),
        other => panic!("unread Lean disposition {other}"),
    }
}

/// `QCore.StorageDuration.formTable` = `FORM_TABLE`, row for row.
#[test]
fn cg15_lean_form_table_matches_the_rust_table() {
    let rows = block("def formTable : List (String × Disposition) := [")
        .lines()
        .filter(|line| line.trim_start().starts_with('('))
        .map(|line| {
            let row = line.trim().trim_end_matches(',');
            let row = row
                .strip_prefix('(')
                .and_then(|row| row.strip_suffix(')'))
                .expect("a parenthesised row");
            let (name, disposition) = row.split_once(", ").expect("two fields");
            (
                name.trim_matches('"').to_owned(),
                lean_disposition(disposition),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), FORM_TABLE.len());
    for ((name, disposition), (_, rust_name, rust)) in rows.iter().zip(FORM_TABLE) {
        assert_eq!(name, rust_name);
        assert_eq!(*disposition, rust, "{name}");
    }
}

#[test]
fn the_trace_counts_every_access_and_orders_only_sequenced_pairs() {
    let read = |name: &str| {
        serde_json::json!({"kind": "ImplicitCastExpr", "castKind": "LValueToRValue",
            "type": {"qualType": "volatile int"},
            "inner": [{"kind": "DeclRefExpr", "type": {"qualType": "volatile int"},
                "referencedDecl": {"kind": "VarDecl", "name": name}}]})
    };
    // `v + v`: two reads of one object, unsequenced (C11 6.5p2): counted
    // twice, never ordered, and the pair is named.
    let sum = serde_json::json!({"kind": "BinaryOperator", "opcode": "+",
        "inner": [read("v"), read("v")]});
    let trace = volatile_trace(&sum);
    assert_eq!(trace.accesses.len(), 2);
    assert!(trace.sequenced.is_empty());
    assert_eq!(trace.unsequenced, vec![(0, 1)]);
    assert_eq!(trace.unsequenced_conflict(), Some("v"));
    // `v, w`: the comma sequences them.
    let comma = serde_json::json!({"kind": "BinaryOperator", "opcode": ",",
        "inner": [read("v"), read("w")]});
    let trace = volatile_trace(&comma);
    assert_eq!(trace.sequenced, vec![(0, 1)]);
    assert!(trace.unsequenced.is_empty());
    assert_eq!(trace.unsequenced_conflict(), None);
    // A non-volatile expression has no trace.
    let plain = serde_json::json!({"kind": "IntegerLiteral"});
    assert!(volatile_trace(&plain).accesses.is_empty());
}

#[test]
fn static_declarators_are_masked_and_initializers_kept() {
    let body = [
        "static", "void", "*", "p", "=", "&", "x", ";", "q", "=", "0", ";",
    ];
    let masked = mask_static_declarators(&body);
    assert_eq!(masked[3], "0");
    assert_eq!(masked[6], "x", "an initializer's `&x` is still read");
    assert_eq!(masked[8], "q", "a later statement is untouched");
    let body = [
        "int", "n", "=", "0", ";", "const", "static", "char", "*", "t", "[", "]", "=", "{", "a",
        "}", ";",
    ];
    let masked = mask_static_declarators(&body);
    assert_eq!(masked[1], "n", "an automatic declaration is untouched");
    assert_eq!(masked[9], "0");
    assert_eq!(masked[14], "a");
}
