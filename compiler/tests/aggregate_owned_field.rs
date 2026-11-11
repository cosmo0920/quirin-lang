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

//! End-to-end admission and refusal for the owned record field (card
//! `AGGREGATE-OWNED-FIELD`, wave 39): a `source.record.v1` field declared
//! `bytes`, moved in at construction, addressed by path at every custody
//! position, moved out by a field read, and released by `drop RECORD;`.
//!
//! Four families of test carry the card. The **custody** family pins the
//! operation list the route builds and shows every refusal arriving from the
//! path-extended ownership IR at `Q054` with that checker's own message. The
//! **route** family is the admitted/refused table with codes — including the
//! deliberate lift of `Q055` (a buffer at a `bytes` field) and the refusals
//! that stay exactly as they were (a buffer at an `i32` field, `drop` of a
//! record of scalars, `drop` of a string). The **emission** family pins the C:
//! the struct member, the initializer that *is* the move, the accesses
//! through the member, and the whole-record release. The **native** family
//! builds and runs the flagship under ASan/UBSan/LSan and compares both
//! streams with the reference execution.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::ownership::{FieldPath, OwnershipOp};
use qcore_core::source_record::{FieldType, FieldValue, RecordField, RecordRefusal, RecordType};
use qcore_core::std_buffer::{self, BufferEvent, BufferOperand, BufferProgram, FieldMove};
use qcore_core::typed_errors::EffectRow;

const OWNED_FIELD: &str = include_str!("../../examples/records/owned_field.qr");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("aggregate_owned_field.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn messages(source: &str) -> String {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A one-entry module holding a buffer beside `declaration`, so each test
/// measures the rule it names against the same surroundings.
fn module(declaration: &str, body: &str) -> String {
    format!(
        "#import std.buffer;\n#import std.fmt;\n#import std.io;\n\n{declaration}\n\nfn main() -> unit \
         effects {{ io, alloc, mut }} {{\n{body}    return;\n}}\n"
    )
}

fn certificate(module: &qcore_compiler::CompiledModule) -> &std_buffer::BufferCertificate {
    match &module.checked {
        qcore_compiler::CheckedSourceModule::StandardBuffer(checked) => &checked.certificate,
        other => panic!("expected a byte-buffer module, got {other:?}"),
    }
}

fn lowered(module: &qcore_compiler::CompiledModule) -> &BufferProgram {
    match &module.checked {
        qcore_compiler::CheckedSourceModule::StandardBuffer(checked) => &checked.lowered.program,
        other => panic!("expected a byte-buffer module, got {other:?}"),
    }
}

const PACKET: &str = "type Packet { tag: i32, buf: bytes }";

// --- The custody family: the operation list, and refusals arriving from the IR. ---

/// **The operation list the route builds for the flagship**, pinned exactly.
/// A scalar field read contributes one live-record check (card
/// `W48-QR-USABLE-RECORDS`, which made a read of a released record a use
/// after drop; before it, nothing); every owned-field statement is one
/// operation of the path-extended IR; the construction is a `Compose` of the
/// owned fields in declared order, and the whole-record `drop` is the plain
/// `Drop` the IR walks.
#[test]
fn the_flagship_builds_exactly_the_aggregate_operations() {
    let module = compile(OWNED_FIELD).unwrap();
    let operations = std_buffer::custody_operations(lowered(&module));
    let path = |base: &str, field: &str| FieldPath::new(base, field);
    let kinds: Vec<String> = operations
        .iter()
        .map(|operation| match operation {
            OwnershipOp::Acquire { owner, .. } => format!("acquire {owner}"),
            OwnershipOp::Write(name) => format!("write {name}"),
            OwnershipOp::Read(name) => format!("read {name}"),
            OwnershipOp::Drop(name) => format!("drop {name}"),
            OwnershipOp::Compose { owner, fields } => format!("compose {owner} {fields:?}"),
            OwnershipOp::ReadField(p) => format!("read-field {}", p.spelling()),
            OwnershipOp::WriteField(p) => format!("write-field {}", p.spelling()),
            OwnershipOp::MoveOut { from, to } => format!("move-out {} -> {to}", from.spelling()),
            OwnershipOp::DropField(p) => format!("drop-field {}", p.spelling()),
            OwnershipOp::AccessRecord(base) => format!("access {base}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "acquire h",
            "write h",
            "write h",
            "acquire b",
            "write b",
            "compose f [(\"head\", \"h\"), (\"body\", \"b\")]",
            "access f",
            "read-field f.body",
            "write-field f.body",
            "read-field f.head",
            "read-field f.body",
            "read-field f.body",
            "move-out f.body -> taken",
            "write taken",
            "read taken",
            "drop f",
            "drop taken",
        ]
    );
    assert!(operations.contains(&OwnershipOp::MoveOut {
        from: path("f", "body"),
        to: "taken".to_owned(),
    }));
    assert!(qcore_core::ownership::check_ownership(&operations).is_ok());
}

/// A record of scalars still contributes **no** operation — the
/// `AGGREGATES-V1` decision, unchanged for the records it was made about.
#[test]
fn a_record_of_scalars_still_contributes_no_operation() {
    let program = BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: EffectRow::default(),
        records: vec![RecordType {
            name: "Point".to_owned(),
            fields: vec![RecordField {
                name: "x".to_owned(),
                field_type: FieldType::I32,
            }],
        }],
        events: vec![
            BufferEvent::Construct {
                binding: "p".to_owned(),
                type_name: "Point".to_owned(),
                fields: vec![("x".to_owned(), BufferOperand::Literal(1).into())],
            },
            BufferEvent::Field {
                binding: "a".to_owned(),
                record: "p".to_owned(),
                path: vec!["x".to_owned()],
            },
            BufferEvent::Return,
        ],
    };
    assert!(std_buffer::custody_operations(&program).is_empty());
}

/// The moved-in binding is a use after move from the construction on — the
/// IR's own refusal, at `Q054`, with its own words.
#[test]
fn using_the_buffer_after_moving_it_into_a_record_refuses_as_a_use_after_move() {
    let source = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    b[0] = 1;\n    drop p;\n",
    );
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source).contains("use of `b` after move"),
        "{}",
        messages(&source)
    );
}

