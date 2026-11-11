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

//! The user load path: a load-path manifest that **extends** the shipped one
//! with the user's crates (`qcore_contract::load_path_manifest`).
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "extends": "/opt/quirin/providers/contract-load-path.json",
//!   "crates": [ { "name": "quirin-provider-acme", "namespace": "acme", "version": "0.1.0",
//!                 "contracts": "quirin-provider-acme/contracts", "dependencies": [],
//!                 "modules": ["acme/pool.qv"], "published": {} } ],
//!   "overrides": [],
//!   "checkers": [ { "crate": "quirin-provider-acme", "module": "acme/pool.qv",
//!                   "functions": ["acme_pool_get", "acme_pool_put"],
//!                   "package": "quirin-provider-acme/checkers/pool-check" } ]
//! }
//! ```
//!
//! Every crate directory must sit under the manifest's directory, so its
//! `contracts` directory and its checker packages are relative paths without
//! traversal. `resource_view` is not written here: the loader decides it, so
//! run `quirin-audit plugin views --load-path FILE --write` after generating,
//! and whenever a module changes.

use std::path::Path;

use serde_json::{Value, json};

use crate::user_crate::UserCrate;
use crate::{LOAD_PATH_SCHEMA, relative_without_traversal, sha256_hex};

/// A crate directory relative to `base`, without traversal.
fn relative(base: &Path, directory: &Path) -> Result<String, String> {
    let base = base
        .canonicalize()
        .map_err(|error| format!("{}: {error}", base.display()))?;
    let directory = directory
        .canonicalize()
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    let relative = directory
        .strip_prefix(&base)
        .map_err(|_| {
            format!(
                "{} is not under the load path's directory {}",
                directory.display(),
                base.display()
            )
        })?
        .to_string_lossy()
        .replace('\\', "/");
    if relative_without_traversal(&relative) {
        Ok(relative)
    } else {
        Err(format!(
            "{relative} is not a relative path without traversal"
        ))
    }
}

/// The user load path over `crates`, extending `extends`, for a manifest to be
/// written in `base`.
///
/// # Errors
///
/// A crate outside `base`, two crates of one name, or a dependency on a
/// contract crate that is neither another user crate nor (by name) a shipped
/// one the core will resolve.
pub fn user_load_path(extends: &str, base: &Path, crates: &[UserCrate]) -> Result<Value, String> {
    let mut records = Vec::new();
    let mut checkers = Vec::new();
    let mut overrides = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    for krate in crates {
        if !names.insert(krate.name.clone()) {
            return Err(format!("crate `{}` is listed twice", krate.name));
        }
        let directory = relative(base, &krate.directory)?;
        records.push(json!({
            "name": krate.name,
            "namespace": krate.namespace,
            "version": krate.version,
            "contracts": format!("{directory}/contracts"),
            "dependencies": krate.dependencies,
            "modules": krate.modules.keys().collect::<Vec<_>>(),
            "published": krate.published.iter().map(|module| {
                (module.clone(), json!(sha256_hex(krate.modules[module].as_bytes())))
            }).collect::<serde_json::Map<_, _>>(),
        }));
        for checker in &krate.manifest.checkers {
            checkers.push(json!({
                "crate": krate.name,
                "module": checker.module,
                "functions": checker.functions,
                "package": format!("{directory}/{}", checker.package),
            }));
        }
        for record in &krate.manifest.overrides {
            overrides.push(json!({
                "function": record.function,
                "crate": krate.name,
                "shipped_crate": record.shipped_crate,
                "reason": record.reason,
            }));
        }
    }
    records.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));
    Ok(json!({
        "schema_version": LOAD_PATH_SCHEMA,
        "extends": extends,
        "crates": records,
        "overrides": overrides,
        "checkers": checkers,
    }))
}
