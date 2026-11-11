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

//! The **intrinsic pipe stage** (card `PIPE-INTRINSIC-STAGE`).
//!
//! Card `COMPOSE-CONCURRENCY` filed this follow-on for the brief's literal
//! `buffer |> read |> render |> log`, recording that "there is no `read`
//! pipeline stage: the buffer read is `qcore_buffer_read(NAME)`, a task-route
//! intrinsic with its own `let`". The census this card starts from measures the
//! source language's model intrinsics and finds they fall into two shapes:
//!
//! * **call-shaped** — `qcore_buffer_new`, `qcore_buffer_read`,
//!   `qcore_buffer_write`, `qcore_error_payload` on the task routes, and
//!   `qcore_acquire`, `qcore_check` on the sequential route. Each is spelled
//!   `NAME(ARGS)` and each already has a statement form, so `source.pipe.v1`'s
//!   **generic call stage** produces exactly that statement form with the
//!   hygienic temporary. No stage table entry, no new keyword and no new
//!   diagnostic code is needed: what the wave-21 note was missing is the
//!   spelling, not the mechanism.
//! * **keyword-shaped** — `spawn`, `join`, `share`, `read`, `write`, `drop`,
//!   `grow`, `open`, `close`, `bound`, `overhead`. These are statements whose
//!   operands are not a parenthesized argument list, so a call stage cannot
//!   produce them and the closed sink table deliberately does not name them.
//!   Each is refused, and each refusal is pinned below by name. (`drop` was the
//!   one this card withheld rather than settled; card `PIPE-DROP-STAGE` named
//!   it in the table, and the pin here moved by name with it.)
//!
//! One route could tell a `|>` had been written, and that is what this card
//! lands in production code: the sequential route re-lexed the **raw source
//! slice** of each statement instead of reading the program's own
//! post-desugaring token stream, so its two call-shaped intrinsics refused
//! through a pipe while their hand-written twins were admitted. See §4.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, CompiledModule, SourceExecutionProfile, SourceFile, compile_files,
    parse_source_program,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use qcore_core::prelude::concurrency::task_execution::{
    TaskExecutionScript, execute_with_source_lines,
};

fn compile(source: &str) -> Result<CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("intrinsic.qcore", source)],
        &CompileOptions::default(),
    )
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code.to_owned())
        .collect()
}

fn message(source: &str) -> String {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>()
        .join(" | ")
}

// ---------------------------------------------------------------- the modules

/// A composed `source.std-io-tasks.v1` module: a worker owns a buffer, the
/// parent joins it back and says something about it. This is the shape
/// `examples/threads/owned_buffer.qr` has.
const COMPOSED: &str = "#import std.concurrency;
#import std.logger;
#import std.fmt;
concurrency checked;

fn worker(buffer: own<Buffer> use one) -> own<Buffer> {
    let seen = qcore_buffer_read(buffer);
    return move buffer;
}

fn main() -> i32 effects { io, alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    let first = qcore_buffer_new(42)?;
    thread scope workers {
        match spawn workers worker(move first) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
BODY
                let value = qcore_buffer_read(returned);
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(first);
                drop first;
                return value;
            }
        }
    }
}
";

fn composed(body: &str) -> String {
    COMPOSED.replace("BODY", body)
}

/// The same module on the plain task route: no `std.io`, so the body's last
/// statement must bind `value` itself.
const TASKS: &str = "#import std.concurrency;
concurrency checked;

fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }

fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
ALLOC
    thread scope workers {
        match spawn workers worker(move first) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
BODY
                drop returned;
                return value;
            }
            Result::Err { error: failure } => {
                let value = qcore_buffer_read(first);
                drop first;
                return value;
            }
        }
    }
}
";

fn tasks(body: &str) -> String {
    TASKS
        .replace("ALLOC", "    let first = qcore_buffer_new(42)?;")
        .replace("BODY", body)
}

/// The same module with its *entry* allocation written however the caller likes.
/// The allocation is the one point where nothing else is live, so a refusal here
/// is about the allocation rather than about what the error arm would leak.
fn tasks_allocating(allocation: &str) -> String {
    TASKS.replace("ALLOC", allocation).replace(
        "BODY",
        "                let value = qcore_buffer_read(returned);",
    )
}

