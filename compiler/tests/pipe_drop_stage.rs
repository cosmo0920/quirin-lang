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

//! The **releasing pipe stage** (card `PIPE-DROP-STAGE`).
//!
//! Card `PIPE-INTRINSIC-STAGE` closed its census with one entry deliberately
//! withheld: `drop` "takes a single operand exactly as `print` does, but it
//! *releases custody*, and a releasing table entry is a different kind of entry
//! from a rendering one". This card takes that decision. `drop` joins the closed
//! statement-stage table and **no other keyword-shaped head does**, on a
//! criterion applied to the whole census — see [`SINK_STAGES`] in
//! `crates/qcore-frontend/src/pipe.rs` and §2 of the card's report.
//!
//! Three things this file is written to measure rather than assert:
//!
//! * **The desugaring is the same one.** A releasing stage emits
//!   `HEAD OPERAND ";"` exactly as a rendering stage does, so custody flows
//!   through the *unedited* checkers and linearity is inherited by
//!   construction. Every positive test below compares emitted C byte for byte
//!   against the hand-written statement, and the negative tests are the
//!   checkers' own refusals at their own codes.
//! * **Terminality is inherited, not added.** A release binds no result, so the
//!   pre-existing "only a call stage may feed a later stage" rule already
//!   forbids a released value from reaching one. The message differs, the rule
//!   does not.
//! * **The payoff.** `PIPE-INTRINSIC-STAGE` §3.4 measured that an allocating
//!   intermediate leaks, because its hygienic temporary is an owned buffer no
//!   reader can name and therefore cannot `drop`. With this entry the allocating
//!   chain is spellable end to end — `7 |> qcore_buffer_new? |> drop;` — and
//!   the temporary is released by the pipeline that invented it.

use qcore_compiler::{
    CompileOptions, CompiledModule, SourceExecutionProfile, SourceFile, compile_files,
    parse_source_program,
};

fn compile(source: &str) -> Result<CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("release.qcore", source)],
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

/// A composed `source.std-io-tasks.v1` module — the shape
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
                let value = qcore_buffer_read(returned);
                value |> log info dec;
RELEASE
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

fn composed(release: &str) -> String {
    COMPOSED.replace("RELEASE", release)
}

/// The plain task route, with the entry's own allocation written by the caller.
/// The allocation is the one point where nothing else is live, so a refusal
/// there is about the allocation rather than about what an error arm leaks.
const TASKS: &str = "#import std.concurrency;
concurrency checked;

fn worker(buffer: own<Buffer> use one) -> own<Buffer> { return move buffer; }

fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
ALLOC
    thread scope workers {
        match spawn workers worker(move first) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
BODY
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

/// The same module with its **entry** allocation written however the caller
/// likes. Measured, not assumed: the entry is the one point where no other
/// buffer is live, and a `?` adapter propagates out of the function. Anywhere
/// deeper, a propagating allocation abandons whatever the enclosing arm already
/// owns and refuses at `Q600` for that reason rather than for anything to do
/// with the pipe — the same premise `PIPE-INTRINSIC-STAGE` §3.4 rests on.
fn tasks_allocating(allocation: &str) -> String {
    TASKS
        .replace("ALLOC", allocation)
        .replace("BODY", "                drop returned;")
}

/// A `SequentialCleanupV1` entry.
fn sequential(body: &str) -> String {
    format!("error Failure {{ Failed; }}\nfn main() -> i32 {{\n{body}\n}}\n")
}

/// A `source.buffer.v1` entry — the `std.buffer` route, whose acquisition
/// `bytes(N)` and extent read `extent(B)` are call-shaped intrinsics of its own.
fn buffer(body: &str) -> String {
    format!(
        "#import std.buffer;\n#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects {{ io, alloc, mut }} {{\n{body}\n    return;\n}}\n"
    )
}

// ---------------------------------------------------------------------------
// 1. The release is a stage, on every route that has one
// ---------------------------------------------------------------------------

#[test]
fn a_release_is_a_pipe_stage_on_the_task_route() {
    let piped =
        compile(&tasks("                returned |> drop;")).expect("`drop` is a statement stage");
    let twin = compile(&tasks("                drop returned;")).unwrap();
    assert_eq!(
        piped.execution_profile,
        SourceExecutionProfile::ScopedOwnedTasksV1
    );
    assert_eq!(
        piped.c_source, twin.c_source,
        "a releasing stage must emit the C its hand-written statement emits"
    );
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn the_release_pipeline_and_its_hand_written_twin_are_the_same_program() {
    // The strongest available statement that the desugaring adds no semantics:
    // `syntax_equivalent` compares the whole post-parse token stream. `drop` is
    // a pure permutation — a statement stage synthesizes nothing at all.
    let piped = parse_source_program(&tasks("                returned |> drop;")).unwrap();
    let plain = parse_source_program(&tasks("                drop returned;")).unwrap();
    assert!(
        piped.syntax_equivalent(&plain),
        "a releasing stage must parse to the statement it desugars to"
    );
}

#[test]
fn a_release_is_a_pipe_stage_on_the_composed_task_route() {
    let piped = compile(&composed("                returned |> drop;"))
        .expect("`drop` is a stage here too");
    let twin = compile(&composed("                drop returned;")).unwrap();
    assert_eq!(
        piped.execution_profile,
        SourceExecutionProfile::ComposedStdIoTasksV1
    );
    assert_eq!(piped.c_source, twin.c_source);
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn a_release_is_a_pipe_stage_on_the_sequential_route() {
    // `SequentialCleanupV1`'s own release, reached through the statement reader
    // card `PIPE-INTRINSIC-STAGE` §4 repaired.
    let piped = compile(&sequential(
        "  let a = qcore_acquire(1, true, Failure);\n  a |> drop;\n  return 0;",
    ))
    .expect("the sequential route's release is a stage");
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
fn a_release_is_a_pipe_stage_on_the_standard_buffer_route() {
    // `source.buffer.v1` spells its release with the same reserved `drop`,
    // because the operation *is* `OwnershipOp::Drop`. One table entry therefore
    // reaches every route that releases, with no per-route knowledge at all.
    let piped = compile(&buffer("    let b = bytes(8);\n    b |> drop;"))
        .expect("the byte buffer's release is a stage");
    let twin = compile(&buffer("    let b = bytes(8);\n    drop b;")).unwrap();
    assert_eq!(
        piped.execution_profile,
        SourceExecutionProfile::StandardBufferV1
    );
    assert_eq!(piped.c_source, twin.c_source);
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

// ---------------------------------------------------------------------------
// 2. The payoff: the allocating chain, end to end
// ---------------------------------------------------------------------------

#[test]
fn the_allocating_chain_is_spellable_end_to_end() {
    // `PIPE-INTRINSIC-STAGE` §3.4: an allocating intermediate bound an owned
    // buffer to a temporary no reader could name, so it could not be dropped and
    // the task route refused the leak at `Q600`. The releasing stage is what
    // names it — not by giving the reader a spelling for the temporary, but by
    // letting the pipeline that invented it release it.
    let piped = compile(&tasks_allocating(
        "    7 |> qcore_buffer_new? |> drop;\n    let first = qcore_buffer_new(42)?;",
    ))
    .expect("an allocating chain may end in its own release");
    let twin = compile(&tasks_allocating(
        "    let scratch = qcore_buffer_new(7)?;\n    drop scratch;\n    let first = qcore_buffer_new(42)?;",
    ))
    .unwrap();
    assert_eq!(
        piped.c_source, twin.c_source,
        "the chain must emit the C its hand-written pair emits"
    );
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn the_allocating_chain_still_leaks_when_it_does_not_end_in_a_release() {
    // The other half of the same measurement, unchanged: this entry admits a
    // release, it does not weaken the leak rule. A chain that allocates and ends
    // in a *read* still refuses at the pre-existing `Q600`, because a read
    // consumes nothing and the buffer is still live at the exit.
    // Written at the entry, where nothing else is live, so the only thing the
    // program can leak is the temporary itself.
    let source = tasks_allocating(
        "    let unreachable_value = 7 |> qcore_buffer_new? |> qcore_buffer_read;\n    let first = qcore_buffer_new(42)?;",
    );
    assert_eq!(codes(&source), vec!["Q600"]);
    assert!(message(&source).contains("discards a handle"));
}

#[test]
fn an_error_payload_take_can_be_released_in_the_same_chain() {
    // The census's one *custody-transferring* call-shaped intrinsic
    // (`PIPE-INTRINSIC-STAGE` §2.2) moves an owned Buffer out of a bound typed
    // error. Taking it into a chain and releasing it there is the shape that
    // makes the releasing entry worth its place: the payload reaches no binding
    // the reader writes.
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
            return 0;
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
    let piped = compile(&PAYLOAD.replace("RECOVER", "failure |> qcore_error_payload |> drop;"))
        .expect("a payload take may be released in the chain that took it");
    let twin = compile(&PAYLOAD.replace(
        "RECOVER",
        "let recovered = qcore_error_payload(failure);\n            drop recovered;",
    ))
    .unwrap();
    assert_eq!(piped.c_source, twin.c_source);
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn the_standard_buffers_allocating_chain_is_spellable_too() {
    // `source.buffer.v1`'s acquisition is call-shaped (`bytes(N)`), so the same
    // two-stage chain works there with no adapter tail at all. This is the
    // census of `PIPE-INTRINSIC-STAGE` §1 extended by one route, measured rather
    // than assumed.
    let piped =
        compile(&buffer("    8 |> bytes |> drop;")).expect("the byte buffer's chain is spellable");
    let twin = compile(&buffer(
        "    let qcore__pipe0 = bytes(8);\n    drop qcore__pipe0;",
    ))
    .unwrap();
    assert_eq!(piped.c_source, twin.c_source);
    assert_eq!(piped.reference_observation, twin.reference_observation);
}

#[test]
fn the_temporarys_name_is_observable_on_the_standard_buffer_route() {
    // **Measured, and worth stating rather than hiding in a twin's spelling.**
    // `PIPE-INTRINSIC-STAGE` §2.1 could compare a chain against a twin binding
    // any name a reader would choose, because the *task* backend names slots and
    // the source identifier reaches no emitted byte. `source.buffer.v1`'s
    // backend derives its C identifiers from the source binding, so the same
    // comparison there is against the twin that spells `qcore__pipe0` — the
    // programs differ in exactly that identifier and in nothing else.
    //
    // This is a statement about the *backend*, not about the pipe: a chain on
    // this route publishes the reserved prefix into the emitted C. Follow-on
    // `PIPE-BUFFER-TEMPORARY-NAME`.
    let piped = compile(&buffer("    8 |> bytes |> drop;")).unwrap();
    let named = compile(&buffer("    let scratch = bytes(8);\n    drop scratch;")).unwrap();
    assert_ne!(piped.c_source, named.c_source);
    assert_eq!(
        piped
            .c_source
            .as_deref()
            .unwrap()
            .replace("qcore__pipe0", "scratch"),
        named.c_source.as_deref().unwrap(),
        "the two programs must differ in the temporary's identifier and nothing else"
    );
    assert_eq!(
        piped.reference_observation, named.reference_observation,
        "and the reference execution must not observe the name at all"
    );
}

// ---------------------------------------------------------------------------
// 3. Terminality, and the refusals around it
// ---------------------------------------------------------------------------

#[test]
fn a_release_cannot_feed_a_later_stage() {
    // The rule is the pre-existing one — a stage that binds no result may not
    // stand non-finally — and it lands on a releasing stage for exactly the
    // right reason: there is no value to feed. The *message* is the releasing
    // one, because "it is a sink" is not what a reader needs to hear here.
    let source = composed("                returned |> drop |> log info dec;");
    assert_eq!(codes(&source), vec!["Q037"]);
    let message = message(&source);
    assert!(
        message.contains("releases the value piped into it"),
        "{message}"
    );
    assert!(message.contains("a released value is gone"), "{message}");
}

#[test]
fn a_release_cannot_feed_a_later_release() {
    // The double-release *within one pipeline* is refused by terminality alone,
    // before any checker sees a statement: `b |> drop |> drop;` never becomes
    // two statements.
    let source = tasks("                returned |> drop |> drop;");
    assert_eq!(codes(&source), vec!["Q037"]);
}

#[test]
fn releasing_the_same_binding_twice_refuses_under_the_unedited_checker() {
    // Across two statements the pipe has nothing to say, and does not try to:
    // the second release is an ordinary use-after-release, refused by the
    // route's own checker at its own code. Custody flows through the desugared
    // statements — card `PIPE-CHAIN`'s discipline, unedited.
    let source = tasks("                returned |> drop;\n                returned |> drop;");
    assert_eq!(codes(&source), vec!["Q600"]);
    let hand = tasks("                drop returned;\n                drop returned;");
    assert_eq!(
        codes(&hand),
        codes(&source),
        "the piped double release must refuse exactly as the hand-written one does"
    );
}

#[test]
fn a_moved_operand_piped_into_a_release_refuses() {
    // The **operand** side of the same "one spelling per meaning" rule. A
    // statement stage appends the incoming tokens verbatim, so
    // `move returned |> drop;` is `drop move returned;` — not the release
    // statement — and it refuses rather than being quietly accepted as a second
    // spelling of `returned |> drop`. Measured because this shape reached a
    // *call* stage before the entry existed and reaches the statement branch
    // now, and both refuse.
    //
    // **Where it refuses is recorded rather than repaired.** Name resolution
    // gets there first: a `drop`'s operand is read as its first word, which here
    // is the token `move`, so the refusal is `Q013` naming `move` as an unknown
    // binding. It refuses, and it refuses for a true reason, but the message
    // names a token the reader wrote as a keyword. Improving it would mean the
    // pipe inspecting a *releasing* stage's operand — a rule the rendering
    // entries do not have and would then be asymmetric with. Follow-on
    // `PIPE-MOVED-OPERAND-MESSAGE`.
    let source = tasks("                move returned |> drop;");
    assert_eq!(codes(&source), vec!["Q013"]);
    assert!(message(&source).contains("move"), "{}", message(&source));
}

#[test]
fn a_bound_pipeline_cannot_end_in_a_release() {
    // A release binds nothing, so there is nothing for `let NAME =` to receive.
    // This is the pre-existing bound-pipeline rule at `Q036`, unchanged.
    let source = tasks("                let nothing = returned |> drop;");
    assert_eq!(codes(&source), vec!["Q036"]);
    assert!(
        message(&source).contains("a bound pipeline cannot end in it"),
        "{}",
        message(&source)
    );
}

#[test]
fn a_release_stage_takes_no_move_prefix() {
    // `|> move STAGE` consumes the value a previous stage produced. A release
    // *is* that consumption and `drop NAME;` is how the language spells it, so
    // the prefix is refused at the pipe rather than handed to the route as
    // `drop move t;`. One spelling per meaning.
    let source = tasks(
        "                7 |> qcore_buffer_new? |> move drop;\n                drop returned;",
    );
    assert_eq!(codes(&source), vec!["Q036"]);
    assert!(
        message(&source).contains("it takes no `move`"),
        "{}",
        message(&source)
    );
}

#[test]
fn a_release_stage_names_no_further_word() {
    // `drop` has no selector words, so anything after it is not a rendering
    // choice but a second operand the statement has no room for.
    let source = tasks("                returned |> drop dec;");
    assert_eq!(codes(&source), vec!["Q036"]);
    assert!(
        message(&source).contains("takes the value piped into it and nothing else"),
        "{}",
        message(&source)
    );
}

#[test]
fn releasing_something_that_is_not_owned_is_the_routes_own_refusal() {
    // The pipe knows nothing about custody and asks nothing about it. What may
    // be released is the route's question, answered at the statement the reader
    // wrote: a scalar has no custody to release.
    let source = tasks("                value |> drop;\n                drop returned;");
    assert_eq!(codes(&source), vec!["Q500"]);
    assert!(
        message(&source).contains("drop requires an owned buffer"),
        "{}",
        message(&source)
    );
}

#[test]
fn releasing_a_borrow_is_the_routes_own_refusal() {
    // A read loan is not a value and carries no custody, so the borrower cannot
    // release it — through a pipe exactly as by hand. `ScopedReadLoansV1`'s own
    // refusal, at its own code.
    const LOAN: &str = "#import std.concurrency;
concurrency checked;
fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
RELEASE
    return seen;
}
fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s reader(read buffer share FULL) {
            Result::Ok { value: handle } => {
                let seen = join move handle?;
                drop buffer;
                return seen;
            }
            Result::Err { error: failure } => {
                let restored = qcore_buffer_read(buffer);
                drop buffer;
                return restored;
            }
        }
    }
}
";
    let piped = codes(&LOAN.replace("RELEASE", "    payload |> drop;"));
    let hand = codes(&LOAN.replace("RELEASE", "    drop payload;"));
    assert_eq!(
        piped, hand,
        "a piped release of a borrow must refuse exactly as the hand-written one does"
    );
    assert!(!piped.is_empty(), "a borrow must not be releasable");
}

// ---------------------------------------------------------------------------
// 4. The published example (card `PIPE-INTRINSIC-EXAMPLE`)
// ---------------------------------------------------------------------------

/// `examples/threads/owned_buffer.qr`, as published.
const OWNED_BUFFER: &str = include_str!("../../examples/threads/owned_buffer.qr");

#[test]
fn the_published_example_emits_what_its_hand_written_form_emits() {
    // Card `PIPE-INTRINSIC-STAGE` §5 recorded that the example's reading pair
    // "is exactly the chain this card admits and would compile to identical C",
    // and left the rewrite to follow-on `PIPE-INTRINSIC-EXAMPLE` because the
    // file was shared ground that wave. This is that rewrite, and the claim it
    // rested on, **verified rather than carried**: the published file and the
    // form it replaced emit the same bytes and observe the same run.
    //
    // The `drop second;` → `second |> drop;` half is this card's own.
    let published = compile(OWNED_BUFFER).expect("the published example checks");
    let hand_written = compile(
        &OWNED_BUFFER
            .replace(
                "second |> qcore_buffer_read |> log info dec;",
                "let local = qcore_buffer_read(second);\n                        local |> log info dec;",
            )
            .replace("second |> drop;", "drop second;"),
    )
    .expect("so does the form it replaced");
    assert_eq!(
        published.c_source, hand_written.c_source,
        "the rewritten example must emit the C the statement form emitted"
    );
    assert_eq!(
        published.reference_observation,
        hand_written.reference_observation
    );
    assert_eq!(
        published.execution_profile,
        SourceExecutionProfile::ComposedStdIoTasksV1
    );
}

#[test]
fn the_published_example_still_carries_the_statement_form_where_a_chain_cannot_reach() {
    // The honest half of the rewrite. The post-join read binds `value` because
    // the number is used **twice** — rendered and then returned — and a pipeline
    // is linear: a chain consumes what it produces. That statement pair is not a
    // chain waiting to be written, and the example says so in a comment rather
    // than leaving a reader to wonder why two shapes appear in one file.
    assert!(
        OWNED_BUFFER.contains("let value = qcore_buffer_read(returned);"),
        "the post-join read keeps its binding"
    );
    assert!(
        OWNED_BUFFER.contains("second |> qcore_buffer_read |> log info dec;"),
        "and the parent's own read is the chain"
    );
}

// ---------------------------------------------------------------------------
// 5. The census: `drop` joined the table and nothing else did
// ---------------------------------------------------------------------------

#[test]
fn the_other_keyword_shaped_heads_did_not_join_the_table() {
    // Each of these passes the *shape* half of the criterion and fails the
    // other, and each therefore keeps the refusal it already had. The codes
    // differ because each is refused by whichever reader reaches it first —
    // which is the point: the pipe states no rule about any of them.
    for (stage, code) in [
        // `spawn(returned);` does not parse at all.
        ("spawn", "Q000"),
        ("join", "Q500"),
        // `move` reaches the call-stage branch, because the pipe's own `move`
        // *prefix* needs a stage after it and there is none here. What it
        // desugars to is `move(returned);`, which the route refuses — the
        // prefix's meaning is untouched.
        ("move", "Q500"),
        ("return", "Q500"),
        ("share", "Q500"),
    ] {
        let source = tasks(&format!(
            "                returned |> {stage};\n                drop returned;"
        ));
        assert_eq!(
            codes(&source),
            vec![code],
            "`{stage}` must not be a stage: {}",
            message(&source)
        );
    }
}

#[test]
fn a_mutex_route_statement_head_is_not_a_stage() {
    // `read N;` and `write N;` are `source.mutex.guard.v1` statement heads of
    // exactly the admitted shape, and they are excluded by the second half of
    // the criterion: that route's model has no call-shaped intrinsic at all, so
    // no stage can ever produce their operand and an entry would buy nothing but
    // a second spelling.
    let source = "#import std.concurrency;
fn main() -> unit {
    thread scope s {
        let buf = protected 4;
        let m = mutex guards buf;
        buf |> read;
    }
}
";
    assert!(
        compile(source).is_err(),
        "a mutex statement head is not a pipeline stage"
    );
}

#[test]
fn the_operandless_stage_is_still_refused() {
    // `flush` takes no operand, and the releasing entry does not change that:
    // piping into it still refuses rather than silently dropping the value.
    let source = composed("                value |> flush;\n                drop returned;");
    assert_eq!(codes(&source), vec!["Q037"]);
    assert!(
        message(&source).contains("takes no operand"),
        "{}",
        message(&source)
    );
}

#[test]
fn the_rendering_stages_are_unchanged() {
    // The table grew by one entry and the two that were in it are untouched:
    // same selector arity, same last-stage-only rule, same `move` refusal.
    let rendered = compile(&composed(
        "                value |> log info dec;\n                drop returned;",
    ))
    .expect("a rendering stage still compiles");
    let twin = compile(&composed(
        "                log info dec value;\n                drop returned;",
    ))
    .unwrap();
    assert_eq!(rendered.c_source, twin.c_source);
    let non_final = composed(
        "                value |> log info dec |> log warn dec;\n                drop returned;",
    );
    assert_eq!(codes(&non_final), vec!["Q037"]);
    assert!(
        message(&non_final).contains("it is a sink"),
        "a rendering stage keeps the rendering message: {}",
        message(&non_final)
    );
}
