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

//! End-to-end admission and refusal for `source.record.v1` — the first
//! source-level aggregate type (card `AGGREGATES-V1`).
//!
//! The construct is admitted on `source.buffer.v1` in v1, so a record module is
//! checked, emitted and executed by that route. Every other route's refusal is
//! **measured and pinned** rather than widened (`AGGREGATE-ROUTE-WIDENING`).
//!
//! Three families of test carry the card. The **census** family pins what the
//! spelling does and does not claim, including that the archived `record R { }`
//! probe still refuses. The **distinctness** family is CT09's own scenario: two
//! fields of one value read back as the two values supplied for them, in the
//! model, in the certificate and in a sanitized native run. The **custody**
//! family pins the field-custody decision from the accusing side: a record
//! contributes no ownership operation, derives no effect, and is refused at
//! every custody position.
#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::source_record::{
    self, FieldType, FieldValue, MAX_FIELDS, RecordField, RecordRefusal, RecordType,
};
use qcore_core::std_buffer::{self, BufferEvent, BufferOperand, BufferProgram};
use qcore_frontend::frontend::{DeclarationKind, parse_source_program};
use qcore_frontend::lexer::{Symbol, TokenKind, lex};

const POINT: &str = include_str!("fixtures/source_record/point.qr");

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("source_record.qcore", source)],
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

/// A one-entry record module around `declaration` and `body`, so each refusal
/// test measures the rule it names against the same surroundings.
fn module(declaration: &str, body: &str) -> String {
    format!(
        "#import std.fmt;\n#import std.io;\n#import std.logger;\n\n{declaration}\n\nfn main() -> unit \
         effects {{ io, alloc, mut }} {{\n{body}    return;\n}}\n"
    )
}

/// The same, plus `#import std.buffer;` — for the tests that hold a buffer
/// beside a record. The import is the buffer's and stays the buffer's: a record
/// needs none, which is itself pinned by every other test here.
fn buffer_module(declaration: &str, body: &str) -> String {
    format!(
        "#import std.buffer;\n#import std.fmt;\n#import std.io;\n#import std.logger;\n\n{declaration}\n\
         \nfn main() -> unit effects {{ io, alloc, mut }} {{\n{body}    return;\n}}\n"
    )
}

// --- The census, and what the spelling claims. -----------------------------

/// The constraint-14 finding that decided the spelling. An archived probe
/// fixture is the single line `record R { }`, and two archived reports pin it
/// to `Q000`; claiming `record` at declaration position would have falsified a
/// frozen archive's own table. This card reserves **no** word, so the probe
/// still refuses exactly as it did.
#[test]
fn record_is_still_not_a_declaration_head() {
    let source = "record R { }\n";
    assert_eq!(codes(source), ["Q000"]);
}

/// `type NAME = …;` is the alias it always was, told from a record by the one
/// token after the name.
#[test]
fn a_type_alias_is_still_a_type_alias() {
    let parsed = parse_source_program("type Count = i32;\n").unwrap();
    assert_eq!(
        parsed.declarations[0].value.kind,
        DeclarationKind::TypeAlias
    );
}

/// `resource type NAME {` is claimed by the branch above the `type` branch and
/// never reaches it — which is what keeps the eleven resource types of the
/// pinned contract corpus parsing exactly as they did.
#[test]
fn a_resource_type_is_not_a_record() {
    let source = "resource type Chunk {\n    repr: ptr<mut, u8>;\n    footprint: one;\n    destructor: \
         free;\n}\n";
    let parsed = parse_source_program(source).unwrap();
    assert_eq!(
        parsed.declarations[0].value.kind,
        DeclarationKind::ResourceType
    );
}

/// `type NAME {` is the record head.
#[test]
fn a_braced_type_declaration_is_a_record() {
    let parsed = parse_source_program("type Point { x: i32, y: i32 }\n").unwrap();
    assert_eq!(parsed.declarations[0].value.kind, DeclarationKind::Record);
    assert_eq!(parsed.declarations[0].value.name, "Point");
}

/// Neither a field name nor a record type name is reserved: this module binds
/// locals called `x` and `y` **and** declares a record with fields of those
/// names, and both resolve.
#[test]
fn a_field_name_is_not_a_reserved_word() {
    let source = module(
        "type Pair { x: i32, y: i32 }",
        "    let x = 7;\n    let y = 8;\n    let p = Pair { x: x, y: y };\n    let a = p.x;\n    \
         print dec a;\n",
    );
    let module = compile(&source).expect("both uses of `x` resolve");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
}

