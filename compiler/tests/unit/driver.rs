use super::*;
use crate::{ExecutionOutcome, TraceEvent, parse_c_observation};
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn compile(source: &str) -> CompiledModule {
    compile_files(
        &[SourceFile::new("main.qc", source)],
        &CompileOptions::default(),
    )
    .unwrap()
}

#[test]
fn compiles_values_and_maps_emitted_source() {
    let module = compile("fn main() -> i32 { return 42; }");
    assert_eq!(
        module.reference_observation.outcome,
        ExecutionOutcome::Ok {
            value: 42,
            payload: Vec::new()
        }
    );
    assert!(module.c_source.contains("#line 1 \"main.qc\""));
    assert_eq!(module.source_map[0].source.path, "main.qc");
}

#[test]
fn combines_module_files_and_checks_calls() {
    let files = [
        SourceFile::new("leaf.qc", "fn leaf() -> i32 { return 1; }"),
        SourceFile::new("main.qc", "fn main() -> i32 { leaf(); return 2; }"),
    ];
    let module = compile_files(&files, &CompileOptions::default()).unwrap();
    assert_eq!(module.checked.functions.len(), 2);
    assert_eq!(module.entry, "main");
}

#[test]
fn rejects_observable_calls_that_the_reduced_c_cfg_cannot_encode() {
    let files = [
        SourceFile::new(
            "leaf.qc",
            "error Failure { Failed; } fn leaf() -> i32 { qcore_check(false, Failure); return 1; }",
        ),
        SourceFile::new("main.qc", "fn main() -> i32 { leaf(); return 2; }"),
    ];
    let errors = compile_files(&files, &CompileOptions::default()).unwrap_err();
    assert_eq!(errors[0].code, "Q500");
    assert_eq!(errors[0].location.path, "main.qc");
}

#[test]
fn synthesized_quantitative_bound_covers_the_resource_trace() {
    let module = compile(
        "error Failure { Failed; } fn main() -> i32 { let a = qcore_acquire(3, true, Failure); let b = qcore_acquire(4, true, Failure); drop b; drop a; return 0; }",
    );
    let summary = &module.checked.functions[&module.checked.source.globals[&module.entry]].summary;
    assert_eq!(summary.flow.peak().get("resources"), 2);
    assert_eq!(summary.flow.total().get("allocations"), 2);
}

#[test]
fn preserves_error_cleanup_and_ownership_transfer_observations() {
    let failed = compile(
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(7, true, Failure); qcore_check(false, Failure); drop r; return 1; }",
    );
    assert_eq!(
        failed.reference_observation.outcome,
        ExecutionOutcome::Error {
            name: "Failure".into(),
            tag: 1
        }
    );
    assert_eq!(
        failed.reference_observation.trace,
        vec![TraceEvent::Acquire(7), TraceEvent::Release(7)]
    );
    let moved = compile(
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(9, true, Failure); move r; return -3; }",
    );
    assert_eq!(
        moved.reference_observation.outcome,
        ExecutionOutcome::Ok {
            value: -3,
            payload: vec![9]
        }
    );
    assert_eq!(
        moved.reference_observation.trace,
        vec![TraceEvent::Acquire(9), TraceEvent::Transfer(9)]
    );
}

#[test]
fn reports_unsupported_source_at_the_owning_file() {
    let errors = compile_files(
        &[SourceFile::new(
            "bad.qc",
            "fn main() -> i32 { if (true) { return 1; } }",
        )],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].location.path, "bad.qc");
    assert_eq!(errors[0].code, "Q500");
}

