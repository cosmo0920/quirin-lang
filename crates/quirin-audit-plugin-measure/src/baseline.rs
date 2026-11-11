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

//! Published baselines: `<root>/<sha>/<profile>/` (plan seam F).
//!
//! **Publish** (`measure publish`) copies one measured side's route directory
//! into the plugin's output, which must be `<root>/<sha>` for the side's own
//! revision, as `<profile>/<route>/`, with the side's `run.json` and a
//! manifest `baseline.json` (schema [`BASELINE_SCHEMA`]): the revision, the
//! profile, the route definition and its digest, the binary digest, the
//! inputs (pinned and as spelled), the contract-bundle digests and the SHA-256
//! of every published file. A failed or unstable route, a side without a
//! revision (a dirty checkout, a binary) and an existing profile directory are
//! refused: a baseline is published once and never rewritten.
//!
//! **Reuse** (`--side NAME=baseline:<sha>`) verifies identity first
//! ([`verify`]): every published file has its recorded digest and nothing is
//! added; the recorded revision is the requested one; the route definition
//! and the inputs as this measurement spells them are the recorded ones; and
//! the contract-bundle digests recomputed from the revision's git tree are
//! the recorded ones. Any difference refuses the side, naming it.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::inputs::Route;
use crate::sides::bundle_of;
use crate::util::{
    identity_digest, is_full_revision, is_sha256, now, read_json, sha_file, tree_files, write_json,
};

/// The manifest's schema.
pub const BASELINE_SCHEMA: &str = "quirin-measure-baseline/1";

/// The manifest's file name.
pub const MANIFEST: &str = "baseline.json";

/// Contract-bundle digests of a revision, read from git: the same formula as
/// [`crate::sides::bundle_digests`] over the files of the revision's tree
/// (regular files only; symbolic links and gitlinks are not bundle files).
///
/// # Errors
///
/// A failing git command or a truncated blob stream.
pub fn git_bundle_digests(
    repository: &Path,
    revision: &str,
) -> Result<BTreeMap<String, String>, String> {
    let listing = crate::util::capture(
        "git",
        &[
            "ls-tree",
            "-r",
            "--full-tree",
            revision,
            "--",
            "providers",
            "contract",
        ],
        Some(repository),
    )?;
    let mut files = Vec::new();
    for line in listing.lines() {
        let Some((meta, path)) = line.split_once('\t') else {
            continue;
        };
        let mut parts = meta.split_whitespace();
        let (mode, kind, object) = (parts.next(), parts.next(), parts.next());
        if kind == Some("blob")
            && matches!(mode, Some("100644" | "100755"))
            && let (Some(object), Some(bundle)) = (object, bundle_of(path))
        {
            let inner = path[bundle.len() + 1..].to_owned();
            files.push((bundle, inner, object.to_owned()));
        }
    }
    let mut child = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(repository)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("git cat-file: {error}"))?;
    let mut stdin = child.stdin.take().ok_or("git cat-file: no stdin")?;
    let objects: Vec<String> = files.iter().map(|(_, _, object)| object.clone()).collect();
    let writer = std::thread::spawn(move || {
        for object in objects {
            if writeln!(stdin, "{object}").is_err() {
                break;
            }
        }
    });
    let mut reader = BufReader::new(child.stdout.take().ok_or("git cat-file: no stdout")?);
    let mut bundles: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (bundle, inner, object) in &files {
        let mut header = String::new();
        reader
            .read_line(&mut header)
            .map_err(|error| format!("git cat-file: {error}"))?;
        let fields: Vec<&str> = header.split_whitespace().collect();
        let size: usize = match fields.as_slice() {
            [name, "blob", size] if name == object => size
                .parse()
                .map_err(|_| format!("git cat-file: bad header `{header}`"))?,
            _ => {
                return Err(format!(
                    "git cat-file: unexpected header `{header}` for {object}"
                ));
            }
        };
        let mut bytes = vec![0; size + 1];
        reader
            .read_exact(&mut bytes)
            .map_err(|error| format!("git cat-file: {error}"))?;
        bytes.pop();
        bundles
            .entry(bundle.clone())
            .or_default()
            .insert(inner.clone(), qcore_schemas::sha256_hex(&bytes));
    }
    let _ = writer.join();
    let _ = child.wait();
    Ok(bundles
        .into_iter()
        .map(|(bundle, identity)| (bundle, identity_digest(&identity)))
        .collect())
}

/// The inputs a measurement records for a route and compares on reuse.
#[must_use]
pub fn inputs_record(
    route: &Route,
    spelling: &Value,
    databases: &BTreeMap<String, String>,
    source_date_epoch: Option<u64>,
) -> Value {
    json!({
        "route_digest": route.digest(),
        "source_date_epoch": source_date_epoch,
        "spelling": spelling["spelling"],
        "spelled_prefix": spelling["spelled_prefix"],
        "canonical_prefix": spelling["canonical_prefix"],
        "databases_as_spelled": databases,
    })
}