/// A record declaration **after** the entry. Card `LOOP-V1` §9.3's hazard, from
/// the declaration side: a function body ends where the next declaration
/// begins, and `type` has been in `is_declaration_start` since long before this
/// card — so the entry's body does not swallow the declaration that follows it.
#[test]
fn a_record_declared_after_the_entry_is_still_a_declaration() {
    let source = "#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects { io } {\n    let \
                  p = Point { x: 7 };\n    let a = p.x;\n    print dec a;\n    return;\n}\n\ntype \
                  Point { x: i32 }\n";
    let module = compile(source).expect("the declaration is not part of the entry's body");
    assert_eq!(certificate(&module).projections[0].value, 7);
}

/// **The one collision this card measured and deliberately did not repair.**
///
/// Two route selectors scan the **whole token stream** (constraint 14), so a
/// record whose type name or field name is one of their six words selects the
/// concurrency route from its own spelling. A *declaration* of one refuses at
/// `Q015`; a **construction** of one refuses one layer earlier still, at the
/// parser's `Q000`, because the task-expression reader claims the `spawn:`
/// token. Both are fail-closed and neither is a wrong acceptance, which is why
/// the `NATIVE-LIBRARY-SOURCE-BINDING` span exclusion was **not** copied here:
/// it would retire the `Q015` half and leave the `Q000` half standing, so the
/// type would declare and could not be constructed. `AGGREGATE-SELECTOR-WORDS`.
#[test]
fn a_field_named_for_a_route_selector_refuses_fail_closed() {
    let declared = module("type Job { spawn: i32 }", "    print dec 1;\n");
    assert_eq!(codes(&declared), ["Q015"]);
    let named = module("type spawn { x: i32 }", "    print dec 1;\n");
    assert_eq!(codes(&named), ["Q015"]);
    let constructed = module(
        "type Job { spawn: i32 }",
        "    let j = Job { spawn: 1 };\n    let a = j.spawn;\n    print dec a;\n",
    );
    assert_eq!(codes(&constructed), ["Q000"]);
}

/// And the words that are *not* selectors are ordinary field names, including
/// every reserved statement keyword — which is the measurement that keeps the
/// row above a narrow collision rather than a general restriction.
#[test]
fn a_field_named_for_a_reserved_statement_word_is_admitted() {
    let source = module(
        "type Frame { drop: i32, close: i32, print: i32 }",
        "    let f = Frame { drop: 1, close: 2, print: 3 };\n    let a = f.close;\n    print dec \
         a;\n",
    );
    let module = compile(&source).expect("a reserved statement head is still a field name");
    assert_eq!(certificate(&module).projections[0].value, 2);
}

// --- The `.` adjacency, pinned rather than assumed. ------------------------

/// A field access is `Word Dot Word` and can never collapse into a float: the
/// lexer starts a `Float` only at a digit.
#[test]
fn a_field_access_lexes_as_word_dot_word() {
    let tokens: Vec<TokenKind> = lex("p.x").unwrap().into_iter().map(|t| t.kind).collect();
    assert_eq!(
        tokens,
        [
            TokenKind::Word("p".to_owned()),
            TokenKind::Symbol(Symbol::Dot),
            TokenKind::Word("x".to_owned()),
            TokenKind::Eof,
        ]
    );
}

/// Card `SOCKET-INET-ADDRESSES`' measurement, re-measured here because this
/// card reads the same `.`: a dotted quad is `Float Dot Float`, and that rule is
/// untouched.
#[test]
fn a_dotted_quad_still_lexes_as_float_dot_float() {
    let tokens: Vec<TokenKind> = lex("127.0.0.1")
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect();
    assert_eq!(
        tokens,
        [
            TokenKind::Float("127.0".to_owned()),
            TokenKind::Symbol(Symbol::Dot),
            TokenKind::Float("0.1".to_owned()),
            TokenKind::Eof,
        ]
    );
}

