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

//! What a measurement measures: one or more **routes**, each the inputs of one
//! evaluation profile.
//!
//! Two readers produce the same [`Route`]:
//!
//! * [`read_manifest`], the explicit inputs manifest (`--inputs FILE`, schema
//!   [`INPUTS_SCHEMA`]): a compile database (or the pinned corpus's inputs
//!   root and databases) and a unit list (the pinned `audit.json` selection or
//!   an explicit `--selection` file) per route.
//! * [`read_profile`], the **adapter** to card EVALUATION-PROFILES' record
//!   `contract/evaluations/<name>/evaluation.json` (schema
//!   [`PROFILE_SCHEMA`], specified by `scripts/evaluation_profiles.py`). It is
//!   the one place that format is read: every untracked input with a digest is
//!   verified before a run, and every one without (a prepared directory) must
//!   exist.
//!
//! Paths have three bases, named per field: the **repository** the tool runs
//! for (`--repository`, the link map), the **inputs base** holding untracked
//! prepared inputs (`--inputs-base`, default the repository: `build/cycle19`,
//! `private/...` preparations), and each **side's source tree** (selections,
//! source-set profiles, the runner script), which may differ per side.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::util::{is_sha256, read_json};

/// The explicit inputs manifest's schema.
pub const INPUTS_SCHEMA: &str = "quirin-measure-inputs/1";

/// EVALUATION-PROFILES' evaluation-profile schema.
pub const PROFILE_SCHEMA: &str = "quirin-evaluation-profile/1";

/// A file with an optional expected SHA-256 (verified when given).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedFile {
    pub path: String,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// How a route is run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum RouteKind {
    /// `python3 <side>/scripts/run_pinned_audit.py OUT SOURCE_COMMIT
    /// --inputs-root ROOT --jobs N` (the pinned eight-home corpus).
    PinnedCorpus {
        source_commit: String,
        /// Relative to the inputs base.
        inputs_root: String,
        /// Relative to the inputs root.
        compile_databases: Vec<PinnedFile>,
        /// Relative to the inputs root: the trees a re-spelled mirror copies.
        mirror_trees: Vec<String>,
    },
    /// `<binary> current --checkout C --profile-directory P
    /// --compile-database D --selection S --jobs N --report OUT/...`.
    ConfiguredSelection {
        /// Relative to the inputs base.
        checkout: String,
        /// Relative to the inputs base.
        compile_database: PinnedFile,
        /// Relative to the side's source tree: the unit list.
        selection: String,
        /// Relative to the side's source tree.
        profile_directory: String,
    },
}

/// One route: one evaluation profile's inputs. (`flatten` cannot be combined
/// with serde's `deny_unknown_fields`, so [`read_manifest`] checks the keys.)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Route {
    /// The label used for the route's output directory and file prefixes
    /// (`corpus`, `system`, `slice`, as the copied drivers spelled them).
    pub name: String,
    /// The evaluation profile it measures: a published baseline's
    /// subdirectory.
    pub profile: String,
    #[serde(flatten)]
    pub kind: RouteKind,
    /// The cross-unit link map, relative to the repository.
    pub link_map: PinnedFile,
    /// `QCORE_CLANG` for the route (the pinned runner overrides it).
    pub clang: String,
    /// Tracked inputs read from each side's source tree; their digests are
    /// recorded per side and compared.
    #[serde(default)]
    pub tracked: Vec<String>,
    /// Untracked prepared inputs without a digest (directories), relative to
    /// the inputs base: each must exist before anything runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_inputs: Vec<String>,
}

impl Route {
    /// The route's canonical JSON, whose SHA-256 every run and baseline
    /// records: two runs measured the same inputs only when it is equal.
    #[must_use]
    pub fn digest(&self) -> String {
        let value = serde_json::to_value(self).expect("a route is plain data");
        qcore_schemas::sha256_hex(value.to_string().as_bytes())
    }

    /// The report path, relative to the route directory.
    #[must_use]
    pub fn report(&self) -> &'static str {
        match self.kind {
            RouteKind::PinnedCorpus { .. } => "corpus/resource-diagnostics.json",
            RouteKind::ConfiguredSelection { .. } => "resource-diagnostics.json",
        }
    }
}

