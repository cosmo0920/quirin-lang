//! CLI process tests relocated with the binary (W0 of
//! `docs/GENERALIZATION_PLAN.md`); library-level toolchain and module-mode
//! tests remain in `qcore-compiler`.

use std::process::Command;

#[test]
fn core_cli_works_without_clang() {
    let directory = std::env::temp_dir().join(format!("qcore-no-clang-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let input = directory.join("main.qcore");
    std::fs::write(&input, "fn main() -> i32 { return 17; }").unwrap();
    for operation in ["check", "emit"] {
        let output = Command::new(env!("CARGO_BIN_EXE_qcorec"))
            .env("QCORE_CLANG", directory.join("missing-clang.exe"))
            .env("PATH", "")
            .arg(operation)
            .arg(&input)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn cli_checks_contracts_and_refuses_standalone_emit_without_overwriting_output() {
    let directory = std::env::temp_dir().join(format!("qcore-module-mode-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("api.qcore");
    let output = directory.join("output.c");
    std::fs::write(&source, "module contract; extern fn f() -> unit;").unwrap();
    std::fs::write(&output, "keep existing output").unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_qcorec"))
        .arg("check")
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(String::from_utf8_lossy(&checked.stdout).contains("no entry point"));
    let emitted = Command::new(env!("CARGO_BIN_EXE_qcorec"))
        .arg("emit")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!emitted.status.success());
    assert_eq!(
        std::fs::read_to_string(&output).unwrap(),
        "keep existing output"
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn plugin_discovery_errors_are_reported_without_builtin_fallback() {
    let missing =
        std::env::temp_dir().join(format!("qcore-absent-profiles-{}", std::process::id()));
    for value in [missing.as_os_str(), std::ffi::OsStr::new("")] {
        let output = Command::new(env!("CARGO_BIN_EXE_qcorec"))
            .env("QCORE_PROFILES", value)
            .args(["plugins", "list"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("provider discovery failed"));
    }
}

#[test]
#[cfg(target_os = "linux")]
fn cli_checks_emits_and_executes_native_source_tasks() {
    let directory =
        std::env::temp_dir().join(format!("qcore-cli-source-tasks-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let input = directory.join("tasks.qcore");
    let c = directory.join("tasks.c");
    let exe = directory.join("tasks");
    std::fs::write(
        &input,
        r#"
concurrency checked;
fn worker(x: i32) -> i32 { return x; }
fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = 42;
    thread scope s {
        let h = spawn s worker(copy x)?;
        let y = join move h?;
        return y;
    }
}
"#,
    )
    .unwrap();
    let checked = Command::new(env!("CARGO_BIN_EXE_qcorec"))
        .arg("check")
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(String::from_utf8_lossy(&checked.stdout).contains("scoped copy tasks (Linux C11)"));
    let emitted = Command::new(env!("CARGO_BIN_EXE_qcorec"))
        .arg("emit")
        .arg(&input)
        .arg("-o")
        .arg(&c)
        .output()
        .unwrap();
    assert!(
        emitted.status.success(),
        "{}",
        String::from_utf8_lossy(&emitted.stderr)
    );
    let compiled = Command::new(std::env::var("QCORE_CLANG").unwrap_or_else(|_| "clang".into()))
        .args(["-std=c11", "-pthread", "-Wall", "-Wextra", "-Werror"])
        .arg(&c)
        .arg("-o")
        .arg(&exe)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executed = Command::new(exe).output().unwrap();
    assert!(executed.status.success());
    assert!(String::from_utf8_lossy(&executed.stdout).contains("QCORE_TASK_RESULT 0 I32 42"));
    // A source failure must not replace an existing emitted artifact.
    let original = std::fs::read(&c).unwrap();
    std::fs::write(&input, "concurrency checked; fn main() -> unit { thread scope s { let h = spawn s main()?; } return; }").unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_qcorec"))
        .arg("emit")
        .arg(&input)
        .arg("-o")
        .arg(&c)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert_eq!(std::fs::read(&c).unwrap(), original);
    std::fs::remove_dir_all(directory).unwrap();
}