/// **The partial-move rule, from the accusing side.** After `let c = p.buf;`
/// the field is a hole: a byte read through it is a use after move of the
/// *path*.
#[test]
fn using_a_field_after_moving_it_out_refuses_by_its_path() {
    let source = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    let c = p.buf;\n    let \
         x = p.buf[0];\n    drop p;\n    drop c;\n",
    );
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source).contains("use of `p.buf` after move"),
        "{}",
        messages(&source)
    );

    let twice = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    let c = p.buf;\n    let \
         d = p.buf;\n    drop p;\n    drop c;\n",
    );
    assert_eq!(codes(&twice), ["Q054"]);
    assert!(messages(&twice).contains("use of `p.buf` after move"));
}

/// **The leak rule that sees into the aggregate.** An unreleased owned field
/// at the normal exit is a leak naming the path — the inherited `Q054`,
/// reached for a field.
#[test]
fn an_unreleased_owned_field_is_a_leak_named_by_its_path() {
    let source = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    let t = p.tag;\n    \
         print dec t;\n",
    );
    assert_eq!(codes(&source), ["Q054"]);
    assert!(
        messages(&source).contains("live resources escape normal exit: p.buf"),
        "{}",
        messages(&source)
    );

    // A moved-out buffer left live is a leak of the binding, not of the field.
    let moved = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    let c = p.buf;\n    \
         drop p;\n",
    );
    assert_eq!(codes(&moved), ["Q054"]);
    assert!(messages(&moved).contains("live resources escape normal exit: c"));
}

