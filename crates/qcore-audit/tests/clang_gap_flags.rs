//! Card CG00-CLANG-GAP-BASELINE: executed BEFORE controls for the compile-flag
//! halves of the Clang gap analysis's I01 (dialects), I02 (target and ABI
//! flags) and I06 (extension flags). The compile-database reader admits an
//! allowlist and refuses the rest of the command; these pin the refusals.

use std::path::PathBuf;

use qcore_audit::compile_db::{CompileEntry, compiler_options};

/// The configured Clang driver, so a refusal is the flag's and never the
/// driver check's (`require_clang_driver`).
fn driver() -> Option<String> {
    match qcore_c_gateway::clang_command() {
        Ok(command) => command.get_program().to_str().map(str::to_owned),
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG00 flag control: {reason}");
            None
        }
    }
}

fn options(driver: &str, extra: &str) -> Result<(), String> {
    compiler_options(&CompileEntry {
        directory: PathBuf::from("/tmp"),
        command: format!("{driver} {extra} -c /tmp/unit.c -o /tmp/unit.o"),
        file: PathBuf::from("/tmp/unit.c"),
    })
    .map(|_| ())
}

#[test]
fn control_an_admitted_command() {
    let Some(driver) = driver() else { return };
    options(&driver, "-std=gnu17 -Wall -O2").expect("the admitted flags parse");
}

#[test]
fn i01_dialects_outside_the_four_are_refused() {
    let Some(driver) = driver() else { return };
    for flag in ["-std=c17", "-std=c23", "-std=gnu23", "-std=c2y"] {
        let error = options(&driver, flag).expect_err(flag);
        assert!(
            !error.contains("configured Clang executable"),
            "{flag}: {error}"
        );
    }
}

#[test]
fn i02_target_and_abi_flags_are_refused() {
    let Some(driver) = driver() else { return };
    for flag in [
        "--target=x86_64-pc-linux-gnu",
        "-target x86_64-pc-linux-gnu",
        "-mabi=lp64",
        "-m32",
    ] {
        let error = options(&driver, flag).expect_err(flag);
        assert!(
            !error.contains("configured Clang executable"),
            "{flag}: {error}"
        );
    }
}

#[test]
fn i06_extension_flags_are_refused() {
    let Some(driver) = driver() else { return };
    for flag in ["-fopenmp", "-fblocks", "-fms-extensions"] {
        let error = options(&driver, flag).expect_err(flag);
        assert!(
            !error.contains("configured Clang executable"),
            "{flag}: {error}"
        );
    }
}
