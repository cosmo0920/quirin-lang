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
    // Card QUIRIN-AUDIT-CONNECT (C15.2): each user crate's declared contract
    // dependencies, read from its module texts with the loader's own clause
    // reader, written before the strict read below (which refuses a stale
    // entry) when `--write` is given.
    let dependencies_current = user_contract_dependencies(load_path, write)?;
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
    Ok(current && dependencies_current)
}

/// Card QUIRIN-AUDIT-CONNECT: compare (and with `write`, set) every user
/// crate record's `contract_dependencies` with the links and cross-crate
/// inherit directives its modules state
/// (`qcore_contract::contract_dependency::scan_module`). Card
/// CONNECT-ADOPT-DECLARED-DEPENDENCIES: each entry carries the identity a
/// link never writes, the target crate's version (`since`) and the target
/// declaration's lock digest (`sha256`), read from the load path this one
/// extends or from this one. Returns whether every record was current.
fn user_contract_dependencies(load_path: &std::path::Path, write: bool) -> Result<bool, String> {
    let text = fs::read_to_string(load_path)
        .map_err(|error| format!("{}: {error}", load_path.display()))?;
    let mut file: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    if file.get("extends").is_none() {
        return Ok(true);
    }
    let base = load_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();
    // Namespace to (crate, crate root, version), over this load path and the
    // one it extends.
    let mut owners: std::collections::BTreeMap<String, (String, std::path::PathBuf, String)> =
        std::collections::BTreeMap::new();
    let mut collect = |manifest: &Value, base: &std::path::Path| {
        for record in manifest["crates"].as_array().into_iter().flatten() {
            let contracts = base.join(record["contracts"].as_str().unwrap_or_default());
            owners
                .entry(record["namespace"].as_str().unwrap_or_default().to_owned())
                .or_insert((
                    record["name"].as_str().unwrap_or_default().to_owned(),
                    contracts
                        .parent()
                        .map(std::path::Path::to_path_buf)
                        .unwrap_or_default(),
                    record["version"].as_str().unwrap_or_default().to_owned(),
                ));
        }
    };
    collect(&file, &base);
    if let Some(extends) = file["extends"].as_str() {
        let shipped_path = base.join(extends);
        let shipped: Value = serde_json::from_str(
            &fs::read_to_string(&shipped_path)
                .map_err(|error| format!("{}: {error}", shipped_path.display()))?,
        )
        .map_err(|error| error.to_string())?;
        let shipped_base = shipped_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        collect(&shipped, &shipped_base);
    }
    let mut current = true;
    for record in file["crates"].as_array_mut().into_iter().flatten() {
        let name = record["name"].as_str().unwrap_or_default().to_owned();
        let own_namespace = record["namespace"].as_str().unwrap_or_default().to_owned();
        let contracts = base.join(record["contracts"].as_str().unwrap_or_default());
        let mut scanned = Vec::new();
        for module in record["modules"].as_array().into_iter().flatten() {
            let module = module.as_str().unwrap_or_default();
            let path = contracts.join(module);
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            for (function, written) in qcore_contract::contract_dependency::scan_module(&text)
                .map_err(|error| format!("{name}:{module}: {error}"))?
            {
                if written.target_namespace == own_namespace
                    && written.form == qcore_schemas::connect::DependencyForm::Inherit
                {
                    continue;
                }
                let refuse = |message: String| {
                    format!(
                        "{name}:{module}: {}",
                        qcore_contract::contract_dependency::refusal(
                            qcore_contract::contract_dependency::LinkageCode::Ql007,
                            message
                        )
                    )
                };
                let Some((target, root, version)) = owners.get(&written.target_namespace) else {
                    return Err(refuse(format!(
                        "`{}::{}`: no load-path crate derives the namespace",
                        written.target_namespace, written.target_function
                    )));
                };
                let mut dependency = written.of_target(&name, &function, module, target);
                dependency.since = Some(
                    qcore_schemas::library_version::LibraryVersion::parse(version)
                        .map_err(|error| refuse(format!("{target}: {error}")))?,
                );
                // A link names a reviewed declaration (QL007 otherwise); an
                // inherit directive whose parent no lock records carries its
                // version alone and resolves `Pending` (an unrecorded digest
                // matches nothing).
                dependency.sha256 = lock_digest(root, &written.target_function);
                if dependency.sha256.is_none()
                    && written.form == qcore_schemas::connect::DependencyForm::Link
                {
                    return Err(refuse(format!(
                        "`{}::{}`: {target}'s lock records no such declaration",
                        written.target_namespace, written.target_function
                    )));
                }
                scanned.push(serde_json::to_value(dependency).map_err(|error| error.to_string())?);
            }
        }
        let declared = record.get("contract_dependencies").cloned();
        let expected = (!scanned.is_empty()).then_some(Value::Array(scanned));
        if declared != expected {
            current = false;
            println!("{name}: contract_dependencies declared {declared:?} probed {expected:?}");
            if write && let Some(object) = record.as_object_mut() {
                match expected {
                    Some(value) => {
                        object.insert("contract_dependencies".into(), value);
                    }
                    None => {
                        object.remove("contract_dependencies");
                    }
                }
            }
        }
    }
    if write && !current {
        let mut bytes = serde_json::to_vec_pretty(&file).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        fs::write(load_path, bytes).map_err(|error| format!("{}: {error}", load_path.display()))?;
    }
    Ok(current)
}

/// The reviewed digest a crate's `contracts.lock` records for `function`.
fn lock_digest(root: &std::path::Path, function: &str) -> Option<String> {
    let lock: Value =
        serde_json::from_str(&fs::read_to_string(root.join("contracts.lock")).ok()?).ok()?;
    ["pinned", "flexible", "tip"]
        .iter()
        .flat_map(|kind| lock[*kind].as_array().into_iter().flatten())
        .find(|entry| entry["function"] == function)
        .and_then(|entry| entry["contract_sha256"].as_str().map(str::to_owned))
}

/// Entry point of `quirin-audit plugin`.
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