/// **The drop-through rule, from both sides.** `drop p;` releases every live
/// owned field in declared order — here the second-declared field was
/// acquired first, so the order is the declaration's — and a second `drop p;`
/// is a double drop of `p`.
#[test]
fn a_whole_record_drop_releases_the_live_fields_in_declared_order() {
    let source = module(
        "type Pair { second: bytes, first: bytes }",
        "    let a = bytes(1);\n    let b = bytes(2);\n    let p = Pair { first: a, second: b };\n    \
         drop p;\n",
    );
    let compiled = compile(&source).expect("both fields released by the whole-record drop");
    let certificate = certificate(&compiled);
    assert_eq!(
        certificate.acquisitions,
        [("a".to_owned(), 1), ("b".to_owned(), 2)]
    );
    assert_eq!(
        certificate.releases,
        ["p.second".to_owned(), "p.first".to_owned()]
    );
    assert_eq!(certificate.peak_live_bytes, 3);
    let c = compiled.c_source.as_deref().unwrap();
    let second = c
        .find("qcore_bytes_drop(r_p.f_second);")
        .expect("second released");
    let first = c
        .find("qcore_bytes_drop(r_p.f_first);")
        .expect("first released");
    assert!(
        second < first,
        "declaration order, not acquisition order:\n{c}"
    );

    let twice = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    drop p;\n    drop p;\n",
    );
    assert_eq!(codes(&twice), ["Q054"]);
    assert!(messages(&twice).contains("resource `p` is dropped more than once"));
}

/// A hole is tolerated by the whole-record drop, whether it was left by a
/// move or by a field release; releasing the same field twice is a double
/// drop of the path.
#[test]
fn a_whole_record_drop_tolerates_a_hole_and_a_field_is_not_released_twice() {
    let released = module(
        "type Pair { first: bytes, second: bytes }",
        "    let a = bytes(1);\n    let b = bytes(2);\n    let p = Pair { first: a, second: b };\n    \
         drop p.first;\n    drop p;\n",
    );
    let compiled = compile(&released).expect("the hole is tolerated");
    assert_eq!(
        certificate(&compiled).releases,
        ["p.first".to_owned(), "p.second".to_owned()]
    );
    let c = compiled.c_source.as_deref().unwrap();
    assert_eq!(c.matches("qcore_bytes_drop(r_p.f_first);").count(), 1);
    assert_eq!(c.matches("qcore_bytes_drop(r_p.f_second);").count(), 1);

    let twice = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    drop p.buf;\n    drop \
         p.buf;\n    drop p;\n",
    );
    assert_eq!(codes(&twice), ["Q054"]);
    assert!(messages(&twice).contains("resource `p.buf` is dropped more than once"));
}

/// One buffer cannot be moved into two fields: the second move is a use after
/// move, decided by the IR.
#[test]
fn one_buffer_moved_into_two_fields_refuses_at_the_second() {
    let source = module(
        "type Pair { first: bytes, second: bytes }",
        "    let a = bytes(1);\n    let p = Pair { first: a, second: a };\n    drop p;\n",
    );
    assert_eq!(codes(&source), ["Q054"]);
    assert!(messages(&source).contains("use of `a` after move"));
}

/// A move-out binding is a fresh name. From source, rebinding a live one is
/// refused **before** this route runs — name resolution's `Q012` for a
/// duplicate `let` in one scope — so the IR's own `BindingAlreadyExists` arm
/// for `MoveOut` is reachable only by a model-level caller (pinned in the
/// ownership unit tests) and is not the refusal a source program sees.
#[test]
fn a_move_out_binding_that_shadows_a_live_name_refuses() {
    let source = module(
        PACKET,
        "    let b = bytes(4);\n    let z = bytes(1);\n    let p = Packet { tag: 1, buf: b };\n    \
         let z = p.buf;\n    drop p;\n    drop z;\n",
    );
    assert_eq!(codes(&source), ["Q012"]);
}

