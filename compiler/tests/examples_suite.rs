//! The published `examples/` programs outside `examples/threads`, verified
//! rather than described (card `EXAMPLES-SUITE`, wave 21).
//!
//! # Why this file exists
//!
//! `examples/threads/README.md` set the standard this suite is held to: every
//! example compiles through the real pipeline, its emitted C builds under
//! `-Werror`-class flags, it runs under the sanitizers, and the transcript the
//! README publishes is read out of the file and compared with a real run, so
//! the documentation cannot drift silently in either direction.
//!
//! Five families are covered here — standard IO, pipes, branching, declared
//! extern calls and unsafe linkage. For each example this file establishes:
//!
//! 1. it **checks**, and takes the route its README says it takes;
//! 2. its emitted C compiles under `-Wall -Wextra -Werror -pedantic` and runs
//!    clean under ASan/UBSan (and LSan on Linux);
//! 3. its native stdout and stderr equal, **per stream**, the reference
//!    execution's projection of the same module; and
//! 4. every transcript, refused program and quoted diagnostic in the README is
//!    extracted from the file and compared with what really happens.
//!
//! # What this does not establish
//!
//! Nothing about the C behind the declared contracts. That agreement is a
//! declared premise the certificate records by name; a sanitized run can refute
//! it for these programs — a lost release is a leak, a double release a double
//! free — but no run establishes it in general. The same holds, more strongly,
//! for the unsafe-linkage example, whose callees nothing describes at all.
//!
//! No order between stdout and stderr is asserted anywhere, because none is
//! modeled: C gives none between two separately buffered streams.
#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::collections::BTreeMap;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files, format_diagnostic,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

// --- The published sources. ---

const CONTRACTS: &str = include_str!("../../examples/extern/contracts/buffer_api.qcore");
const BUFFER_API_C: &str = include_str!("../../examples/extern/buffer_api.c");
const UNCHECKED_API_C: &str = include_str!("../../examples/unsafe-linkage/unchecked_api.c");

const RENDERINGS: &str = include_str!("../../examples/stdio/renderings.qcore");
const SEVERITIES: &str = include_str!("../../examples/stdio/severities.qcore");
const BARRIER: &str = include_str!("../../examples/stdio/barrier.qcore");
const PIPELINE: &str = include_str!("../../examples/stdio/pipeline.qcore");

const RENDERED_LINES: &str = include_str!("../../examples/pipes/rendered_lines.qcore");
const SCALAR_STAGES: &str = include_str!("../../examples/pipes/scalar_stages.qcore");
const SCALAR_STAGES_HAND: &str =
    include_str!("../../examples/pipes/scalar_stages_hand_written.qcore");

const CONDITIONAL_RELEASE: &str =
    include_str!("../../examples/branching/conditional_release.qcore");
const CONDITION_TIER: &str = include_str!("../../examples/branching/condition_tier.qcore");
const EARLY_RETURN: &str = include_str!("../../examples/branching/early_return.qcore");

const BUFFER_ROUND_TRIP: &str = include_str!("../../examples/extern/buffer_round_trip.qcore");
const SCALAR_RESULTS: &str = include_str!("../../examples/extern/scalar_results.qcore");
const BORROWED_RENDER: &str = include_str!("../../examples/extern/borrowed_render.qcore");

const UNCHECKED_CALLS: &str = include_str!("../../examples/unsafe-linkage/unchecked_calls.qcore");

// --- The published pages. ---

const STDIO_README: &str = include_str!("../../examples/stdio/README.md");
const PIPES_README: &str = include_str!("../../examples/pipes/README.md");
const BRANCHING_README: &str = include_str!("../../examples/branching/README.md");
const EXTERN_README: &str = include_str!("../../examples/extern/README.md");
const UNSAFE_README: &str = include_str!("../../examples/unsafe-linkage/README.md");

// ---------------------------------------------------------------------------
// Compiling
// ---------------------------------------------------------------------------

