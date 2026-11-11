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

use std::io::Write;
use std::sync::{Once, OnceLock};

static CLANG_AVAILABILITY: OnceLock<Result<(), String>> = OnceLock::new();
static CLANG_WARNING: Once = Once::new();

/// Normalize compile-time text fixtures before syntax-preserving assembly
/// or comparison, independently of Git's platform checkout policy.
pub(crate) fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n")
}

/// Execute syntax-only POSIX LP64 fixtures with their declared target, even
/// when the installed Clang defaults to Windows LLP64. Child-only settings
/// keep concurrent native-target tests and the user's override untouched.
pub(crate) fn posix_clang_test() -> bool {
    if !clang_available() {
        assert!(
            std::env::var_os("QCORE_TEST_POSIX_CHILD").is_none(),
            "the isolated POSIX child must execute its Clang-backed test"
        );
        return false;
    }
    #[cfg(windows)]
    if std::env::var_os("QCORE_TEST_POSIX_CHILD").is_none() {
        use std::path::PathBuf;
        use std::process::Command;
        static LAUNCHER: OnceLock<PathBuf> = OnceLock::new();
        let clang = qcore_c_gateway::clang_command().expect("POSIX fixtures require Clang");
        let launcher = LAUNCHER.get_or_init(|| {
            let directory = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join(format!("qcore-posix-clang-{}", std::process::id()));
            std::fs::create_dir_all(&directory).unwrap();
            let source = directory.join("launcher.rs");
            let executable = directory.join("clang-posix-fixture.exe");
            std::fs::write(
                &source,
                r#"
fn main() {
    let status = std::process::Command::new(std::env::var_os("QCORE_TEST_REAL_CLANG").unwrap())
        .arg("--target=x86_64-unknown-linux-gnu")
        .args(std::env::args_os().skip(1))
        .status().expect("launch discovered Clang");
    std::process::exit(status.code().unwrap_or(1));
}
"#,
            )
            .unwrap();
            let output = Command::new("rustc")
                .arg(&source)
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            executable
        });
        let thread = std::thread::current();
        let test = thread.name().expect("libtest names its test threads");
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env("QCORE_TEST_POSIX_CHILD", "1")
            .env("QCORE_TEST_REAL_CLANG", clang.get_program())
            .env("QCORE_CLANG", launcher)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "POSIX target test {test}:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "child must execute the selected test: {output:?}"
        );
        return false;
    }
    true
}

/// Report whether the Clang-backed tests can run, emitting one visible
/// warning when the executable is absent or unusable.
pub(crate) fn clang_available() -> bool {
    let availability =
        CLANG_AVAILABILITY.get_or_init(|| qcore_c_gateway::clang_command().map(|_| ()));
    match availability {
        Ok(()) => true,
        Err(reason) => {
            CLANG_WARNING.call_once(|| {
                // Write directly so libtest's successful-test capture does
                // not hide the reason these tests did not execute.
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "warning: skipping Clang-dependent tests: {reason}"
                );
            });
            false
        }
    }
}

#[test]
fn newline_normalization_preserves_lf_syntax() {
    let source = "int f(void) {\n    return 0;\n}\n";
    assert_eq!(normalize_newlines(source), source);
}

#[test]
fn newline_normalization_maps_crlf_and_mixed_syntax_to_lf() {
    assert_eq!(
        normalize_newlines("int f(void) {\r\n    return 0;\n}\r\n"),
        "int f(void) {\n    return 0;\n}\n"
    );
}

#[test]
fn newline_normalization_is_idempotent() {
    let once = normalize_newlines("a\r\nb\nc\r\n");
    assert_eq!(normalize_newlines(&once), once);
}
