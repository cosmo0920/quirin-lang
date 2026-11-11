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

//! Canonical inputs: the digest-manifested, re-spelled private mirror.
//!
//! The pinned compile databases spell the checkout path they were prepared
//! under (`/home/cosmo/Documents/GitHub/qcore-c`, since 2026-10-23 a symbolic
//! link to the real checkout). An analyzer built before card
//! AUDIT-CANONICAL-PATHS selects units by comparing path strings, so it
//! matches no unit when the spelled root differs from its canonical location
//! (`PINNED-INPUTS-PATH-AFTER-MIGRATION`). This module replaces the copied
//! `docs/reports/w48-bu-single-leaf-bodies/scripts/mirror_inputs.py` once:
//!
//! * a pinned-corpus route's mirror trees are copied byte for byte (`cp -a`)
//!   under the mirror, at their path relative to the inputs root, and each
//!   compile database is re-spelled so the tree holding it names the mirror
//!   copy and every other spelled path names the canonical location;
//! * a configured route's compile database is re-spelled to the canonical
//!   location at `<mirror>/<route>-compile_commands.json`; its checkout is
//!   read in place.
//!
//! The only rewrite is the textual prefix substitution recorded in
//! `manifest.json`, with the SHA-256 of every original and re-spelled
//! database and a content digest of every copied tree beside its original's.
//! No byte a unit reads changes. Whether a mirror is needed is decided once
//! per measurement, for every side alike ([`Mode`]), because the re-spelled
//! databases enter the digests of the compile records a report binds: sides
//! measured on different spellings are not byte-comparable.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use crate::inputs::{Route, RouteKind};
use crate::util::{differing_prefixes, identity_digest, read_json, sha_file, tree_identity};

/// When to re-spell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// When some compile database spells a directory whose canonical path
    /// differs from its spelling.
    Auto,
    Always,
    Never,
}

impl std::str::FromStr for Mode {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, String> {
        match text {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            other => Err(format!(
                "--mirror takes auto, always or never, not `{other}`"
            )),
        }
    }
}

/// The inputs a route reads, as spelled for this measurement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spelled {
    /// `pinned-corpus`: the inputs root passed to the runner.
    pub inputs_root: Option<PathBuf>,
    /// `configured-selection`: the compile database and the checkout.
    pub compile_database: Option<PathBuf>,
    pub checkout: Option<PathBuf>,
    /// The databases the route reads, with their digests now, for the
    /// before/after identity check.
    pub databases: Vec<PathBuf>,
}

/// The original compile databases of a route (absolute) and the tree each
/// belongs to (pinned corpus only).
fn databases(route: &Route, base: &Path) -> Vec<(PathBuf, Option<String>)> {
    match &route.kind {
        RouteKind::PinnedCorpus {
            inputs_root,
            compile_databases,
            mirror_trees,
            ..
        } => compile_databases
            .iter()
            .map(|file| {
                let tree = mirror_trees
                    .iter()
                    .filter(|tree| Path::new(&file.path).starts_with(tree))
                    .max_by_key(|tree| tree.len())
                    .cloned();
                (base.join(inputs_root).join(&file.path), tree)
            })
            .collect(),
        RouteKind::ConfiguredSelection {
            compile_database, ..
        } => vec![(base.join(&compile_database.path), None)],
    }
}

/// The spelled and canonical prefixes of a compile database: from its first
/// entry's `directory`, canonicalized.
///
/// # Errors
///
/// An unreadable database, no entry, or a directory that does not resolve.
pub fn spelling(database: &Path) -> Result<Option<(PathBuf, PathBuf)>, String> {
    let value = read_json(database)?;
    let directory = value
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry["directory"].as_str())
        .ok_or_else(|| format!("{}: no entry with a `directory`", database.display()))?;
    let canonical = Path::new(directory)
        .canonicalize()
        .map_err(|error| format!("{}: directory {directory}: {error}", database.display()))?;
    Ok(differing_prefixes(Path::new(directory), &canonical))
}

/// Re-spell `text` by the rules, in order; refuse when the spelled prefix
/// survives.
///
/// # Errors
///
/// The spelled prefix remains after every rule.
pub fn respell(text: &str, rules: &[(String, String)], spelled: &str) -> Result<String, String> {
    let mut text = text.to_owned();
    for (old, new) in rules {
        text = text.replace(old.as_str(), new);
    }
    if text.contains(spelled) {
        return Err(format!(
            "the spelled prefix `{spelled}` is left after re-spelling"
        ));
    }
    Ok(text)
}