// --- The route family: the admitted/refused table, with codes. ---

/// **`Q055`'s deliberate lift, and its unchanged half.** A buffer at a field
/// declared `bytes` is admitted — moved in — and a buffer at a field declared
/// `i32` is still the crossing refusal it always was.
#[test]
fn a_buffer_at_a_bytes_field_is_admitted_and_at_an_i32_field_still_refused() {
    let admitted = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    drop p;\n",
    );
    let compiled = compile(&admitted).expect("the lift");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    assert_eq!(
        certificate(&compiled).constructions[0].owned,
        [("buf".to_owned(), "b".to_owned())]
    );
    assert_eq!(
        certificate(&compiled).constructions[0].fields,
        [("tag".to_owned(), 1)]
    );

    let refused = module(
        "type Point { x: i32 }",
        "    let b = bytes(4);\n    let p = Point { x: b };\n    drop b;\n",
    );
    assert_eq!(codes(&refused), ["Q055"]);
}

/// An integer, a scalar name or a record at a `bytes` field is refused at
/// `Q052`: the field takes an owned buffer this entry acquired.
#[test]
fn a_non_buffer_at_a_bytes_field_is_refused() {
    let literal = module(PACKET, "    let p = Packet { tag: 1, buf: 2 };\n");
    assert_eq!(codes(&literal), ["Q052"]);
    assert!(messages(&literal).contains("takes an owned buffer this entry acquired"));

    let scalar = module(
        PACKET,
        "    let n = 2;\n    let p = Packet { tag: 1, buf: n };\n",
    );
    assert_eq!(codes(&scalar), ["Q052"]);

    let record = module(
        "type Point { x: i32 }\ntype Packet { tag: i32, buf: bytes }",
        "    let q = Point { x: 1 };\n    let p = Packet { tag: 1, buf: q };\n",
    );
    assert_eq!(codes(&record), ["Q052"]);
}

/// A lend cannot cross into a record: the lend's existing crossing refusal,
/// reached at a `bytes` field.
#[test]
fn a_lend_at_a_bytes_field_is_the_lend_crossing_refusal() {
    let source = module(
        PACKET,
        "    let b = bytes(4);\n    let l = lend(b, 0, 2);\n    let p = Packet { tag: 1, buf: l };\n    \
         drop b;\n",
    );
    // The window gate refuses the lend beside the record first, by name;
    // without the record statement the construction's own refusal is the
    // lend's `Q055`, measured over the model.
    let codes = codes(&source);
    assert!(codes.contains(&"Q051".to_owned()), "{codes:?}");
}

/// A custody position naming a scalar field is refused at `Q052`, with the
/// field named.
#[test]
fn a_custody_position_at_a_scalar_field_is_refused() {
    let store = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    p.tag[0] = 1;\n    drop p;\n",
    );
    assert_eq!(codes(&store), ["Q052"]);
    assert!(messages(&store).contains("`p.tag` is a `Packet` field declared `i32`, not a buffer"));

    let release = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    drop p.tag;\n    drop p;\n",
    );
    assert_eq!(codes(&release), ["Q052"]);

    let extent = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    let n = \
         extent(p.tag);\n    drop p;\n",
    );
    assert_eq!(codes(&extent), ["Q052"]);

    let undeclared = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    let x = p.nope[0];\n    \
         drop p;\n",
    );
    assert_eq!(codes(&undeclared), ["Q052"]);
    assert!(messages(&undeclared).contains("declares no field `nope`"));
}

/// The bounds rule at a field path: an index outside the moved-in buffer's
/// extent is `Q053`, naming the path.
#[test]
fn an_index_through_a_field_is_bounded_by_the_buffer_it_holds() {
    let source = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    p.buf[4] = 1;\n    drop p;\n",
    );
    assert_eq!(codes(&source), ["Q053"]);
    assert!(messages(&source).contains("index 4 is outside `p.buf`"));
}

