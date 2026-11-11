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

//! `quirin-audit plugin views` (card **W45-USER-CONTRACT-PLUGINS**): the
//! resource view of every user crate on a user load path, computed by the
//! same probe `providers/contracts.py` runs for the shipped crates
//! ([`crate::providers::probe_resource_views`]).
//!
//! ```text
//! quirin-audit plugin views --load-path USER.json [--write]
//! ```
//!
//! Without `--write` it compares each user crate's declared `resource_view`
//! with the probe and exits 1 on any difference; with `--write` it writes the
//! probed views into the user load path. The extended (shipped) load path is
//! never probed or written. The rest of the user workflow — a new crate from
//! the template, the load path itself, packaging a checker — is the SDK's
//! (`providers/quirin-provider-plugin`), which links no core crate.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::Value;

/// Usage text of `quirin-audit plugin`.
pub const USAGE: &str = "usage: quirin-audit plugin views --load-path USER.json [--write]";

/// The probed views of the user crates, and whether the file declares them.
///
/// # Errors
///
/// A load path that is not a user load path, or a probe failure.
pub fn views(load_path: &std::path::Path, write: bool) -> Result<bool, String> {
    let manifest = qcore_contract::load_path_manifest::load_contract_load_path_manifest(load_path)
        .map_err(|error| error.to_string())?;
    if manifest.extension.is_none() {
        return Err(format!(
            "{} is not a user load path (it extends no other); the shipped load path's views \
             are maintained by providers/contracts.py",
            load_path.display()
        ));
    }
    let probe = crate::providers::probe_resource_views(load_path)?;
    let text = fs::read_to_string(load_path)
        .map_err(|error| format!("{}: {error}", load_path.display()))?;
    let mut file: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let mut current = true;
    for record in file["crates"].as_array_mut().into_iter().flatten() {
        let Some(name) = record["name"].as_str().map(str::to_owned) else {
            continue;
        };
        let probed = &probe["crates"][&name]["resource_view"];
        if &record["resource_view"] != probed {
            current = false;
            println!(
                "{name}: declared {} probed {}",
                record["resource_view"], probed
            );
            if write {
                record["resource_view"] = probed.clone();
            }
        }
        for (module, reason) in probe["crates"][&name]["refused"]
            .as_object()
            .into_iter()
            .flatten()
        {
            println!("{name}: {module} is outside the resource view: {reason}");
        }
    }
    if write && !current {
        let mut bytes = serde_json::to_vec_pretty(&file).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        fs::write(load_path, bytes).map_err(|error| format!("{}: {error}", load_path.display()))?;
    }
    Ok(current)
}

/// Entry point of `quirin-audit plugin`.
#[must_use]
pub fn main(arguments: Vec<String>) -> ExitCode {
    let mut iter = arguments.into_iter();
    if iter.next().as_deref() != Some("views") {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    }
    let mut load_path = None;
    let mut write = false;
    while let Some(argument) = iter.next() {
        match argument.as_str() {
            "--load-path" => load_path = iter.next().map(PathBuf::from),
            "--write" => write = true,
            _ => {
                eprintln!("unknown argument `{argument}`\n{USAGE}");
                return ExitCode::FAILURE;
            }
        }
    }
    let Some(load_path) = load_path else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    match views(&load_path, write) {
        Ok(true) => {
            println!("plugin views: every user crate's resource view is current");
            ExitCode::SUCCESS
        }
        Ok(false) if write => {
            println!("plugin views: written to {}", load_path.display());
            ExitCode::SUCCESS
        }
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("plugin views refused: {error}");
            ExitCode::FAILURE
        }
    }
}
