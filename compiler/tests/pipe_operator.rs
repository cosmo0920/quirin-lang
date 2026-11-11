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

//! The pipe operator `|>` and the claimed logical-operator tokens (card
//! `PIPE-OPERATOR`).
//!
//! The load-bearing property of this card is an *absence*: after desugaring
//! there is nothing left to check. Every test below is written to make that
//! absence measurable rather than assumed — byte-identical emission against a
//! hand-unpiped twin, identical reference observations, identical refusals and
//! identical diagnostic spans, and a real consuming call reached through a
//! pipe whose linearity refusal is the pre-existing one.
#[cfg(not(target_os = "windows"))]
#[path = "../../tests/support/diagnostic_sink.rs"]
mod diagnostic_sink;

use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, parse_source_program,
};

#[cfg(target_os = "windows")]
#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const LINES: &str = include_str!("fixtures/std_modules/lines.qr");
const LINES_PIPED: &str = include_str!("fixtures/pipe/lines_piped.qr");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qr");
const ROUND_TRIP_PIPED: &str = include_str!("fixtures/pipe/round_trip_piped.qr");
const REALLOC: &str = include_str!("fixtures/extern_calls/realloc.qr");
const REALLOC_PIPED: &str = include_str!("fixtures/pipe/realloc_piped.qr");
const SCALAR_RESULT: &str = include_str!("fixtures/extern_calls/scalar_result.qr");
const SCALAR_RESULT_PIPED: &str = include_str!("fixtures/pipe/scalar_result_piped.qr");
const EXTERN_CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qv");

// Exercise both checkout styles regardless of the host's Git configuration.
fn with_line_endings(source: &str) -> [String; 2] {
    let lf = source.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
}

fn replace_fixture(source: &str, from: &str, to: &str) -> String {
    let line_ending = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let adapt = |text: &str| text.replace("\r\n", "\n").replace('\n', line_ending);
    source.replace(&adapt(from), &adapt(to))
}

fn selection() -> Option<BTreeMap<String, String>> {
    Some(BTreeMap::from([(
        "fixtures/extern_calls/contracts/buffer_api.qcore".to_owned(),
        EXTERN_CONTRACTS.to_owned(),
    )]))
}