/// The explicit inputs manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: String,
    #[serde(default)]
    pub description: String,
    pub routes: Vec<Route>,
}

/// Check a set of routes: names and profiles distinct and spelled plainly,
/// digests well formed, paths relative.
///
/// # Errors
///
/// The first violation.
pub fn validate(routes: &[Route]) -> Result<(), String> {
    if routes.is_empty() {
        return Err("no route to measure".into());
    }
    let mut names = BTreeSet::new();
    let mut profiles = BTreeSet::new();
    let plain = |text: &str| {
        !text.is_empty()
            && text
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    };
    for route in routes {
        if !plain(&route.name) || !names.insert(route.name.as_str()) {
            return Err(format!(
                "route name `{}` is not plain or is repeated",
                route.name
            ));
        }
        if !plain(&route.profile) || !profiles.insert(route.profile.as_str()) {
            return Err(format!(
                "route `{}`: profile `{}` is not plain or is repeated",
                route.name, route.profile
            ));
        }
        let mut pinned = vec![&route.link_map];
        let mut relative: Vec<&str> = route
            .tracked
            .iter()
            .chain(&route.required_inputs)
            .map(String::as_str)
            .collect();
        match &route.kind {
            RouteKind::PinnedCorpus {
                inputs_root,
                compile_databases,
                mirror_trees,
                source_commit,
            } => {
                if source_commit.is_empty() || compile_databases.is_empty() {
                    return Err(format!(
                        "route `{}`: a pinned corpus needs a source commit and its compile databases",
                        route.name
                    ));
                }
                pinned.extend(compile_databases);
                relative.push(inputs_root);
                relative.extend(mirror_trees.iter().map(String::as_str));
            }
            RouteKind::ConfiguredSelection {
                checkout,
                compile_database,
                selection,
                profile_directory,
            } => {
                pinned.push(compile_database);
                relative.extend([checkout.as_str(), selection, profile_directory]);
            }
        }
        for file in &pinned {
            relative.push(&file.path);
            if let Some(digest) = &file.sha256
                && !is_sha256(digest)
            {
                return Err(format!(
                    "route `{}`: `{}` has a malformed sha256",
                    route.name, file.path
                ));
            }
        }
        for path in relative {
            // A root or a drive prefix is refused as well as `is_absolute`:
            // on Windows `/etc/passwd` and `C:x` are not absolute, yet
            // neither is relative to the repository.
            if path.is_empty()
                || Path::new(path).is_absolute()
                || Path::new(path).components().any(|part| {
                    matches!(
                        part,
                        Component::Prefix(_) | Component::RootDir | Component::ParentDir
                    )
                })
            {
                return Err(format!(
                    "route `{}`: `{path}` must be a relative path without `..`",
                    route.name
                ));
            }
        }
    }
    Ok(())
}

/// Read an explicit inputs manifest.
///
/// # Errors
///
/// An unreadable or malformed manifest, another schema, an invalid route.
pub fn read_manifest(path: &Path) -> Result<Vec<Route>, String> {
    let value = read_json(path)?;
    for route in value["routes"].as_array().into_iter().flatten() {
        let allowed: &[&str] = match route["kind"].as_str() {
            Some("pinned-corpus") => &[
                "source_commit",
                "inputs_root",
                "compile_databases",
                "mirror_trees",
            ],
            Some("configured-selection") => &[
                "checkout",
                "compile_database",
                "selection",
                "profile_directory",
            ],
            _ => &[],
        };
        let common = [
            "name",
            "profile",
            "kind",
            "link_map",
            "clang",
            "tracked",
            "required_inputs",
        ];
        for key in route
            .as_object()
            .into_iter()
            .flat_map(|object| object.keys())
        {
            if !common.contains(&key.as_str()) && !allowed.contains(&key.as_str()) {
                return Err(format!("{}: unknown route field `{key}`", path.display()));
            }
        }
    }
    let manifest: Manifest =
        serde_json::from_value(value).map_err(|error| format!("{}: {error}", path.display()))?;
    if manifest.schema != INPUTS_SCHEMA {
        return Err(format!(
            "{}: schema `{}` is not `{INPUTS_SCHEMA}`",
            path.display(),
            manifest.schema
        ));
    }
    validate(&manifest.routes)?;
    Ok(manifest.routes)
}

