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
//! Five families were covered when this file was written — standard IO, pipes,
//! branching, declared extern calls and unsafe linkage. Card
//! `EXAMPLES-SUITE-WIRE` (wave 26) added section 6, which brings **every**
//! published family under the same page-level standard and publishes one new
//! family, `examples/loops`. Read that section's own note for what it adds and,
//! as importantly, what it deliberately does not build a second time.
//!
//! For each example in the first five families this file establishes:
//!
//! 1. it **checks**, and takes the route its README says it takes;
//! 2. its emitted C compiles under `-Wall -Wextra -Werror -pedantic` and runs
//!    clean under ASan/UBSan (and LSan on Linux);
//! 3. with diagnostic logging omitted, its native source-I/O streams equal the
//!    reference projection; explicit diagnostic transcripts are checked below;
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

#[path = "../../tests/support/tracked_files.rs"]
mod tracked_files;

use qcore_compiler::{
    CompileOptions, CompiledModule, FileDiagnostic, SourceExecutionProfile, SourceFile,
    compile_files, format_diagnostic,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

// --- The published sources. ---

const CONTRACTS: &str = include_str!("../../examples/extern/contracts/buffer_api.qv");
const BUFFER_API_C: &str = include_str!("../../examples/extern/buffer_api.c");
const UNCHECKED_API_C: &str = include_str!("../../examples/unsafe-linkage/unchecked_api.c");

const RENDERINGS: &str = include_str!("../../examples/stdio/renderings.qr");
const SEVERITIES: &str = include_str!("../../examples/stdio/severities.qr");
const BARRIER: &str = include_str!("../../examples/stdio/barrier.qr");
const PIPELINE: &str = include_str!("../../examples/stdio/pipeline.qr");

const RENDERED_LINES: &str = include_str!("../../examples/pipes/rendered_lines.qr");
const SCALAR_STAGES: &str = include_str!("../../examples/pipes/scalar_stages.qr");
const SCALAR_STAGES_HAND: &str = include_str!("../../examples/pipes/scalar_stages_hand_written.qr");

const CONDITIONAL_RELEASE: &str = include_str!("../../examples/branching/conditional_release.qr");
const CONDITION_TIER: &str = include_str!("../../examples/branching/condition_tier.qr");
const EARLY_RETURN: &str = include_str!("../../examples/branching/early_return.qr");

const BUFFER_ROUND_TRIP: &str = include_str!("../../examples/extern/buffer_round_trip.qr");
const SCALAR_RESULTS: &str = include_str!("../../examples/extern/scalar_results.qr");
const BORROWED_RENDER: &str = include_str!("../../examples/extern/borrowed_render.qr");

const UNCHECKED_CALLS: &str = include_str!("../../examples/unsafe-linkage/unchecked_calls.qr");

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
        "examples/extern/contracts/buffer_api.qv".to_owned(),
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
    compile_named("refusal.qr", source, extern_contracts)
        .map(|module| module.execution_profile)
        .expect_err("this program must be refused")
        .iter()
        .map(format_diagnostic)
        .collect()
}

