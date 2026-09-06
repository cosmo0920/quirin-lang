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