/// The route label the copied drivers used for a live profile, else the
/// profile's own name.
#[must_use]
pub fn route_label(profile: &str) -> String {
    match profile {
        "eight-home" => "corpus".into(),
        "fluent-bit-system" => "system".into(),
        "fluent-bit-tail-pause" => "slice".into(),
        other => other.into(),
    }
}

/// Where a profile's record lies: a path given directly, else
/// `<repository>/contract/evaluations/<name>/evaluation.json` (card
/// EVALUATION-PROFILES; `scripts/evaluation_profiles.py` is the schema's
/// authority).
#[must_use]
pub fn profile_record(repository: &Path, name: &str) -> PathBuf {
    if name.ends_with(".json") || name.contains('/') {
        PathBuf::from(name)
    } else {
        repository
            .join("contract/evaluations")
            .join(name)
            .join("evaluation.json")
    }
}

fn entries<'a>(record: &'a Value, list: &str) -> Vec<&'a Value> {
    record["inputs"][list]
        .as_array()
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn by_role<'a>(record: &'a Value, list: &str, role: &str) -> Result<&'a Value, String> {
    let matches: Vec<_> = entries(record, list)
        .into_iter()
        .filter(|entry| {
            entry["role"]
                .as_str()
                .is_some_and(|text| text.contains(role))
        })
        .collect();
    match matches.as_slice() {
        [only] => Ok(only),
        _ => Err(format!(
            "the profile record names {} `inputs.{list}` entries whose role contains `{role}`; \
             the adapter needs exactly one",
            matches.len()
        )),
    }
}

fn path_of(entry: &Value) -> Result<String, String> {
    entry["path"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "a profile input entry without a `path`".into())
}

fn pinned_of(entry: &Value) -> Result<PinnedFile, String> {
    Ok(PinnedFile {
        path: path_of(entry)?,
        sha256: entry["sha256"].as_str().map(str::to_owned),
    })
}

fn strip(path: &str, prefix: &str) -> Result<String, String> {
    Path::new(path)
        .strip_prefix(prefix)
        .map(|rest| rest.display().to_string())
        .map_err(|_| format!("`{path}` does not lie under the inputs root `{prefix}`"))
}

/// The common directory prefix of relative paths.
fn common_directory(paths: &[String]) -> String {
    let split: Vec<Vec<&str>> = paths.iter().map(|path| path.split('/').collect()).collect();
    let Some(first) = split.first() else {
        return String::new();
    };
    let mut length = first.len().saturating_sub(1);
    for parts in &split[1..] {
        length = length.min(parts.len().saturating_sub(1));
        length = (0..length)
            .take_while(|&index| parts[index] == first[index])
            .count();
    }
    first[..length].join("/")
}