/// The one place the float rule reaches this construct: a numeric field path is
/// a single floating-point token after the `Dot`, and the reader refuses it by
/// name rather than reading a field out of it.
#[test]
fn a_numeric_field_path_is_refused_by_name() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 };\n    let a = p.1.2;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(
        messages(&source).contains("floating-point token"),
        "{}",
        messages(&source)
    );
}

/// A field read of a **literal** parses into two tokens and is refused as a
/// name that is not a record, not as a lexical accident.
#[test]
fn a_field_read_of_an_integer_is_refused_as_a_name() {
    let source = module(
        "type Point { x: i32 }",
        "    let n = 3;\n    let a = n.x;\n",
    );
    assert_eq!(codes(&source), ["Q052"]);
    assert!(messages(&source).contains("is not a record this entry constructed"));
}

// --- Type formation. -------------------------------------------------------

#[test]
fn a_record_declaring_no_field_is_refused() {
    let source = module("type Empty { }", "    let a = 1;\n    print dec a;\n");
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("declares no field"));
}

/// **The distinctness rule, from the declaring side.** Two fields of one record
/// may not share a name, because which of two declarations a read answers to
/// would otherwise be decided by the order a reader walks the list in.
#[test]
fn a_repeated_field_name_is_refused() {
    let source = module(
        "type Point { x: i32, x: i32 }",
        "    let a = 1;\n    print dec a;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("declares `x` twice"));
}

/// The refusal names the gap that remains. Before card `AGGREGATE-OWNED-FIELD`
/// it named that card; the owned field now exists (`bytes`), so a field of any
/// other type is `AGGREGATE-FIELD-TYPES` and the message says so. Card
/// `W48-QR-SCALAR-TYPES-R3` admits every §G02t scalar (`u8` here before it),
/// so the unmodelled spelling is now one outside that table.
#[test]
fn a_field_type_this_revision_does_not_model_is_refused() {
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS` admits `f64` fields on this route
    // (its rows are `float_buffer_payloads.rs`), so the row pins a spelling
    // no route models.
    let source = module(
        "type Point { x: f16 }",
        "    let a = 1;\n    print dec a;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("AGGREGATE-FIELD-TYPES"));
    assert!(!messages(&source).contains("AGGREGATE-OWNED-FIELD"));
}

#[test]
fn a_field_declared_without_its_colon_is_refused() {
    let source = module("type Point { x i32 }", "    let a = 1;\n    print dec a;\n");
    assert_eq!(codes(&source), ["Q051"]);
}

/// The field bound is a refusal this model owns, decided over the model rather
/// than through a source fixture: `MAX_FIELDS + 1` fields is past what it
/// enumerates, and it is refused rather than truncated.
#[test]
fn a_record_beyond_the_field_bound_is_refused() {
    let fields: Vec<RecordField> = (0..=MAX_FIELDS)
        .map(|index| RecordField {
            name: format!("f{index}"),
            field_type: FieldType::I32,
        })
        .collect();
    let declared = RecordType {
        name: "Wide".to_owned(),
        fields,
    };
    let refusal = source_record::admit_type(&declared).expect_err("past the bound");
    assert_eq!(refusal.code(), "Q053");
    assert!(matches!(refusal, RecordRefusal::TooManyFields { .. }));
}

#[test]
fn a_record_at_the_field_bound_is_admitted() {
    let fields: Vec<RecordField> = (0..MAX_FIELDS)
        .map(|index| RecordField {
            name: format!("f{index}"),
            field_type: FieldType::I32,
        })
        .collect();
    let declared = RecordType {
        name: "Wide".to_owned(),
        fields,
    };
    assert!(source_record::admit_type(&declared).is_ok());
}

// --- Construction totality. ------------------------------------------------

/// **The totality rule.** A construction supplies every declared field exactly
/// once, which is what makes a field read total on the declared names.
#[test]
fn a_construction_missing_a_field_is_refused() {
    // Nothing reads `p` afterwards, so the one diagnostic is the one rule
    // this test names rather than a cascade behind it.
    let source = module(
        "type Point { x: i32, y: i32 }",
        "    let p = Point { x: 1 };\n    print dec 0;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("supplies no `y`"));
}

#[test]
fn a_construction_supplying_an_undeclared_field_is_refused() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1, z: 2 };\n    print dec 0;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("declares no field `z`"));
}

#[test]
fn a_construction_supplying_one_field_twice_is_refused() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1, x: 2 };\n    print dec 0;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("supplies `x` twice"));
}

/// A construction naming a type no declaration introduces refuses at `Q013`,
/// **before** this construct's reader sees it: the type name is an ordinary
/// reference and every declaration's name is a global, which is why this card
/// needed no name-resolution rule of its own.
#[test]
fn a_construction_of_an_undeclared_type_refuses_at_name_resolution() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Other { x: 1 };\n    print dec 0;\n",
    );
    assert_eq!(codes(&source), ["Q013"]);
}

/// A field value is this route's own operand, so a **buffer** at a field
/// position is refused by this route's existing crossing rule rather than by
/// anything the aggregate added.
#[test]
fn a_buffer_at_a_field_position_is_refused_by_the_existing_rule() {
    let source = buffer_module(
        "type Point { x: i32 }",
        "    let b = bytes(4);\n    let p = Point { x: b };\n    drop b;\n",
    );
    assert_eq!(codes(&source), ["Q055"]);
}

#[test]
fn a_trailing_comma_in_a_construction_is_admitted() {
    let source = module(
        "type Point { x: i32, y: i32 }",
        "    let p = Point { x: 1, y: 2, };\n    let a = p.y;\n    print dec a;\n",
    );
    let module = compile(&source).expect("a trailing comma changes no token that means anything");
    let certificate = certificate(&module);
    assert_eq!(certificate.projections[0].value, 2);
}

#[test]
fn a_trailing_comma_in_a_declaration_is_admitted() {
    let source = module(
        "type Point { x: i32, y: i32, }",
        "    let p = Point { x: 1, y: 2 };\n    let a = p.y;\n    print dec a;\n",
    );
    assert!(compile(&source).is_ok());
}

#[test]
fn a_field_supplied_more_than_one_operand_is_refused() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 2 };\n    print dec 0;\n",
    );
    assert_eq!(codes(&source), ["Q051"]);
    assert!(messages(&source).contains("more than one operand"));
}

// --- Field access soundness, and CT09's distinctness. ----------------------

fn certificate(module: &qcore_compiler::CompiledModule) -> &std_buffer::BufferCertificate {
    match &module.checked {
        qcore_compiler::CheckedSourceModule::StandardBuffer(checked) => &checked.certificate,
        other => panic!("expected a byte-buffer module, got {other:?}"),
    }
}

/// **CT09's own scenario.** Three declared fields, three different values, three
/// reads: each read produces the value supplied for *its* field and no other.
/// A model that aliased two fields would produce one value twice.
#[test]
fn distinct_fields_read_back_the_values_supplied_for_them() {
    let module = compile(POINT).unwrap();
    let certificate = certificate(&module);
    let read: Vec<(&str, &str, i32)> = certificate
        .projections
        .iter()
        .map(|projection| {
            (
                projection.record.as_str(),
                projection.field.as_str(),
                projection.value,
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            ("p", "x", 65),
            ("p", "y", 66),
            ("p", "weight", 40),
            ("q", "weight", 9),
            ("q", "x", 1),
        ]
    );
}

/// The certificate names every construction and the value supplied for each
/// declared field, **in declared order** — so two certificates of one program
/// compare field for field however the source wrote them. `q` is written
/// `weight, y, x` and is published `x, y, weight`.
#[test]
fn the_certificate_publishes_every_construction_in_declared_order() {
    let module = compile(POINT).unwrap();
    let certificate = certificate(&module);
    let published: Vec<(&str, Vec<(&str, i32)>)> = certificate
        .constructions
        .iter()
        .map(|construction| {
            (
                construction.binding.as_str(),
                construction
                    .fields
                    .iter()
                    .map(|(name, value)| (name.as_str(), *value))
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        published,
        [
            ("p", vec![("x", 65), ("y", 66), ("weight", 40)]),
            ("q", vec![("x", 1), ("y", 2), ("weight", 9)]),
        ]
    );
}

/// Field access is a **function of the field name**, decided over the model:
/// projecting two different fields of one constructed value gives the two
/// values it was constructed with.
#[test]
fn projection_returns_what_construction_supplied() {
    let declared = RecordType {
        name: "Point".to_owned(),
        fields: vec![
            RecordField {
                name: "x".to_owned(),
                field_type: FieldType::I32,
            },
            RecordField {
                name: "y".to_owned(),
                field_type: FieldType::I32,
            },
        ],
    };
    let value = source_record::construct(
        &declared,
        &[
            ("y".to_owned(), FieldValue::Scalar(9)),
            ("x".to_owned(), FieldValue::Scalar(4)),
        ],
    )
    .expect("a complete field list");
    assert_eq!(value.scalar("x"), Some(4));
    assert_eq!(value.scalar("y"), Some(9));
    assert_eq!(value.project("z"), None);
}

#[test]
fn reading_a_field_the_type_does_not_declare_is_refused() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 };\n    let a = p.z;\n",
    );
    assert_eq!(codes(&source), ["Q052"]);
    assert!(messages(&source).contains("declares no field `z`"));
}

// --- The field-custody decision, from the accusing side. -------------------

/// **A record contributes no ownership operation at all.** That is the whole of
/// the v1 field-custody decision, stated where it can be checked: the cycle-3
/// checker is handed nothing for a record, because there is nothing about a
/// value of scalars for it to decide.
#[test]
fn a_record_contributes_no_custody_operation() {
    let program = BufferProgram {
        float_effects: Default::default(),
        entry: "main".to_owned(),
        declared_effects: qcore_core::typed_errors::EffectRow::default(),
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

/// A record operation derives **no effect**, so a record-only entry may declare
/// the empty row and still be admitted.
#[test]
fn a_record_derives_no_effect() {
    let source = "type Point { x: i32 }\n\nfn main() -> unit {\n    let p = Point { x: 1 };\n    \
                  let a = p.x;\n    return;\n}\n";
    let module = compile(source).expect("a record performs no effect to declare");
    let certificate = certificate(&module);
    assert!(certificate.effects.iter().next().is_none());
}

/// A record is not a resource, so a custody position naming one is refused by
/// this route's **existing** rule — the refusal is an existing route's rather
/// than a new one this card wrote.
#[test]
fn a_record_cannot_be_dropped() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 };\n    drop p;\n",
    );
    assert_eq!(codes(&source), ["Q052"]);
    assert!(messages(&source).contains("is not a buffer this entry acquired"));
}

/// A record at a position that takes an integer is refused, with the message
/// naming the thing this revision can actually do about it.
#[test]
fn a_record_at_a_rendering_operand_is_refused() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 };\n    print dec p;\n",
    );
    assert_eq!(codes(&source), ["Q052"]);
    assert!(messages(&source).contains("read the field you mean first"));
}

#[test]
fn a_record_at_an_index_position_is_refused() {
    let source = buffer_module(
        "type Point { x: i32 }",
        "    let b = bytes(4);\n    let p = Point { x: 1 };\n    b[p] = 1;\n    drop b;\n",
    );
    assert_eq!(codes(&source), ["Q052"]);
}

/// A record beside a buffer: the buffer's custody is decided exactly as it was,
/// and the record adds nothing to the ledger.
#[test]
fn a_record_beside_a_buffer_leaves_the_custody_discipline_unchanged() {
    let source = buffer_module(
        "type Header { len: i32 }",
        "    let b = bytes(4);\n    let n = extent(b);\n    let h = Header { len: n };\n    let l \
         = h.len;\n    b[0] = l;\n    print dec l;\n    drop b;\n",
    );
    let module = compile(&source).expect("the record neither adds nor removes an obligation");
    let certificate = certificate(&module);
    assert_eq!(certificate.acquisitions, [("b".to_owned(), 4)]);
    assert_eq!(certificate.releases, ["b".to_owned()]);
    assert_eq!(certificate.peak_live_bytes, 4);
}

/// A leak is still a leak beside a record: the aggregate launders nothing.
#[test]
fn a_record_does_not_launder_a_leak() {
    let source = buffer_module(
        "type Point { x: i32 }",
        "    let b = bytes(4);\n    let p = Point { x: 1 };\n    let a = p.x;\n    print dec a;\n",
    );
    assert_eq!(codes(&source), ["Q054"]);
    assert!(messages(&source).contains("live resources escape"));
}

// --- Composition with the bounded counted loop. ----------------------------

/// A record constructed in a loop body is body-local, exactly as a scalar and a
/// buffer binding are — one scoping rule over every kind of binding this route
/// holds.
#[test]
fn a_record_constructed_in_a_loop_body_does_not_outlive_it() {
    let source = module(
        "type Point { x: i32 }",
        "    for i in 0..2 {\n        let p = Point { x: i };\n        let a = p.x;\n        print \
         dec a;\n    }\n    let after = p.x;\n    print dec after;\n",
    );
    assert!(
        codes(&source).contains(&"Q013".to_owned()) || codes(&source).contains(&"Q052".to_owned()),
        "{:?}",
        codes(&source)
    );
}

/// A body binding shadowing a record the entry already made is refused at the
/// loop's own code, not resolved.
#[test]
fn a_loop_body_shadowing_a_record_is_refused() {
    let source = module(
        "type Point { x: i32 }",
        "    let p = Point { x: 1 };\n    for i in 0..2 {\n        let p = Point { x: i };\n        let a = p.x;\n        print dec a;\n    }\n",
    );
    assert!(
        codes(&source).contains(&"Q097".to_owned()),
        "{:?}",
        codes(&source)
    );
}

/// The loop runs the body once per iteration and the record is constructed
/// afresh each time, with the index as a field value.
#[test]
fn a_record_in_a_loop_body_is_constructed_once_per_iteration() {
    let source = module(
        "type Point { x: i32 }",
        "    for i in 0..3 {\n        let p = Point { x: i };\n        let a = p.x;\n        print \
         dec a;\n    }\n",
    );
    let module = compile(&source).expect("a record body is the identity on custody");
    let certificate = certificate(&module);
    assert_eq!(certificate.constructions.len(), 3);
    let values: Vec<i32> = certificate
        .projections
        .iter()
        .map(|projection| projection.value)
        .collect();
    assert_eq!(values, [0, 1, 2]);
}

// --- Route admission, measured. --------------------------------------------

/// A module that declares a record type reaches the byte-buffer route, which
/// models the aggregate — even with no buffer in it and no `std.buffer` import,
/// because a record is not a bundled-module capability.
#[test]
fn a_record_only_module_reaches_the_buffer_route() {
    let module = compile(POINT).unwrap();
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
}

/// **The route-selection change is the declaration and nothing else.** A module
/// that constructs a value of a type it does not declare fails the guard and
/// keeps the route it had — which is what makes this one new door opened only
/// by one new spelling.
#[test]
fn a_construction_without_a_declaration_keeps_the_route_it_had() {
    let source = "#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects { io } {\n    let p \
                  = Point { x: 1 };\n    print dec 1;\n    return;\n}\n";
    assert_eq!(codes(source), ["Q013"]);
}

/// Card `W48-QR-TASK-RECORD-CAPTURES` moved this row by name: the concurrent
/// route is still selected above this card's guard, and since that card it
/// reads the record itself, so the module is **admitted** on the task route
/// rather than refused there. What the row still pins is the route: the
/// aggregate route's own codes (`Q051`, `Q052`) are never reached.
#[test]
fn a_task_module_declaring_a_record_keeps_its_own_refusal() {
    // A genuine task statement, so the concurrent route actually selects.
    let source = "#import std.concurrency;\nconcurrency checked;\n\ntype Point { x: i32 }\n\nfn \
                  main() -> unit {\n    thread scope s { let p = Point { x: 1 }; }\n                      return;\n}\n";
    let module = compile(source).unwrap_or_else(|errors| {
        panic!("the task route now reads a record of scalars: {errors:#?}")
    });
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScopedCopyTasksV1,
        "the concurrent route is selected above this card's"
    );
}

/// A record construct outside the entry is never left unmodelled (invariant
/// one). Before card `W48-QR-USABLE-RECORDS` the buffer route, which lowers the
/// entry only, refused it by name ("models the entry only"); since that card a
/// module declaring a record and a second function, and holding no buffer,
/// reaches `source.scalar-calls.v1`, which **checks** every function: this
/// helper, which has no final `return`, is refused there, and the same helper
/// with one is admitted.
#[test]
fn a_record_outside_the_entry_is_checked_rather_than_going_unchecked() {
    let source = "#import std.fmt;\n#import std.io;\n\ntype Point { x: i32 }\n\nfn helper() -> unit \
                  {\n    let p = Point { x: 1 };\n}\n\nfn main() -> unit effects { io } {\n    let \
                  q = Point { x: 2 };\n    let a = q.x;\n    print dec a;\n    return;\n}\n";
    let messages = messages(source);
    assert!(
        messages.contains("source.scalar-calls.v1: a final explicit return is required"),
        "{messages}"
    );
    let returning = source.replace(
        "let p = Point { x: 1 };\n}",
        "let p = Point { x: 1 };\n    return;\n}",
    );
    let module = compile(&returning).expect("the helper is checked and admitted");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ScalarCallsV1
    );
}

/// The composed string/buffer route models no record, and says so rather than
/// emitting nothing for one.
#[test]
fn the_composed_string_route_refuses_a_record_statement() {
    let source = "#import std.buffer;\n#import std.string;\n#import std.fmt;\n#import std.io;\n\ntype \
                  Point { x: i32 }\n\nfn main() -> unit effects { io, alloc, mut } {\n    let b = \
                  bytes(2);\n    b[0] = 104;\n    b[1] = 105;\n    let s = borrow_utf8(b);\n    \
                  let p = Point { x: 1 };\n    print s;\n    drop b;\n    return;\n}\n";
    assert_eq!(
        codes(source),
        ["Q068"],
        "a record reaching the string route must be refused at that route's own code, never \
         silently unmodelled"
    );
}

/// The extern route is selected above this card's guard, so a module that
/// declares a record and calls a declared extern function keeps that route's
/// existing refusal — measured, not assumed (`AGGREGATE-ACROSS-CALL`).
#[test]
fn an_extern_module_declaring_a_record_keeps_its_own_refusal() {
    let source = "#import std.extern;\n\nextern fn qcore_probe(value: i32) -> i32\n    contract \
                  {\n        requires true;\n    }\n\ntype Point { x: i32 }\n\nfn main() -> unit \
                  {\n    let p = Point { x: 1 };\n    let r = qcore_probe(1);\n    return;\n}\n";
    let codes = codes(source);
    assert!(
        !codes.contains(&"Q051".to_owned()) && !codes.contains(&"Q052".to_owned()),
        "the extern route is selected above this card's and keeps its refusal: {codes:?}"
    );
}

/// The sequential cleanup route models no record either, and a module that
/// declares one no longer reaches it — this card's guard sends it to the buffer
/// route instead. **No program written before this card can take that door**,
/// because `type NAME { … }` did not parse at all.
#[test]
fn a_module_with_no_record_declaration_still_reaches_the_sequential_route() {
    let source = "fn main() -> i32 {\n    return 0;\n}\n";
    let module = compile(source).expect("an ordinary sequential entry");
    assert_ne!(
        module.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
}

// --- Emission. --------------------------------------------------------------

/// One C struct per declared type, with one distinctly named member per field,
/// in declared order. The prefixes are what make a QCore name that is a C
/// keyword still emit a compilable struct.
#[test]
fn the_emitted_c_declares_one_struct_per_record_type() {
    let module = compile(POINT).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(source.contains(
        "struct qcore_record_Point {\n    int32_t f_x;\n    int32_t f_y;\n    int32_t f_weight;\n};"
    ));
    assert_eq!(source.matches("struct qcore_record_Point {").count(), 1);
}

/// A construction is one designated initializer naming every declared field,
/// and a field read is one member read — so the emitted C is the program a C
/// reader would have written.
#[test]
fn the_emitted_c_constructs_and_reads_by_member() {
    let module = compile(POINT).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(source.contains(
        "struct qcore_record_Point r_p = { .f_x = INT32_C(65), .f_y = INT32_C(66), .f_weight = \
         s_base };"
    ));
    assert!(source.contains("int32_t s_a = r_p.f_x;"));
    assert!(source.contains("int32_t s_b = r_p.f_y;"));
}

/// A record, a buffer and a scalar of one source name are three distinct C
/// objects.
#[test]
fn the_three_binding_prefixes_are_distinct() {
    let source = buffer_module(
        "type Header { len: i32 }",
        "    let b = bytes(4);\n    let h = Header { len: 1 };\n    let n = h.len;\n    b[0] = \
         n;\n    print dec n;\n    drop b;\n",
    );
    let module = compile(&source).unwrap();
    let emitted = module.c_source.as_deref().unwrap();
    assert!(emitted.contains("unsigned char *b_b = qcore_bytes_new"));
    assert!(emitted.contains("struct qcore_record_Header r_h ="));
    assert!(emitted.contains("int32_t s_n = r_h.f_len;"));
}

/// **A program that acquires no buffer carries no buffer prelude**, which is
/// what makes a record-only module compile under `-Wall -Wextra -Werror`: the
/// helpers are `static`, and an unused one is an error.
#[test]
fn a_record_only_program_carries_no_buffer_prelude() {
    let module = compile(POINT).unwrap();
    let source = module.c_source.as_deref().unwrap();
    assert!(!source.contains("qcore_bytes_new"));
    assert!(!source.contains("qcore_bytes_drop"));
}

/// And a program that does acquire one still carries it, unchanged.
#[test]
fn a_buffer_program_still_carries_its_prelude() {
    let source = buffer_module(
        "type Header { len: i32 }",
        "    let b = bytes(4);\n    let h = Header { len: 1 };\n    let n = h.len;\n    b[0] = \
         n;\n    print dec n;\n    drop b;\n",
    );
    let module = compile(&source).unwrap();
    let emitted = module.c_source.as_deref().unwrap();
    assert!(emitted.contains("static unsigned char *qcore_bytes_new(size_t extent)"));
    assert!(emitted.contains("static void qcore_bytes_store("));
}

/// A module declaring no record contributes no fragment at all, so every
/// program written before this card travels an identical code path.
#[test]
fn a_module_without_a_record_carries_no_record_fragment() {
    let source = "#import std.buffer;\n#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects \
                  { io, alloc, mut } {\n    let b = bytes(2);\n    b[0] = 7;\n    let v = \
                  b[0];\n    print dec v;\n    drop b;\n    return;\n}\n";
    let module = compile(source).unwrap();
    let emitted = module.c_source.as_deref().unwrap();
    assert!(!emitted.contains("qcore_record_"));
    assert!(!emitted.contains("source.record.v1"));
}

// --- The certificate is never one the driver did not recompute. ------------

#[test]
fn a_tampered_projection_fails_the_recheck() {
    let module = compile(POINT).unwrap();
    let lowered = match &module.checked {
        qcore_compiler::CheckedSourceModule::StandardBuffer(checked) => &checked.lowered,
        other => panic!("expected a byte-buffer module, got {other:?}"),
    };
    let mut certificate = certificate(&module).clone();
    certificate.projections[0].value += 1;
    assert_eq!(
        std_buffer::recheck_buffer_certificate(&lowered.program, &certificate),
        Err(std_buffer::BufferRecheckFailure::Projections)
    );
}

#[test]
fn a_tampered_construction_fails_the_recheck() {
    let module = compile(POINT).unwrap();
    let lowered = match &module.checked {
        qcore_compiler::CheckedSourceModule::StandardBuffer(checked) => &checked.lowered,
        other => panic!("expected a byte-buffer module, got {other:?}"),
    };
    let mut certificate = certificate(&module).clone();
    certificate.constructions[0].fields[0].1 += 1;
    assert_eq!(
        std_buffer::recheck_buffer_certificate(&lowered.program, &certificate),
        Err(std_buffer::BufferRecheckFailure::Constructions)
    );
}

// --- The native run. --------------------------------------------------------

/// The card's executable evidence, and the reason distinctness is a *measured*
/// property rather than a modelled one: the emitted C reads three distinct
/// struct members, and the bytes it writes are compared with the reference
/// observation per stream. A lowering that gave two fields one member would
/// print one value twice and this test would fail.
#[test]
fn the_emitted_program_matches_the_reference_per_stream() {
    let module = compile(POINT).unwrap();
    let observation = module.extern_observation.as_deref().unwrap();
    let expected_out: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: "))
        .collect();
    let expected_err: Vec<&str> = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: "))
        .collect();
    assert_eq!(expected_out, ["65", "66", "40"]);
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "point");
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// A record owns nothing, so this run is a **negative** allocation control
/// rather than a positive leak control: the program allocates nothing at all,
/// and LSan reporting a leak would mean the aggregate had acquired something
/// the model says it cannot. Apple ASan provides no LSan, so leak detection
/// stays a Linux-only observation.
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
            "qcore-source-record-{_label}-{}-{}",
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