#[test]
fn checks_parameter_usage_even_for_uncalled_functions() {
    for grade in ["one", "relevant", "2", "[1, inf]"] {
        let source = format!(
            "fn discard(x: i32 use {grade}) -> i32 {{ return 0; }}\nfn main() -> i32 {{ return 0; }}"
        );
        let errors = compile_files(
            &[SourceFile::new("usage.qc", source)],
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert_eq!(errors[0].code, "Q100", "{grade}");
        assert_eq!(errors[0].location.path, "usage.qc");
        assert!(errors[0].message.contains("`x`"));
    }
    for grade in ["zero", "affine", "many", "[0, 3]"] {
        let source = format!(
            "fn discard(x: i32 use {grade}) -> i32 {{ return 0; }}\nfn main() -> i32 {{ return 0; }}"
        );
        assert_eq!(compile(&source).checked.functions.len(), 2);
    }
}

#[test]
fn a_shadowing_local_does_not_discharge_a_parameter_usage_obligation() {
    let source = "error Failure { Failed; } fn discard(x: i32 use one) -> i32 { let x = qcore_acquire(1, true, Failure); drop x; return 0; } fn main() -> i32 { return 0; }";
    let errors = compile_files(
        &[SourceFile::new("usage.qc", source)],
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert_eq!(errors[0].code, "Q100");
}

#[test]
fn records_source_demands_and_rejects_unsupported_local_contracts() {
    let module = compile(
        "error Failure { Failed; } fn main() -> i32 { let x = qcore_acquire(1, true, Failure); drop x; return 0; }",
    );
    let main = &module.checked.functions[&module.checked.source.globals["main"]];
    assert!(
        main.usage
            .iter()
            .any(|(name, demand)| name.ends_with(":x") && demand == Usage::ONE)
    );
    let failed = compile(
        "error Failure { Failed; } fn main() -> i32 { let x = qcore_acquire(1, true, Failure); qcore_check(false, Failure); drop x; return 0; }",
    );
    let main = &failed.checked.functions[&failed.checked.source.globals["main"]];
    assert!(main.usage.iter().next().is_none());
    let errors = compile_files(&[SourceFile::new("usage.qc", "error Failure { Failed; } fn main() -> i32 { let x: own<File> use zero = qcore_acquire(1, true, Failure); drop x; return 0; }")], &CompileOptions::default()).unwrap_err();
    assert_eq!(errors[0].code, "Q100");
}

#[test]
fn call_continuations_have_conservative_usage_lower_bounds() {
    let module = compile(
        "error Failure { Failed; } fn leaf() -> i32 { return 0; } fn main() -> i32 { let x = qcore_acquire(1, true, Failure); leaf(); drop x; return 0; }",
    );
    let main = &module.checked.functions[&module.checked.source.globals["main"]];
    assert!(
        main.usage
            .iter()
            .any(|(name, demand)| name.ends_with(":x") && demand == Usage::AFFINE)
    );
    assert!(
        main.usage
            .iter()
            .any(|(name, demand)| name.starts_with("$function:") && demand == Usage::ONE)
    );
}

#[test]
fn emitted_c_matches_reference_for_value_error_trace_and_transfer() {
    if Command::new("cc").arg("--version").output().is_err() {
        return;
    }
    for source in [
        "fn main() -> i32 { return 17; }",
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(1, true, Failure); qcore_check(false, Failure); drop r; return 0; }",
        "error Failure { Failed; } fn main() -> i32 { let r = qcore_acquire(2, true, Failure); move r; return 5; }",
    ] {
        let module = compile(source);
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let directory =
            std::env::temp_dir().join(format!("qcore-cycle12-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let c_path = directory.join("program.c");
        let exe_path = directory.join("program");
        fs::write(&c_path, &module.c_source).unwrap();
        let status = Command::new("cc")
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
            .arg(&c_path)
            .arg("-o")
            .arg(&exe_path)
            .status()
            .unwrap();
        assert!(status.success());
        let output = Command::new(&exe_path).output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            parse_c_observation(
                std::str::from_utf8(&output.stdout).unwrap(),
                module.checked.functions[&module.checked.source.globals[&module.entry]]
                    .cfg
                    .error_abi()
            )
            .unwrap(),
            module.reference_observation
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