/// The adapter: read EVALUATION-PROFILES' `evaluation.json` record into
/// a route. A frozen profile is refused: its figures describe the analyzer at
/// its frozen revision and are never re-measured (plan seam A).
///
/// # Errors
///
/// An unreadable record, another schema, a frozen policy, an unknown route
/// kind, or inputs the adapter cannot place.
pub fn read_profile(path: &Path) -> Result<Route, String> {
    let record = read_json(path)?;
    let at = |error: String| format!("{}: {error}", path.display());
    if record["schema"].as_str() != Some(PROFILE_SCHEMA) {
        return Err(at(format!("schema is not `{PROFILE_SCHEMA}`")));
    }
    let name = record["name"]
        .as_str()
        .ok_or_else(|| at("no profile `name`".into()))?
        .to_owned();
    match record["policy"]["kind"].as_str() {
        Some("live") => {}
        Some("frozen") => {
            return Err(at(format!(
                "profile `{name}` is frozen: its figures describe the analyzer at its frozen \
                 revision and are never re-measured"
            )));
        }
        other => return Err(at(format!("unknown policy {other:?}"))),
    }
    let tracked: Vec<String> = entries(&record, "tracked")
        .into_iter()
        .map(path_of)
        .collect::<Result<_, _>>()
        .map_err(at)?;
    let link = by_role(&record, "tracked", "link map").map_err(at)?;
    let link_map = pinned_of(link).map_err(at)?;
    let clang = "/usr/lib/llvm-21/bin/clang".to_owned();
    let kind = match record["route"]["kind"].as_str() {
        Some("pinned-corpus") => {
            let untracked: Vec<String> = entries(&record, "untracked")
                .into_iter()
                .map(path_of)
                .collect::<Result<_, _>>()
                .map_err(at)?;
            let root = common_directory(&untracked);
            let databases = [
                by_role(&record, "untracked", "Fluent Bit compile database"),
                by_role(&record, "untracked", "Monkey compile database"),
            ];
            let mut compile_databases = Vec::new();
            for database in databases {
                let mut file = pinned_of(database.map_err(at)?).map_err(at)?;
                file.path = strip(&file.path, &root).map_err(at)?;
                compile_databases.push(file);
            }
            let mut mirror_trees = BTreeSet::new();
            for path in &untracked {
                let relative = strip(path, &root).map_err(at)?;
                let parts: Vec<&str> = relative.split('/').take(2).collect();
                mirror_trees.insert(parts.join("/"));
            }
            let revision = record["source"]["revision"]
                .as_str()
                .ok_or_else(|| at("no `source.revision`".into()))?;
            RouteKind::PinnedCorpus {
                source_commit: revision.chars().take(8).collect(),
                inputs_root: root,
                compile_databases,
                mirror_trees: mirror_trees.into_iter().collect(),
            }
        }
        Some("configured-selection") => {
            let selection =
                path_of(by_role(&record, "tracked", "unit selection").map_err(at)?).map_err(at)?;
            let profile_json = tracked
                .iter()
                .find(|path| path.ends_with("/profile.json"))
                .ok_or_else(|| at("no tracked source-set `profile.json`".into()))?;
            RouteKind::ConfiguredSelection {
                checkout: path_of(
                    by_role(&record, "untracked", "prepared Fluent Bit checkout").map_err(at)?,
                )
                .map_err(at)?,
                compile_database: pinned_of(
                    by_role(&record, "untracked", "compile database").map_err(at)?,
                )
                .map_err(at)?,
                selection,
                profile_directory: Path::new(profile_json)
                    .parent()
                    .map(|parent| parent.display().to_string())
                    .unwrap_or_default(),
            }
        }
        other => return Err(at(format!("route kind {other:?} has no adapter"))),
    };
    let required_inputs = entries(&record, "untracked")
        .into_iter()
        .filter(|entry| entry["sha256"].is_null())
        .map(path_of)
        .collect::<Result<_, _>>()
        .map_err(at)?;
    let route = Route {
        name: route_label(&name),
        profile: name,
        kind,
        link_map,
        clang,
        tracked,
        required_inputs,
    };
    validate(std::slice::from_ref(&route))?;
    Ok(route)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_common_directory_is_a_whole_component_prefix() {
        assert_eq!(
            common_directory(&[
                "build/cycle19/a/x.json".into(),
                "build/cycle19/b/y".into(),
                "build/cycle19/c".into()
            ]),
            "build/cycle19"
        );
        assert_eq!(common_directory(&["a/bc/x".into(), "a/b/y".into()]), "a");
    }

    #[test]
    fn invalid_routes_are_refused() {
        let route = Route {
            name: "corpus".into(),
            profile: "eight-home".into(),
            kind: RouteKind::ConfiguredSelection {
                checkout: "c".into(),
                compile_database: PinnedFile {
                    path: "d".into(),
                    sha256: None,
                },
                selection: "s".into(),
                profile_directory: "p".into(),
            },
            link_map: PinnedFile {
                path: "l".into(),
                sha256: Some("x".into()),
            },
            clang: "clang".into(),
            tracked: Vec::new(),
            required_inputs: Vec::new(),
        };
        assert!(
            validate(std::slice::from_ref(&route))
                .unwrap_err()
                .contains("malformed")
        );
        let mut absolute = route.clone();
        absolute.link_map.sha256 = None;
        absolute.tracked.push("/etc/passwd".into());
        assert!(validate(&[absolute]).unwrap_err().contains("relative"));
        let mut good = route.clone();
        good.link_map.sha256 = None;
        assert!(
            validate(&[good.clone(), good])
                .unwrap_err()
                .contains("repeated")
        );
        assert!(validate(&[]).is_err());
    }
}
