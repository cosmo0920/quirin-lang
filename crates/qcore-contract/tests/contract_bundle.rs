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

use qcore_contract::bundle::ContractBundle;
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-bundle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("contracts/example")).unwrap();
        fs::write(
            root.join("contracts/example/policy.qcore"),
            "module contract;\n",
        )
        .unwrap();
        Self(root)
    }
    fn inventory(&self) -> Value {
        json!({"schema_version":1,"plugin_id":"example.policy","contracts":{"contracts/example/policy.qcore":sha256_hex(b"module contract;\n")}})
    }
    fn write(&self, value: &Value) -> String {
        let bytes = serde_json::to_vec(value).unwrap();
        fs::write(self.0.join("contracts.json"), &bytes).unwrap();
        sha256_hex(&bytes)
    }
    fn load(&self, digest: &str) -> Result<ContractBundle, String> {
        ContractBundle::load(&self.0.join("contracts.json"), digest, "example.policy")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn pinned_data_snapshot_needs_no_executable_and_survives_file_changes() {
    let f = Fixture::new();
    let hash = f.write(&f.inventory());
    let bundle = f.load(&hash).unwrap();
    assert!(!f.0.join("provider").exists());
    fs::write(f.0.join("contracts/example/policy.qcore"), "changed").unwrap();
    assert_eq!(
        bundle.contract("contracts/example/policy.qcore").unwrap(),
        "module contract;\n"
    );
    assert!(bundle.contract("contracts/example/missing.qcore").is_err());
    assert!(f.load(&hash).is_err());
}

#[test]
fn independent_manifest_pin_and_owner_are_required() {
    let f = Fixture::new();
    let hash = f.write(&f.inventory());
    assert!(f.load(&"0".repeat(64)).is_err());
    assert!(f.load("not-a-digest").is_err());
    assert!(ContractBundle::load(&f.0.join("contracts.json"), &hash, "another.owner").is_err());
    let mut changed = f.inventory();
    changed["plugin_id"] = json!("another.owner");
    f.write(&changed);
    assert!(f.load(&hash).is_err());
}

#[test]
fn nested_paths_compare_as_paths_rather_than_string_sort_order() {
    let f = Fixture::new();
    let mut inventory = f.inventory();
    // String order puts example.qcore before example/policy.qcore, whereas
    // PathBuf compares the directory component first. Both are valid assets.
    fs::write(f.0.join("contracts/example.qcore"), "module contract;\n").unwrap();
    inventory["contracts"]["contracts/example.qcore"] = json!(sha256_hex(b"module contract;\n"));
    let bundle = f.load(&f.write(&inventory)).unwrap();
    assert_eq!(
        bundle.contract("contracts/example.qcore").unwrap(),
        "module contract;\n"
    );
    assert_eq!(
        bundle.contract("contracts/example/policy.qcore").unwrap(),
        "module contract;\n"
    );
}

#[test]
fn unsupported_ambiguous_and_escaping_inventories_reject() {
    let f = Fixture::new();
    for (field, value) in [
        ("schema_version", json!(2)),
        ("contracts", json!({})),
        ("extra", json!(true)),
        ("plugin_id", json!("invalid owner")),
    ] {
        let mut inventory = f.inventory();
        inventory[field] = value;
        assert!(f.load(&f.write(&inventory)).is_err(), "{inventory}");
    }
    for path in [
        "../policy.qcore",
        "/contracts/x.qcore",
        "contracts/../x.qcore",
        "contracts//x.qcore",
        "contracts/./x.qcore",
        "contracts/a\\b.qcore",
        "contracts/C:x.qcore",
        "contracts/file.txt",
    ] {
        let mut inventory = f.inventory();
        inventory["contracts"] = json!({path:sha256_hex(b"module contract;\n")});
        assert!(f.load(&f.write(&inventory)).is_err(), "{path}");
    }
    let raw = format!(
        "{{\"schema_version\":1,\"plugin_id\":\"example.policy\",\"contracts\":{{\"contracts/example/policy.qcore\":\"{}\",\"contracts/example/policy.qcore\":\"{}\"}}}}",
        "0".repeat(64),
        sha256_hex(b"module contract;\n")
    );
    fs::write(f.0.join("contracts.json"), &raw).unwrap();
    assert!(f.load(&sha256_hex(raw.as_bytes())).is_err());
}

#[test]
fn exact_file_set_and_utf8_nonempty_bounds_are_checked() {
    let f = Fixture::new();
    let hash = f.write(&f.inventory());
    fs::write(f.0.join("contracts/extra.qcore"), "extra").unwrap();
    assert!(f.load(&hash).is_err());
    fs::remove_file(f.0.join("contracts/extra.qcore")).unwrap();
    let path = f.0.join("contracts/example/policy.qcore");
    fs::remove_file(&path).unwrap();
    assert!(f.load(&hash).is_err());
    for bytes in [vec![255], b" \n".to_vec(), vec![b'x'; 8_388_609]] {
        fs::write(&path, &bytes).unwrap();
        let mut inv = f.inventory();
        inv["contracts"]["contracts/example/policy.qcore"] = json!(sha256_hex(&bytes));
        assert!(f.load(&f.write(&inv)).is_err());
    }
}

#[test]
fn aggregate_bundle_budget_includes_unselected_files() {
    let f = Fixture::new();
    let mut inv = f.inventory();
    let bytes = vec![b'x'; 8_388_608];
    for n in 0..4 {
        let path = format!("contracts/example/large{n}.qcore");
        fs::write(f.0.join(&path), &bytes).unwrap();
        inv["contracts"][&path] = json!(sha256_hex(&bytes));
    }
    assert!(f.load(&f.write(&inv)).is_err());
}

#[cfg(unix)]
#[test]
fn symlink_assets_directories_and_manifest_reject() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let hash = f.write(&f.inventory());
    let asset = f.0.join("contracts/example/policy.qcore");
    fs::rename(&asset, f.0.join("outside.qcore")).unwrap();
    symlink(f.0.join("outside.qcore"), &asset).unwrap();
    assert!(f.load(&hash).is_err());
    fs::remove_file(&asset).unwrap();
    fs::rename(f.0.join("outside.qcore"), &asset).unwrap();
    fs::rename(f.0.join("contracts/example"), f.0.join("outside")).unwrap();
    symlink(f.0.join("outside"), f.0.join("contracts/example")).unwrap();
    assert!(f.load(&hash).is_err());
    fs::remove_file(f.0.join("contracts/example")).unwrap();
    fs::rename(f.0.join("outside"), f.0.join("contracts/example")).unwrap();
    fs::rename(f.0.join("contracts.json"), f.0.join("manifest.json")).unwrap();
    symlink(f.0.join("manifest.json"), f.0.join("contracts.json")).unwrap();
    assert!(f.load(&hash).is_err());
}
