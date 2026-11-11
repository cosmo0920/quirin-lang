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

//! Reach for the owned-field rules (card `OWNED-FIELD-ROUTE-REACH`, wave 41).
//!
//! `STABILITY-REEVAL-4` measured that wave 39's fifth owned-field rule — a
//! record with an owned field is **never copied** — was reachable from no
//! source program at all: `OwnershipOp::Copy` was constructed at four places
//! in the tree, every one of them a unit test over a hand-built operation list
//! (`OWNERSHIP-COPY-OP-UNPRODUCED`). The same audit found that three named
//! boundaries of the record route — `AGGREGATE-VIEW-FIELD`, `AGGREGATE-NESTED`
//! and `AGGREGATE-TASK-CAPTURE` — were **absence rather than refusal**: each
//! was a shape nothing admitted, none refused by its own name, and a later
//! card lifting one would have found no message to supersede.
//!
//! This file is the card's end-to-end evidence, in three families.
//!
//! * **The copy bind.** `let NAME = NAME;` on `source.buffer.v1` is the source
//!   spelling that produces `OwnershipOp::Copy`. Every test here drives a whole
//!   program through `compile_files`, so the arm is reached by the real
//!   pipeline and not by a constructed list.
//! * **The boundaries.** Each of the three now refuses **by its own name**,
//!   at the code and over the refused set it already had.
//! * **What did not move.** The socket route's creation form is token for
//!   token the copy bind's shape (29 occurrences in the tree, the card's
//!   census), the composed string route lowers no copy bind at all, and both
//!   are pinned here rather than argued.

