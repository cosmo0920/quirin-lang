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

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "quirin-runtime-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("lib")).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn check_and_emit_load_recursive_runtime_imports_from_the_source_directory() {
    let temp = Temp::new();
    std::fs::write(
        temp.0.join("main.qr"),
        "#import \"lib/helper\"; fn main() -> i32 { return helper(); }",
    )
    .unwrap();
    std::fs::write(
        temp.0.join("lib/helper.qr"),
        "module library; #import \"../value.qr\"; fn helper() -> i32 { return value(); }",
    )
    .unwrap();
    std::fs::write(temp.0.join("value.qr"), "fn value() -> i32 { return 6; }").unwrap();
    for command in ["check", "emit"] {
        let output = Command::new(env!("CARGO_BIN_EXE_quirin"))
            .arg(command)
            .arg(temp.0.join("main.qr"))
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
    }
    std::fs::remove_file(temp.0.join("value.qr")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quirin"))
        .arg("check")
        .arg(temp.0.join("main.qr"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Q014") && stderr.contains("helper.qr"),
        "{stderr}"
    );
}