/// **Unchanged refusals, re-measured.** `drop p;` of a record of scalars is
/// still `Q052` with the route's own message; a record with an owned field at
/// a scalar position is still `Q052`; `drop s;` of a string is still `Q068`,
/// because route selection did not move.
#[test]
fn the_refusals_that_stay_where_they_were() {
    let scalar_record = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 };\n    drop p;\n",
    );
    assert_eq!(codes(&scalar_record), ["Q052"]);
    assert!(messages(&scalar_record).contains("is not a buffer this entry acquired"));

    let rendered = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    print dec p;\n    drop p;\n",
    );
    assert_eq!(codes(&rendered), ["Q052"]);
    assert!(messages(&rendered).contains("read the field you mean first"));

    let string = "#import std.string;\n#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects \
                  { io } {\n    let s = \"hi\";\n    drop s;\n    return;\n}\n";
    assert_eq!(codes(string), ["Q068"]);
}

/// The two window premises, each refused by name (`OWNED-FIELD-LEND-WINDOW`,
/// `OWNED-FIELD-LOOP-BODY`).
#[test]
fn a_lend_beside_an_owned_field_and_an_owned_field_in_a_loop_body_refuse_by_name() {
    let lend = module(
        PACKET,
        "    let b = bytes(4);\n    let z = bytes(2);\n    let l = lend(z, 0, 1);\n    l[0] = 1;\n    \
         let p = Packet { tag: 1, buf: b };\n    drop p;\n    drop z;\n",
    );
    assert_eq!(codes(&lend), ["Q051"]);
    assert!(messages(&lend).contains("OWNED-FIELD-LEND-WINDOW"));

    let body = module(
        PACKET,
        "    for i in 0..2 {\n        let b = bytes(4);\n        let p = Packet { tag: i, buf: b \
         };\n        drop p;\n    }\n",
    );
    assert_eq!(codes(&body), ["Q051"]);
    assert!(messages(&body).contains("OWNED-FIELD-LOOP-BODY"));

    let outer = module(
        PACKET,
        "    let b = bytes(4);\n    let p = Packet { tag: 1, buf: b };\n    for i in 0..2 {\n        \
         let x = p.buf[0];\n        print dec x;\n    }\n    drop p;\n",
    );
    assert_eq!(codes(&outer), ["Q051"]);
    assert!(messages(&outer).contains("OWNED-FIELD-LOOP-BODY"));
}

