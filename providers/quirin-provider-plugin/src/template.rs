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

//! A new user contract crate from the template (`quirin-plugin new`).
//!
//! The template is the directory `template/` of this crate, compiled in. Its
//! one contract is held awaiting review (`resource_review_pending`), so a new
//! crate can never make anything `Safe` until a person has written and
//! reviewed a real contract.

use std::fs;
use std::path::Path;

use crate::derive_namespace;

const FILES: &[(&str, &str)] = &[
    ("Cargo.toml", include_str!("../template/Cargo.toml.tmpl")),
    ("src/lib.rs", include_str!("../template/src/lib.rs.tmpl")),
    (
        "contracts/{{namespace}}/example.qv",
        include_str!("../template/example.qv.tmpl"),
    ),
    (
        "quirin-plugin.json",
        include_str!("../template/quirin-plugin.json.tmpl"),
    ),
    ("README.md", include_str!("../template/README.md.tmpl")),
];

fn fill(text: &str, name: &str, namespace: &str) -> String {
    text.replace("{{name}}", name)
        .replace("{{namespace}}", namespace)
}

/// Write a new crate named `name` into `directory`, which must not exist.
///
/// # Errors
///
/// A name that is not `quirin-provider-NAME` with an identifier-shaped
/// `NAME`, or an existing directory.
pub fn instantiate(directory: &Path, name: &str) -> Result<(), String> {
    let namespace = derive_namespace(name).ok_or_else(|| {
        format!("a contract crate is named `quirin-provider-NAME` with an identifier-shaped NAME, not `{name}`")
    })?;
    fs::create_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    for (path, text) in FILES {
        let path = directory.join(fill(path, name, &namespace));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(&path, fill(text, name, &namespace))
            .map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}
