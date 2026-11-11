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

//! Card CG00-CLANG-GAP-BASELINE: executed BEFORE controls for the compile-flag
//! halves of the Clang gap analysis's I01 (dialects), I02 (target and ABI
//! flags) and I06 (extension flags). The compile-database reader admits an
//! allowlist and refuses the rest of the command; these pin the refusals.

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
    let directory = std::env::temp_dir();
    let file = directory.join("unit.c");
    let output = directory.join("unit.o");
    compiler_options(&CompileEntry {
        directory,
        command: format!(
            "\"{driver}\" {extra} -c '{}' -o '{}'",
            file.display(),
            output.display()
        ),
        file,
    })
    .map(|_| ())
}

#[test]
fn control_an_admitted_command() {
    let Some(driver) = driver() else { return };
    options(&driver, "-std=gnu17 -Wall -O2").expect("the admitted flags parse");
}

/// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): C17, C23 and C2y are explicit
/// identities now; every other spelling, alias included, stays refused.
#[test]
fn i01_dialects_outside_the_seven_are_refused() {
    let Some(driver) = driver() else { return };
    for flag in ["-std=c17", "-std=c23", "-std=c2y"] {
        options(&driver, flag).unwrap_or_else(|error| panic!("{flag}: {error}"));
    }
    for flag in [
        "-std=gnu23",
        "-std=c18",
        "-std=c2x",
        "-std=gnu2y",
        "-std=c99",
    ] {
        let error = options(&driver, flag).expect_err(flag);
        assert!(
            !error.contains("configured Clang executable"),
            "{flag}: {error}"
        );
    }
}

/// Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): a structured LP64 target or ABI
/// is admitted; an ILP32/LLP64 selection and `-m32` stay refused.
#[test]
fn i02_target_and_abi_flags_are_structured() {
    let Some(driver) = driver() else { return };
    for flag in [
        "-std=gnu17 --target=x86_64-pc-linux-gnu",
        "-std=gnu17 -target x86_64-pc-linux-gnu",
        "-std=gnu17 -mabi=lp64",
    ] {
        options(&driver, flag).unwrap_or_else(|error| panic!("{flag}: {error}"));
    }
    for flag in [
        "-std=gnu17 --target=i686-pc-linux-gnu",
        "-std=gnu17 --target=x86_64-pc-windows-msvc",
        "-std=gnu17 -mabi=ilp32",
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