/// **The spelling census, as a test.** `bytes` is the field type and it is the
/// acquisition's already-reserved word; `Buffer` — the task route's word for a
/// different resource — is not a field type here and refuses at the formation
/// rule's `Q051`, naming the gap that remains.
#[test]
fn the_field_type_is_spelled_bytes_and_buffer_is_not_one() {
    let source = module(
        "type Packet { tag: i32, buf: Buffer }",
        "    let n = 1;\n    print dec n;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("AGGREGATE-FIELD-TYPES"));
    assert_eq!(FieldType::from_spelling("bytes"), Some(FieldType::Bytes));
    assert_eq!(FieldType::from_spelling("Buffer"), None);
}

/// The model's own construction rule: type agreement is checked in one place,
/// and a scalar at a `bytes` field or a buffer at an `i32` field is refused
/// there whatever route asked.
#[test]
fn the_model_refuses_a_value_of_the_other_kind_at_a_field() {
    let declared = RecordType {
        name: "Packet".to_owned(),
        fields: vec![
            RecordField {
                name: "tag".to_owned(),
                field_type: FieldType::I32,
            },
            RecordField {
                name: "buf".to_owned(),
                field_type: FieldType::Bytes,
            },
        ],
    };
    let bytes = |name: &str| FieldValue::Bytes {
        moved_from: name.to_owned(),
    };
    let value = qcore_core::source_record::construct(
        &declared,
        &[
            ("tag".to_owned(), FieldValue::Scalar(1)),
            ("buf".to_owned(), bytes("b")),
        ],
    )
    .expect("agreeing kinds");
    assert_eq!(value.scalar("tag"), Some(1));
    assert_eq!(value.project("buf"), Some(&bytes("b")));
    assert_eq!(value.scalar("buf"), None);

    let scalar_at_bytes = qcore_core::source_record::construct(
        &declared,
        &[
            ("tag".to_owned(), FieldValue::Scalar(1)),
            ("buf".to_owned(), FieldValue::Scalar(2)),
        ],
    )
    .unwrap_err();
    assert!(matches!(
        scalar_at_bytes,
        RecordRefusal::FieldTakesABuffer { .. }
    ));
    assert_eq!(scalar_at_bytes.code(), "Q052");

    let bytes_at_scalar = qcore_core::source_record::construct(
        &declared,
        &[
            ("tag".to_owned(), bytes("b")),
            ("buf".to_owned(), bytes("c")),
        ],
    )
    .unwrap_err();
    assert!(matches!(
        bytes_at_scalar,
        RecordRefusal::FieldTakesAScalar { .. }
    ));
    assert_eq!(bytes_at_scalar.code(), "Q055");
}

/// The composed string/buffer route still refuses a record statement at its
/// own code, owned field or not.
#[test]
fn the_composed_string_route_still_refuses_a_record_with_an_owned_field() {
    let source = "#import std.buffer;\n#import std.string;\n#import std.fmt;\n#import std.io;\n\ntype \
                  Packet { tag: i32, buf: bytes }\n\nfn main() -> unit effects { io, alloc, mut } \
                  {\n    let b = bytes(2);\n    b[0] = 104;\n    b[1] = 105;\n    let s = \
                  borrow_utf8(b);\n    let p = Packet { tag: 1, buf: b };\n    print s;\n    drop \
                  p;\n    return;\n}\n";
    assert_eq!(codes(source), ["Q068"]);
}

// --- The emission family. ---

/// The struct member of a `bytes` field is the buffer pointer, the paragraph
/// about owned members is written only for a module that declares one, and
/// the initializer *is* the move.
#[test]
fn the_emitted_c_holds_the_pointer_in_the_struct_and_moves_by_initializer() {
    let compiled = compile(OWNED_FIELD).unwrap();
    let c = compiled.c_source.as_deref().unwrap();
    assert!(c.contains(
        "struct qcore_record_Frame {\n    int32_t f_tag;\n    unsigned char *f_head;\n    unsigned \
         char *f_body;\n};"
    ));
    assert!(c.contains("AGGREGATE-OWNED-FIELD: a member declared `bytes`"));
    assert!(c.contains(
        "struct qcore_record_Frame r_f = { .f_tag = INT32_C(7), .f_head = b_h, .f_body = b_b };"
    ));
    assert!(c.contains(
        "qcore_bytes_store(r_f.f_body, (size_t) 4u, (size_t) (INT32_C(1)), INT32_C(67));"
    ));
    assert!(c.contains(
        "int32_t s_h0 = qcore_bytes_load(r_f.f_head, (size_t) 2u, (size_t) (INT32_C(0)));"
    ));
    assert!(c.contains("int32_t s_n = INT32_C(4);"));
    assert!(c.contains("unsigned char *b_taken = r_f.f_body;"));
    assert!(c.contains("qcore_bytes_drop(r_f.f_head);"));
    assert!(!c.contains("qcore_bytes_drop(r_f.f_body);"));
    assert!(c.contains("qcore_bytes_drop(b_taken);"));

    // A module of scalar records carries no owned-member paragraph: the
    // header's bytes are exactly what they were.
    let scalar = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 };\n    let a = p.x;\n    print dec a;\n",
    );
    let scalar_c = compile(&scalar).unwrap().c_source.unwrap();
    assert!(!scalar_c.contains("AGGREGATE-OWNED-FIELD"));
}