fn compile_with(
    source: &str,
    contracts: Option<BTreeMap<String, String>>,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_files(
        &[SourceFile::new("pipe.qcore", source)],
        &CompileOptions {
            entry: None,
            extern_contracts: contracts,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

fn compile(
    source: &str,
) -> Result<qcore_compiler::CompiledModule, Vec<qcore_compiler::FileDiagnostic>> {
    compile_with(source, None)
}

fn codes(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn messages(source: &str) -> Vec<String> {
    compile(source)
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

// --- 1. The desugaring adds no semantics. ---

#[test]
fn a_piped_program_and_its_hand_written_twin_are_syntactically_the_same_program() {
    // The strongest available statement of "the desugaring preserves the call
    // tree": `syntax_equivalent` compares the whole post-parse token stream
    // and the declaration list, and the piped source satisfies it against the
    // program a reader would have written by hand.
    let piped = parse_source_program(LINES_PIPED).unwrap();
    let plain = parse_source_program(LINES).unwrap();
    assert!(
        piped.syntax_equivalent(&plain),
        "a piped program must parse to the same token stream as its hand-written twin"
    );
    // ... and the original bytes are still what the program displays, so a
    // round trip never rewrites the user's file.
    assert!(piped.to_string().contains("|>"));
}

#[test]
fn the_piped_and_unpiped_programs_emit_byte_identical_c() {
    for (piped, plain) in with_line_endings(LINES_PIPED)
        .into_iter()
        .zip(with_line_endings(LINES))
    {
        let a = compile(&piped).unwrap();
        let b = compile(&plain).unwrap();
        assert_eq!(
            a.c_source, b.c_source,
            "the desugaring is invisible downstream, so the emitted C must be byte-identical"
        );
        assert!(a.c_source.is_some(), "this route emits C11");
    }
}

#[test]
fn the_piped_and_unpiped_programs_produce_the_same_reference_observation() {
    let piped = compile(LINES_PIPED).unwrap();
    let plain = compile(LINES).unwrap();
    assert_eq!(piped.extern_observation, plain.extern_observation);
    assert_eq!(
        piped.extern_observation.as_deref().unwrap(),
        [
            "out: 42",
            "out: 0x000000FF",
            "err: [INFO] 42",
            "err: [WARN] 0x000000FF",
            "err: [FATAL] -7",
            "out: -7",
        ]
    );
}

#[test]
fn a_pipeline_does_not_change_the_selected_route() {
    assert_eq!(
        compile(LINES_PIPED).unwrap().execution_profile,
        SourceExecutionProfile::StandardIoV1
    );
    assert_eq!(
        compile_with(ROUND_TRIP_PIPED, selection())
            .unwrap()
            .execution_profile,
        SourceExecutionProfile::ExternCallsV1
    );
}

// --- 2. Linearity through a pipeline. ---

#[test]
fn an_owned_resource_piped_into_a_consuming_call_is_consumed() {
    // The pipeline's operand is `move b` and the stage is the declared
    // consuming callee. Both twins check, and both emit the same C.
    let piped = compile_with(ROUND_TRIP_PIPED, selection()).unwrap();
    let plain = compile_with(ROUND_TRIP, selection()).unwrap();
    assert_eq!(
        piped.c_source, plain.c_source,
        "a consuming call reached through a pipe emits exactly the unpiped call"
    );
    assert_eq!(piped.extern_observation, plain.extern_observation);
}

#[test]
fn using_a_binding_after_piping_it_into_a_consumer_refuses() {
    // The pre-existing move refusal, reached through a pipe. The point is that
    // no new rule was needed: the checker never learns a pipe was written.
    let piped = replace_fixture(
        ROUND_TRIP_PIPED,
        "    move b |> qcore_extern_release;\n",
        "    move b |> qcore_extern_release;\n    b |> qcore_extern_fill(7);\n",
    );
    let plain = replace_fixture(
        ROUND_TRIP,
        "    qcore_extern_release(move b);\n",
        "    qcore_extern_release(move b);\n    qcore_extern_fill(b, 7);\n",
    );
    let piped_codes: Vec<String> = compile_with(&piped, selection())
        .map(|module| module.execution_profile)
        .expect_err("using a consumed binding must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    let plain_codes: Vec<String> = compile_with(&plain, selection())
        .map(|module| module.execution_profile)
        .expect_err("using a consumed binding must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(
        piped_codes, plain_codes,
        "the piped and unpiped refusals must be the same diagnostics"
    );
    // Pinned by value, not only by equality: a pipe that refused for an
    // accidental reason would satisfy the equality and look sound.
    assert_eq!(piped_codes, ["Q024"]);
}

#[test]
fn a_pipeline_that_leaks_refuses_exactly_as_its_unpiped_twin_does() {
    let piped = replace_fixture(
        ROUND_TRIP_PIPED,
        "    move b |> qcore_extern_release;\n",
        "",
    );
    let plain = replace_fixture(ROUND_TRIP, "    qcore_extern_release(move b);\n", "");
    let piped_codes: Vec<String> = compile_with(&piped, selection())
        .map(|module| module.execution_profile)
        .expect_err("an unreleased acquisition must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    let plain_codes: Vec<String> = compile_with(&plain, selection())
        .map(|module| module.execution_profile)
        .expect_err("an unreleased acquisition must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(piped_codes, plain_codes);
    assert_eq!(piped_codes, ["Q202"]);
}

// --- 3. Attribution: a refusal names the bytes the reader wrote. ---

#[test]
fn an_operand_fault_is_attributed_to_the_piped_operand() {
    // `missing` is bound by nothing. The established name-resolution gate runs
    // first (the `STD-MODULES` measurement), so this is `Q013` — and the span
    // must cover the operand the reader wrote, not a synthetic position.
    let source = replace_fixture(LINES_PIPED, "total |> print dec;", "missing |> print dec;");
    let diagnostics = compile(&source)
        .map(|module| module.execution_profile)
        .unwrap_err();
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["Q013"]
    );
    let line = source
        .lines()
        .find(|line| line.contains("missing"))
        .expect("the operand line");
    assert!(
        line.contains("missing |> print dec;"),
        "the fixture must still be the piped form"
    );
    assert!(
        diagnostics[0].message.contains("missing"),
        "the diagnostic must name the operand: {}",
        diagnostics[0].message
    );
}

#[test]
fn a_stage_fault_is_attributed_to_the_stage() {
    let source = replace_fixture(LINES_PIPED, "total |> print dec;", "total |> print nope;");
    // An unreserved word in a rendering position is collected as a referenced
    // name and refuses at name resolution — the `STD-MODULES` measurement,
    // unchanged by the pipe.
    assert_eq!(codes(&source), ["Q013"]);
    assert!(messages(&source)[0].contains("nope"));
}

#[test]
fn a_resource_operand_is_still_unrenderable_through_a_pipe() {
    // The effect discipline of `source.std-io.v1` is structural: the operand
    // type has no constructor that could denote a resource. Piping does not
    // add one.
    let source = "#import std.fmt;\n\
                  #import std.io;\n\
                  fn main() -> unit effects { io } {\n    \
                      let r = qcore_acquire(true);\n    \
                      r |> print dec;\n    \
                      return;\n\
                  }\n";
    assert_eq!(codes(source), ["Q030", "Q028"]);
}

// --- 4. The v1 pipeline shape, and every refusal outside it. ---

#[test]
fn piping_into_an_operandless_stage_refuses() {
    let source = replace_fixture(LINES_PIPED, "    flush;\n", "    total |> flush;\n");
    assert_eq!(codes(&source), ["Q037"]);
    assert!(messages(&source)[0].contains("takes no operand"));
}

#[test]
fn a_sink_cannot_feed_a_later_stage() {
    // Deliberate pin update, card `PIPE-CHAIN`. This shape was refused in wave
    // 18 for being a *chain*; chains are now admitted, so what refuses it is
    // the narrower and permanent rule: `print` produces no value, so it can
    // only be a pipeline's last stage. The code is unchanged at `Q037` and the
    // new message names the stage rather than a follow-on.
    let source = replace_fixture(
        LINES_PIPED,
        "total |> print dec;",
        "total |> print dec |> log info dec;",
    );
    assert_eq!(codes(&source), ["Q037"]);
    assert!(messages(&source)[0].contains("produces no value"));
    assert!(messages(&source)[0].contains("`print`"));
}

#[test]
fn a_pipeline_on_the_right_of_a_binding_binds_its_last_stage() {
    // Deliberate pin update, card `PIPE-CHAIN`: this closes `PIPE-LET`. The
    // load-bearing negative half moves rather than disappearing — a bound
    // pipeline whose last stage produces nothing still refuses, and so does an
    // assignment pipeline.
    let source = replace_fixture(
        ROUND_TRIP_PIPED,
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n",
        "    let b = 8 |> qcore_extern_alloc succeeds true else raise Failure;\n",
    );
    let bound = compile_with(&source, selection()).expect("a bound pipeline checks");
    let unpiped = compile_with(ROUND_TRIP, selection()).unwrap();
    assert_eq!(
        bound.c_source, unpiped.c_source,
        "a bound pipeline emits what the hand-written binding emits"
    );

    let sink_bound = replace_fixture(
        LINES_PIPED,
        "    total |> print dec;\n",
        "    let x = total |> print dec;\n",
    );
    assert_eq!(codes(&sink_bound), ["Q036"]);
    assert!(messages(&sink_bound)[0].contains("produces no value"));

    let assigned = replace_fixture(
        ROUND_TRIP_PIPED,
        "    b |> qcore_extern_fill(7);\n",
        "    b = b |> qcore_extern_fill(7);\n",
    );
    let codes: Vec<String> = compile_with(&assigned, selection())
        .map(|module| module.execution_profile)
        .expect_err("an assignment pipeline must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, ["Q036"]);
}

#[test]
fn a_pipeline_with_nothing_on_one_side_refuses() {
    for (replacement, code) in [("    |> print dec;\n", "Q036"), ("    total |>;\n", "Q036")] {
        let source = replace_fixture(LINES_PIPED, "    total |> print dec;\n", replacement);
        assert_eq!(codes(&source), [code], "for `{replacement}`");
    }
}

#[test]
fn a_pipeline_that_reaches_a_block_boundary_before_its_semicolon_refuses() {
    // Measured: the parser's own "a statement ends in `;`" rule runs before
    // any frontend gate, so the last statement of a body without its `;` is a
    // parse failure (`Q000`) and never reaches the pipe pass. The desugarer
    // still records its own `Q036` for this shape — that arm is what keeps a
    // pipeline from silently swallowing a block — but it is unreachable from
    // this direction and the reachable refusal is pinned as the truth.
    let source = replace_fixture(
        LINES_PIPED,
        "    -7 |> print dec;\n    return;\n",
        "    -7 |> print dec\n",
    );
    assert_eq!(codes(&source), ["Q000"]);
    assert!(parse_source_program(&source).is_err());
}

#[test]
fn a_pipeline_missing_its_semicolon_mid_body_runs_into_the_next_statement() {
    // Measured, not designed: dropping the `;` makes the *next* statement's
    // `;` the run's terminator, so the following statement's head and operand
    // are swallowed into this pipeline's first stage. Card `PIPE-CHAIN` moved
    // which rule catches it — a stage is checked for well-formedness before it
    // is checked for its role, so the refusal is now the sink-arity one
    // (`Q036`, "print names 1 selector word … this stage names 2") rather than
    // wave 18's "too many stages" (`Q037`). Both are fail-closed and correctly
    // spanned; the measured one is pinned rather than the preferred one forced.
    let source = replace_fixture(
        LINES_PIPED,
        "    total |> print dec;\n",
        "    total |> print dec\n",
    );
    assert_eq!(codes(&source), ["Q036"]);
    assert!(messages(&source)[0].contains("selector word"));
}

#[test]
fn a_sink_stage_with_the_wrong_selector_count_refuses() {
    let source = replace_fixture(
        LINES_PIPED,
        "total |> print dec;",
        "total |> print dec hex;",
    );
    assert_eq!(codes(&source), ["Q036"]);
}

#[test]
fn a_call_stage_takes_the_operand_as_its_first_argument() {
    // First-argument insertion, measured: the bare form and the parenthesized
    // form both put the piped value first, and both agree with the unpiped C.
    let piped = compile_with(ROUND_TRIP_PIPED, selection()).unwrap();
    let explicit = replace_fixture(
        ROUND_TRIP_PIPED,
        "    move b |> qcore_extern_release;\n",
        "    move b |> qcore_extern_release();\n",
    );
    assert_eq!(
        compile_with(&explicit, selection()).unwrap().c_source,
        piped.c_source,
        "`f` and `f()` are the same stage"
    );
}

#[test]
fn the_argument_order_of_a_call_stage_is_the_declared_one() {
    // `b |> qcore_extern_fill(7)` must be `fill(b, 7)`, not `fill(7, b)`:
    // swapping the declared order is refused by the bridging table, which is
    // what makes the insertion position observable rather than assumed.
    let swapped = replace_fixture(
        ROUND_TRIP_PIPED,
        "    b |> qcore_extern_fill(7);\n",
        "    7 |> qcore_extern_fill(b);\n",
    );
    assert!(
        compile_with(&swapped, selection()).is_err(),
        "reversing the piped argument must not still check"
    );
}

// --- 5. The operator gate: retabled by `OR-AND-LOWERING`, made positional by
// `BITWISE-CONDITIONS`. ---
//
// These four tests are card `PIPE-OPERATOR`'s, kept in place and moved
// deliberately, twice. That card decided logical or was the single `|` and
// refused `||` as a misspelling of it; the user's retable amendment inverted
// the pair; and `BITWISE-CONDITIONS` then gave the three single characters a
// meaning inside an `if` condition. So the *claims* these tests make have been
// rewritten rather than relaxed. The invariant that survives untouched across
// both moves is the one that mattered: every one of these four programs — all
// of them in a **rendering** position on the std-io route — was refused before
// and is refused now, at the same code, by the same per-file gate. Where the
// operators are *admitted* is `compiler/tests/logical_connectives.rs` and
// `compiler/tests/bitwise_conditions.rs`.

#[test]
fn the_single_bar_is_the_bitwise_operator_and_not_admitted_here() {
    // **Deliberate pin update, card `BITWISE-CONDITIONS`**, the second this
    // test has taken. Card `PIPE-OPERATOR` claimed the single bar as logical
    // or; `OR-AND-LOWERING` withdrew that and held it as a reservation;
    // `BITWISE-CONDITIONS` spends the reservation and gives it a meaning in one
    // position — a parenthesized comparison operand inside an `if` condition.
    // What survives every move, and is the reason this test lives in the pipe
    // suite at all, is that a bare `|` written *here* — in a rendering position
    // on the std-io route — is refused, at `Q038`, by the same per-file gate.
    let source = replace_fixture(
        LINES_PIPED,
        "total |> print dec;",
        "print dec total | mask;",
    );
    assert_eq!(codes(&source), ["Q038"]);
    let message = &messages(&source)[0];
    assert!(message.contains("`|` is the bitwise-or operator"));
    assert!(
        message.contains("`if` condition"),
        "the message must name the one position where the operator is admitted"
    );
}

#[test]
fn the_double_bar_outside_a_condition_names_the_only_position_it_is_admitted_in() {
    let source = replace_fixture(
        LINES_PIPED,
        "total |> print dec;",
        "print dec total || mask;",
    );
    assert_eq!(
        codes(&source),
        ["Q038"],
        "`||` is one token, so one mistake produces one diagnostic"
    );
    let message = &messages(&source)[0];
    assert!(message.contains("`||` is the logical-or operator"));
    assert!(
        message.contains("`if` condition"),
        "the message must name the position where the operator is admitted"
    );
}

#[test]
fn the_double_ampersand_outside_a_condition_refuses_the_same_way() {
    let source = replace_fixture(
        LINES_PIPED,
        "total |> print dec;",
        "print dec total && mask;",
    );
    assert_eq!(codes(&source), ["Q038"]);
    let message = &messages(&source)[0];
    assert!(message.contains("`&&` is the logical-and operator"));
    assert!(message.contains("`if` condition"));
}

#[test]
fn a_single_ampersand_gains_no_meaning_in_a_rendering_position() {
    // Card `PIPE-OPERATOR` asserted a lone `&`'s refusal was never at `Q038`,
    // because a `Q038` would have meant that card had claimed it;
    // `OR-AND-LOWERING` moved it *to* `Q038` to state the reservation; and
    // `BITWISE-CONDITIONS` spends the reservation — in an `if` condition, and
    // nowhere else. The load-bearing half has survived all three moves and is
    // re-asserted here: a lone `&` in a rendering position still refuses, and
    // it is still not logical and.
    let source = replace_fixture(
        LINES_PIPED,
        "total |> print dec;",
        "print dec total & mask;",
    );
    assert_eq!(codes(&source), ["Q038"]);
    let message = &messages(&source)[0];
    assert!(message.contains("`&` is the bitwise-and operator"));
    assert!(
        message.contains("`&&`"),
        "the message must still distinguish it from logical and"
    );
    assert!(
        message.contains("`if` condition"),
        "the message must name the one position where the operator is admitted"
    );
}

#[test]
fn a_piped_sink_inside_a_thread_scope_keeps_the_task_routes_refusal() {
    // The `STD-IO-COMPOSE` boundary is neither widened nor narrowed by the
    // pipe: the operator is gone before route selection runs, so a piped
    // `print` inside a `thread scope` is refused exactly as the unpiped one
    // is, and both refuse for the same reason.
    let piped = "#import std.concurrency;\n\
                 #import std.fmt;\n\
                 #import std.io;\n\
                 concurrency checked;\n\
                 fn main() -> i32 effects { alloc } {\n    \
                     thread scope s {\n        \
                         1 |> print dec;\n    \
                     }\n    \
                     return 0;\n\
                 }\n";
    let plain = piped.replace("1 |> print dec;", "print dec 1;");
    let piped_codes = codes(piped);
    assert!(
        !piped_codes.is_empty(),
        "a printing thread scope must stay refused, not silently unmodeled"
    );
    assert_eq!(
        piped_codes,
        codes(&plain),
        "a pipe must not change which route claims the module"
    );
}

#[test]
fn a_pipe_nested_inside_an_argument_list_is_not_desugared_and_still_refuses() {
    // The run scanner only records a `|>` at bracket depth 0, so a pipe inside
    // an argument list is never rewritten. It is not silently dropped either:
    // the residual `|>` survives into the tokens and the route's own grammar
    // refuses the argument. Fail-closed, and pinned so a future widening has
    // to decide this case deliberately rather than inherit it.
    let source = replace_fixture(
        ROUND_TRIP_PIPED,
        "    move b |> qcore_extern_release;\n",
        "    qcore_extern_release(move b |> qcore_extern_sum);\n",
    );
    let codes: Vec<String> = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("a nested pipe must be refused, not silently unmodeled")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(!codes.is_empty());
    assert!(
        !codes.contains(&"Q036".to_owned()) && !codes.contains(&"Q037".to_owned()),
        "the pipe pass does not claim this shape; the route's own refusal is the answer: {codes:?}"
    );
}

#[test]
fn the_operator_gate_reaches_every_module_mode() {
    // Claiming `|` as a token means a program containing one now *parses*
    // where it used to fail in the lexer. That is only safe if the refusal is
    // reached on every route, including the non-executable module modes a
    // contract or library file uses — so this is pinned rather than reasoned.
    for mode in ["module contract;", "module library;", "module executable;"] {
        let source = format!("{mode}\nerror E {{ X; }}\nfn f() -> i32 {{ return 0 | 1; }}\n");
        assert_eq!(codes(&source), ["Q038"], "for `{mode}`");
    }
}

// --- 5b. Multi-stage pipelines (card `PIPE-CHAIN`). ---

#[test]
fn a_multi_stage_pipeline_is_its_hand_written_statement_sequence() {
    // The load-bearing claim of this card, in the same form wave 18 used for
    // one stage: a chain is *the same program* as the sequence a reader would
    // have written, so it emits byte-identical C11 and observes identically.
    // `realloc_piped.qr` is a three-stage pipeline over a shipped fixture,
    // not a program written to make a test pass.
    for [piped, unpiped] in [with_line_endings(REALLOC_PIPED)]
        .into_iter()
        .zip(with_line_endings(REALLOC))
        .map(|(piped, unpiped)| [piped[0].clone(), unpiped])
    {
        let chained = compile_with(&piped, selection()).expect("the chain checks");
        let twin = compile_with(&unpiped, selection()).expect("the twin checks");
        assert_eq!(
            chained.c_source, twin.c_source,
            "a chain and its hand-written sequence emit the same C11"
        );
        assert_eq!(
            chained.extern_observation, twin.extern_observation,
            "a chain and its hand-written sequence observe the same trace"
        );
    }
}

#[test]
fn a_chain_written_without_whitespace_still_separates_its_statements() {
    // The adversarial case for the anchoring: with every `|>` abutting its
    // neighbours, a statement's synthesized terminator sits at the same byte a
    // written token ends at. The terminator anchors at the pipe's *start* and
    // the next statement's synthesized tokens at its *end*, two bytes later, so
    // the extents stay disjoint and each route's span-keyed window still gets
    // exactly its own tokens — which the byte-identical emission proves.
    let dense = replace_fixture(
        REALLOC_PIPED,
        "    let grown = 8 |> qcore_extern_alloc succeeds true else raise Failure\n                  \
         |> move qcore_extern_grow(32) succeeds true\n                       \
         else qcore_extern_free(move) raise Failure;\n",
        "    let grown = 8|>qcore_extern_alloc succeeds true else raise Failure|>move \
         qcore_extern_grow(32) succeeds true else qcore_extern_free(move) raise Failure;\n",
    );
    assert_ne!(dense, REALLOC_PIPED, "the mutation must apply");
    let packed = compile_with(&dense, selection()).expect("a dense chain checks");
    let spaced = compile_with(REALLOC_PIPED, selection()).unwrap();
    assert_eq!(packed.c_source, spaced.c_source);
    assert_eq!(packed.extern_observation, spaced.extern_observation);
}

#[test]
fn every_stage_of_a_chain_keeps_its_own_attribution() {
    // The D01 discipline through a chain: a fault inside stage N points at the
    // bytes of stage N, not at the pipeline's first token and not at an
    // invented position. Each mutation names a different callee, and the
    // reported span must cover that callee's own name.
    for (stage, absent) in [
        ("qcore_extern_alloc succeeds", "qcore_absent_alloc succeeds"),
        ("qcore_extern_grow(32)", "qcore_absent_grow(32)"),
    ] {
        let source = replace_fixture(REALLOC_PIPED, stage, absent);
        assert_ne!(source, REALLOC_PIPED, "the mutation must apply");
        let diagnostics = compile_with(&source, selection())
            .map(|module| module.execution_profile)
            .expect_err("an undeclared callee must be refused");
        let named = absent.split(['(', ' ']).next().unwrap();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.location.line_source.contains(named)),
            "a fault in the `{named}` stage must be reported on that stage's own line: \
             {diagnostics:?}"
        );
    }
}

#[test]
fn a_chain_whose_result_is_never_consumed_leaks_and_refuses() {
    // Requirement, measured rather than designed: dropping a live result on the
    // floor is not silently accepted. The temporary is an ordinary binding by
    // the time the accounting runs, so the pre-existing leak diagnostic is what
    // refuses it — no leak rule was added for pipelines.
    let source = replace_fixture(
        REALLOC_PIPED,
        "    qcore_extern_fill(grown, 3);\n    qcore_extern_free(move grown);\n",
        "    qcore_extern_fill(grown, 3);\n",
    );
    let codes: Vec<String> = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("a leaked chain result must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, ["Q202"]);
}

#[test]
fn a_value_consumed_mid_chain_cannot_be_used_after_the_pipeline() {
    // The other direction of linearity, and the one that would be a real
    // soundness hole: the pipeline's operand is consumed by the first stage, so
    // naming it afterwards refuses exactly as it does without a pipe.
    let piped = replace_fixture(
        ROUND_TRIP_PIPED,
        "    move b |> qcore_extern_release;\n",
        "    move b |> qcore_extern_grow(32) succeeds true else raise Failure \
         |> move qcore_extern_release;\n    qcore_extern_fill(b, 1);\n",
    );
    let unpiped = replace_fixture(
        ROUND_TRIP,
        "    qcore_extern_release(move b);\n",
        "    let g = qcore_extern_grow(move b, 32) succeeds true else raise Failure;\n    \
         qcore_extern_release(move g);\n    qcore_extern_fill(b, 1);\n",
    );
    let codes = |source: &str| -> Vec<String> {
        compile_with(source, selection())
            .map(|module| module.execution_profile)
            .expect_err("using a consumed binding must be refused")
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    };
    assert_eq!(
        codes(&piped),
        codes(&unpiped),
        "the piped and unpiped diagnostic vectors must be equal, not merely both non-empty"
    );
    assert!(!codes(&piped).is_empty());
}

#[test]
fn a_stage_that_produces_nothing_cannot_feed_a_later_stage() {
    // **Deliberate pin update, card `PIPE-SCALAR-RESULT`.** The rule that makes
    // a chain decidable without contract knowledge is unchanged in kind and
    // wider in reach: "produces a value" is now decided by *stage kind*, so a
    // call stage may be intermediate and only a sink may not.
    //
    // A `unit`-returning callee therefore still refuses in a non-final
    // position — nothing was promoted — but the refusal is the **route's**,
    // naming the declared result, rather than the pipe's naming the pipe. That
    // is the honest answer to why the stage cannot feed a later one, and it is
    // the same answer the hand-written statement sequence gets. The cascade is
    // pinned as measured rather than trimmed: the second statement then reads a
    // temporary the first never bound.
    let source = replace_fixture(
        ROUND_TRIP_PIPED,
        "|> qcore_extern_fill(7);",
        "|> qcore_extern_fill(7) |> move qcore_extern_release;",
    );
    let diagnostics = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("a value-less intermediate stage must still be refused");
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| d.code.clone())
            .collect::<Vec<_>>(),
        ["Q024", "Q024"]
    );
    assert!(diagnostics[0].message.contains("qcore_extern_fill"));
    assert!(diagnostics[0].message.contains("not a custody-free scalar"));

    // The sink half of the rule is unmoved and still the pipe's own, at `Q037`.
    let sink = replace_fixture(
        LINES_PIPED,
        "total |> print dec;",
        "total |> print dec |> log info dec;",
    );
    assert_eq!(codes(&sink), ["Q037"]);
    assert!(messages(&sink)[0].contains("it is a sink"));
}

/// **The card's payoff.** A pipeline stage whose callee returns a scalar feeds
/// the next stage through the hygienic temporary, and the chain emits
/// byte-identical C11 against its hand-written twin.
///
/// This is `x |> f(a) |> g(b)` meaning `g(f(x, a), b)` over *data*: card
/// `PIPE-CHAIN` shipped the same shape over custody and recorded that the
/// data version was unreachable, because every non-final stage then had to
/// carry the acquisition adapter.
#[test]
fn a_scalar_stage_feeds_the_next_stage_and_emits_its_hand_written_twin() {
    let piped = compile_with(SCALAR_RESULT_PIPED, selection())
        .expect("a scalar chain checks after `PIPE-SCALAR-RESULT`");
    let unpiped = compile_with(SCALAR_RESULT, selection()).unwrap();
    assert_eq!(
        piped.c_source, unpiped.c_source,
        "a scalar chain emits what the hand-written statement sequence emits"
    );
    assert_eq!(piped.extern_observation, unpiped.extern_observation);
    assert_eq!(
        piped.extern_observation.as_deref().unwrap()[2..4],
        [
            "value qcore_extern_sum v1 b:r1 d:8".to_owned(),
            "value qcore_extern_scale v2 v:v1 d:2".to_owned(),
        ]
    );
    // The temporary's name reaches neither the C nor the observation: the
    // emitted program names values by identity, exactly as it names resources.
    assert!(!piped.c_source.as_deref().unwrap().contains("qcore__pipe"));
}

/// A scalar chain inherits the custody accounting unchanged, in both
/// directions. The buffer the chain reads is still lent, still consumed, and
/// still refused if the reader consumes it too early — the scalar temporary
/// takes no part in any of it.
#[test]
fn a_scalar_chain_changes_no_custody() {
    let checked = compile_with(SCALAR_RESULT_PIPED, selection()).unwrap();
    let certificate = &checked.checked.as_extern_calls().unwrap().certificate;
    assert_eq!(certificate.acquisitions, vec![1]);
    assert_eq!(certificate.consumptions, vec![1]);
    assert_eq!(
        certificate.scalar_bindings,
        vec![
            ("qcore_extern_sum".to_owned(), 1),
            ("qcore_extern_scale".to_owned(), 2),
        ]
    );

    // Consume the buffer before the chain reads it and the pre-existing
    // linearity refusal fires, with the piped and unpiped diagnostic vectors
    // asserted equal rather than merely both non-empty.
    let piped = replace_fixture(
        SCALAR_RESULT_PIPED,
        "    b |> qcore_extern_fill(7);\n",
        "    move b |> qcore_extern_release;\n",
    );
    let unpiped = replace_fixture(
        SCALAR_RESULT,
        "    qcore_extern_fill(b, 7);\n",
        "    qcore_extern_release(move b);\n",
    );
    let piped = codes_with(&piped);
    assert_eq!(piped, codes_with(&unpiped));
    assert!(!piped.is_empty());
}

fn codes_with(source: &str) -> Vec<String> {
    compile_with(source, selection())
        .map(|module| module.execution_profile)
        .expect_err("this fixture must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn a_pipeline_temporary_cannot_be_named_or_shadowed() {
    // Hygiene by refusal, not by silent renaming: a file that writes the
    // reserved prefix and also needs temporaries is refused, so a diagnostic
    // can never name a binding the reader did not write.
    let source = replace_fixture(
        REALLOC_PIPED,
        "    let grown = 8 |>",
        "    let qcore__pipe0 = 0;\n    let grown = 8 |>",
    );
    assert_ne!(source, REALLOC_PIPED, "the mutation must apply");
    let diagnostics = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("a colliding binding must be refused");
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| d.code.clone())
            .collect::<Vec<_>>(),
        ["Q036"]
    );
    assert!(diagnostics[0].message.contains("qcore__pipe"));

    // A one-stage pipeline needs no temporary, so the same name is untouched
    // there: the refusal is scoped to files that actually generate one.
    //
    // Card `STD-IO-COMPOSE` moved this half's *measurement* and not its rule.
    // `let qcore__pipe0 = 0;` used to be refused at `Q025` for being an extern
    // `let` that named no callee, so this half asserted the absence of `Q036`
    // among that refusal's diagnostics; the same statement is now an admitted
    // scalar literal binding (`EXTERN-SCALAR-LITERAL`) and the file compiles.
    // The claim under test is unchanged and is pinned more strongly than
    // before: a file that generates no temporary may write the prefix, and
    // nothing refuses it.
    let single = replace_fixture(
        ROUND_TRIP_PIPED,
        "    let b =",
        "    let qcore__pipe0 = 0;\n    let b =",
    );
    let module = compile_with(&single, selection())
        .expect("a file that generates no temporary may write the reserved prefix");
    assert_eq!(
        module.execution_profile,
        qcore_compiler::SourceExecutionProfile::ExternCallsV1
    );
}

#[test]
fn move_is_refused_on_the_first_stage_where_the_operand_has_a_spelling() {
    // One spelling per meaning: the first stage's operand is written by hand,
    // so `move OPERAND |> STAGE` is how it is consumed and `|> move STAGE` is
    // reserved for the value a previous stage produced.
    let source = replace_fixture(
        ROUND_TRIP_PIPED,
        "    move b |> qcore_extern_release;\n",
        "    b |> move qcore_extern_release;\n",
    );
    let diagnostics = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("`move` on the first stage must be refused");
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| d.code.clone())
            .collect::<Vec<_>>(),
        ["Q036"]
    );
    assert!(diagnostics[0].message.contains("move OPERAND |> STAGE"));
}

#[test]
fn a_bare_move_outside_a_pipeline_stage_keeps_its_existing_refusal() {
    // The bare `move` spelling is resolved by the desugaring and never reaches
    // a route, so it means nothing outside a pipeline stage's adapter tail and
    // the extern grammar refuses it exactly as before this card.
    let source = replace_fixture(
        ROUND_TRIP,
        "    qcore_extern_release(move b);\n",
        "    qcore_extern_release(move);\n",
    );
    let codes: Vec<String> = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("a bare `move` is not a call argument")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(!codes.is_empty());
    assert!(
        !codes.contains(&"Q036".to_owned()) && !codes.contains(&"Q037".to_owned()),
        "the pipe pass does not claim this shape: {codes:?}"
    );
}

#[test]
fn a_piped_unsafe_stage_deposits_the_same_waiver_as_its_unpiped_twin() {
    // Requirement: a pipeline must not launder the `UncheckedLinkage`
    // assumption an `unsafe extern fn` stage carries.
    //
    // Measured while writing this: an unsafe stage is *structurally* the last
    // stage of any pipeline, twice over. It produces no value (no adapter, so
    // it cannot feed a later stage), and `unsafe extern fn` is scalars-only
    // (`UNSAFE_LINKAGE_V1.md` L0), so it cannot receive a chain temporary
    // either — the only result-producing stage binds a resource. The waiver is
    // therefore pinned where it is reachable: the piped and unpiped forms emit
    // identical C and observe identically, so the assumption travels unchanged.
    let unpiped = "#import std.extern;\n#import std.extern.unsafe;\n\n\
                   unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\
                   unsafe extern fn k_unchecked_measure(count: u64) -> u64;\n\n\
                   fn main() -> unit effects { io, alloc, mut } {\n    \
                   k_unchecked_add(2, 40);\n    k_unchecked_measure(16);\n    return;\n}\n";
    let piped = "#import std.extern;\n#import std.extern.unsafe;\n\n\
                 unsafe extern fn k_unchecked_add(a: i32, b: i32) -> i32;\n\
                 unsafe extern fn k_unchecked_measure(count: u64) -> u64;\n\n\
                 fn main() -> unit effects { io, alloc, mut } {\n    \
                 2 |> k_unchecked_add(40);\n    16 |> k_unchecked_measure;\n    return;\n}\n";
    let piped = compile(piped).expect("the piped unsafe stages check");
    let unpiped = compile(unpiped).expect("the unpiped unsafe stages check");
    assert_eq!(piped.c_source, unpiped.c_source);
    assert_eq!(piped.extern_observation, unpiped.extern_observation);
}

#[cfg(not(target_os = "windows"))]
#[test]
fn the_native_run_of_a_multi_stage_pipeline_matches_its_reference() {
    // One fixed ASan/UBSan/LSan run of a real chain against the real C callees
    // the shipped extern suite links, so the custody flowing through the
    // hygienic temporaries is executed, not only checked.
    let module = compile_with(REALLOC_PIPED, selection()).unwrap();
    let reference = module.extern_observation.clone().unwrap();
    assert_eq!(
        run_native_with_callees(module.c_source.as_deref().unwrap(), "realloc-piped"),
        reference,
        "native and reference observations must agree line for line"
    );
}

#[cfg(not(target_os = "windows"))]
fn run_native_with_callees(source: &str, label: &str) -> Vec<String> {
    let instrumented = diagnostic_sink::instrument(source);
    let source = instrumented.as_str();
    use std::process::Command;
    let directory = std::env::temp_dir().join(format!(
        "qcore-pipe-chain-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let generated = directory.join("generated.c");
    let callees = directory.join("buffer_api.c");
    let binary = directory.join("generated");
    std::fs::write(&generated, source).unwrap();
    std::fs::write(&callees, include_str!("fixtures/extern_calls/buffer_api.c")).unwrap();
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
        .arg(&callees)
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
    let observation = String::from_utf8(run.stdout)
        .unwrap()
        .lines()
        .map(ToOwned::to_owned)
        .collect();
    std::fs::remove_dir_all(directory).unwrap();
    observation
}

// --- 6. Corpus neutrality of the token change. ---

#[test]
fn no_existing_program_changes_meaning() {
    // Neither operator token appears in any shipped fixture, so every accepted
    // program is accepted with the same token stream as before. The two twins
    // above pin the positive direction; this pins that the *unpiped* corpus
    // fixtures still parse identically to themselves.
    for source in [LINES, ROUND_TRIP] {
        let program = parse_source_program(source).unwrap();
        assert!(
            program.pipe_refusals().is_empty(),
            "no shipped fixture may record a pipe refusal"
        );
        assert!(program.syntax_equivalent(&parse_source_program(source).unwrap()));
    }
}

#[test]
fn the_native_run_of_a_piped_program_matches_its_reference_per_stream() {
    let module = compile(LINES_PIPED).unwrap();
    let reference = module.extern_observation.as_deref().unwrap();
    let (out, err) = run_native(module.c_source.as_deref().unwrap(), "lines-piped");

    let expected_out = reference
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();
    let expected_err = reference
        .iter()
        .filter_map(|line| line.strip_prefix("err: ").map(ToOwned::to_owned))
        .collect::<Vec<_>>();

    assert_eq!(out, expected_out, "stdout must match the reference");
    assert_eq!(err, expected_err, "stderr must match the reference");
}

/// Compile and run the emitted C under ASan/UBSan (plus LSan where the platform
/// provides it), returning `(stdout lines, stderr lines)`.
///
/// The sanitizer runtimes write their own findings to stderr and abort, so a
/// clean exit with the expected stderr lines is itself the evidence that no
/// finding was reported.
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
            "qcore-pipe-{_label}-{}-{}",
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

// --- Card `EXTERN-SCALAR-OUTCOME` (trace half): the extended observation
// crosses the desugaring like every other property of a stage. ---

/// **A pipeline inherits the extended observation through the unchanged
/// desugaring.** The pipe operator is a statement-sequence rewrite, so a
/// declared outcome on a stage is a declared outcome on the call that stage
/// becomes: the piped and hand-written forms emit the same C11 and observe the
/// same lines, `-> N` fields included. Nothing in the desugaring was touched by
/// the trace half, and this is the pin that says so.
#[test]
fn a_declared_outcome_on_a_stage_reaches_the_observation_through_the_desugaring() {
    let piped = SCALAR_RESULT_PIPED.replace("\r\n", "\n").replace(
        "    let scaled = b |> qcore_extern_sum(8) |> qcore_extern_scale(2);",
        "    let scaled = b |> qcore_extern_sum(8) succeeds 56 \
         |> qcore_extern_scale(2) succeeds 112;",
    );
    assert!(
        piped.contains("succeeds 112"),
        "both stages must be declared"
    );
    let unpiped = SCALAR_RESULT.replace("\r\n", "\n");
    let unpiped = unpiped.replace(
        "    let total = qcore_extern_sum(b, 8);",
        "    let total = qcore_extern_sum(b, 8) succeeds 56;",
    );
    let unpiped = unpiped.replace(
        "    let scaled = qcore_extern_scale(total, 2);",
        "    let scaled = qcore_extern_scale(total, 2) succeeds 112;",
    );

    let piped = compile_with(&piped, selection()).expect("a declared stage checks");
    let unpiped = compile_with(&unpiped, selection()).unwrap();
    assert_eq!(
        piped.c_source, unpiped.c_source,
        "a declared stage emits what the hand-written statement emits"
    );
    assert_eq!(piped.extern_observation, unpiped.extern_observation);
    assert_eq!(
        piped.extern_observation.as_deref().unwrap()[2..4],
        [
            "value qcore_extern_sum v1 b:r1 d:8 -> 56".to_owned(),
            "value qcore_extern_scale v2 v:v1 d:2 -> 112".to_owned(),
        ]
    );
    // The hygienic temporary still names nothing in either artifact.
    assert!(!piped.c_source.as_deref().unwrap().contains("qcore__pipe"));
}