/// Decide and, when needed, write the mirror for every route; return each
/// route's spelled inputs and the record of what was done.
///
/// # Errors
///
/// An input missing or failing its pinned digest, an unresolvable spelling
/// in `auto`, a copy whose digest differs from its original, or a write.
pub fn prepare(
    routes: &[Route],
    base: &Path,
    mirror: &Path,
    fixed: &Path,
    mode: Mode,
) -> Result<(BTreeMap<String, Spelled>, Value), String> {
    // Every pinned digest first: a stale input refuses the measurement.
    for route in routes {
        for required in &route.required_inputs {
            if !base.join(required).exists() {
                return Err(format!(
                    "route `{}`: the prepared input {} is missing",
                    route.name,
                    base.join(required).display()
                ));
            }
        }
        let pinned = match &route.kind {
            RouteKind::PinnedCorpus {
                inputs_root,
                compile_databases,
                ..
            } => compile_databases
                .iter()
                .map(|file| (base.join(inputs_root).join(&file.path), &file.sha256))
                .collect::<Vec<_>>(),
            RouteKind::ConfiguredSelection {
                compile_database, ..
            } => vec![(base.join(&compile_database.path), &compile_database.sha256)],
        };
        for (path, expected) in pinned {
            let digest = sha_file(&path)?;
            if let Some(expected) = expected
                && &digest != expected
            {
                return Err(format!(
                    "route `{}`: {} has SHA-256 {digest}, not the pinned {expected}",
                    route.name,
                    path.display()
                ));
            }
        }
    }
    let mut prefixes = None;
    for route in routes {
        for (database, _) in databases(route, base) {
            let found = spelling(&database)?;
            if let Some(found) = found {
                match &prefixes {
                    None => prefixes = Some(found),
                    Some(known) if *known == found => {}
                    Some(known) => {
                        return Err(format!(
                            "{} spells prefix {} -> {}, another database {} -> {}; one \
                             measurement re-spells one prefix",
                            database.display(),
                            found.0.display(),
                            found.1.display(),
                            known.0.display(),
                            known.1.display()
                        ));
                    }
                }
            }
        }
    }
    let use_mirror = match mode {
        Mode::Never => false,
        Mode::Always => true,
        Mode::Auto => prefixes.is_some(),
    };
    let mut spelled = BTreeMap::new();
    if !use_mirror {
        for route in routes {
            let entry = match &route.kind {
                RouteKind::PinnedCorpus { inputs_root, .. } => Spelled {
                    inputs_root: Some(base.join(inputs_root)),
                    compile_database: None,
                    checkout: None,
                    databases: databases(route, base)
                        .into_iter()
                        .map(|(path, _)| path)
                        .collect(),
                },
                RouteKind::ConfiguredSelection {
                    checkout,
                    compile_database,
                    ..
                } => Spelled {
                    inputs_root: None,
                    compile_database: Some(base.join(&compile_database.path)),
                    checkout: Some(base.join(checkout)),
                    databases: vec![base.join(&compile_database.path)],
                },
            };
            spelled.insert(route.name.clone(), entry);
        }
        let record = json!({
            "spelling": "original",
            "mode": format!("{mode:?}").to_lowercase(),
            "spelled_prefix": prefixes.as_ref().map(|found| found.0.display().to_string()),
            "canonical_prefix": prefixes.as_ref().map(|found| found.1.display().to_string()),
        });
        return Ok((spelled, record));
    }
    let (old, real) = prefixes
        .map(|(old, real)| (old.display().to_string(), real.display().to_string()))
        .ok_or("--mirror always: no compile database spells a non-canonical prefix")?;
    fs::create_dir(mirror).map_err(|error| format!("{}: {error}", mirror.display()))?;
    let mut trees = BTreeMap::new();
    let mut rewritten = BTreeMap::new();
    for route in routes {
        let entry = match &route.kind {
            RouteKind::PinnedCorpus {
                inputs_root,
                mirror_trees,
                ..
            } => {
                let root = base.join(inputs_root);
                let skip: Vec<String> = databases(route, base)
                    .iter()
                    .filter_map(|(path, _)| path.strip_prefix(&root).ok())
                    .map(|path| path.display().to_string())
                    .collect();
                for tree in mirror_trees {
                    if trees.contains_key(tree) {
                        continue;
                    }
                    let original = root.join(tree);
                    let copy = mirror.join(tree);
                    if let Some(parent) = copy.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|error| format!("{}: {error}", parent.display()))?;
                    }
                    let status = Command::new("cp")
                        .arg("-a")
                        .arg(&original)
                        .arg(&copy)
                        .status()
                        .map_err(|error| format!("cp: {error}"))?;
                    if !status.success() {
                        return Err(format!("cp -a {} failed", original.display()));
                    }
                    let digest = |at: &Path| -> Result<String, String> {
                        let mut identity = tree_identity(at, &[])?;
                        identity.retain(|path, _| {
                            !skip
                                .iter()
                                .any(|database| Path::new(tree).join(path) == Path::new(database))
                        });
                        Ok(identity_digest(&identity))
                    };
                    let (left, right) = (digest(&original)?, digest(&copy)?);
                    if left != right {
                        return Err(format!(
                            "the mirror copy of {tree} has digest {right}, not its original's {left}"
                        ));
                    }
                    trees.insert(
                        tree.clone(),
                        json!({"original": original.display().to_string(),
                               "copy": copy.display().to_string(),
                               "original_digest": left, "copy_digest": right}),
                    );
                }
                let mut paths = Vec::new();
                for (database, tree) in databases(route, base) {
                    let relative = database.strip_prefix(&root).map_err(|_| {
                        format!("{} is outside its inputs root", database.display())
                    })?;
                    let target = mirror.join(relative);
                    let canonical_root = root
                        .canonicalize()
                        .map_err(|error| format!("{}: {error}", root.display()))?;
                    let mut rules = Vec::new();
                    if let Some(tree) = &tree {
                        let under = canonical_root
                            .strip_prefix(&real)
                            .map_err(|_| {
                                format!(
                                    "the inputs root {} is not under the canonical prefix {real}",
                                    canonical_root.display()
                                )
                            })?
                            .join(tree);
                        rules.push((
                            Path::new(&old).join(under).display().to_string(),
                            fixed.join(tree).display().to_string(),
                        ));
                    }
                    rules.push((old.clone(), real.clone()));
                    rewritten.insert(
                        format!("{}:{}", route.name, relative.display()),
                        write_respelled(&database, &target, &rules, &old)?,
                    );
                    paths.push(target);
                }
                Spelled {
                    inputs_root: Some(fixed.to_path_buf()),
                    compile_database: None,
                    checkout: None,
                    databases: paths,
                }
            }
            RouteKind::ConfiguredSelection {
                checkout,
                compile_database,
                ..
            } => {
                let original = base.join(&compile_database.path);
                let file = format!("{}-compile_commands.json", route.name);
                let target = mirror.join(&file);
                let rules = vec![(old.clone(), real.clone())];
                rewritten.insert(
                    route.name.clone(),
                    write_respelled(&original, &target, &rules, &old)?,
                );
                Spelled {
                    inputs_root: None,
                    compile_database: Some(fixed.join(&file)),
                    checkout: Some(base.join(checkout)),
                    databases: vec![target],
                }
            }
        };
        spelled.insert(route.name.clone(), entry);
    }
    let manifest = json!({
        "schema": "quirin-measure-mirror/1",
        "old_prefix": old,
        "real_prefix": real,
        "mirror": mirror.display().to_string(),
        "mirror_as_run": fixed.display().to_string(),
        "trees": trees,
        "databases": rewritten,
    });
    let manifest_path = mirror.join("manifest.json");
    crate::util::write_json(&manifest_path, &manifest)?;
    let record = json!({
        "spelling": "mirror",
        "mode": format!("{mode:?}").to_lowercase(),
        "spelled_prefix": old,
        "canonical_prefix": real,
        "manifest": manifest_path.display().to_string(),
        "manifest_sha256": sha_file(&manifest_path)?,
    });
    Ok((spelled, record))
}

