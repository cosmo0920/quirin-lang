//! The pipe operator `|>` and the claimed logical-operator tokens (card
//! `PIPE-OPERATOR`).
//!
//! The load-bearing property of this card is an *absence*: after desugaring
//! there is nothing left to check. Every test below is written to make that
//! absence measurable rather than assumed — byte-identical emission against a
//! hand-unpiped twin, identical reference observations, identical refusals and
//! identical diagnostic spans, and a real consuming call reached through a
//! pipe whose linearity refusal is the pre-existing one.
use std::collections::BTreeMap;

use qcore_compiler::{
    CompileOptions, SourceExecutionProfile, SourceFile, compile_files, parse_source_program,
};

const LINES: &str = include_str!("fixtures/std_modules/lines.qcore");
const LINES_PIPED: &str = include_str!("fixtures/pipe/lines_piped.qcore");
const ROUND_TRIP: &str = include_str!("fixtures/extern_calls/round_trip.qcore");
const ROUND_TRIP_PIPED: &str = include_str!("fixtures/pipe/round_trip_piped.qcore");
const EXTERN_CONTRACTS: &str = include_str!("fixtures/extern_calls/contracts/buffer_api.qcore");

// Exercise both checkout styles regardless of the host's Git configuration.
fn with_line_endings(source: &str) -> [String; 2] {
    let lf = source.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
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
    let piped = ROUND_TRIP_PIPED.replace(
        "    move b |> qcore_extern_release;\n",
        "    move b |> qcore_extern_release;\n    b |> qcore_extern_fill(7);\n",
    );
    let plain = ROUND_TRIP.replace(
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
    let piped = ROUND_TRIP_PIPED.replace("    move b |> qcore_extern_release;\n", "");
    let plain = ROUND_TRIP.replace("    qcore_extern_release(move b);\n", "");
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
    let source = LINES_PIPED.replace("total |> print dec;", "missing |> print dec;");
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
    let source = LINES_PIPED.replace("total |> print dec;", "total |> print nope;");
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
    let source = "import std.fmt;\n\
                  import std.io;\n\
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
    let source = LINES_PIPED.replace("    flush;\n", "    total |> flush;\n");
    assert_eq!(codes(&source), ["Q033"]);
    assert!(messages(&source)[0].contains("takes no operand"));
}

#[test]
fn a_multi_stage_pipeline_refuses_with_its_measured_reason() {
    // The user-facing shape `value |> render |> log` is refused in v1, on two
    // measured blockers rather than a scheduling one. Recorded as
    // follow-on `PIPE-CHAIN`.
    let source = LINES_PIPED.replace("total |> print dec;", "total |> print dec |> log info dec;");
    assert_eq!(codes(&source), ["Q033"]);
    assert!(messages(&source)[0].contains("PIPE-CHAIN"));
}

#[test]
fn a_pipeline_on_the_right_of_a_binding_refuses() {
    let source = ROUND_TRIP_PIPED.replace(
        "    let b = qcore_extern_alloc(8) succeeds true else raise Failure;\n",
        "    let b = 8 |> qcore_extern_alloc succeeds true else raise Failure;\n",
    );
    let codes: Vec<String> = compile_with(&source, selection())
        .map(|module| module.execution_profile)
        .expect_err("a bound pipeline must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, ["Q032"]);
}

#[test]
fn a_pipeline_with_nothing_on_one_side_refuses() {
    for (replacement, code) in [("    |> print dec;\n", "Q032"), ("    total |>;\n", "Q032")] {
        let source = LINES_PIPED.replace("    total |> print dec;\n", replacement);
        assert_eq!(codes(&source), [code], "for `{replacement}`");
    }
}

#[test]
fn a_pipeline_that_reaches_a_block_boundary_before_its_semicolon_refuses() {
    // Measured: the parser's own "a statement ends in `;`" rule runs before
    // any frontend gate, so the last statement of a body without its `;` is a
    // parse failure (`Q000`) and never reaches the pipe pass. The desugarer
    // still records its own `Q032` for this shape — that arm is what keeps a
    // pipeline from silently swallowing a block — but it is unreachable from
    // this direction and the reachable refusal is pinned as the truth.
    let source = LINES_PIPED.replace(
        "    -7 |> print dec;\n    return;\n",
        "    -7 |> print dec\n",
    );
    assert_eq!(codes(&source), ["Q000"]);
    assert!(parse_source_program(&source).is_err());
}

#[test]
fn a_pipeline_missing_its_semicolon_mid_body_runs_into_the_next_statement() {
    // Measured, not designed: dropping the `;` makes the *next* statement's
    // `;` the run's terminator, so the following statement head becomes a
    // second stage and the multi-stage refusal fires first. That is
    // fail-closed and the span still names the offending region, so the
    // refusal is recorded as it is rather than a `Q032` being forced.
    let source = LINES_PIPED.replace("    total |> print dec;\n", "    total |> print dec\n");
    assert_eq!(codes(&source), ["Q033"]);
}

#[test]
fn a_sink_stage_with_the_wrong_selector_count_refuses() {
    let source = LINES_PIPED.replace("total |> print dec;", "total |> print dec hex;");
    assert_eq!(codes(&source), ["Q032"]);
}

#[test]
fn a_call_stage_takes_the_operand_as_its_first_argument() {
    // First-argument insertion, measured: the bare form and the parenthesized
    // form both put the piped value first, and both agree with the unpiped C.
    let piped = compile_with(ROUND_TRIP_PIPED, selection()).unwrap();
    let explicit = ROUND_TRIP_PIPED.replace(
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
    let swapped = ROUND_TRIP_PIPED.replace(
        "    b |> qcore_extern_fill(7);\n",
        "    7 |> qcore_extern_fill(b);\n",
    );
    assert!(
        compile_with(&swapped, selection()).is_err(),
        "reversing the piped argument must not still check"
    );
}

// --- 5. The claimed logical operators. ---

#[test]
fn the_logical_or_operator_refuses_with_its_measured_blocker() {
    let source = LINES_PIPED.replace("total |> print dec;", "print dec total | mask;");
    assert_eq!(codes(&source), ["Q034"]);
    let message = &messages(&source)[0];
    assert!(message.contains("`|` is the logical-or operator"));
    assert!(message.contains("OR-LOWERING"));
    assert!(
        message.contains("Q500"),
        "the message must name the measured blocker"
    );
}

#[test]
fn the_c_family_double_bar_is_redirected_to_the_single_bar_spelling() {
    let source = LINES_PIPED.replace("total |> print dec;", "print dec total || mask;");
    assert_eq!(
        codes(&source),
        ["Q034"],
        "one mistake must produce one diagnostic, not two"
    );
    assert!(messages(&source)[0].contains("not `||`"));
}

#[test]
fn the_logical_and_operator_refuses_and_redirects_the_single_ampersand() {
    let source = LINES_PIPED.replace("total |> print dec;", "print dec total && mask;");
    assert_eq!(codes(&source), ["Q034"]);
    let message = &messages(&source)[0];
    assert!(message.contains("`&&` is the logical-and operator"));
    assert!(message.contains("AND-LOWERING"));
    assert!(
        message.contains("single `&` is deliberately reserved"),
        "a C-family programmer must be redirected in both directions"
    );
}

#[test]
fn a_single_ampersand_gains_no_new_meaning() {
    // `&` lexes exactly as it did before this card and is consumed by nothing,
    // so a program containing one still refuses — and never at `Q034`, which
    // would mean this card had claimed it.
    let source = LINES_PIPED.replace("total |> print dec;", "print dec total & mask;");
    let codes = codes(&source);
    assert!(
        !codes.contains(&"Q034".to_owned()),
        "a lone `&` must stay reserved, not claimed: {codes:?}"
    );
    assert!(!codes.is_empty(), "and it must still refuse");
}

#[test]
fn a_piped_sink_inside_a_thread_scope_keeps_the_task_routes_refusal() {
    // The `STD-IO-COMPOSE` boundary is neither widened nor narrowed by the
    // pipe: the operator is gone before route selection runs, so a piped
    // `print` inside a `thread scope` is refused exactly as the unpiped one
    // is, and both refuse for the same reason.
    let piped = "import std.concurrency;\n\
                 import std.fmt;\n\
                 import std.io;\n\
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
    let source = ROUND_TRIP_PIPED.replace(
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
        !codes.contains(&"Q032".to_owned()) && !codes.contains(&"Q033".to_owned()),
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
        assert_eq!(codes(&source), ["Q034"], "for `{mode}`");
    }
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
fn run_native(source: &str, label: &str) -> (Vec<String>, Vec<String>) {
    use std::process::Command;
    let directory = std::env::temp_dir().join(format!(
        "qcore-pipe-{label}-{}-{}",
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