/// A `SequentialCleanupV1` entry.
fn sequential(body: &str) -> String {
    format!("error Failure {{ Failed; }}\nfn main() -> i32 {{\n{body}\n}}\n")
}

// ---------------------------------------------------------------------------
// 1. The payoff: the census's honest spelling of `buffer |> read |> log`
// ---------------------------------------------------------------------------

#[test]
fn a_buffer_read_is_a_non_final_pipe_stage_on_the_composed_task_route() {
    // The brief's `buffer |> read |> render |> log`, in the vocabulary the
    // language actually has: `read` is spelled `qcore_buffer_read`, and the
    // rendering selector `dec` is the sink's own word rather than a `render`
    // stage of its own (card `PIPE-SCALAR-RESULT` measured the same thing).
    let module = compile(&composed(
        "                returned |> qcore_buffer_read |> log info dec;",
    ))
    .expect("an intrinsic call stage feeds the sink it was piped into");
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ComposedStdIoTasksV1
    );
}

#[test]
fn the_intrinsic_chain_and_its_hand_written_twin_are_the_same_program() {
    // The strongest available statement that the desugaring adds no semantics:
    // `syntax_equivalent` compares the whole post-parse token stream. The twin
    // spells the temporary the desugaring invents, because that is exactly the
    // statement sequence a reader would have written.
    let piped = parse_source_program(&composed(
        "                returned |> qcore_buffer_read |> log info dec;",
    ))
    .unwrap();
    let plain = parse_source_program(&composed(
        "                let qcore__pipe0 = qcore_buffer_read(returned);\n                log info dec qcore__pipe0;",
    ))
    .unwrap();
    assert!(
        piped.syntax_equivalent(&plain),
        "an intrinsic stage must parse to its hand-written statement sequence"
    );
}