/// Just the codes, from the same refusal.
fn refusal_codes(source: &str, extern_contracts: Option<BTreeMap<String, String>>) -> Vec<String> {
    compile_named("refusal.qr", source, extern_contracts)
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

// The source stream observation excludes diagnostic records. Obtain the
// independent structured extern trace from the checked extern projection.
fn extern_trace(module: &CompiledModule) -> Vec<String> {
    let checked = module.checked.as_composed().unwrap();
    qcore_core::extern_c_emit::interpret_extern_program(
        &checked.lowered.program.externs,
        &checked.certificate.externs,
    )
    .unwrap()
}

/// The `out:`/`err:` projections of source-authored I/O.
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
///
/// Newline-agnostic (card `WINDOWS-NATIVE-SOCKETS-HARNESS`'s recorded
/// precedent, applied here by `EXAMPLES-SUITE-WIRE`): a checkout may hold these
/// files with CRLF endings, and neither the marker's own line terminator nor a
/// line's `\r` is part of the published block. On a Unix checkout the
/// normalisation is the identity, so nothing this file already pinned moves.
fn block(readme: &str, marker: &str) -> String {
    let readme = readme.replace("\r\n", "\n");
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
        ("stdio/renderings.qr", RENDERINGS),
        ("stdio/severities.qr", SEVERITIES),
        ("stdio/barrier.qr", BARRIER),
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
        // is the claim `renderings.qr` makes in prose.
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
    let barrier = plain("stdio/barrier.qr", BARRIER);
    let checked = barrier.checked.as_standard_io().unwrap();
    assert_eq!(checked.certificate.flushes, 1);
    assert!(checked.certificate.renderings.is_empty());
    assert!(!BARRIER.contains("#import std.fmt;"));

    // And the composed example is composed: neither half is published alone.
    let pipeline = contracted("stdio/pipeline.qr", PIPELINE);
    assert_eq!(
        pipeline.execution_profile,
        SourceExecutionProfile::ComposedStdIoExternV1
    );
    assert!(pipeline.checked.as_extern_calls().is_none());
    assert!(pipeline.checked.as_standard_io().is_none());
    assert!(pipeline.checked.as_composed().is_some());
}

#[test]
fn every_published_example_defines_exactly_one_entry_point() {
    // Card `C-EMIT-MAIN-UNIFY`. Before that card ten backends each wrote their
    // own `int main(` and the driver's mutually exclusive route dispatch was
    // what kept a program down to one; now the assembler writes it, and the
    // crate spells the signature in exactly one place. This is the whole-program
    // half of that claim, measured on the published examples rather than
    // asserted: the assembler guarantees it writes one entry point, not that no
    // fragment carries another.
    for (name, module) in [
        ("stdio/renderings", plain("stdio/renderings.qr", RENDERINGS)),
        ("stdio/severities", plain("stdio/severities.qr", SEVERITIES)),
        ("stdio/barrier", plain("stdio/barrier.qr", BARRIER)),
        ("stdio/pipeline", contracted("stdio/pipeline.qr", PIPELINE)),
        (
            "pipes/rendered_lines",
            plain("pipes/rendered_lines.qr", RENDERED_LINES),
        ),
        (
            "branching/conditional_release",
            plain("branching/conditional_release.qr", CONDITIONAL_RELEASE),
        ),
        (
            "branching/condition_tier",
            plain("branching/condition_tier.qr", CONDITION_TIER),
        ),
        (
            "branching/early_return",
            plain("branching/early_return.qr", EARLY_RETURN),
        ),
        (
            "extern/buffer_round_trip",
            contracted("extern/buffer_round_trip.qr", BUFFER_ROUND_TRIP),
        ),
        (
            "extern/scalar_results",
            contracted("extern/scalar_results.qr", SCALAR_RESULTS),
        ),
        (
            "extern/borrowed_render",
            contracted("extern/borrowed_render.qr", BORROWED_RENDER),
        ),
        (
            "unsafe-linkage/unchecked_calls",
            plain("unsafe-linkage/unchecked_calls.qr", UNCHECKED_CALLS),
        ),
    ] {
        let c = module
            .c_source
            .as_deref()
            .unwrap_or_else(|| panic!("examples/{name} emits no C"));
        assert_eq!(
            qcore_core::c_program::entry_definitions(c).len(),
            1,
            "examples/{name} does not define exactly one entry point"
        );
        // Non-vacuous, and the signature is the injected one rather than some
        // other spelling the scan happens to match.
        assert!(
            c.contains(qcore_core::c_program::ENTRY_SIGNATURE),
            "examples/{name} does not carry the injected entry signature"
        );
    }
}

#[test]
fn every_stdio_example_prints_through_the_single_emission_route() {
    for (name, module) in [
        ("renderings", plain("stdio/renderings.qr", RENDERINGS)),
        ("severities", plain("stdio/severities.qr", SEVERITIES)),
        ("pipeline", contracted("stdio/pipeline.qr", PIPELINE)),
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
    // The documented examples are also run verbatim by the transcript tests.
    // This comparison selects their source-I/O projection without diagnostics.
    for (label, module, extra) in [
        (
            "renderings",
            plain("stdio/renderings.qr", RENDERINGS),
            Vec::new(),
        ),
        (
            "severities",
            plain("stdio/severities.qr", SEVERITIES),
            Vec::new(),
        ),
        ("barrier", plain("stdio/barrier.qr", BARRIER), Vec::new()),
        (
            "pipeline",
            contracted(
                "stdio/pipeline.qr",
                &PIPELINE.replace("log_level trace;", ""),
            ),
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
        plain("stdio/renderings.qr", RENDERINGS)
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
        plain("stdio/severities.qr", SEVERITIES)
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
        plain("stdio/barrier.qr", BARRIER)
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
        contracted("stdio/pipeline.qr", PIPELINE)
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
    let without_io = RENDERINGS.replace("#import std.io;\n", "");
    assert_eq!(refusal_codes(&without_io, None), ["Q015"]);
    let without_fmt = RENDERINGS.replace("#import std.fmt;\n", "");
    assert_eq!(refusal_codes(&without_fmt, None), ["Q015"]);
    let without_logger = SEVERITIES.replace("#import std.logger;\n", "");
    assert_eq!(refusal_codes(&without_logger, None), ["Q015"]);
    // `flush` is the logger's, not `std.fmt`'s: dropping the barrier's only
    // import refuses, and the barrier has no `std.fmt` import to drop.
    let bare_barrier = BARRIER.replace("#import std.logger;\n", "");
    assert_eq!(refusal_codes(&bare_barrier, None), ["Q015"]);
    // A misspelled module is unknown rather than ignored.
    let misspelled = RENDERINGS.replace("#import std.io;", "#import std.stdio;");
    assert_eq!(refusal_codes(&misspelled, None), ["Q014"]);
    // `flush` takes no operand.
    let flush_operand = SEVERITIES.replace("flush;", "flush 1;");
    assert_eq!(refusal_codes(&flush_operand, None), ["Q030"]);
}

/// The composed route's declared outcomes reach the emitted program as real
/// comparisons — the page's "the emitted C does not believe it" claim.
#[test]
fn the_composed_example_does_not_believe_its_declared_outcomes() {
    let module = contracted("stdio/pipeline.qr", PIPELINE);
    let c = module.c_source.as_deref().unwrap();
    assert!(c.contains("if (v1 != 56)"), "emitted C: {c}");
    assert!(c.contains("QCORE_DIAG_EXTERN_OUTCOME"));
    assert!(c.contains("return 70;"));
    // The trace line and the rendering name the same number, which is the
    // `-> N` field's whole purpose.
    let observation = module.extern_observation.as_deref().unwrap();
    assert!(extern_trace(&module).contains(&"value demo_sum v1 b:r1 d:8 -> 56".to_owned()));
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
    let piped = contracted("pipes/scalar_stages.qr", SCALAR_STAGES);
    let hand = contracted("pipes/scalar_stages_hand_written.qr", SCALAR_STAGES_HAND);
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
    let lines = plain("pipes/rendered_lines.qr", RENDERED_LINES);
    let (expected_out, expected_err) = projections(&lines);
    let (out, err) = run("rendered-lines", lines.c_source.as_deref().unwrap(), &[]);
    assert_eq!(out, expected_out);
    assert_eq!(err, expected_err);

    // The explicit trace adapter renders the structured extern observations
    // on stdout; source-level IO remains a separate stream projection.
    let stages = contracted("pipes/scalar_stages.qr", SCALAR_STAGES);
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
    let lines = plain("pipes/rendered_lines.qr", RENDERED_LINES);
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

    let stages = contracted("pipes/scalar_stages.qr", SCALAR_STAGES);
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
/// distinction the pipes page draws against `../stdio/pipeline.qr`.
#[test]
fn an_undeclared_outcome_prints_no_arrow() {
    let stages = contracted("pipes/scalar_stages.qr", SCALAR_STAGES);
    let observation = extern_trace(&stages);
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
    let declared = contracted("extern/scalar_results.qr", SCALAR_RESULTS);
    assert!(
        extern_trace(&declared)
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
        let module = plain(&format!("branching/{label}.qr"), source);
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
    compile_named("grouped.qr", &grouped, None).expect("the parenthesized spelling checks");
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
        compile_named("table.qr", source, None)
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
    let module = plain("branching/early_return.qr", EARLY_RETURN);
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
            "extern/buffer_round_trip.qr",
            BUFFER_ROUND_TRIP,
            SourceExecutionProfile::ComposedStdIoExternV1,
        ),
        (
            "extern/scalar_results.qr",
            SCALAR_RESULTS,
            SourceExecutionProfile::ComposedStdIoExternV1,
        ),
        (
            "extern/borrowed_render.qr",
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
    // Verbatim examples, including their explicit traces, are checked against
    // the published transcripts below. Here compare their source-I/O streams.
    for (label, source, single_stream) in [
        ("buffer-round-trip", BUFFER_ROUND_TRIP, true),
        ("scalar-results", SCALAR_RESULTS, true),
        ("borrowed-render", BORROWED_RENDER, false),
    ] {
        let module = contracted(label, &source.replace("log_level trace;", ""));
        let (out, err) = run(
            label,
            module.c_source.as_deref().unwrap(),
            &[("buffer_api.c", BUFFER_API_C)],
        );
        if single_stream {
            assert!(out.is_empty(), "{label}: source has no stdout writes");
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
    let module = contracted("extern/borrowed_render.qr", BORROWED_RENDER);
    let observation = module.extern_observation.as_deref().unwrap();
    assert!(extern_trace(&module).contains(&"acquire demo_alloc r1 ok".to_owned()));
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

    let module = plain("unsafe-linkage/unchecked_calls.qr", UNCHECKED_CALLS);
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
    let module = plain("unsafe-linkage/unchecked_calls.qr", UNCHECKED_CALLS);
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
    const MANIFEST: &str = include_str!("../../examples/unsafe-linkage/unchecked_demo.qrlib");
    let broken = MANIFEST.replace("data_model lp64;", "data_model ilp32;");
    let diagnostics = compile_files(
        &[SourceFile::new(
            "unchecked_calls.qr".to_owned(),
            UNCHECKED_CALLS,
        )],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: Some(BTreeMap::from([(
                "examples/unsafe-linkage/unchecked_demo.qrlib".to_owned(),
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
            "unchecked_calls.qr".to_owned(),
            UNCHECKED_CALLS,
        )],
        &CompileOptions {
            entry: None,
            extern_contracts: None,
            native_libraries: Some(BTreeMap::from([(
                "examples/unsafe-linkage/unchecked_demo.qrlib".to_owned(),
                MANIFEST.to_owned(),
            )])),
            native_library_headers: None,
        },
    )
    .expect("the published manifest loads");
    assert_eq!(sharpened.declared_assumptions().len(), 5);
}

// ---------------------------------------------------------------------------
// 6. The newer families — strings, sockets, http, events, loops
//    (card `EXAMPLES-SUITE-WIRE`, wave 26)
// ---------------------------------------------------------------------------
//
// Three follow-ons said the same thing: `examples/strings`, `examples/http` and
// `examples/sockets` are each verified by their own card's test file rather than
// by this one, whose list is explicit rather than a directory scan. That is not
// a gap in what those files check — each already compiles its programs through
// the real pipeline, builds the emitted C under `-Werror`-class flags, runs it
// under the sanitizers and byte-compares both streams. The gap is what this
// suite adds *beyond* a per-card test:
//
//   1. **A directory scan.** Every `.qr` program in a family directory is
//      indexed on that family's page, and every `.qr` a page names exists.
//      An explicit list cannot notice a file nobody added it to — which is
//      exactly how `examples/sockets/` came to be published without a row in
//      `examples/README.md` until the HTTP card added one.
//   2. **Marked transcript extraction, in one place.** Every published
//      transcript is read out of its page under the same
//      `<!-- transcript: NAME stream -->` convention and compared with the
//      pipeline's own reference execution.
//   3. **A route assertion per program**, so a page that says which route its
//      program takes is checked rather than believed.
//
// **The sanitized native runs are not duplicated.** Where a card's test file
// already builds and runs a program, this suite compares the *page* with the
// *model* and stops there; the card's file compares the model with the machine.
// Together they close the triangle — page = model = native run — and each build
// happens once. `examples/loops/` is the exception, because it is new here and
// no other file owns it: this suite performs its native run, exactly as it does
// for the five wave-21 families. `examples/threads` remains
// `thread_examples.rs`'s in every respect, as it was in wave 21.

const STRINGS_README: &str = include_str!("../../examples/strings/README.md");
const SOCKETS_README: &str = include_str!("../../examples/sockets/README.md");
const HTTP_README: &str = include_str!("../../examples/http/README.md");
const EVENTS_README: &str = include_str!("../../examples/events/README.md");
const LOOPS_README: &str = include_str!("../../examples/loops/README.md");
const RECORDS_README: &str = include_str!("../../examples/records/README.md");
const INDEX_README: &str = include_str!("../../examples/README.md");

const STATUS_LINE: &str = include_str!("../../examples/strings/status_line.qr");
const COMPUTED_LENGTH: &str = include_str!("../../examples/strings/computed_length.qr");
const TCP_ECHO: &str = include_str!("../../examples/sockets/tcp_echo.qr");
const UDP_ECHO: &str = include_str!("../../examples/sockets/udp_echo.qr");
const EXCHANGE: &str = include_str!("../../examples/http/exchange.qr");
const RAW_REQUEST: &str = include_str!("../../examples/http/raw_request.qr");
const SELF_PIPE: &str = include_str!("../../examples/events/self_pipe.qr");
const WAKE_PAYLOAD: &str = include_str!("../../examples/events/wake_payload.qr");
const WRITE_INTEREST: &str = include_str!("../../examples/events/write_interest.qr");
const SOCKET_SERVER: &str = include_str!("../../examples/events/socket_server.qr");
const BUFFER_COPY: &str = include_str!("../../examples/loops/buffer_copy.qr");
const RECORD_POINT: &str = include_str!("../../examples/records/point.qr");
const PIXEL_WRITES: &str = include_str!("../../examples/records/pixel_writes.qr");
const OWNED_FIELD: &str = include_str!("../../examples/records/owned_field.qr");
// Card `LOOPS-ON-COMPOSED-ROUTES` (wave 39): a `for` beside a lend and a `for`
// in a composed string/buffer module. Their sanitized native runs are
// `loops_on_composed_routes.rs`'s; this suite holds their pages to the model.
const LEND_FILL: &str = include_str!("../../examples/loops/lend_fill.qr");
const LOOPED_FILL: &str = include_str!("../../examples/strings/looped_fill.qr");
const CYCLE12: &str = include_str!("../../examples/cycle12.qr");

/// `examples/`, on disk. The scan below is the point: a constant list cannot
/// notice a program nobody added to it.
fn examples_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate directory has a parent")
        .join("examples")
}

/// `examples/`'s tracked files, as `/`-separated paths relative to it. The
/// scan reads the tracked tree only (card W45-TEST-HYGIENE): an untracked or
/// ignored directory under `examples/` is not a published family.
fn tracked_example_paths() -> Vec<String> {
    let root = examples_root();
    tracked_files::tracked_files(&root)
        .iter()
        .map(|path| {
            path.strip_prefix(&root)
                .expect("under examples/")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

/// The family directories, sorted — every directory directly under `examples/`
/// that holds a tracked file.
fn family_directories() -> Vec<String> {
    let mut families: Vec<String> = tracked_example_paths()
        .iter()
        .filter_map(|path| path.split_once('/').map(|(family, _)| family.to_owned()))
        .collect();
    families.sort();
    families.dedup();
    families
}

/// The `.qr` programs one family directory publishes, sorted. Not recursive:
/// `examples/extern/contracts/` holds a declared contract module, which is a
/// support file rather than a published program, and its page says so.
fn published_programs(family: &str) -> Vec<String> {
    let prefix = format!("{family}/");
    let mut programs: Vec<String> = tracked_example_paths()
        .iter()
        .filter_map(|path| path.strip_prefix(&prefix))
        .filter(|name| !name.contains('/') && name.ends_with(".qr"))
        .map(str::to_owned)
        .collect();
    programs.sort();
    programs
}

/// Every markdown link target in `page` that ends with `suffix`.
fn link_targets(page: &str, suffix: &str) -> Vec<String> {
    let mut targets = Vec::new();
    let mut rest = page;
    while let Some((_, tail)) = rest.split_once("](") {
        match tail.split_once(')') {
            Some((target, after)) => {
                if target.ends_with(suffix) && !target.contains(char::is_whitespace) {
                    targets.push(target.to_owned());
                }
                rest = after;
            }
            None => break,
        }
    }
    targets
}

/// Every published program is indexed on its own family page, and every
/// `.qr` a family page links exists.
///
/// This is the check an explicit list cannot make. `examples/sockets/` was
/// published with no row in `examples/README.md` at all until the HTTP card
/// noticed and added two; nothing failed in the meantime.
#[test]
fn every_published_program_is_indexed_and_every_indexed_program_exists() {
    let families = family_directories();
    assert!(
        families.len() >= 11,
        "the scan found no families: {families:?}"
    );
    for family in &families {
        let page_path = examples_root().join(family).join("README.md");
        let page = std::fs::read_to_string(&page_path)
            .unwrap_or_else(|_| panic!("examples/{family}/ publishes no README.md"));
        let programs = published_programs(family);
        assert!(
            !programs.is_empty(),
            "examples/{family}/ publishes no program"
        );
        for program in &programs {
            assert!(
                link_targets(&page, ".qr")
                    .iter()
                    .any(|target| target == program || target.ends_with(&format!("/{program}"))),
                "examples/{family}/{program} is published but its page does not link it"
            );
        }
        for target in link_targets(&page, ".qr") {
            let resolved = examples_root().join(family).join(&target);
            assert!(
                resolved.exists(),
                "examples/{family}/README.md links `{target}`, which does not exist"
            );
        }
    }
}

/// And the top-level index names every family directory and only real ones.
#[test]
fn the_examples_index_names_every_family_and_only_real_ones() {
    for family in family_directories() {
        assert!(
            INDEX_README.contains(&format!("]({family}/)")),
            "examples/README.md does not index the `{family}/` family"
        );
    }
    for target in link_targets(INDEX_README, "/") {
        assert!(
            examples_root().join(target.trim_end_matches('/')).is_dir(),
            "examples/README.md links `{target}`, which is not a directory"
        );
    }
    // Non-vacuous in the other direction: the index links files too, and each
    // of those exists as well.
    for target in link_targets(INDEX_README, ".qr") {
        assert!(
            examples_root().join(&target).exists(),
            "examples/README.md links `{target}`, which does not exist"
        );
    }
}

/// Each newer family's programs take the route its page names.
#[test]
fn the_newer_families_take_the_routes_their_pages_say_they_take() {
    for (name, source, profile) in [
        (
            "strings/status_line.qr",
            STATUS_LINE,
            SourceExecutionProfile::StandardStringV1,
        ),
        (
            "strings/computed_length.qr",
            COMPUTED_LENGTH,
            SourceExecutionProfile::StandardStringV1,
        ),
        (
            "sockets/tcp_echo.qr",
            TCP_ECHO,
            SourceExecutionProfile::StandardSocketV1,
        ),
        (
            "sockets/udp_echo.qr",
            UDP_ECHO,
            SourceExecutionProfile::StandardSocketV1,
        ),
        (
            "http/exchange.qr",
            EXCHANGE,
            SourceExecutionProfile::StandardHttpV1,
        ),
        // The page's own contrast: the hand-written twin is **not** on the HTTP
        // route, because nothing in it declares a message.
        (
            "http/raw_request.qr",
            RAW_REQUEST,
            SourceExecutionProfile::StandardSocketV1,
        ),
        (
            "events/self_pipe.qr",
            SELF_PIPE,
            SourceExecutionProfile::EventV1,
        ),
        (
            "events/wake_payload.qr",
            WAKE_PAYLOAD,
            SourceExecutionProfile::EventV1,
        ),
        (
            "events/write_interest.qr",
            WRITE_INTEREST,
            SourceExecutionProfile::EventV1,
        ),
        // And the composed one is the socket route's, which models both halves —
        // the events page says exactly that.
        (
            "events/socket_server.qr",
            SOCKET_SERVER,
            SourceExecutionProfile::StandardSocketV1,
        ),
        (
            "loops/buffer_copy.qr",
            BUFFER_COPY,
            SourceExecutionProfile::StandardBufferV1,
        ),
        // A `for` beside a lend stays on the buffer route; a `for` in a module
        // that spells the bridge is the composed string route's.
        (
            "loops/lend_fill.qr",
            LEND_FILL,
            SourceExecutionProfile::StandardBufferV1,
        ),
        (
            "strings/looped_fill.qr",
            LOOPED_FILL,
            SourceExecutionProfile::StandardStringV1,
        ),
        // A record is admitted on the buffer route and on no other, so both
        // records programs are that route's — including `point.qr`, which
        // holds no buffer at all.
        (
            "records/point.qr",
            RECORD_POINT,
            SourceExecutionProfile::StandardBufferV1,
        ),
        (
            "records/pixel_writes.qr",
            PIXEL_WRITES,
            SourceExecutionProfile::StandardBufferV1,
        ),
        // Card `AGGREGATE-OWNED-FIELD`: a record that owns is a program that
        // acquires, so route selection did not move.
        (
            "records/owned_field.qr",
            OWNED_FIELD,
            SourceExecutionProfile::StandardBufferV1,
        ),
        (
            "cycle12.qr",
            CYCLE12,
            SourceExecutionProfile::SequentialCleanupV1,
        ),
    ] {
        let module = plain(name, source);
        assert_eq!(module.execution_profile, profile, "{name}");
    }
}

/// The one published program that deliberately emits nothing, and the index
/// sentence that says so.
#[test]
fn the_check_only_example_is_the_only_one_that_emits_no_c() {
    let write_interest = plain("events/write_interest.qr", WRITE_INTEREST);
    assert!(
        write_interest.c_source.is_none(),
        "the event route's write arm is checked and not executed"
    );
    assert!(write_interest.extern_observation.is_none());
    assert!(INDEX_README.contains("events/write_interest.qr"));

    // Every other program in every family this file names does emit C. The
    // threads family is `thread_examples.rs`'s and is not read here.
    for (name, source) in [
        ("strings/status_line.qr", STATUS_LINE),
        ("strings/computed_length.qr", COMPUTED_LENGTH),
        ("sockets/tcp_echo.qr", TCP_ECHO),
        ("sockets/udp_echo.qr", UDP_ECHO),
        ("http/exchange.qr", EXCHANGE),
        ("http/raw_request.qr", RAW_REQUEST),
        ("events/self_pipe.qr", SELF_PIPE),
        ("events/wake_payload.qr", WAKE_PAYLOAD),
        ("events/socket_server.qr", SOCKET_SERVER),
        ("loops/buffer_copy.qr", BUFFER_COPY),
        ("loops/lend_fill.qr", LEND_FILL),
        ("strings/looped_fill.qr", LOOPED_FILL),
        ("records/point.qr", RECORD_POINT),
        ("records/pixel_writes.qr", PIXEL_WRITES),
        ("records/owned_field.qr", OWNED_FIELD),
        ("cycle12.qr", CYCLE12),
    ] {
        assert!(
            plain(name, source).c_source.is_some(),
            "examples/{name} emits no C"
        );
    }
}

/// Every transcript the newer pages publish is the projection its program's
/// reference execution computes, read out of the page under the shared marker
/// convention.
///
/// No program is built here. The card test files listed on each page perform
/// the sanitized native run and compare it with this same reference execution;
/// this test compares the *page* with it. Page = model = machine, with each
/// build performed once.
#[test]
fn the_newer_pages_publish_the_transcripts_their_programs_produce() {
    for (name, source, readme, out_marker, err_marker) in [
        (
            "strings/status_line.qr",
            STATUS_LINE,
            STRINGS_README,
            "transcript: status_line stdout",
            None,
        ),
        (
            "sockets/tcp_echo.qr",
            TCP_ECHO,
            SOCKETS_README,
            "transcript: tcp_echo",
            None,
        ),
        (
            "sockets/udp_echo.qr",
            UDP_ECHO,
            SOCKETS_README,
            "transcript: udp_echo",
            None,
        ),
        (
            "http/exchange.qr",
            EXCHANGE,
            HTTP_README,
            "transcript: exchange",
            None,
        ),
        (
            "http/raw_request.qr",
            RAW_REQUEST,
            HTTP_README,
            "transcript: raw_request",
            None,
        ),
        (
            "events/self_pipe.qr",
            SELF_PIPE,
            EVENTS_README,
            "transcript: self_pipe stdout",
            None,
        ),
        (
            "events/wake_payload.qr",
            WAKE_PAYLOAD,
            EVENTS_README,
            "transcript: wake_payload stdout",
            Some("transcript: wake_payload stderr"),
        ),
        (
            "events/socket_server.qr",
            SOCKET_SERVER,
            EVENTS_README,
            "transcript: socket_server stdout",
            None,
        ),
        (
            "loops/buffer_copy.qr",
            BUFFER_COPY,
            LOOPS_README,
            "transcript: buffer_copy stdout",
            Some("transcript: buffer_copy stderr"),
        ),
        (
            "loops/lend_fill.qr",
            LEND_FILL,
            LOOPS_README,
            "transcript: lend_fill stdout",
            None,
        ),
        (
            "strings/looped_fill.qr",
            LOOPED_FILL,
            STRINGS_README,
            "transcript: looped_fill stdout",
            None,
        ),
        (
            "records/point.qr",
            RECORD_POINT,
            RECORDS_README,
            "transcript: point stdout",
            Some("transcript: point stderr"),
        ),
        (
            "records/pixel_writes.qr",
            PIXEL_WRITES,
            RECORDS_README,
            "transcript: pixel_writes stdout",
            None,
        ),
        (
            "records/owned_field.qr",
            OWNED_FIELD,
            RECORDS_README,
            "transcript: owned_field stdout",
            None,
        ),
    ] {
        let module = plain(name, source);
        let (out, err) = projections(&module);
        assert!(!out.is_empty(), "{name}: nothing to compare");
        assert_eq!(out, documented(readme, out_marker), "{name}: stdout");
        match err_marker {
            Some(marker) => assert_eq!(err, documented(readme, marker), "{name}: stderr"),
            None => assert!(
                err.is_empty(),
                "{name}: the page publishes no stderr transcript, so there must be none"
            ),
        }
    }

    // `computed_length.qr` is the one page that shows its transcript with the
    // carriage returns removed, and says so in the sentence above the block: the
    // program writes a CRLF-framed head, and a `\r` is not something a reader
    // can see. Compared with the `\r`s stripped from the *projection* rather
    // than with the page's claim weakened — and the presence of those `\r`s is
    // asserted right here so the strip cannot hide their loss.
    let computed = plain("strings/computed_length.qr", COMPUTED_LENGTH);
    let (out, err) = projections(&computed);
    assert!(err.is_empty());
    assert!(
        out.iter().filter(|line| line.ends_with('\r')).count() == 4,
        "the head this program writes is CRLF-framed: {out:?}"
    );
    let shown: Vec<String> = out
        .iter()
        .map(|line| line.trim_end_matches('\r').to_owned())
        .collect();
    assert_eq!(
        shown,
        documented(STRINGS_README, "transcript: computed_length stdout")
    );
}

/// The extraction above would notice a truncated or misaddressed block.
#[test]
fn the_newer_transcript_checks_would_notice_a_changed_line() {
    for (readme, marker, first) in [
        (
            STRINGS_README,
            "transcript: status_line stdout",
            "HTTP/1.1 200 OK",
        ),
        (
            STRINGS_README,
            "transcript: computed_length stdout",
            "HTTP/1.1 200 OK",
        ),
        (SOCKETS_README, "transcript: tcp_echo", "101"),
        (SOCKETS_README, "transcript: udp_echo", "101"),
        (HTTP_README, "transcript: exchange", "200"),
        (HTTP_README, "transcript: raw_request", "204"),
        (
            EVENTS_README,
            "transcript: self_pipe stdout",
            "QCORE_EVENT_OPEN",
        ),
        (
            EVENTS_README,
            "transcript: wake_payload stdout",
            "QCORE_EVENT_OPEN",
        ),
        (
            EVENTS_README,
            "transcript: wake_payload stderr",
            "[INFO] 72",
        ),
        (EVENTS_README, "transcript: socket_server stdout", "81"),
        (LOOPS_README, "transcript: buffer_copy stdout", "0"),
        (LOOPS_README, "transcript: buffer_copy stderr", "[INFO] 4"),
        (LOOPS_README, "transcript: lend_fill stdout", "81"),
        (STRINGS_README, "transcript: looped_fill stdout", "aaaa"),
        (RECORDS_README, "transcript: point stdout", "65"),
        (RECORDS_README, "transcript: point stderr", "[INFO] 9"),
        (RECORDS_README, "transcript: pixel_writes stdout", "81"),
    ] {
        let lines = documented(readme, marker);
        assert!(!lines.is_empty(), "`{marker}` extracted nothing");
        assert_eq!(lines[0], first, "`{marker}` extracted the wrong block");
    }
}

// --- examples/loops — the native half, which this suite owns. ---

/// The loop example builds and runs under the sanitizers, and both its streams
/// equal the reference execution's projection.
///
/// A **positive** leak control: the program allocates two buffers and releases
/// both, and one of the four writes into the second is performed only by the
/// loop body. A loop that ran past an end, or a release that went missing,
/// would be an ASan or LSan finding rather than a passing test.
#[test]
fn the_loop_example_runs_clean_and_matches_its_reference_execution() {
    let module = plain("loops/buffer_copy.qr", BUFFER_COPY);
    let (expected_out, expected_err) = projections(&module);
    let (out, err) = run("buffer-copy", module.c_source.as_deref().unwrap(), &[]);
    assert_eq!(out, expected_out, "buffer_copy: stdout");
    assert_eq!(err, expected_err, "buffer_copy: stderr");
    assert_eq!(
        out,
        documented(LOOPS_README, "transcript: buffer_copy stdout")
    );
    assert_eq!(
        err,
        documented(LOOPS_README, "transcript: buffer_copy stderr")
    );
}

/// The page's claim about the emitted C, read out of the page: a real `for`
/// loop whose body is written once rather than once per iteration. That is what
/// makes the run above a check of the unrolling rather than a restatement of it.
#[test]
fn the_loop_page_publishes_the_c_the_backend_really_writes() {
    let module = plain("loops/buffer_copy.qr", BUFFER_COPY);
    let c = module.c_source.as_deref().unwrap();
    let published = block(LOOPS_README, "emitted: copy-loop");
    let published = published.trim_end();
    assert!(
        c.contains(published),
        "the page publishes a line the backend does not write: {published}"
    );
    // Non-vacuous, and the point: the copy's store is emitted **once**, so the
    // four iterations are the C's own rather than four copies of a statement.
    assert_eq!(
        c.matches("qcore_bytes_store(b_dst,").count(),
        1,
        "the loop body is emitted once:\n{c}"
    );
    // And the reference execution unrolled it: seven stdout lines from two
    // loops plus four reads, none of which the C writes literally.
    assert_eq!(projections(&module).0.len(), 7);
}

/// The refused program the loops page publishes, compiled, and the diagnostic it
/// quotes, compared with the real one — span caret included.
#[test]
fn the_documented_zero_bound_refusal_is_the_refusal_the_compiler_gives() {
    let program = block(LOOPS_README, "refusal: Q098");
    assert!(program.contains("for i in 0..0 {"), "{program}");
    assert_eq!(refusal_codes(&program, None), ["Q098"]);
    assert_eq!(
        refusal(&program, None),
        vec![
            block(LOOPS_README, "diagnostic: Q098")
                .trim_end()
                .to_owned()
        ],
        "the page quotes a diagnostic the compiler does not give"
    );
    // Non-vacuous: the same program with a bound of one is admitted, so what the
    // page pins is the zero rather than the shape.
    let one = program.replace("0..0", "0..1");
    compile_named("one.qr", &one, None).expect("a bound of one is a loop");
}

/// The three refusals the page names beside it, and the one it says is not this
/// route's.
#[test]
fn the_loop_refusals_the_page_names_beside_it_hold() {
    let body = |statement: &str| {
        format!(
            "#import std.buffer;\n\
             \n\
             fn main() -> unit effects {{ alloc, mut }} {{\n    \
                 let b = bytes(4);\n    \
                 for i in 0..4 {{\n        {statement}\n    }}\n    \
                 drop b;\n    \
                 return;\n\
             }}\n"
        )
    };
    assert_eq!(refusal_codes(&body("break;"), None), ["Q097"]);
    assert_eq!(refusal_codes(&body("return;"), None), ["Q097"]);
    // An acquisition the body does not release: this route's own `Q099`, and
    // then the unedited ownership checker's `Q054` behind it.
    assert_eq!(
        refusal_codes(&body("let t = bytes(2);"), None),
        ["Q099", "Q054"]
    );
    // And the index is body-local, so naming it afterwards is an unknown name
    // rather than anything this route decides — which is what the page says.
    let after = "#import std.buffer;\n#import std.fmt;\n#import std.io;\n\nfn main() -> unit effects { io, alloc, mut } {\n    let b = bytes(4);\n    for i in 0..4 {\n        b[i] = 1;\n    }\n    print dec i;\n    drop b;\n    return;\n}\n";
    assert_eq!(refusal_codes(after, None), ["Q013"]);
    // Every code the page's table names is one the page also spells.
    for code in ["Q097", "Q098", "Q099", "Q054", "Q013"] {
        assert!(LOOPS_README.contains(code), "the page does not name {code}");
    }
}

// --- examples/records — the native half, which this suite owns
//     (card `EXAMPLES-AGGREGATES`). ---

/// Both record examples build and run under the sanitizers, and both streams
/// of each equal the reference execution's projection and the page's published
/// transcripts.
///
/// The two programs are each other's leak control, and the page says so.
/// `point.qr` allocates nothing at all, so its run is a **negative**
/// control: LSan reporting a leak would mean the aggregate had acquired
/// something the model says it cannot. `pixel_writes.qr` allocates one real
/// buffer and releases it, so its run is a **positive** control: a lost
/// release would be a finding, and the two records beside the buffer change
/// nothing about that.
#[test]
fn the_record_examples_run_clean_and_match_their_reference_execution() {
    for (label, name, source, out_marker, err_marker) in [
        (
            "record-point",
            "records/point.qr",
            RECORD_POINT,
            "transcript: point stdout",
            Some("transcript: point stderr"),
        ),
        (
            "pixel-writes",
            "records/pixel_writes.qr",
            PIXEL_WRITES,
            "transcript: pixel_writes stdout",
            None,
        ),
        // Card `AGGREGATE-OWNED-FIELD`: the record that owns is a second
        // positive leak control — one buffer released through the
        // whole-record drop, the other through the binding it moved out to.
        (
            "owned-field",
            "records/owned_field.qr",
            OWNED_FIELD,
            "transcript: owned_field stdout",
            None,
        ),
    ] {
        let module = plain(name, source);
        let (expected_out, expected_err) = projections(&module);
        let (out, err) = run(label, module.c_source.as_deref().unwrap(), &[]);
        assert_eq!(out, expected_out, "{name}: stdout");
        assert_eq!(err, expected_err, "{name}: stderr");
        assert_eq!(out, documented(RECORDS_README, out_marker), "{name}");
        match err_marker {
            Some(marker) => assert_eq!(err, documented(RECORDS_README, marker), "{name}"),
            None => assert!(err.is_empty(), "{name}: the page publishes no stderr"),
        }
    }
}

/// The page's claims about the emitted C, read out of the page: one struct per
/// declared type with one distinctly named member per field, constructions
/// initialized in declared order however the source wrote them, and no buffer
/// prelude in a program that acquires nothing.
#[test]
fn the_records_page_publishes_the_struct_the_backend_really_writes() {
    let point = plain("records/point.qr", RECORD_POINT);
    let c = point.c_source.as_deref().unwrap();
    let published = block(RECORDS_README, "emitted: point-struct");
    let published = published.trim_end();
    assert!(
        c.contains(published),
        "the page publishes a struct the backend does not write: {published}"
    );
    // Declared order is the emission order: the second value is written
    // `{ weight: 9, y: 2, x: 1 }` and initialized `x, y, weight`.
    assert!(RECORD_POINT.contains("Point { weight: 9, y: 2, x: 1 }"));
    assert!(
        c.contains(".f_x = INT32_C(1), .f_y = INT32_C(2), .f_weight = INT32_C(9)"),
        "constructions must be emitted in declared order:\n{c}"
    );
    // The page's "no buffer prelude" sentence: a record is a plain value, so a
    // program holding none carries no allocator at all.
    assert!(
        !c.contains("qcore_bytes_new"),
        "point.qr acquires nothing and must carry no buffer prelude"
    );
    // And the neighbouring program does hold a buffer, so the contrast is a
    // fact about the programs rather than about the backend.
    let pixel = plain("records/pixel_writes.qr", PIXEL_WRITES);
    assert!(
        pixel
            .c_source
            .as_deref()
            .unwrap()
            .contains("qcore_bytes_new"),
        "pixel_writes.qr acquires a real buffer"
    );
    // Card `AGGREGATE-OWNED-FIELD`: the page's struct for the record that
    // owns — pointer members — is the one the backend writes, the initializer
    // is the move, and the whole-record drop frees the one member still held.
    let owned = plain("records/owned_field.qr", OWNED_FIELD);
    let owned_c = owned.c_source.as_deref().unwrap();
    let published = block(RECORDS_README, "emitted: frame-struct");
    assert!(
        owned_c.contains(published.trim_end()),
        "the page publishes a struct the backend does not write: {published}"
    );
    assert!(owned_c.contains(".f_head = b_h, .f_body = b_b"));
    assert!(owned_c.contains("unsigned char *b_taken = r_f.f_body;"));
    assert!(owned_c.contains("qcore_bytes_drop(r_f.f_head);"));
    assert!(!owned_c.contains("qcore_bytes_drop(r_f.f_body);"));
}

/// The refused program the records page publishes, compiled, and the
/// diagnostic it quotes, compared with the real one — span caret included.
#[test]
fn the_documented_incomplete_construction_is_the_refusal_the_compiler_gives() {
    let program = block(RECORDS_README, "refusal: Q051");
    assert!(program.contains("let p = Point { x: 1 };"), "{program}");
    assert_eq!(refusal_codes(&program, None), ["Q051"]);
    assert_eq!(
        refusal(&program, None),
        vec![
            block(RECORDS_README, "diagnostic: Q051")
                .trim_end()
                .to_owned()
        ],
        "the page quotes a diagnostic the compiler does not give"
    );
    // Non-vacuous: the same construction with the field supplied is admitted,
    // so what the page pins is the totality rule rather than the shape.
    let complete = program.replace("Point { x: 1 }", "Point { x: 1, y: 2 }");
    compile_named("complete.qr", &complete, None).expect("the complete construction checks");
}

/// The four refusals the page names beside it, each earned by exactly the
/// statement the table writes.
#[test]
fn the_record_refusals_the_page_names_beside_it_hold() {
    const DECLARATIONS: &str = "#import std.fmt;\n\
         #import std.io;\n\
         \n\
         type Point { x: i32, y: i32 }\n\
         \n";
    // A record is a value, not a resource: `drop` on one refuses with the
    // route's own custody message even where the effect row would admit an
    // allocation.
    let dropped = format!(
        "{DECLARATIONS}fn main() -> unit effects {{ io, alloc, mut }} {{\n    \
             let p = Point {{ x: 1, y: 2 }};\n    \
             drop p;\n    \
             return;\n\
         }}\n"
    );
    assert_eq!(refusal_codes(&dropped, None), ["Q052"]);
    // A field the type does not declare.
    let undeclared_field = format!(
        "{DECLARATIONS}fn main() -> unit effects {{ io }} {{\n    \
             let p = Point {{ x: 1, y: 2 }};\n    \
             let z = p.z;\n    \
             print dec 7;\n    \
             return;\n\
         }}\n"
    );
    assert_eq!(refusal_codes(&undeclared_field, None), ["Q052"]);
    // A construction of a type nothing declares is name resolution's, before
    // this route's reader ever runs.
    let undeclared_type = format!(
        "{DECLARATIONS}fn main() -> unit effects {{ io }} {{\n    \
             let p = Square {{ x: 1, y: 2 }};\n    \
             print dec 7;\n    \
             return;\n\
         }}\n"
    );
    assert_eq!(refusal_codes(&undeclared_type, None), ["Q013"]);
    // And the published program without its release: the records beside the
    // buffer launder nothing, so the unedited ownership checker refuses.
    let leaked = PIXEL_WRITES.replace("    drop pixels;\n", "");
    assert_eq!(refusal_codes(&leaked, None), ["Q054"]);
    // Every code the page's table names is one the page also spells.
    for code in ["Q051", "Q052", "Q013", "Q054"] {
        assert!(
            RECORDS_README.contains(code),
            "the page does not name {code}"
        );
    }
}
