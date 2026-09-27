//! Native Windows pins for examples that explicitly select the hosted system
//! trace adapter. The sanitizer suite in `examples_suite.rs` covers the same
//! pages on Unix; these tests keep the MSVC path from becoming documentation-
//! only evidence.
#![cfg(target_os = "windows")]

use std::collections::BTreeMap;

use qcore_compiler::{CompileOptions, CompiledModule, SourceFile, compile_files};

#[path = "../../tests/support/windows_c.rs"]
mod windows_c;

const CONTRACTS: &str = include_str!("../../examples/extern/contracts/buffer_api.qcore");
const BUFFER_API_C: &str = include_str!("../../examples/extern/buffer_api.c");

fn compile(name: &str, source: &str, with_contracts: bool) -> CompiledModule {
    compile_files(
        &[SourceFile::new(name, source)],
        &CompileOptions {
            entry: None,
            extern_contracts: with_contracts.then(|| {
                BTreeMap::from([(
                    "examples/extern/contracts/buffer_api.qcore".to_owned(),
                    CONTRACTS.to_owned(),
                )])
            }),
            native_libraries: None,
            native_library_headers: None,
        },
    )
    .unwrap_or_else(|errors| panic!("{name} did not compile: {errors:?}"))
}

fn transcript(readme: &str, marker: &str) -> String {
    let tail = readme
        .split_once(&format!("<!-- {marker} -->"))
        .unwrap_or_else(|| panic!("missing transcript marker {marker}"))
        .1;
    let fenced = tail.split_once("```text").unwrap().1;
    let body = fenced
        .split_once("```")
        .unwrap()
        .0
        .replace("\r\n", "\n")
        .trim_matches('\n')
        .to_owned();
    if body.is_empty() { body } else { body + "\n" }
}

fn run(name: &str, source: &str, with_contracts: bool) -> (String, String) {
    let module = compile(name, source, with_contracts);
    let extras = if with_contracts {
        &[("buffer_api.c", BUFFER_API_C)][..]
    } else {
        &[]
    };
    let output = windows_c::run_with_sources(
        module.c_source.as_deref().expect("executable example"),
        None,
        &[],
        extras,
    );
    assert!(output.status.success(), "{name}: {output:?}");
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn two_workers_prints_the_documented_system_trace() {
    let source = include_str!("../../examples/threads/two_workers.qr");
    let readme = include_str!("../../examples/threads/README.md");
    let (out, err) = run("two_workers.qcore", source, false);
    assert_eq!(out, transcript(readme, "transcript:stdout"));
    assert_eq!(err, transcript(readme, "transcript:stderr"));
}

#[test]
fn branching_examples_print_the_documented_system_traces() {
    let readme = include_str!("../../examples/branching/README.md");
    for (name, source, marker) in [
        (
            "conditional_release.qcore",
            include_str!("../../examples/branching/conditional_release.qr"),
            "transcript: conditional_release stdout",
        ),
        (
            "condition_tier.qcore",
            include_str!("../../examples/branching/condition_tier.qr"),
            "transcript: condition_tier stdout",
        ),
        (
            "early_return.qcore",
            include_str!("../../examples/branching/early_return.qr"),
            "transcript: early_return stdout",
        ),
    ] {
        let (out, err) = run(name, source, false);
        assert_eq!(out, transcript(readme, marker), "{name}");
        assert!(err.is_empty(), "{name}: {err}");
    }
}

#[test]
fn pipeline_and_scalar_stages_print_the_documented_system_traces() {
    for (name, source, readme, out_marker, err_marker) in [
        (
            "pipeline.qcore",
            include_str!("../../examples/stdio/pipeline.qr"),
            include_str!("../../examples/stdio/README.md"),
            "transcript: pipeline stdout",
            Some("transcript: pipeline stderr"),
        ),
        (
            "scalar_stages.qcore",
            include_str!("../../examples/pipes/scalar_stages.qr"),
            include_str!("../../examples/pipes/README.md"),
            "transcript: scalar_stages stdout",
            None,
        ),
    ] {
        assert!(source.contains("log_level trace;"), "{name}");
        let (out, err) = run(name, source, true);
        assert_eq!(out, transcript(readme, out_marker), "{name}");
        match err_marker {
            Some(marker) => assert_eq!(err, transcript(readme, marker), "{name}"),
            None => assert!(err.is_empty(), "{name}: {err}"),
        }
    }
}

#[test]
fn declared_extern_examples_print_the_documented_system_traces() {
    let readme = include_str!("../../examples/extern/README.md");
    for (name, source, out_marker, err_marker) in [
        (
            "buffer_round_trip.qcore",
            include_str!("../../examples/extern/buffer_round_trip.qr"),
            "transcript: buffer_round_trip stdout",
            None,
        ),
        (
            "scalar_results.qcore",
            include_str!("../../examples/extern/scalar_results.qr"),
            "transcript: scalar_results stdout",
            None,
        ),
        (
            "borrowed_render.qcore",
            include_str!("../../examples/extern/borrowed_render.qr"),
            "transcript: borrowed_render stdout",
            Some("transcript: borrowed_render stderr"),
        ),
    ] {
        assert!(source.contains("log_level trace;"), "{name}");
        let (out, err) = run(name, source, true);
        assert_eq!(out, transcript(readme, out_marker), "{name}");
        match err_marker {
            Some(marker) => assert_eq!(err, transcript(readme, marker), "{name}"),
            None => assert!(err.is_empty(), "{name}: {err}"),
        }
    }
}

#[test]
fn unsafe_linkage_prints_only_its_source_authored_trace_logs() {
    let source = include_str!("../../examples/unsafe-linkage/unchecked_calls.qr");
    let api = include_str!("../../examples/unsafe-linkage/unchecked_api.c");
    let readme = include_str!("../../examples/unsafe-linkage/README.md");
    let module = compile("unchecked_calls.qcore", source, false);
    let output = windows_c::run_with_sources(
        module.c_source.as_deref().expect("executable example"),
        None,
        &[],
        &[("unchecked_api.c", api)],
    );
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        transcript(readme, "transcript: unchecked_calls stdout")
    );
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        transcript(readme, "transcript: unchecked_calls stderr")
    );
}