fn write_respelled(
    source: &Path,
    target: &Path,
    rules: &[(String, String)],
    old: &str,
) -> Result<Value, String> {
    let text =
        fs::read_to_string(source).map_err(|error| format!("{}: {error}", source.display()))?;
    let text =
        respell(&text, rules, old).map_err(|error| format!("{}: {error}", source.display()))?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(target, text).map_err(|error| format!("{}: {error}", target.display()))?;
    Ok(json!({
        "original": source.display().to_string(),
        "original_sha256": sha_file(source)?,
        "rewritten": target.display().to_string(),
        "rewritten_sha256": sha_file(target)?,
        "rules": rules.iter().map(|(old, new)| json!([old, new])).collect::<Vec<_>>(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respelling_applies_rules_in_order_and_refuses_leftovers() {
        let rules = vec![
            ("/old/build/cycle19/t".to_owned(), "/mirror/t".to_owned()),
            ("/old".to_owned(), "/real".to_owned()),
        ];
        let text = "/old/build/cycle19/t/a.c /old/build/cycle19/u/b.c";
        assert_eq!(
            respell(text, &rules, "/old").unwrap(),
            "/mirror/t/a.c /real/build/cycle19/u/b.c"
        );
        assert!(respell("/old/x", &rules[..1], "/old").is_err());
    }
}