/// Publish the routes of a measured side into `output` (`<root>/<sha>`).
///
/// # Errors
///
/// A side without `run.json`, a revision, or a passing route; an output not
/// named for the revision; an existing profile directory; a copy failure.
pub fn publish(side: &Path, output: &Path, only: &[String]) -> Result<Vec<PathBuf>, String> {
    let run = read_json(&side.join("run.json"))?;
    if run["schema"] != json!(crate::RUN_SCHEMA) {
        return Err(format!(
            "{}: not a measure run record",
            side.join("run.json").display()
        ));
    }
    let revision = run["revision"]
        .as_str()
        .filter(|text| is_full_revision(text))
        .ok_or_else(|| {
            format!(
                "side {}: no measured revision (a dirty checkout or a bare binary is not \
                 published)",
                side.display()
            )
        })?;
    if output.file_name().and_then(|name| name.to_str()) != Some(revision) {
        return Err(format!(
            "publish writes <baselines>/<sha>: the output {} is not named for the side's revision \
             {revision}",
            output.display()
        ));
    }
    let routes = run["routes"].as_object().ok_or("run.json has no routes")?;
    let mut written = Vec::new();
    for (name, route_run) in routes {
        if !only.is_empty() && !only.contains(name) {
            continue;
        }
        let profile = route_run["profile"]
            .as_str()
            .ok_or("a route without a profile")?;
        if route_run["exit"] != json!(0) || route_run["stable"] != json!(true) {
            return Err(format!(
                "route {name}: exit {} stable {}; a failed or unstable run is never published",
                route_run["exit"], route_run["stable"]
            ));
        }
        let target = output.join(profile);
        if target.exists() {
            return Err(format!(
                "{} exists: a baseline is published once and never rewritten",
                target.display()
            ));
        }
        let source = side.join(name);
        fs::create_dir_all(&target).map_err(|error| format!("{}: {error}", target.display()))?;
        let status = Command::new("cp")
            .arg("-r")
            .arg(&source)
            .arg(target.join(name))
            .status()
            .map_err(|error| format!("cp: {error}"))?;
        if !status.success() {
            return Err(format!("copying {} failed", source.display()));
        }
        let mut side_record = run.clone();
        side_record["routes"] = json!({name.clone(): route_run});
        write_json(&target.join("run.json"), &side_record)?;
        let mut files = BTreeMap::new();
        for relative in tree_files(&target, &[])? {
            let spelled = relative.display().to_string();
            files.insert(spelled, sha_file(&target.join(&relative))?);
        }
        let manifest = json!({
            "schema": BASELINE_SCHEMA,
            "revision": revision,
            "profile": profile,
            "route": name,
            "route_definition": route_run["route_definition"],
            "inputs": route_run["inputs"],
            "binary_sha256": run["binary"]["sha256"],
            "build_commit": run["build_commit"],
            "bundles": run["bundles"],
            "files": files,
            "published": now(),
            "tool": crate::tool(),
        });
        write_json(&target.join(MANIFEST), &manifest)?;
        written.push(target);
    }
    if written.is_empty() {
        return Err("no route was published".into());
    }
    Ok(written)
}

/// Verify a published baseline before reuse; return its directory (a side
/// directory holding `<route>/`).
///
/// # Errors
///
/// Every refusal names what differs.
pub fn verify(
    repository: &Path,
    root: &Path,
    revision: &str,
    route: &Route,
    current_inputs: &Value,
) -> Result<(PathBuf, Value), String> {
    let directory = root.join(revision).join(&route.profile);
    let manifest = read_json(&directory.join(MANIFEST))
        .map_err(|error| format!("baseline {revision}/{}: {error}", route.profile))?;
    let refuse = |why: String| Err(format!("baseline {revision}/{}: {why}", route.profile));
    if manifest["schema"] != json!(BASELINE_SCHEMA) {
        return refuse(format!("schema is not {BASELINE_SCHEMA}"));
    }
    if manifest["revision"] != json!(revision) || manifest["profile"] != json!(route.profile) {
        return refuse(format!(
            "records revision {} profile {}",
            manifest["revision"], manifest["profile"]
        ));
    }
    if manifest["route"] != json!(route.name) {
        return refuse(format!(
            "records route {}, not {}",
            manifest["route"], route.name
        ));
    }
    let recorded: BTreeMap<String, String> = manifest["files"]
        .as_object()
        .ok_or("no file list")?
        .iter()
        .map(|(path, digest)| (path.clone(), digest.as_str().unwrap_or_default().to_owned()))
        .collect();
    let mut present = BTreeMap::new();
    for relative in tree_files(&directory, &[])? {
        let spelled = relative.display().to_string();
        if spelled == MANIFEST {
            continue;
        }
        present.insert(spelled, sha_file(&directory.join(&relative))?);
    }
    if present != recorded {
        let differing: Vec<&String> = recorded
            .keys()
            .chain(present.keys())
            .filter(|path| recorded.get(*path) != present.get(*path))
            .collect();
        return refuse(format!(
            "published files differ from the manifest: {differing:?}"
        ));
    }
    if !manifest["binary_sha256"].as_str().is_some_and(is_sha256) {
        return refuse("no binary digest".into());
    }
    for field in [
        "route_digest",
        "spelling",
        "databases_as_spelled",
        "source_date_epoch",
    ] {
        if manifest["inputs"][field] != current_inputs[field] {
            return refuse(format!(
                "inputs.{field} is {}, this measurement's is {}",
                manifest["inputs"][field], current_inputs[field]
            ));
        }
    }
    let bundles = git_bundle_digests(repository, revision)?;
    if manifest["bundles"] != json!(bundles) {
        return refuse(format!(
            "contract-bundle digests differ from the revision's git tree: recorded {}, tree {}",
            manifest["bundles"],
            json!(bundles)
        ));
    }
    Ok((directory, manifest))
}