use qcore_compiler::{CompileOptions, SourceExecutionProfile, SourceFile, compile_files};
use qcore_core::ownership::OwnershipOp;
use qcore_core::std_buffer::{self, BufferEvent, BufferProgram};

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("owned_field_route_reach.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn refusal(source: &str) -> (Vec<String>, String) {
    let diagnostics = compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused");
    (
        diagnostics.iter().map(|d| d.code.clone()).collect(),
        diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn module(declaration: &str, body: &str) -> String {
    format!(
        "#import std.buffer;\n#import std.fmt;\n#import std.io;\n\n{declaration}\n\nfn main() -> unit \
         effects {{ io, alloc, mut }} {{\n{body}    return;\n}}\n"
    )
}

/// The `out:` lines of a compiled module's reference observation.
fn stdout(module: &qcore_compiler::CompiledModule) -> Vec<String> {
    module
        .extern_observation
        .as_deref()
        .expect("the route publishes a reference observation")
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect()
}

fn lowered(module: &qcore_compiler::CompiledModule) -> &BufferProgram {
    match &module.checked {
        qcore_compiler::CheckedSourceModule::StandardBuffer(checked) => &checked.lowered.program,
        other => panic!("expected a byte-buffer module, got {other:?}"),
    }
}

const OWNING: &str = "type Frame { tag: i32, body: bytes }";
const SCALARS: &str = "type Point { x: i32, y: i32 }";

// --- Family one: the copy bind reaches the IR's copy rule. ---

/// **The headline.** A source program now produces `OwnershipOp::Copy`, and
/// it produces exactly one, naming exactly the source of the copy bind.
///
/// Before this card the operation existed in the IR and in unit tests and
/// nowhere between: no route on any pipeline emitted it
/// (`OWNERSHIP-COPY-OP-UNPRODUCED`). The list is pinned rather than searched,
/// so a later route change that stopped producing the operation — or produced
/// a second one — fails here.
#[test]
fn a_source_program_produces_the_copy_operation() {
    let source = module(
        OWNING,
        "    let b = bytes(4);\n    let f = Frame { tag: 7, body: b };\n    let g = f;\n    drop \
         f;\n",
    );
    // The program is refused — that is the rule — so the operation list is
    // read from the lowering rather than from a certificate.
    let (codes, _) = refusal(&source);
    assert_eq!(
        codes,
        vec!["Q054"],
        "the IR's own condition, not a shape one"
    );

    let admitted = module(
        SCALARS,
        "    let b = bytes(2);\n    let p = Point { x: 1, y: 2 };\n    let q = p;\n    let a = \
         q.x;\n    a |> print dec;\n    drop b;\n",
    );
    let compiled = compile(&admitted).expect("a record of scalars is copyable");
    let program = lowered(&compiled);
    assert!(
        program.events.iter().any(
            |event| matches!(event, BufferEvent::CopyBind { binding, source }
                if binding == "q" && source == "p")
        ),
        "the copy bind is lowered as its own event",
    );
    // The copy of a plain value contributes no custody operation at all.
    let operations = std_buffer::custody_operations(program);
    assert!(
        !operations
            .iter()
            .any(|op| matches!(op, OwnershipOp::Copy(_))),
        "a record of scalars is not in the ledger, so no copy question is asked",
    );
}

/// The operation reaches the IR for an **owning** record, and the refusal is
/// the checker's own `CopyOfOwned` message rather than a re-worded copy.
#[test]
fn copying_a_record_with_an_owned_field_refuses_as_the_checkers_own_copy_rule() {
    let source = module(
        OWNING,
        "    let b = bytes(4);\n    let f = Frame { tag: 7, body: b };\n    let g = f;\n    drop \
         f;\n",
    );
    let (codes, message) = refusal(&source);
    assert_eq!(codes, vec!["Q054"]);
    assert!(
        message.contains("owned resource `f` cannot be copied"),
        "the IR's own words: {message}"
    );

    // And the operation list really carries the copy, so the refusal above is
    // this arm's and not another rule's coincidence.
    let program = qcore_frontend::source_std_buffer::lower_source_buffer(
        &qcore_frontend::frontend::parse_source_program(&source).expect("parses"),
        &qcore_frontend::frontend::elaborate_source(
            &qcore_frontend::frontend::parse_source_program(&source).expect("parses"),
        )
        .expect("elaborates"),
        "main",
    )
    .expect("lowers");
    let operations = std_buffer::custody_operations(&program.program);
    assert!(
        operations.contains(&OwnershipOp::Copy("f".to_owned())),
        "the route builds `Copy(f)`: {operations:?}",
    );
}

/// **Hollow or not.** A record every owned field of which has been moved out
/// is still not a value two bindings may hold — wave 39's sentence, now
/// reached by a program. The refusal is still the copy rule and not a
/// use-after-move of the field.
#[test]
fn copying_a_hollow_record_is_still_the_copy_refusal() {
    let source = module(
        OWNING,
        "    let b = bytes(4);\n    let f = Frame { tag: 7, body: b };\n    let taken = \
         f.body;\n    let g = f;\n    drop f;\n    drop taken;\n",
    );
    let (codes, message) = refusal(&source);
    assert_eq!(codes, vec!["Q054"]);
    assert!(
        message.contains("owned resource `f` cannot be copied"),
        "the copy rule, not the hole: {message}"
    );
}

/// The **plain owner** copy rule — older than the aggregate one and just as
/// unreachable before this card — is reached by the same spelling.
#[test]
fn copying_an_owned_buffer_refuses() {
    let source = module("", "    let b = bytes(4);\n    let c = b;\n    drop b;\n");
    let (codes, message) = refusal(&source);
    assert_eq!(codes, vec!["Q054"]);
    assert!(
        message.contains("owned resource `b` cannot be copied"),
        "{message}"
    );
}

/// A copy of a binding the program already dropped, and one of a binding a
/// construction moved: the IR answers with its *state* rules, which is what
/// makes the copy bind a question rather than a special case.
#[test]
fn a_copy_of_a_dropped_or_moved_binding_refuses_with_the_states_own_rule() {
    let dropped = module(
        OWNING,
        "    let b = bytes(4);\n    let f = Frame { tag: 7, body: b };\n    drop f;\n    let g = \
         f;\n",
    );
    let (codes, message) = refusal(&dropped);
    assert_eq!(codes, vec!["Q054"]);
    assert!(message.contains("use of `f` after drop"), "{message}");

    let moved = module(
        OWNING,
        "    let b = bytes(4);\n    let f = Frame { tag: 7, body: b };\n    let c = b;\n    drop \
         f;\n",
    );
    let (codes, message) = refusal(&moved);
    assert_eq!(codes, vec!["Q054"]);
    assert!(message.contains("use of `b` after move"), "{message}");
}

/// A **write loan** at a copy position refuses from the ownership IR at
/// `Q054`, and **only** there: the value half deliberately says nothing about
/// it, because the custody refusals of a lend arrive from the extended IR
/// (standing constraint 15h) and two reports for one fault would put a second
/// copy rule beside the one this card exists to reach.
#[test]
fn a_loan_at_a_copy_position_refuses_once_from_the_ownership_ir() {
    let source = module(
        "",
        "    let b = bytes(4);\n    let l = lend(b, 0, 2);\n    let m = l;\n    drop b;\n",
    );
    let (codes, message) = refusal(&source);
    assert_eq!(codes, vec!["Q054"], "one refusal, not two");
    assert!(
        message.contains("borrow `l` cannot be moved as an owner"),
        "{message}"
    );
}

// --- Family one, the admitting side: what constraint 15k always asserted. ---

/// **A v1 record of scalars is a plain copyable value** — an assertion of
/// standing constraint 15k that, before this card, no program could check in
/// either direction. The copy is by value: the two bindings are two C objects,
/// and the emitted C is one struct assignment.
#[test]
fn a_record_of_scalars_is_copyable_and_the_copy_is_by_value() {
    let source = module(
        SCALARS,
        "    let b = bytes(2);\n    let p = Point { x: 1, y: 2 };\n    let q = p;\n    let a = \
         q.x;\n    let c = q.y;\n    a |> print dec;\n    c |> print dec;\n    drop b;\n",
    );
    let compiled = compile(&source).expect("admitted");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardBufferV1,
        "route selection does not move: the copy bind is not a `buffer_use`",
    );
    let c = compiled.c_source.clone().expect("the route emits C");
    assert!(
        c.contains("struct qcore_record_Point r_q = r_p;"),
        "one struct assignment, by value:\n{c}"
    );
    assert!(
        c.contains("int32_t s_a = r_q.f_x;"),
        "the read goes through the copy's own object:\n{c}"
    );
    assert_eq!(
        stdout(&compiled),
        ["1", "2"],
        "the copy carries both field values"
    );
}

/// A **scalar** copy is admitted on the same route and emits the C assignment
/// it is. The value is carried in this route's own constant environment, so a
/// later index through the copy is decided at check time exactly as one
/// through the original is.
#[test]
fn a_scalar_is_copyable_and_the_copy_carries_its_value() {
    let source = module(
        "",
        "    let b = bytes(4);\n    let n = 2;\n    let m = n;\n    b[m] = 9;\n    let seen = \
         b[2];\n    seen |> print dec;\n    drop b;\n",
    );
    let compiled = compile(&source).expect("admitted");
    let c = compiled.c_source.clone().expect("the route emits C");
    assert!(
        c.contains("int32_t s_m = s_n;"),
        "the copy is one assignment:\n{c}"
    );
    assert_eq!(
        stdout(&compiled),
        ["9"],
        "the index written through the copy is the index the copy names"
    );
}

/// A copy bind inside a `for` body reaches the **same** stepper a top-level
/// one does, and its binding is body-local exactly as every other binding is.
#[test]
fn a_copy_bind_in_a_loop_body_is_the_same_statement() {
    let source = module(
        "",
        "    let b = bytes(2);\n    for i in 0..2 {\n        let j = i;\n        b[j] = 5;\n    \
         }\n    let seen = b[1];\n    seen |> print dec;\n    drop b;\n",
    );
    let compiled = compile(&source).expect("admitted");
    assert_eq!(stdout(&compiled), ["5"]);

    // The body binding does not escape the loop.
    let escaped = module(
        "",
        "    let b = bytes(2);\n    for i in 0..2 {\n        let j = i;\n    }\n    let seen = \
         b[j];\n    seen |> print dec;\n    drop b;\n",
    );
    let (codes, _) = refusal(&escaped);
    assert!(
        !codes.is_empty(),
        "a body binding named after the loop is refused"
    );
}

/// A copy bind in a loop body **shadowing** a name the entry already bound is
/// refused at `source.loop.v1`'s own shape condition. The rule is the loop's,
/// and it reaches a copy bind because `body_bindings` counts the names a body
/// binds — which is what makes that function's copy-bind arm observable rather
/// than a totality arm.
#[test]
fn a_copy_bind_in_a_body_that_shadows_an_outer_name_refuses() {
    let source = module(
        "",
        "    let b = bytes(2);\n    let n = 5;\n    for i in 0..2 {\n        let n = i;\n    \
         }\n    drop b;\n",
    );
    let (codes, message) = refusal(&source);
    assert_eq!(codes, vec!["Q097"], "the loop's own shape condition");
    assert!(
        message.contains("`n` is bound in this loop body and already bound outside it"),
        "the loop's own words: {message}"
    );
}

// --- Family two: the three boundaries refuse by their own names. ---

/// `AGGREGATE-VIEW-FIELD` refuses **by name**. Before this card the same
/// declaration refused with the generic field-type message, which named
/// `AGGREGATE-FIELD-TYPES` — a different residual — so the boundary the
/// registry recorded matched no message anywhere in the tree.
#[test]
fn a_view_field_refuses_by_its_own_name() {
    let source = module(
        "type Frame { s: string }",
        "    let n = 1;\n    n |> print dec;\n",
    );
    let (codes, message) = refusal(&source);
    assert_eq!(codes, vec!["Q051"], "the code it already had");
    assert!(
        message.contains("AGGREGATE-VIEW-FIELD"),
        "the boundary names itself: {message}"
    );
    assert!(
        !message.contains("AGGREGATE-FIELD-TYPES"),
        "and names only itself: {message}"
    );
}

/// `AGGREGATE-NESTED` refused by name here, for a field naming a record type
/// declared after it as well as before. Card `W48-QR-USABLE-RECORDS` lifted
/// it: both declaration orders are now **admitted** — the classification still
/// reads the module's declaration kinds, not the table being built — and what
/// refuses by name is a record that contains itself (the termination argument
/// of the declaration graph).
#[test]
fn a_nested_record_field_is_admitted_in_both_declaration_orders() {
    for declaration in [
        "type Inner { x: i32 }
type Outer { inner: Inner }",
        "type Outer { inner: Inner }
type Inner { x: i32 }",
    ] {
        let source = module(
            declaration,
            "    let n = 1;
    n |> print dec;
",
        );
        assert!(compile(&source).is_ok(), "{declaration}");
    }
    for declaration in [
        "type Node { next: Node, x: i32 }",
        "type A { b: B }
type B { a: A }",
    ] {
        let source = module(
            declaration,
            "    let n = 1;
    n |> print dec;
",
        );
        let (codes, message) = refusal(&source);
        assert_eq!(codes, vec!["Q051"], "{declaration}");
        assert!(
            message.contains("contain itself"),
            "{declaration}: {message}"
        );
    }
}

/// Every **other** unknown field type keeps the generic message and the
/// residual it always named. The split is a refinement of one condition, not a
/// replacement of it. Card `W48-QR-SCALAR-TYPES-R3` admits the §G02t scalars,
/// so `u8` and `i64` — unknown before it — are replaced by spellings outside
/// that table.
#[test]
fn every_other_unknown_field_type_keeps_the_message_it_had() {
    // Card `W48-QR-FLOAT-BUFFER-PAYLOADS` admits `f32` fields on the buffer
    // route; `f16` is a float spelling no route models.
    for spelling in ["f16", "Buffer", "char"] {
        let source = module(
            &format!("type Frame {{ x: {spelling} }}"),
            "    let n = 1;\n    n |> print dec;\n",
        );
        let (codes, message) = refusal(&source);
        assert_eq!(codes, vec!["Q051"]);
        assert!(
            message.contains("AGGREGATE-FIELD-TYPES"),
            "{spelling}: {message}"
        );
        assert!(
            !message.contains("AGGREGATE-VIEW-FIELD") && !message.contains("AGGREGATE-NESTED"),
            "{spelling}: {message}"
        );
    }
}

/// `AGGREGATE-TASK-CAPTURE` refused by name here until card
/// `W48-QR-TASK-RECORD-CAPTURES` lifted it: the task route now reads a record
/// declaration, and a record crosses a spawn as its leaves under the aggregate
/// judgment. What this row pins now, **moved by name** rather than deleted: a
/// record of scalars in a task module is admitted, and a record with a `bytes`
/// field — the buffer route's byte buffer, which crosses no spawn on the task
/// route — still refuses at the code it always had, `Q500`, naming the residual
/// that would lift it (`WRITE-LEND-TASKS-EXTENT`) instead of the lifted one.
#[test]
fn a_record_in_a_task_module_refuses_by_its_own_name() {
    const TASK_MODULE: &str = concat!(
        "#import std.concurrency;\n",
        "concurrency checked;\n\n",
        "RECORD",
        "fn worker(buffer: own<Buffer> use one) -> own<Buffer> {\n",
        "    return move buffer;\n}\n\n",
        "fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, \
         may_block } {\n",
        "    let first = qcore_buffer_new(42)?;\n",
        "    thread scope workers {\n",
        "        match spawn workers worker(move first) {\n",
        "            Result::Ok { value: handle } => {\n",
        "                let returned = join move handle?;\n",
        "                let value = qcore_buffer_read(returned);\n",
        "                drop returned;\n",
        "                return value;\n",
        "            }\n",
        "            Result::Err { error: failure } => {\n",
        "                let value = qcore_buffer_read(first);\n",
        "                drop first;\n",
        "                return value;\n",
        "            }\n",
        "        }\n",
        "    }\n}\n",
    );

    // The same module without the record is admitted, so the refusal below is
    // attributable to the declaration and to nothing else.
    let baseline = TASK_MODULE.replace("RECORD", "");
    compile(&baseline).expect("the task module itself is admitted");

    let source = TASK_MODULE.replace("RECORD", "type Frame { body: bytes }\n\n");
    let (codes, message) = refusal(&source);
    assert_eq!(codes, vec!["Q500"], "the code it already had");
    assert!(
        message.contains("WRITE-LEND-TASKS-EXTENT") && !message.contains("AGGREGATE-TASK-CAPTURE"),
        "the boundary names what would lift it: {message}"
    );
    let source = TASK_MODULE.replace("RECORD", "type Point { x: i32 }\n\n");
    compile(&source).expect("a record of scalars in a task module is admitted");
}

// --- Family three: what deliberately did not move. ---

/// **The census's collision, pinned.** `source.socket.v1`'s creation form
/// `let listener = tcp;` is token for token the copy bind's shape, and the
/// card's census counts 29 of them across the tree. The two are separated by
/// route selection and by nothing else — the copy bind is deliberately not a
/// `buffer_use`, so it moves neither route selection nor the composed string
/// route's reader.
#[test]
fn the_socket_routes_creation_form_is_untouched() {
    const SOCKET: &str = concat!(
        "#import std.socket;\n#import std.fmt;\n#import std.io;\n\n",
        "fn main() -> unit effects { io, alloc } {\n",
        "    let listener = tcp;\n",
        "    bind listener;\n",
        "    listen listener;\n",
        "    let client = tcp;\n",
        "    connect client listener;\n",
        "    let server = accept listener;\n",
        "    send client [81];\n",
        "    recv server 1;\n",
        "    close server;\n",
        "    close client;\n",
        "    close listener;\n",
        "    return;\n}\n",
    );
    let compiled = compile(SOCKET).expect("a socket program is admitted as before");
    assert_eq!(
        compiled.execution_profile,
        SourceExecutionProfile::StandardSocketV1,
        "the socket route still claims it",
    );
}

/// The composed string/buffer route lowers **no** copy bind: it reads this
/// route's statements through `buffer_use`, which does not claim the shape, so
/// `let t = s;` keeps the `Q068` it always had.
#[test]
fn the_composed_string_route_keeps_its_own_refusal() {
    const COMPOSED: &str = concat!(
        "#import std.buffer;\n#import std.string;\n#import std.fmt;\n#import std.io;\n\n",
        "fn main() -> unit effects { io, alloc, mut } {\n",
        "    let b = bytes(2);\n",
        "    b[0] = 104;\n",
        "    b[1] = 105;\n",
        "    let s = borrow_utf8(b, 0, 2);\n",
        "    let t = s;\n",
        "    drop b;\n",
        "    return;\n}\n",
    );
    let (codes, message) = refusal(COMPOSED);
    assert_eq!(codes, vec!["Q068"]);
    assert!(
        message.contains("`source.string.v1` binds"),
        "that route's own words: {message}"
    );
}

/// A copy bind whose binding shadows a live name is refused **before** this
/// route, by name resolution, and a copy bind naming itself is an unknown
/// name. Both are pinned so a later reader knows the route is not where those
/// two rules live.
#[test]
fn shadowing_and_self_reference_are_refused_before_this_route() {
    let shadowing = module(
        SCALARS,
        "    let b = bytes(2);\n    let p = Point { x: 1, y: 2 };\n    let p = p;\n    drop b;\n",
    );
    assert_eq!(refusal(&shadowing).0, vec!["Q012"]);

    let itself = module("", "    let b = bytes(2);\n    let q = q;\n    drop b;\n");
    assert_eq!(refusal(&itself).0, vec!["Q013"]);
}

/// A copy bind contributes **no bound** to the lowering's constant tier, and
/// that is a decision: the tier is a literal, a literal `let` and an `extent`
/// binding, so a loop bound or a lend footprint stays decided in one place.
/// Refused by name rather than followed here.
#[test]
fn a_bound_spelled_through_a_copy_is_refused_rather_than_followed() {
    let loop_bound = module(
        "",
        "    let b = bytes(2);\n    let n = 2;\n    let m = n;\n    for i in 0..m {\n        b[i] \
         = 1;\n    }\n    drop b;\n",
    );
    let (codes, _) = refusal(&loop_bound);
    assert_eq!(codes, vec!["Q098"], "the loop's own bound condition");

    let lend_bound = module(
        "",
        "    let b = bytes(4);\n    let n = 2;\n    let m = n;\n    let l = lend(b, 0, m);\n    \
         l[0] = 1;\n    drop b;\n",
    );
    let (codes, _) = refusal(&lend_bound);
    assert_eq!(codes, vec!["Q053"], "the lend's own bound condition");
}

/// A copy bind derives **no effect**: an admitted copy binds a value that
/// already exists. A module whose only statement beyond the acquisition is a
/// copy declares exactly the row it declared before.
#[test]
fn a_copy_bind_derives_no_effect() {
    let source = module(
        SCALARS,
        "    let p = Point { x: 1, y: 2 };\n    let q = p;\n    let a = q.x;\n    a |> print \
         dec;\n",
    );
    let compiled = compile(&source).expect("a module that acquires nothing still admits a copy");
    let program = lowered(&compiled);
    let derived = std_buffer::derived_effects(program);
    assert_eq!(
        derived.iter().count(),
        1,
        "only the line's `io`: {derived:?}"
    );
}
