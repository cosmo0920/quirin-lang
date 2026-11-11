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

//! Card CONTRACT-INHERIT-ADOPTION: `ContractBundle` accepts an inventory's
//! `includes` pins (the Rust twin of `providers/package.py verify`'s rule).
//! A bundle whose contracts `#include` another crate's module must pin every
//! module reached and carry exactly those texts under `includes/`; every
//! departure refuses the whole bundle.

use qcore_contract::bundle::ContractBundle;
use qcore_schemas::sha256_hex;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const PARENT: &str = "module contract;\nextern fn free(ptr: ptr<mut, u8>) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_released(ptr, nullable);\n    ensures normal resource_return_none();\n};\n";
const CHILD: &str = "module contract;\n#include \"quirin-provider-libc:libc/memory.qv\";\nextern fn flb_free(ptr: ptr<mut, u8>) -> unit contract inherit libc::free;\n";
const KEY: &str = "quirin-provider-libc:libc/memory.qv";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-bundle-includes-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("contracts/example")).unwrap();
        fs::create_dir_all(root.join("includes/quirin-provider-libc/libc")).unwrap();
        fs::write(root.join("contracts/example/wrap.qcore"), CHILD).unwrap();
        fs::write(
            root.join("includes/quirin-provider-libc/libc/memory.qv"),
            PARENT,
        )
        .unwrap();
        Self(root)
    }

    fn inventory(&self) -> Value {
        json!({
            "schema_version": 1,
            "plugin_id": "example.policy",
            "contracts": {"contracts/example/wrap.qcore": sha256_hex(CHILD.as_bytes())},
            "includes": {KEY: {
                "crate": "quirin-provider-libc", "version": "0.1.0",
                "module": "libc/memory.qv", "sha256": sha256_hex(PARENT.as_bytes())
            }}
        })
    }

    fn load(&self, inventory: &Value) -> Result<ContractBundle, String> {
        let bytes = serde_json::to_vec(inventory).unwrap();
        fs::write(self.0.join("contracts.json"), &bytes).unwrap();
        ContractBundle::load(
            &self.0.join("contracts.json"),
            &sha256_hex(&bytes),
            "example.policy",
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn pinned_includes_load_and_serve_their_exact_text() {
    let f = Fixture::new();
    let bundle = f.load(&f.inventory()).unwrap();
    assert_eq!(bundle.included(KEY).unwrap(), PARENT);
    assert!(
        bundle
            .included("quirin-provider-libc:libc/other.qcore")
            .is_err()
    );
    assert_eq!(
        bundle.contract("contracts/example/wrap.qcore").unwrap(),
        CHILD
    );
}

#[test]
fn every_include_departure_refuses_the_bundle() {
    // An unpinned directive.
    let f = Fixture::new();
    let mut inventory = f.inventory();
    inventory.as_object_mut().unwrap().remove("includes");
    fs::remove_dir_all(f.0.join("includes")).unwrap();
    assert!(f.load(&inventory).unwrap_err().contains("not pinned"));

    // A stray includes/ tree without pins, for a bundle that includes nothing.
    let f = Fixture::new();
    fs::write(f.0.join("contracts/example/wrap.qcore"), PARENT).unwrap();
    let mut inventory = f.inventory();
    inventory.as_object_mut().unwrap().remove("includes");
    inventory["contracts"]["contracts/example/wrap.qcore"] = json!(sha256_hex(PARENT.as_bytes()));
    assert!(
        f.load(&inventory)
            .unwrap_err()
            .contains("no pinned identities")
    );

    // A changed parent.
    let f = Fixture::new();
    fs::write(
        f.0.join("includes/quirin-provider-libc/libc/memory.qv"),
        PARENT.replace("nullable", "nonnull"),
    )
    .unwrap();
    assert!(f.load(&f.inventory()).unwrap_err().contains("changed"));

    // A missing parent text, and an extra one.
    let f = Fixture::new();
    fs::remove_file(f.0.join("includes/quirin-provider-libc/libc/memory.qv")).unwrap();
    assert!(f.load(&f.inventory()).is_err());
    let f = Fixture::new();
    fs::write(
        f.0.join("includes/quirin-provider-libc/libc/extra.qcore"),
        PARENT,
    )
    .unwrap();
    assert!(f.load(&f.inventory()).unwrap_err().contains("file set"));

    // An unused pin.
    let f = Fixture::new();
    fs::write(f.0.join("contracts/example/wrap.qcore"), PARENT).unwrap();
    let mut inventory = f.inventory();
    inventory["contracts"]["contracts/example/wrap.qcore"] = json!(sha256_hex(PARENT.as_bytes()));
    assert!(
        f.load(&inventory)
            .unwrap_err()
            .contains("no contract reaches")
    );

    // Malformed pins: unknown field, key/identity mismatch, bad digest,
    // empty version, empty object.
    let f = Fixture::new();
    for mutate in [
        |v: &mut Value| v["includes"][KEY]["extra"] = json!(true),
        |v: &mut Value| v["includes"][KEY]["module"] = json!("libc/other.qcore"),
        |v: &mut Value| v["includes"][KEY]["sha256"] = json!("0"),
        |v: &mut Value| v["includes"][KEY]["version"] = json!(""),
        |v: &mut Value| v["includes"] = json!({}),
    ] {
        let mut inventory = f.inventory();
        mutate(&mut inventory);
        assert!(f.load(&inventory).is_err(), "{inventory}");
    }
}
