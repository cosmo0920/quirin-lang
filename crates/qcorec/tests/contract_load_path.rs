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

//! The opt-in CLI route admits only owned resource-v1 assumptions through the
//! published inclusion loader. It does not change ordinary structural check.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(args: &[&Path]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quirin"));
    for argument in args {
        command.arg(argument);
    }
    command.output().unwrap()
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn real_inherited_module_requires_explicit_load_path() {
    let root = root();
    let source =
        root.join("providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv");
    let manifest = root.join("providers/contract-load-path.json");
    let bare = run(&[Path::new("check"), &source]);
    assert!(!bare.status.success(), "{bare:?}");
    assert!(text(&bare).contains("Q000"), "{bare:?}");

    let checked = run(&[
        Path::new("check"),
        Path::new("--contract-load-path"),
        &manifest,
        &source,
    ]);
    assert!(checked.status.success(), "{checked:?}");
    let stdout = String::from_utf8_lossy(&checked.stdout);
    assert!(
        stdout.contains("validated resource-v1 contract assumptions"),
        "{checked:?}"
    );
    assert!(
        stdout.contains(&source.display().to_string()),
        "{checked:?}"
    );
    assert!(
        stdout.contains("no C bodies checked or safety verdict established"),
        "{checked:?}"
    );
}

#[test]
fn bad_option_combinations_and_unowned_source_refuse() {
    let root = root();
    let source =
        root.join("providers/quirin-provider-fluent-bit/contracts/fluent_bit/resource_memory.qv");
    let manifest = root.join("providers/contract-load-path.json");
    let runtime = root.join("examples/http/exchange.qr");
    for args in [
        vec![
            Path::new("emit"),
            Path::new("--contract-load-path"),
            &manifest,
            &source,
        ],
        vec![
            Path::new("check"),
            Path::new("--contract-load-path"),
            &manifest,
            Path::new("--entry"),
            Path::new("main"),
            &source,
        ],
        vec![
            Path::new("check"),
            Path::new("--contract-load-path"),
            &manifest,
            &source,
            &source,
        ],
        vec![
            Path::new("check"),
            Path::new("--contract-load-path"),
            &manifest,
            &runtime,
        ],
    ] {
        let output = run(&args);
        assert!(!output.status.success(), "{output:?}");
    }
    let missing = run(&[
        Path::new("check"),
        Path::new("--contract-load-path"),
        Path::new("/nonexistent/qcore-load-path.json"),
        &source,
    ]);
    assert!(!missing.status.success(), "{missing:?}");
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let unique = format!(
            "qcore-contract-cli-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(root.join("demo/contracts/demo")).unwrap();
        Self(root)
    }

    fn write(&self, name: &str, source: &str) -> PathBuf {
        let path = self.0.join("demo/contracts/demo").join(name);
        std::fs::write(&path, source).unwrap();
        path
    }

    fn manifest(&self, published: serde_json::Value) -> PathBuf {
        let path = self.0.join("load-path.json");
        let value = serde_json::json!({
            "schema_version": 1,
            "crates": [{
                "name": "quirin-provider-demo",
                "namespace": "demo",
                "version": "0.1.0",
                "contracts": "demo/contracts",
                "dependencies": [],
                "modules": ["demo/child.qv", "demo/parent.qv"],
                "published": published
            }]
        });
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn unpublished_parent_and_bad_pin_refuse() {
    let fixture = Fixture::new();
    let child = fixture.write("child.qv", "module contract;\n#include \"quirin-provider-demo:demo/parent.qv\";\nextern fn child(p: ptr<mut, u8>) -> unit contract inherit demo::parent;\n");
    fixture.write("parent.qv", "module contract;\nextern fn parent(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };\n");
    let manifest = fixture.manifest(serde_json::json!({}));
    let unpublished = run(&[
        Path::new("check"),
        Path::new("--contract-load-path"),
        &manifest,
        &child,
    ]);
    assert!(!unpublished.status.success(), "{unpublished:?}");
    assert!(text(&unpublished).contains("QI005"), "{unpublished:?}");

    let manifest = fixture.manifest(serde_json::json!({"demo/parent.qv": "0000000000000000000000000000000000000000000000000000000000000000"}));
    let mismatch = run(&[
        Path::new("check"),
        Path::new("--contract-load-path"),
        &manifest,
        &child,
    ]);
    assert!(!mismatch.status.success(), "{mismatch:?}");
    assert!(text(&mismatch).contains("QI014"), "{mismatch:?}");
}

#[test]
fn wrong_mode_and_malformed_manifest_refuse() {
    let fixture = Fixture::new();
    let child = fixture.write(
        "child.qv",
        "module executable; fn main() -> i32 { return 0; }",
    );
    fixture.write("parent.qv", "module contract; extern fn parent() -> unit contract { requires resource_contract_v1; ensures normal resource_return_none(); };");
    let manifest = fixture.manifest(serde_json::json!({}));
    let wrong_mode = run(&[
        Path::new("check"),
        Path::new("--contract-load-path"),
        &manifest,
        &child,
    ]);
    assert!(!wrong_mode.status.success(), "{wrong_mode:?}");

    std::fs::write(&manifest, b"{}").unwrap();
    let malformed = run(&[
        Path::new("check"),
        Path::new("--contract-load-path"),
        &manifest,
        &child,
    ]);
    assert!(!malformed.status.success(), "{malformed:?}");
    assert!(text(&malformed).contains("QI014"), "{malformed:?}");
}
