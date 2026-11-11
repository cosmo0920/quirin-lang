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

use qcore_compiler::legacy_c::{LegacyCError, ResourceContracts, analyze_legacy_c11};
use std::process::Command;

#[test]
fn missing_explicit_clang_fails_closed() {
    // Set the environment only on a child, avoiding races with parallel tests.
    let output = Command::new(std::env::current_exe().unwrap())
        .env("QCORE_TEST_MISSING_CLANG_CHILD", "1")
        .env(
            "QCORE_CLANG",
            std::env::temp_dir().join("qcore-nonexistent-toolchain/clang.exe"),
        )
        .args(["--exact", "missing_clang_child", "--nocapture"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn missing_clang_child() {
    if std::env::var_os("QCORE_TEST_MISSING_CLANG_CHILD").is_none() {
        return;
    }
    let error = analyze_legacy_c11(
        "void subject(void) {}",
        "subject",
        &ResourceContracts::default(),
    )
    .unwrap_err();
    match error {
        LegacyCError::ProductionParserUnavailable(message) => {
            assert!(message.contains("QCORE_CLANG"));
            assert!(message.contains("C++ Clang tools for Windows"));
        }
        other => panic!("expected unavailable parser, got {other:?}"),
    }
}
