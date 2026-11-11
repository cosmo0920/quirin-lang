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

//! The inspector rejects invalid input before publishing decoded contents.
use std::process::Command;

#[test]
fn help_and_argument_errors_do_not_open_a_checkpoint() {
    let help = Command::new(env!("CARGO_BIN_EXE_quirin-dump-checkpoint"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--section"));
    for args in [
        vec![],
        vec!["--unknown"],
        vec!["--section", "not-a-section", "missing.qo"],
        vec!["--full", "--section", "result", "missing.qo"],
        vec!["missing.qo", "second.qo"],
    ] {
        let rejected = Command::new(env!("CARGO_BIN_EXE_quirin-dump-checkpoint"))
            .args(args)
            .output()
            .unwrap();
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("checkpoint dump refused"));
    }
}

#[test]
fn malformed_input_never_produces_a_decoded_dump() {
    let path = std::env::temp_dir().join(format!("quirin-dump-invalid-{}.qo", std::process::id()));
    std::fs::write(&path, b"QO\0\x01\r\n\x1a\n").unwrap();
    let rejected = Command::new(env!("CARGO_BIN_EXE_quirin-dump-checkpoint"))
        .arg("--full")
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_file(path).unwrap();
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("truncated"));
}