/// The certificate publishes the move and the field releases, and each is
/// compared by the recheck.
#[test]
fn the_certificate_publishes_moves_and_field_releases_and_the_recheck_compares_them() {
    let module = compile(OWNED_FIELD).unwrap();
    let certificate = certificate(&module);
    assert_eq!(
        certificate.moves,
        [FieldMove {
            event: 17,
            record: "f".to_owned(),
            field: "body".to_owned(),
            binding: "taken".to_owned(),
            extent: 4,
        }]
    );
    assert_eq!(
        certificate.releases,
        ["f.head".to_owned(), "taken".to_owned()]
    );
    assert_eq!(
        certificate.constructions[0].owned,
        [
            ("head".to_owned(), "h".to_owned()),
            ("body".to_owned(), "b".to_owned())
        ]
    );
    let names: Vec<&str> = certificate
        .accesses
        .iter()
        .map(|access| access.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "h", "h", "b", "f.body", "f.head", "f.body", "f.body", "taken", "taken"
        ]
    );

    let mut tampered = certificate.clone();
    tampered.moves[0].extent += 1;
    assert_eq!(
        std_buffer::recheck_buffer_certificate(lowered(&module), &tampered),
        Err(std_buffer::BufferRecheckFailure::Moves)
    );
    let mut tampered = certificate.clone();
    tampered.releases.pop();
    assert_eq!(
        std_buffer::recheck_buffer_certificate(lowered(&module), &tampered),
        Err(std_buffer::BufferRecheckFailure::Releases)
    );
}

// --- The native family. ---

/// The flagship builds and runs under ASan/UBSan (plus LSan where the platform
/// provides it) and both streams equal the reference execution's projection.
/// LSan is the **positive leak control**: the whole-record drop frees the head
/// and the move-out binding's drop frees the body, and a missing release of
/// either would be reported.
#[test]
fn the_flagship_runs_clean_under_the_sanitizers_and_matches_the_reference() {
    let module = compile(OWNED_FIELD).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    assert_eq!(expected_out, ["7", "4", "72", "81", "67", "111"]);
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "owned-field");
    assert_eq!(out, expected_out);
    assert!(err.is_empty(), "{err:?}");
}

/// Compile and run the emitted C under the sanitizers, returning
/// `(stdout lines, stderr lines)`. The `source_record.rs` helper, unchanged.
fn run_native(source: &str, _label: &str) -> (Vec<String>, Vec<String>) {
    #[cfg(target_os = "windows")]
    {
        let run = windows_c::run(source, None, &[]);
        assert!(
            run.status.success(),
            "emitted program failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        (out, err)
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let directory = std::env::temp_dir().join(format!(
            "qcore-aggregate-owned-field-{_label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let generated = directory.join("generated.c");
        let binary = directory.join("generated");
        std::fs::write(&generated, source).unwrap();
        let compiler = std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".to_owned());
        let compile = Command::new(&compiler)
            .args([
                "-std=c11",
                "-Wall",
                "-Wextra",
                "-Werror",
                "-pedantic",
                "-g",
                "-O0",
                "-fsanitize=address,undefined",
                "-fno-omit-frame-pointer",
            ])
            .arg(&generated)
            .arg("-o")
            .arg(&binary)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "C compiler rejected the emitted program:\n{}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let run = Command::new(&binary)
            .env(
                "ASAN_OPTIONS",
                if cfg!(target_os = "macos") {
                    "detect_leaks=0:halt_on_error=1"
                } else {
                    "detect_leaks=1:halt_on_error=1"
                },
            )
            .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "emitted program failed under ASan/UBSan/LSan:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let out = String::from_utf8(run.stdout)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        let err = String::from_utf8(run.stderr)
            .unwrap()
            .lines()
            .map(ToOwned::to_owned)
            .collect();
        std::fs::remove_dir_all(directory).unwrap();
        (out, err)
    }
}