#[test]
fn the_intrinsic_chain_emits_byte_identical_c_against_its_hand_written_twin() {
    // And the temporary's *name* is not observable: the twin below binds a name
    // a reader would actually choose, and the emitted C is still equal, because
    // the task backend names slots rather than source identifiers.
    let piped = compile(&composed(
        "                returned |> qcore_buffer_read |> log info dec;",
    ))
    .unwrap();
    let twin = compile(&composed(
        "                let rendered = qcore_buffer_read(returned);\n                log info dec rendered;",
    ))
    .unwrap();
    assert_eq!(
        piped.c_source, twin.c_source,
        "an intrinsic stage must emit the C its hand-written statement emits"
    );
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

// ---------------------------------------------------------------------------
// 2. The census: every call-shaped intrinsic is a stage
// ---------------------------------------------------------------------------

#[test]
fn a_buffer_allocation_is_a_stage_and_keeps_its_adapter_tail() {
    // `qcore_buffer_new(N)?` carries a `?` after its argument list. The call
    // stage passes an adapter tail through verbatim, so the piped form is the
    // hand-written allocation, `?` included.
    let piped = compile(&tasks_allocating(
        "    let first = 42 |> qcore_buffer_new?;",
    ))
    .expect("an allocation is a call stage");
    let twin = compile(&tasks_allocating("    let first = qcore_buffer_new(42)?;")).unwrap();
    assert_eq!(piped.c_source, twin.c_source);
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn a_buffer_write_is_a_stage_and_the_piped_buffer_is_its_first_argument() {
    // `qcore_buffer_write(BUFFER, VALUE)` takes two operands, so this is the
    // first-argument insertion rule on an intrinsic: `b |> f(7)` is `f(b, 7)`.
    let piped = compile(&tasks(
        "                let value = qcore_buffer_read(returned);\n                let stored = returned |> qcore_buffer_write(7);",
    ))
    .expect("a buffer write is a call stage");
    let twin = compile(&tasks(
        "                let value = qcore_buffer_read(returned);\n                let stored = qcore_buffer_write(returned, 7);",
    ))
    .unwrap();
    assert_eq!(piped.c_source, twin.c_source);
}

/// The recovery program of `crates/qcore-frontend/tests/error_payloads.rs`, with
/// the parent's payload take written as `RECOVER`.
const PAYLOAD: &str = "#import std.concurrency; concurrency checked;
error Recovery { Failed(own<Buffer>); }
fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
  match qcore_buffer_new(7) {
    Result::Ok { value: first } => {
      let seen = qcore_buffer_read(payload);
      drop payload;
      drop first;
      return seen;
    }
    Result::Err { error: exhausted } => {
      raise Recovery::Failed(payload);
    }
  }
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
  let buffer = qcore_buffer_new(42)?;
  thread scope s {
    match spawn s worker(move buffer) {
      Result::Ok { value: handle } => {
        match join move handle {
          Result::Ok { value: seen } => {
            return seen;
          }
          Result::Err { error: failure } => {
            RECOVER
            let value = qcore_buffer_read(recovered);
            drop recovered;
            return value;
          }
        }
      }
      Result::Err { error: refused } => {
        drop buffer;
        return 0;
      }
    }
  }
}";

#[test]
fn an_error_payload_take_is_a_stage() {
    // `qcore_error_payload(E)` moves an owned Buffer out of a bound typed error.
    // It is the census's one *custody-transferring* call-shaped intrinsic, and
    // it is a stage on the same terms as the rest — the desugaring produces the
    // route's own `let` form, and the ledger that moves the payload is the
    // pre-existing one.
    let piped =
        compile(&PAYLOAD.replace("RECOVER", "let recovered = failure |> qcore_error_payload;"))
            .expect("a payload take is a call stage");
    let twin =
        compile(&PAYLOAD.replace("RECOVER", "let recovered = qcore_error_payload(failure);"))
            .unwrap();
    assert_eq!(piped.c_source, twin.c_source);
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn the_sequential_routes_acquisition_is_a_stage() {
    // `qcore_acquire(ID, SUCCEEDS, ERROR)` is the sequential route's own
    // call-shaped intrinsic, and its `let` form is the only one that route has.
    let piped = compile(&sequential(
        "  let a = 1 |> qcore_acquire(true, Failure);\n  drop a;\n  return 0;",
    ))
    .expect("an acquisition is a call stage");
    let twin = compile(&sequential(
        "  let a = qcore_acquire(1, true, Failure);\n  drop a;\n  return 0;",
    ))
    .unwrap();
    assert_eq!(
        piped.execution_profile,
        SourceExecutionProfile::SequentialCleanupV1
    );
    assert_eq!(piped.c_source, twin.c_source);
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn the_sequential_routes_check_is_a_stage() {
    // `qcore_check(PASSES, ERROR);` is a *statement* intrinsic, so it can only
    // be a pipeline's final stage — which is what this is.
    let piped = compile(&sequential("  true |> qcore_check(Failure);\n  return 0;"))
        .expect("a check is a call stage in final position");
    let twin = compile(&sequential("  qcore_check(true, Failure);\n  return 0;")).unwrap();
    assert_eq!(piped.c_source, twin.c_source);
}

// ---------------------------------------------------------------------------
// 3. The refusals, each by name
// ---------------------------------------------------------------------------

#[test]
fn an_intrinsic_whose_value_is_discarded_is_not_a_bare_final_stage() {
    // `returned |> qcore_buffer_read;` desugars to the expression statement
    // `qcore_buffer_read(returned);`, and the task route has no such statement:
    // every intrinsic that produces a value is a `let` initializer there. The
    // refusal is the route's own, at the statement the reader wrote, on card
    // `PIPE-SCALAR-RESULT`'s precedent that a stage's *bindability* is the
    // route's question and never the pipe's.
    let source = tasks(
        "                let value = qcore_buffer_read(returned);\n                returned |> qcore_buffer_read;",
    );
    assert_eq!(codes(&source), vec!["Q500"]);
    assert!(
        message(&source).contains("unsupported expression statement"),
        "{}",
        message(&source)
    );
}

#[test]
fn a_unit_returning_intrinsic_refuses_mid_chain_at_the_routes_own_code() {
    // `qcore_buffer_write` evaluates to `unit`. The pipe admits it as an
    // intermediate — "produces a value" is decided by stage *kind* and a write
    // is a call — and the composed route refuses the temporary at `Q050`,
    // naming the type rather than the pipe. Exactly the `Q024` answer card
    // `PIPE-SCALAR-RESULT` pinned for a `unit`-returning extern callee.
    let source = composed("                returned |> qcore_buffer_write(7) |> log info dec;");
    assert_eq!(codes(&source), vec!["Q050"]);
    assert!(message(&source).contains("is not an `i32` this entry bound"));
}

#[test]
fn a_custody_bearing_temporary_cannot_cross_into_the_rendering_namespace() {
    // The task route's own `ValueId` wall. `qcore_buffer_new` produces an
    // `own<Buffer>`, and the composed route's rendering operand rule admits
    // exactly `ValueType::Scalar(Scalar::I32)` — so a resource identity bound to
    // a hygienic temporary refuses at the sink rather than being rendered as a
    // number. Nothing crosses the namespace.
    let source = composed("                7 |> qcore_buffer_new? |> log info dec;");
    assert_eq!(codes(&source), vec!["Q050"]);
    assert!(message(&source).contains("is not an `i32` this entry bound"));
}

#[test]
fn an_allocating_intermediate_leaks_its_hygienic_temporary() {
    // A chain that allocates mid-way binds an owned buffer to a temporary the
    // reader cannot name, so it cannot be dropped and the task route refuses
    // the leak. Custody flows through the desugared statements under the
    // unedited checkers, exactly as card `PIPE-CHAIN` requires — the refusal is
    // the pre-existing one, not a pipe rule. Follow-on `PIPE-DROP-STAGE`.
    let source = tasks_allocating(
        "    let first = qcore_buffer_new(42)?;\n    let unreachable_value = 7 |> qcore_buffer_new? |> qcore_buffer_read;",
    );
    assert_eq!(codes(&source), vec!["Q600"]);
    assert!(message(&source).contains("discards a handle"));
}

#[test]
fn consuming_the_temporary_into_a_read_refuses() {
    // `|> move STAGE` is the spelling for consuming what a previous stage
    // produced. A buffer read consumes nothing, so `qcore_buffer_read(move t)`
    // is not its statement form and the route refuses the shape at its own span.
    let source = tasks_allocating(
        "    let first = qcore_buffer_new(42)?;\n    let unreachable_value = 7 |> qcore_buffer_new? |> move qcore_buffer_read;",
    );
    assert_eq!(codes(&source), vec!["Q500"]);
    assert!(message(&source).contains("buffer read requires one exact owned-buffer name"));
}

#[test]
fn the_keyword_shaped_intrinsics_are_not_stages() {
    // `spawn S W(ARGS)?` and `join move H?` are statements whose operands are
    // not a parenthesized argument list, so no call stage can produce them and
    // the closed sink table deliberately does not name them. Both refuse; the
    // codes differ because `spawn`'s desugared form does not even parse.
    assert_eq!(
        codes(&tasks(
            "                let value = qcore_buffer_read(returned);\n                let second = workers |> spawn;"
        )),
        vec!["Q000"]
    );
    assert_eq!(
        codes(&tasks(
            "                let value = qcore_buffer_read(returned);\n                let again = returned |> join;"
        )),
        vec!["Q500"]
    );
}

/// **This card's withheld decision, taken — the pin moved by name.**
///
/// `PIPE-INTRINSIC-STAGE` refused `returned |> drop;` and said why: adding
/// `drop` to the closed sink table "would make a custody-releasing statement
/// pipeable, which is a decision of its own and not this card's". Card
/// `PIPE-DROP-STAGE` took that decision, so the pin it left behind **failed and
/// was rewritten rather than deleted** — the `TRACE-STDIO` precedent, and the
/// mechanism working as intended.
///
/// What is asserted here is only that the census row changed. The admission
/// itself, its byte-equality against the hand-written statement and every
/// refusal around it are `compiler/tests/pipe_drop_stage.rs`.
#[test]
fn drop_is_now_a_pipe_stage_on_the_task_route() {
    // §3.4's allocating chain, now spellable end to end: the temporary this
    // card measured as unreleasable is released by the pipeline that invented
    // it. Written at the entry, where nothing else is live.
    compile(&tasks_allocating(
        "    7 |> qcore_buffer_new? |> drop;\n    let first = qcore_buffer_new(42)?;",
    ))
    .expect("card `PIPE-DROP-STAGE` admitted the releasing stage");
    // The rest of the census is unmoved: `drop` was the only keyword-shaped
    // head admitted, and its terminality is the sink rule it inherits.
    assert_eq!(
        codes(&tasks(
            "                let value = qcore_buffer_read(returned);\n                returned |> drop |> drop;"
        )),
        vec!["Q037"]
    );
}

#[test]
fn a_route_whose_model_has_no_buffer_intrinsic_keeps_its_own_refusal() {
    // `source.mutex.guard.v1` is selected before the task route and models no
    // buffer at all, so an intrinsic stage inside a mutex generation refuses
    // there rather than reaching the route that would have understood it.
    let source = "#import std.concurrency;
fn main() -> unit {
    thread scope s {
        let buf = protected 4;
        let m = mutex guards buf;
        let seen = buf |> qcore_buffer_read;
    }
}
";
    assert!(compile(source).is_err(), "the mutex route models no buffer");
}

// ---------------------------------------------------------------------------
// 4. The one production change: the sequential route's statement reader
// ---------------------------------------------------------------------------

#[test]
fn the_sequential_route_reads_the_desugared_stream_not_the_source_bytes() {
    // Before this card `statement_tokens` re-lexed `source[span]`, which for a
    // pipeline is the *pipeline* rather than the statement it desugared to: the
    // operand sits to the left of the callee in the bytes and to the right of it
    // in the statement, so `parse_acquire` saw `1` where it wanted the callee
    // name and refused with a message naming the very form the reader had
    // written. Reading the parsed program's own token stream is identical on
    // every pipe-free file — it *is* the lex of that source — and this is the
    // observable difference.
    let piped = compile(&sequential(
        "  let a = 1 |> qcore_acquire(true, Failure);\n  drop a;\n  return 0;",
    ))
    .expect("the sequential route no longer sees the pipe");
    let twin = compile(&sequential(
        "  let a = qcore_acquire(1, true, Failure);\n  drop a;\n  return 0;",
    ))
    .unwrap();
    assert_eq!(piped.c_source, twin.c_source);
    // …and the desugarer's own refusals are unchanged: a pipeline that is not
    // rewritten leaves its `|>` in place, and the residual still refuses.
    assert_eq!(
        codes(&sequential("  let a = 1 |> ;\n  return 0;")),
        vec!["Q036"]
    );
}

#[test]
fn a_decidable_condition_still_reads_the_same_tokens() {
    // The second reader this card touches is `source.branch.v1`'s condition
    // window. A `|>` inside a condition is never desugared — the desugarer only
    // sees a top-level pipe — so the condition tier keeps refusing it, and an
    // ordinary condition keeps compiling.
    compile(&sequential(
        "  let n = 3;\n  if (n < 5) { } else { }\n  return 0;",
    ))
    .expect("an ordinary decidable condition is unaffected");
    assert_eq!(
        codes(&sequential("  if (1 |> 2) { } else { }\n  return 0;")),
        vec!["Q045"],
        "a pipeline in a condition is the condition tier's refusal, unchanged"
    );
}

// ------------------------------------------------------------- native evidence

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn the_piped_composed_program_matches_the_reference_per_stream() {
    use std::fs;
    use std::process::Command;

    let module = compile(&composed(
        "                returned |> qcore_buffer_read |> log info dec;",
    ))
    .unwrap();
    let composed_module = module.checked.as_composed_tasks().unwrap();
    let (out, err) = execute_with_source_lines(
        composed_module.tasks(),
        &TaskExecutionScript::default(),
        &BTreeMap::new(),
        composed_module.sites(),
    )
    .unwrap()
    .projections();

    let directory =
        std::env::temp_dir().join(format!("qcore-pipe-intrinsic-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir(&directory).unwrap();
    let c = directory.join("module.c");
    let executable = directory.join("module");
    fs::write(
        &c,
        diagnostic_sink::instrument(module.c_source.as_deref().unwrap()),
    )
    .unwrap();
    let mut compiler =
        Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()));
    compiler.args([
        "-std=c11",
        "-pthread",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-g",
        "-O0",
        "-fsanitize=address,undefined",
        "-fno-omit-frame-pointer",
    ]);
    if cfg!(target_os = "macos") {
        compiler.arg("-I").arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/apple-threads"
        ));
    }
    let compiled = compiler
        .arg(&c)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let leaks = if cfg!(target_os = "linux") { 1 } else { 0 };
    let output = Command::new(&executable)
        .env(
            "ASAN_OPTIONS",
            format!("detect_leaks={leaks}:halt_on_error=1"),
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), out);
    assert_eq!(String::from_utf8(output.stderr).unwrap(), err);
    fs::remove_dir_all(directory).unwrap();
}