/// The declared contract environment the extern-calling examples are checked
/// against, selected under the path the READMEs name.
fn contracts() -> Option<BTreeMap<String, String>> {
    Some(BTreeMap::from([(
        "examples/extern/contracts/buffer_api.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
}

fn compile_named(
    name: &str,
    source: &str,
    extern_contracts: Option<BTreeMap<String, String>>,
) -> Result<CompiledModule, Vec<FileDiagnostic>> {
    compile_files(
        &[SourceFile::new(name.to_owned(), source)],
        &CompileOptions {
            entry: None,
            extern_contracts,
            native_libraries: None,
            native_library_headers: None,
        },
    )
}

/// Compile a published example that calls nothing external.
fn plain(name: &str, source: &str) -> CompiledModule {
    compile_named(name, source, None)
        .unwrap_or_else(|error| panic!("examples/{name} does not check: {error:?}"))
}

/// Compile a published example against the declared contract environment.
fn contracted(name: &str, source: &str) -> CompiledModule {
    compile_named(name, source, contracts())
        .unwrap_or_else(|error| panic!("examples/{name} does not check: {error:?}"))
}

/// The rendered diagnostics a source is refused with, in order.
fn refusal(source: &str, extern_contracts: Option<BTreeMap<String, String>>) -> Vec<String> {
    compile_named("refusal.qcore", source, extern_contracts)
        .map(|module| module.execution_profile)
        .expect_err("this program must be refused")
        .iter()
        .map(format_diagnostic)
        .collect()
}

/// Just the codes, from the same refusal.
fn refusal_codes(source: &str, extern_contracts: Option<BTreeMap<String, String>>) -> Vec<String> {
    compile_named("refusal.qcore", source, extern_contracts)
        .map(|module| module.execution_profile)
        .expect_err("this program must be refused")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

// ---------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------

/// Compile the emitted C beside any extra translation units and run it under
/// ASan/UBSan (plus LSan where the platform provides it), returning
/// `(stdout, stderr)` as line vectors.
///
/// The compile flags are the ones the READMEs publish, plus the sanitizers. The
/// sanitizer runtimes write their findings to stderr and abort, so a clean exit
/// with the expected stderr is itself the evidence that nothing was reported. A
/// declared-outcome disagreement exits 70 and likewise fails here.
fn run(label: &str, c: &str, extra: &[(&str, &str)]) -> (Vec<String>, Vec<String>) {
    let (out, err) = run_raw(label, c, extra);
    (
        out.lines().map(ToOwned::to_owned).collect(),
        err.lines().map(ToOwned::to_owned).collect(),
    )
}

/// The same, returning each stream unsplit — what a parser of the emitted
/// harness's own instrumentation needs.
fn run_raw(label: &str, c: &str, extra: &[(&str, &str)]) -> (String, String) {
    let directory = std::env::temp_dir().join(format!(
        "qcore-examples-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let generated = directory.join("generated.c");
    let binary = directory.join("generated");
    std::fs::write(&generated, c).unwrap();

    let mut compiler = Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "cc".into()));
    compiler.args([
        "-std=c11",
        "-Wall",
        "-Wextra",
        "-Werror",
        "-pedantic",
        "-g",
        "-O0",
        "-fsanitize=address,undefined",
        "-fno-omit-frame-pointer",
    ]);
    compiler.arg(&generated);
    for (file, text) in extra {
        let path = directory.join(file);
        std::fs::write(&path, text).unwrap();
        compiler.arg(&path);
    }
    let compiled = compiler.arg("-o").arg(&binary).output().unwrap();
    assert!(
        compiled.status.success(),
        "{label}: the C compiler rejected the emitted program:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    // Apple's AddressSanitizer ships no leak detector, so leak checking is a
    // Linux-only observation — the standing platform boundary, not a choice
    // this card makes.
    let leaks = if cfg!(target_os = "linux") { 1 } else { 0 };
    let output = Command::new(&binary)
        .env(
            "ASAN_OPTIONS",
            format!("detect_leaks={leaks}:halt_on_error=1"),
        )
        .env("UBSAN_OPTIONS", "halt_on_error=1:print_stacktrace=1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{label}: the emitted program failed under the sanitizers:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let out = String::from_utf8(output.stdout).unwrap();
    let err = String::from_utf8(output.stderr).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    (out, err)
}

/// The `out:`/`err:` projections of a route that publishes a two-stream
/// observation.
fn projections(module: &CompiledModule) -> (Vec<String>, Vec<String>) {
    let observation = module
        .extern_observation
        .as_deref()
        .expect("this route publishes a two-stream observation");
    let out = observation
        .iter()
        .filter_map(|line| line.strip_prefix("out: ").map(ToOwned::to_owned))
        .collect();
    let err = observation
        .iter()
        .filter_map(|line| line.strip_prefix("err: ").map(ToOwned::to_owned))
        .collect();
    (out, err)
}

// ---------------------------------------------------------------------------
// Reading the published pages
// ---------------------------------------------------------------------------

/// The fenced block immediately following `<!-- MARKER -->` in a README.
///
/// Extracting rather than restating is the whole point: an edit to either the
/// page or the program that the other does not follow fails a test.
fn block(readme: &str, marker: &str) -> String {
    let tail = readme
        .split_once(&format!("<!-- {marker} -->\n"))
        .unwrap_or_else(|| panic!("the README no longer carries the marker `{marker}`"))
        .1;
    let body = tail
        .split_once("```")
        .unwrap_or_else(|| panic!("`{marker}` is not followed by a fenced block"))
        .1;
    let body = body
        .split_once('\n')
        .unwrap_or_else(|| panic!("`{marker}`'s fence is unterminated"))
        .1;
    body.split_once("```")
        .unwrap_or_else(|| panic!("`{marker}`'s fenced block is unterminated"))
        .0
        .to_owned()
}

/// The same, as a line vector — the shape a run's output comes back in.
fn documented(readme: &str, marker: &str) -> Vec<String> {
    block(readme, marker)
        .lines()
        .map(ToOwned::to_owned)
        .collect()
}

// ---------------------------------------------------------------------------
// 1. examples/stdio — `std.io`, `std.logger`, `std.fmt`, and the composed route
// ---------------------------------------------------------------------------

#[test]
fn the_stdio_examples_take_the_routes_their_page_says_they_take() {
    for (name, source) in [
        ("stdio/renderings.qcore", RENDERINGS),
        ("stdio/severities.qcore", SEVERITIES),
        ("stdio/barrier.qcore", BARRIER),
    ] {
        let module = plain(name, source);
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::StandardIoV1,
            "{name}"
        );
        assert_eq!(module.entry.as_deref(), Some("main"));
        // The derived effect row is exactly `io`: this route's operand grammar
        // admits no resource, so it can neither allocate nor mutate one. That
        // is the claim `renderings.qcore` makes in prose.
        let checked = module.checked.as_standard_io().unwrap();
        assert_eq!(checked.certificate.effects.iter().count(), 1);
        assert!(
            checked
                .certificate
                .effects
                .contains(&qcore_core::typed_errors::Effect::Io)
        );
    }

    // The barrier-only module needs no rendering import and names none.
    let barrier = plain("stdio/barrier.qcore", BARRIER);
    let checked = barrier.checked.as_standard_io().unwrap();
    assert_eq!(checked.certificate.flushes, 1);
    assert!(checked.certificate.renderings.is_empty());
    assert!(!BARRIER.contains("import std.fmt;"));

    // And the composed example is composed: neither half is published alone.
    let pipeline = contracted("stdio/pipeline.qcore", PIPELINE);
    assert_eq!(
        pipeline.execution_profile,
        SourceExecutionProfile::ComposedStdIoExternV1
    );
    assert!(pipeline.checked.as_extern_calls().is_none());
    assert!(pipeline.checked.as_standard_io().is_none());
    assert!(pipeline.checked.as_composed().is_some());
}

#[test]
fn every_stdio_example_prints_through_the_single_emission_route() {
    for (name, module) in [
        ("renderings", plain("stdio/renderings.qcore", RENDERINGS)),
        ("severities", plain("stdio/severities.qcore", SEVERITIES)),
        ("pipeline", contracted("stdio/pipeline.qcore", PIPELINE)),
    ] {
        let c = module.c_source.as_deref().unwrap();
        if name == "pipeline" {
            assert_eq!(
                c.matches("static void qcore_diagnostic_stdout(").count(),
                2,
                "the explicit system trace must use one shared adapter"
            );
        } else {
            assert_eq!(
                qcore_core::io_emit::direct_stdio_calls(c),
                Vec::new(),
                "{name} emits a direct standard-output call outside the shared route"
            );
        }
        // Non-vacuous: the program really does write through source IO too.
        assert!(c.contains("QCORE_IO_WRITE("), "{name} writes nothing");
    }
}

#[test]
fn the_stdio_examples_match_their_reference_execution_per_stream() {
    for (label, module, extra) in [
        (
            "renderings",
            plain("stdio/renderings.qcore", RENDERINGS),
            Vec::new(),
        ),
        (
            "severities",
            plain("stdio/severities.qcore", SEVERITIES),
            Vec::new(),
        ),
        ("barrier", plain("stdio/barrier.qcore", BARRIER), Vec::new()),
        (
            "pipeline",
            contracted("stdio/pipeline.qcore", PIPELINE),
            vec![("buffer_api.c", BUFFER_API_C)],
        ),
    ] {
        let (expected_out, expected_err) = projections(&module);
        let (out, err) = run(label, module.c_source.as_deref().unwrap(), &extra);
        assert_eq!(
            out, expected_out,
            "{label}: stdout must match the reference"
        );
        assert_eq!(
            err, expected_err,
            "{label}: stderr must match the reference"
        );
    }
}

#[test]
fn the_stdio_page_publishes_the_transcripts_its_programs_print() {
    let (renderings_out, renderings_err) = run(
        "renderings-doc",
        plain("stdio/renderings.qcore", RENDERINGS)
            .c_source
            .as_deref()
            .unwrap(),
        &[],
    );
    assert_eq!(
        renderings_out,
        documented(STDIO_README, "transcript: renderings stdout")
    );
    assert!(
        renderings_err.is_empty(),
        "the page says it writes no log line"
    );

    let (severities_out, severities_err) = run(
        "severities-doc",
        plain("stdio/severities.qcore", SEVERITIES)
            .c_source
            .as_deref()
            .unwrap(),
        &[],
    );
    assert!(severities_out.is_empty(), "the page says it prints nothing");
    assert_eq!(
        severities_err,
        documented(STDIO_README, "transcript: severities stderr")
    );

    let (barrier_out, barrier_err) = run(
        "barrier-doc",
        plain("stdio/barrier.qcore", BARRIER)
            .c_source
            .as_deref()
            .unwrap(),
        &[],
    );
    assert_eq!(
        barrier_out,
        documented(STDIO_README, "transcript: barrier stdout")
    );
    assert!(
        barrier_out.is_empty(),
        "the documented barrier transcript is empty"
    );
    assert!(barrier_err.is_empty());

    let (pipeline_out, pipeline_err) = run(
        "pipeline-doc",
        contracted("stdio/pipeline.qcore", PIPELINE)
            .c_source
            .as_deref()
            .unwrap(),
        &[("buffer_api.c", BUFFER_API_C)],
    );
    assert_eq!(
        pipeline_out,
        documented(STDIO_README, "transcript: pipeline stdout")
    );
    assert_eq!(
        pipeline_err,
        documented(STDIO_README, "transcript: pipeline stderr")
    );
}

#[test]
fn the_stdio_transcript_checks_would_notice_a_changed_line() {
    // Otherwise the tests above could pass on an empty or truncated extraction.
    // The barrier's transcript is deliberately empty and is excluded here; its
    // own emptiness is asserted beside it above.
    for (marker, first) in [
        ("transcript: renderings stdout", "42"),
        ("transcript: severities stderr", "[TRACE] 42"),
        ("transcript: pipeline stdout", "acquire demo_alloc r1 ok"),
        ("transcript: pipeline stderr", "[INFO] 112"),
    ] {
        let lines = documented(STDIO_README, marker);
        assert!(!lines.is_empty(), "`{marker}` extracted nothing");
        assert_eq!(lines[0], first, "`{marker}` extracted the wrong block");
    }
}

#[test]
fn the_page_and_example_cover_all_six_logger_levels() {
    for level in ["trace", "debug", "info", "warn", "error", "fatal"] {
        assert!(SEVERITIES.contains(&format!("log {level} ")));
    }
    // `error` also remains the declaration keyword outside logger context.
    assert!(BUFFER_ROUND_TRIP.contains("error Failure { Failed; }"));
}

/// Each capability is gated by its own import and by no other — the page's
/// table, checked.
#[test]
fn the_import_table_on_the_page_is_the_gate_that_runs() {
    let without_io = RENDERINGS.replace("import std.io;\n", "");
    assert_eq!(refusal_codes(&without_io, None), ["Q015"]);
    let without_fmt = RENDERINGS.replace("import std.fmt;\n", "");
    assert_eq!(refusal_codes(&without_fmt, None), ["Q015"]);
    let without_logger = SEVERITIES.replace("import std.logger;\n", "");
    assert_eq!(refusal_codes(&without_logger, None), ["Q015"]);
    // `flush` is the logger's, not `std.fmt`'s: dropping the barrier's only
    // import refuses, and the barrier has no `std.fmt` import to drop.
    let bare_barrier = BARRIER.replace("import std.logger;\n", "");
    assert_eq!(refusal_codes(&bare_barrier, None), ["Q015"]);
    // A misspelled module is unknown rather than ignored.
    let misspelled = RENDERINGS.replace("import std.io;", "import std.stdio;");
    assert_eq!(refusal_codes(&misspelled, None), ["Q014"]);
    // `flush` takes no operand.
    let flush_operand = SEVERITIES.replace("flush;", "flush 1;");
    assert_eq!(refusal_codes(&flush_operand, None), ["Q030"]);
}

/// The composed route's declared outcomes reach the emitted program as real
/// comparisons — the page's "the emitted C does not believe it" claim.
#[test]
fn the_composed_example_does_not_believe_its_declared_outcomes() {
    let module = contracted("stdio/pipeline.qcore", PIPELINE);
    let c = module.c_source.as_deref().unwrap();
    assert!(c.contains("if (v1 != 56)"), "emitted C: {c}");
    assert!(c.contains("QCORE_DIAG_EXTERN_OUTCOME"));
    assert!(c.contains("return 70;"));
    // The trace line and the rendering name the same number, which is the
    // `-> N` field's whole purpose.
    let observation = module.extern_observation.as_deref().unwrap();
    assert!(observation.contains(&"out: value demo_sum v1 b:r1 d:8 -> 56".to_owned()));
    assert!(observation.contains(&"out: 56".to_owned()));
    assert!(observation.contains(&"err: [INFO] 112".to_owned()));
    // The hygienic temporary's name reaches neither the C nor the observation.
    assert!(!c.contains("qcore__pipe"));
}

// ---------------------------------------------------------------------------
// 2. examples/pipes — the `|>` operator
// ---------------------------------------------------------------------------

#[test]
fn the_piped_and_hand_written_chains_emit_the_same_c_byte_for_byte() {
    let piped = contracted("pipes/scalar_stages.qcore", SCALAR_STAGES);
    let hand = contracted("pipes/scalar_stages_hand_written.qcore", SCALAR_STAGES_HAND);
    assert_eq!(
        piped.c_source, hand.c_source,
        "a chain and its hand-written statement sequence must emit the same C11"
    );
    assert_eq!(
        piped.extern_observation, hand.extern_observation,
        "and observe identically"
    );
    // The temporaries are named in one file and hygienic in the other, and
    // neither set of names reaches the emitted C.
    assert!(SCALAR_STAGES_HAND.contains("let stage0 ="));
    let c = piped.c_source.as_deref().unwrap();
    assert!(!c.contains("stage0") && !c.contains("qcore__pipe"));
}

#[test]
fn the_pipes_examples_match_their_reference_execution() {
    let lines = plain("pipes/rendered_lines.qcore", RENDERED_LINES);
    let (expected_out, expected_err) = projections(&lines);
    let (out, err) = run("rendered-lines", lines.c_source.as_deref().unwrap(), &[]);
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);

    // The explicit trace adapter renders the structured extern observations
    // on stdout; source-level IO remains a separate stream projection.
    let stages = contracted("pipes/scalar_stages.qcore", SCALAR_STAGES);
    assert!(SCALAR_STAGES.contains("log_level trace;"));
    let (out, err) = run(
        "scalar-stages",
        stages.c_source.as_deref().unwrap(),
        &[("buffer_api.c", BUFFER_API_C)],
    );
    assert_eq!(
        out,
        documented(PIPES_README, "transcript: scalar_stages stdout")
    );
    assert!(err.is_empty());
    // Resource observations remain in the typed diagnostic route rather than
    // becoming bytes in the program stream.
    assert!(
        stages
            .c_source
            .as_deref()
            .unwrap()
            .contains("QCORE_DIAG_RESOURCE_ACQUIRE")
    );
}

#[test]
fn the_pipes_page_publishes_the_transcripts_its_programs_print() {
    let lines = plain("pipes/rendered_lines.qcore", RENDERED_LINES);
    let (out, err) = run(
        "rendered-lines-doc",
        lines.c_source.as_deref().unwrap(),
        &[],
    );
    assert_eq!(
        out,
        documented(PIPES_README, "transcript: rendered_lines stdout")
    );
    assert_eq!(
        err,
        documented(PIPES_README, "transcript: rendered_lines stderr")
    );

    let stages = contracted("pipes/scalar_stages.qcore", SCALAR_STAGES);
    let (out, _) = run(
        "scalar-stages-doc",
        stages.c_source.as_deref().unwrap(),
        &[("buffer_api.c", BUFFER_API_C)],
    );
    assert_eq!(
        out,
        documented(PIPES_README, "transcript: scalar_stages stdout")
    );
    assert_eq!(out.len(), 7, "the full explicit call trace is published");
}

/// A call with no declared outcome has no `-> N` field, which is the
/// distinction the pipes page draws against `../stdio/pipeline.qcore`.
#[test]
fn an_undeclared_outcome_prints_no_arrow() {
    let stages = contracted("pipes/scalar_stages.qcore", SCALAR_STAGES);
    let observation = stages.extern_observation.as_deref().unwrap();
    assert!(
        observation
            .iter()
            .any(|line| line.starts_with("value demo_sum "))
    );
    assert!(
        observation.iter().all(|line| !line.contains(" -> ")),
        "no call in this example declares an outcome: {observation:?}"
    );
    // And the neighbouring example that does declare one does print it.
    let declared = contracted("extern/scalar_results.qcore", SCALAR_RESULTS);
    assert!(
        declared
            .extern_observation
            .as_deref()
            .unwrap()
            .iter()
            .any(|line| line.ends_with(" -> 56"))
    );
}

/// The refused program the pipes page publishes, compiled, and the diagnostic
/// it quotes, compared with the real one.
#[test]
fn the_documented_pipe_refusal_is_the_refusal_the_compiler_gives() {
    let program = block(PIPES_README, "refusal: Q037");
    assert!(
        program.contains("|> print dec |> log info dec;"),
        "{program}"
    );
    assert_eq!(refusal_codes(&program, None), ["Q037"]);
    assert_eq!(
        refusal(&program, None),
        vec![
            block(PIPES_README, "diagnostic: Q037")
                .trim_end()
                .to_owned()
        ],
        "the page quotes a diagnostic the compiler does not give"
    );
}

/// The two neighbouring refusals the page names beside it.
#[test]
fn the_pipe_refusals_the_page_names_beside_it_hold() {
    let bound_sink = RENDERED_LINES.replace(
        "    total |> print dec;\n",
        "    let x = total |> print dec;\n",
    );
    assert_eq!(refusal_codes(&bound_sink, None), ["Q036"]);
    let piped_barrier = RENDERED_LINES.replace("    flush;\n", "    total |> flush;\n");
    assert_eq!(refusal_codes(&piped_barrier, None), ["Q037"]);
}

// ---------------------------------------------------------------------------
// 3. examples/branching — `if`/`else` and the condition tier
// ---------------------------------------------------------------------------

#[test]
fn the_branching_examples_take_the_sequential_route_and_run_clean() {
    for (label, source, marker) in [
        (
            "conditional-release",
            CONDITIONAL_RELEASE,
            "transcript: conditional_release stdout",
        ),
        (
            "condition-tier",
            CONDITION_TIER,
            "transcript: condition_tier stdout",
        ),
        (
            "early-return",
            EARLY_RETURN,
            "transcript: early_return stdout",
        ),
    ] {
        let module = plain(&format!("branching/{label}.qcore"), source);
        assert_eq!(
            module.execution_profile,
            SourceExecutionProfile::SequentialCleanupV1,
            "{label}"
        );
        let reference = module
            .reference_observation
            .as_ref()
            .expect("this route publishes a reference execution");
        let (raw_out, raw_err) = run_raw(label, module.c_source.as_deref().unwrap(), &[]);
        assert!(raw_err.is_empty(), "{label} writes nothing to stderr");
        let out: Vec<String> = raw_out.lines().map(ToOwned::to_owned).collect();
        assert!(!reference.trace.is_empty());
        assert_eq!(out, documented(BRANCHING_README, marker), "{label}");
        assert_eq!(reference.trace.len(), 2, "{label}: custody trace drift");
        assert_eq!(out.len(), 3, "{label}: system trace presentation drift");
    }
}

/// The custody join, and the conditional leak it refuses — the page's refused
/// program and quoted diagnostic, both checked.
#[test]
fn the_documented_conditional_leak_is_the_refusal_the_compiler_gives() {
    let program = block(BRANCHING_README, "refusal: Q046");
    assert_eq!(refusal_codes(&program, None), ["Q046"]);
    assert_eq!(
        refusal(&program, None),
        vec![
            block(BRANCHING_README, "diagnostic: Q046")
                .trim_end()
                .to_owned()
        ]
    );
    // Non-vacuous: the published example is the same program with the arm
    // restored, and it checks.
    assert!(CONDITIONAL_RELEASE.contains("        drop a;\n    } else {\n        drop a;"));
}

/// The parenthesization rule, likewise.
#[test]
fn the_documented_bitwise_refusal_is_the_refusal_the_compiler_gives() {
    let program = block(BRANCHING_README, "refusal: Q045");
    assert_eq!(refusal_codes(&program, None), ["Q045"]);
    assert_eq!(
        refusal(&program, None),
        vec![
            block(BRANCHING_README, "diagnostic: Q045")
                .trim_end()
                .to_owned()
        ]
    );
    // The grouped spelling the page tells the reader to write is the program.
    let grouped = program.replace("mask | 3 == 15", "(mask | 3) == 15");
    compile_named("grouped.qcore", &grouped, None).expect("the parenthesized spelling checks");
    // The same rule in the other direction, for `!`.
    let negated = program.replace("mask | 3 == 15", "!3 < 5");
    assert_eq!(refusal_codes(&negated, None), ["Q045"]);
    // And the two shapes the page names under "not expressible".
    for condition in ["1 < 2 < 3", "(1 |> 2) == 3"] {
        let source = program.replace("mask | 3 == 15", condition);
        assert_eq!(refusal_codes(&source, None), ["Q045"], "for `{condition}`");
    }
}

/// Each row of the lowering table the branching page publishes, pinned by
/// compiling both spellings and comparing the emitted C byte for byte.
#[test]
fn every_row_of_the_documented_lowering_table_is_a_byte_identity() {
    fn program(condition: &str, then_arm: &str, else_arm: &str) -> String {
        format!(
            "error Failure {{ Failed; }}\n\
             \n\
             fn main() -> i32 {{\n    \
                 let a = qcore_acquire(1, true, Failure);\n    \
                 if ({condition}) {{\n{then_arm}    \
                 }} else {{\n{else_arm}    \
                 }}\n    \
                 return 0;\n\
             }}\n"
        )
    }
    // A `qcore_check` distinguishes the arms, so each equality below is a
    // statement about the lowering rather than about two identical arms. A
    // check leaves the ledger alone, which is what the custody join requires.
    const X: &str = "        qcore_check(true, Failure);\n        drop a;\n";
    const Y: &str = "        drop a;\n";

    let emit = |source: &str| {
        compile_named("table.qcore", source, None)
            .expect("both spellings check")
            .c_source
            .unwrap()
    };

    // `if (A || B) { X } else { Y }` == `if (A) { X } else if (B) { X } else { Y }`
    let disjunction = emit(&program("3 < 5 || 2 == 2", X, Y));
    let nested = emit(&format!(
        "error Failure {{ Failed; }}\n\
         \n\
         fn main() -> i32 {{\n    \
             let a = qcore_acquire(1, true, Failure);\n    \
             if (3 < 5) {{\n{X}    }} else if (2 == 2) {{\n{X}    }} else {{\n{Y}    }}\n    \
             return 0;\n\
         }}\n"
    ));
    assert_eq!(disjunction, nested, "`||` must be the right-nested branch");

    // `if (A && B) { X } else { Y }` == `if (A) { if (B) { X } else { Y } } else { Y }`
    let conjunction = emit(&program("3 < 5 && 2 == 2", X, Y));
    let inner = emit(&format!(
        "error Failure {{ Failed; }}\n\
         \n\
         fn main() -> i32 {{\n    \
             let a = qcore_acquire(1, true, Failure);\n    \
             if (3 < 5) {{\n        if (2 == 2) {{\n    {X}        }} else {{\n    {Y}        }}\n    }} else {{\n{Y}    }}\n    \
             return 0;\n\
         }}\n"
    ));
    assert_eq!(conjunction, inner, "`&&` must be the inner branch");

    // `if (!C) { X } else { Y }` == `if (C) { Y } else { X }` — an arm swap.
    assert_eq!(
        emit(&program("!(3 < 5)", X, Y)),
        emit(&program("3 < 5", Y, X)),
        "`!` must be an arm swap"
    );
}

/// The early-return account: the arm that returns discharges at its own exit,
/// and removing that discharge refuses there rather than at the join.
#[test]
fn removing_the_early_arms_discharge_refuses_at_that_exit() {
    let module = plain("branching/early_return.qcore", EARLY_RETURN);
    // The published entry returns 1 — the returning arm is the one that runs.
    assert!(
        format!("{:?}", module.reference_observation.as_ref().unwrap()).contains("value: 1"),
        "the page says this entry's result is 1"
    );
    let without = EARLY_RETURN.replace("        drop a;\n        return 1;", "        return 1;");
    let codes = refusal_codes(&without, None);
    assert!(!codes.is_empty(), "an undischarged early exit must refuse");
    assert!(
        !codes.contains(&"Q046".to_owned()),
        "…at the exit, not at the join: {codes:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. examples/extern — declared contracts, `succeeds`, `borrow NAME`
// ---------------------------------------------------------------------------

#[test]
fn the_extern_examples_check_against_the_published_contract_module() {
    for (name, source, profile) in [
        (
            "extern/buffer_round_trip.qcore",
            BUFFER_ROUND_TRIP,
            SourceExecutionProfile::ExternCallsV1,
        ),
        (
            "extern/scalar_results.qcore",
            SCALAR_RESULTS,
            SourceExecutionProfile::ExternCallsV1,
        ),
        (
            "extern/borrowed_render.qcore",
            BORROWED_RENDER,
            SourceExecutionProfile::ComposedStdIoExternV1,
        ),
    ] {
        let module = contracted(name, source);
        assert_eq!(module.execution_profile, profile, "{name}");
        // The page's claim that the contract is a premise, not a proof: every
        // admitted callee is carried by name, beside the standing two.
        use qcore_compiler::extern_call::ExternAssumption;
        let assumptions = module.declared_assumptions();
        assert!(assumptions.contains(&ExternAssumption::DeclaredContract("demo_alloc".to_owned())));
        assert!(assumptions.contains(&ExternAssumption::NoExtentTransport));
        assert!(assumptions.contains(&ExternAssumption::LinkageSuppliedByBuilder));
    }

    // And the environment really is required: without it the same source is
    // refused rather than admitted on some default.
    assert_eq!(refusal_codes(BUFFER_ROUND_TRIP, None), ["Q023"]);
}

#[test]
fn the_extern_examples_match_their_reference_execution() {
    for (label, source, single_stream) in [
        ("buffer-round-trip", BUFFER_ROUND_TRIP, true),
        ("scalar-results", SCALAR_RESULTS, true),
        ("borrowed-render", BORROWED_RENDER, false),
    ] {
        let module = contracted(label, source);
        let (out, err) = run(
            label,
            module.c_source.as_deref().unwrap(),
            &[("buffer_api.c", BUFFER_API_C)],
        );
        if single_stream {
            assert!(!out.is_empty(), "{label}: explicit trace is missing");
            assert!(err.is_empty(), "{label}");
        } else {
            let (expected_out, expected_err) = projections(&module);
            assert_eq!(out, expected_out, "{label}: stdout");
            assert_eq!(err, expected_err, "{label}: stderr");
        }
        // A positive leak control in every case: each program allocates and
        // releases one real buffer, so a lost release would be reported.
        let acquisitions = module
            .checked
            .as_extern_calls()
            .map(|checked| checked.certificate.acquisitions.as_slice())
            .or_else(|| {
                module
                    .checked
                    .as_composed()
                    .map(|checked| checked.certificate.externs.acquisitions.as_slice())
            });
        assert_eq!(acquisitions, Some(&[1][..]));
    }
}

#[test]
fn the_extern_page_publishes_the_transcripts_its_programs_print() {
    for (label, source, out_marker, err_marker) in [
        (
            "buffer-round-trip-doc",
            BUFFER_ROUND_TRIP,
            "transcript: buffer_round_trip stdout",
            None,
        ),
        (
            "scalar-results-doc",
            SCALAR_RESULTS,
            "transcript: scalar_results stdout",
            None,
        ),
        (
            "borrowed-render-doc",
            BORROWED_RENDER,
            "transcript: borrowed_render stdout",
            Some("transcript: borrowed_render stderr"),
        ),
    ] {
        let module = contracted(label, source);
        let (out, err) = run(
            label,
            module.c_source.as_deref().unwrap(),
            &[("buffer_api.c", BUFFER_API_C)],
        );
        assert_eq!(out, documented(EXTERN_README, out_marker), "{label}");
        assert!(out.len() >= 5, "{label}: documented trace was truncated");
        match err_marker {
            Some(marker) => assert_eq!(err, documented(EXTERN_README, marker), "{label}"),
            None => assert!(err.is_empty(), "{label}"),
        }
    }
}

/// The borrowed render's checked precondition: the identity must be live at the
/// statement. This is the page's "move the release above either of them"
/// sentence, executed.
#[test]
fn a_borrowed_render_after_the_release_refuses() {
    // Move the release above the last rendering and change nothing else, so the
    // only statement whose meaning moved is that rendering.
    let moved = BORROWED_RENDER
        .replace("    demo_release(move b);\n", "")
        .replace(
            "    log warn hex borrow b;\n",
            "    demo_release(move b);\n    log warn hex borrow b;\n",
        );
    assert_eq!(
        refusal_codes(&moved, contracts()),
        ["Q043"],
        "a render of a released identity must refuse, and at this route's own code"
    );

    // The renderings really do name the identity the trace names, rather than
    // any byte of the object: `r1` is rendered as `1` and as `0x00000001`.
    let module = contracted("extern/borrowed_render.qcore", BORROWED_RENDER);
    let observation = module.extern_observation.as_deref().unwrap();
    assert!(observation.contains(&"out: acquire demo_alloc r1 ok".to_owned()));
    assert!(observation.contains(&"out: 1".to_owned()));
    assert!(observation.contains(&"err: [WARN] 0x00000001".to_owned()));
    let checked = module.checked.as_composed().unwrap();
    assert_eq!(checked.certificate.borrowed_renders.len(), 2);
}

/// And on a route with no custody model there is nothing a borrow could name —
/// the page's account of why this form needs the composition.
#[test]
fn a_borrowed_render_without_a_custody_model_refuses() {
    let source = RENDERINGS.replace("print dec total;", "print dec borrow total;");
    assert_eq!(refusal_codes(&source, None), ["Q028"]);
}

// ---------------------------------------------------------------------------
// 5. examples/unsafe-linkage — `unsafe extern fn` and the manifest
// ---------------------------------------------------------------------------

#[test]
fn the_unsafe_linkage_example_carries_every_waiver_by_name() {
    use qcore_compiler::extern_call::ExternAssumption;

    let module = plain("unsafe-linkage/unchecked_calls.qcore", UNCHECKED_CALLS);
    assert_eq!(
        module.execution_profile,
        SourceExecutionProfile::ComposedStdIoExternV1
    );
    let assumptions = module.declared_assumptions();
    assert_eq!(assumptions.len(), 5, "{assumptions:?}");
    for (callee, signature) in [
        ("demo_hidden_effect", "(uint32_t) -> void"),
        ("demo_measure", "(uint64_t) -> uint64_t"),
        ("demo_unchecked_add", "(int32_t, int32_t) -> int32_t"),
    ] {
        assert!(
            assumptions.contains(&ExternAssumption::UncheckedLinkage {
                callee: callee.to_owned(),
                signature: signature.to_owned(),
            }),
            "{callee} must be carried by name: {assumptions:?}"
        );
    }
    assert!(assumptions.contains(&ExternAssumption::NoExtentTransport));
    assert!(assumptions.contains(&ExternAssumption::LinkageSuppliedByBuilder));
}

#[test]
fn the_unsafe_linkage_example_runs_against_the_real_uncontracted_c() {
    let module = plain("unsafe-linkage/unchecked_calls.qcore", UNCHECKED_CALLS);
    let (out, err) = run(
        "unchecked-calls",
        module.c_source.as_deref().unwrap(),
        &[("unchecked_api.c", UNCHECKED_API_C)],
    );
    assert!(!module.extern_observation.as_deref().unwrap().is_empty());
    assert_eq!(
        out,
        documented(UNSAFE_README, "transcript: unchecked_calls stdout")
    );
    assert!(out.is_empty());

    // The source explicitly logs two caller-authored markers at trace level.
    // They do not turn an unchecked return or the callee's hidden state
    // mutation into a checked observation.
    assert_eq!(
        err,
        documented(UNSAFE_README, "transcript: unchecked_calls stderr")
    );
    assert_eq!(err, ["[TRACE] 42", "[TRACE] 16"]);
    assert!(
        !module
            .extern_observation
            .as_deref()
            .unwrap()
            .iter()
            .any(|line| line.contains("hidden state")),
        "the checked observation must not mention what it cannot see"
    );
}

/// A manifest that does not load refuses the module rather than being ignored —
/// the page's `Q039` sentence.
#[test]
fn a_manifest_that_does_not_load_refuses_the_module() {
    const MANIFEST: &str = include_str!("../../examples/unsafe-linkage/unchecked_demo.qcorelib");
    let broken = MANIFEST.replace("data_model lp64;", "data_model ilp32;");
    let diagnostics = compile_files(
        &[SourceFile::new(
            "unchecked_calls.qcore".to_owned(),
            UNCHECKED_CALLS,
        )],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: Some(BTreeMap::from([(
                "examples/unsafe-linkage/unchecked_demo.qcorelib".to_owned(),
                broken,
            )])),
            native_library_headers: None,
        },
    )
    .map(|module| module.execution_profile)
    .expect_err("a malformed manifest must refuse");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "Q039"),
        "{diagnostics:?}"
    );

    // And the published manifest does load, sharpening the same five premises
    // rather than adding a sixth.
    let sharpened = compile_files(
        &[SourceFile::new(
            "unchecked_calls.qcore".to_owned(),
            UNCHECKED_CALLS,
        )],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: Some(BTreeMap::from([(
                "examples/unsafe-linkage/unchecked_demo.qcorelib".to_owned(),
                MANIFEST.to_owned(),
            )])),
            native_library_headers: None,
        },
    )
    .expect("the published manifest loads");
    assert_eq!(sharpened.declared_assumptions().len(), 5);
}
