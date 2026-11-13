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

//! Card **SELECTION-ROUTES-SPLIT-CALLER-AND-HOME**: the link-resolution side.
//!
//! A side named by `--link-resolution NAME` runs every route under the
//! route's rule 40 link-resolution artifact
//! (`QCORE_CROSS_UNIT_LINK_RESOLUTION`) in place of the stated map
//! (`QCORE_CROSS_UNIT_LINK_MAP`, which such a side never sets). It is
//! fail-closed at three points, and never falls back to the map:
//!
//! 1. before anything runs ([`check_artifact`]): every route declares an
//!    artifact with its SHA-256; the file is there, readable, of that digest
//!    and of the artifact schema; and the compile database it binds (rule
//!    40's condition 1) is the one the route reads, as spelled (a re-spelled
//!    mirror changes it, so the run would hold no import);
//! 2. after the side is built ([`advertises`]): its analyzer names the
//!    variable, so an analyzer without rule 40 cannot run the route and
//!    silently import nothing;
//! 3. after each route ([`check_run`]): the analyzer printed that it derived
//!    the resolution from that artifact (its digest), and the report records
//!    no whole-run refusal (rule 40's conditions 1 and 2: a bound digest that
//!    changed). Otherwise the route fails.
//!
//! Which homes rule 40 then admits or refuses per home and per caller is the
//! analyzer's decision, recorded in its report; the tool decides nothing.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::inputs::{Route, RouteKind};
use crate::util::{read_json, sha_file};

/// The variable a rule 40 analyzer reads.
pub const RESOLUTION_VARIABLE: &str = "QCORE_CROSS_UNIT_LINK_RESOLUTION";

/// The artifact schema `prepare --link` writes.
pub const ARTIFACT_SCHEMA: &str = "quirin-link-resolution/1";

/// The line the `current` route prints when it derived a resolution.
pub const DERIVED_LINE: &str = "cross-unit link resolution derived (rule 40) from ";

/// The line the `current` route prints for a refused import phase.
pub const REFUSED_LINE: &str = "cross-unit import phase refused: ";

/// A route's checked artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub path: PathBuf,
    pub sha256: String,
}

impl Artifact {
    #[must_use]
    pub fn record(&self) -> Value {
        json!({"path": self.path.display().to_string(), "sha256": self.sha256})
    }
}

/// Check a route's artifact for a linked side before anything runs.
/// `databases` are the compile databases the route reads, as spelled.
///
/// # Errors
///
/// No declared artifact, a pinned route, a missing, unreadable or
/// differently digested file, another schema, or a bound database that is
/// not the one the route reads.
pub fn check_artifact(
    route: &Route,
    inputs_base: &Path,
    databases: &[PathBuf],
) -> Result<Artifact, String> {
    let at = |reason: String| {
        format!(
            "route `{}`: {reason}; a link-resolution side never falls back to the stated map",
            route.name
        )
    };
    if !matches!(route.kind, RouteKind::ConfiguredSelection { .. }) {
        return Err(at(
            "the pinned route refuses a link-resolution artifact (rule 40)".into(),
        ));
    }
    let declared = route
        .link_resolution
        .as_ref()
        .ok_or_else(|| at("the inputs declare no `link_resolution` artifact".into()))?;
    let expected = declared
        .sha256
        .as_deref()
        .ok_or_else(|| at("the link-resolution artifact has no sha256".into()))?;
    let path = inputs_base.join(&declared.path);
    let digest = sha_file(&path).map_err(|error| at(format!("artifact unreadable: {error}")))?;
    if digest != expected {
        return Err(at(format!(
            "artifact {} has sha256 {digest}, the inputs declare {expected}",
            path.display()
        )));
    }
    let artifact = read_json(&path).map_err(|error| at(format!("artifact unreadable: {error}")))?;
    if artifact["schema"].as_str() != Some(ARTIFACT_SCHEMA) {
        return Err(at(format!(
            "artifact {} is not `{ARTIFACT_SCHEMA}`",
            path.display()
        )));
    }
    let bound = artifact["compile_database"]["sha256"]
        .as_str()
        .ok_or_else(|| at("the artifact binds no compile database digest".into()))?;
    if databases.is_empty() {
        return Err(at("the route reads no compile database".into()));
    }
    for database in databases {
        let read = sha_file(database).map_err(at)?;
        if read != bound {
            return Err(at(format!(
                "the artifact binds compile database sha256 {bound}, the route reads {} \
                 (sha256 {read}); rule 40's condition 1 would refuse every import",
                database.display()
            )));
        }
    }
    Ok(Artifact {
        path,
        sha256: digest,
    })
}

/// Whether an analyzer binary names [`RESOLUTION_VARIABLE`].
///
/// # Errors
///
/// An unreadable binary.
pub fn advertises(binary: &Path) -> Result<bool, String> {
    let bytes = std::fs::read(binary).map_err(|error| format!("{}: {error}", binary.display()))?;
    let needle = RESOLUTION_VARIABLE.as_bytes();
    Ok(bytes.windows(needle.len()).any(|window| window == needle))
}

/// Check a linked route that exited 0: `console` is its standard output and
/// `report` its report. Returns what the run recorded.
///
/// # Errors
///
/// No derived line naming the artifact's digest, or a report recording a
/// whole-run refusal or other evidence.
pub fn check_run(console: &str, report: &Value, artifact: &Artifact) -> Result<Value, String> {
    let digest = format!("(sha256 {})", artifact.sha256);
    let derived = console
        .lines()
        .find(|line| line.starts_with(DERIVED_LINE) && line.contains(&digest));
    let refusals: Vec<&str> = console
        .lines()
        .filter(|line| line.starts_with(REFUSED_LINE))
        .collect();
    let Some(derived) = derived else {
        return Err(format!(
            "the analyzer did not derive its link resolution from the artifact (sha256 {}){}",
            artifact.sha256,
            refusals
                .first()
                .map(|line| format!(": {line}"))
                .unwrap_or_default()
        ));
    };
    let assumption = &report["assumptions"]["cross_unit_link_resolution"];
    if !assumption.is_null() {
        if !assumption["refused"].is_null() || assumption["evidence"] != json!("derived") {
            return Err(format!(
                "the report records the link resolution as {assumption}, not derived"
            ));
        }
        if assumption["derived_from"]["artifact_sha256"] != json!(artifact.sha256) {
            return Err(format!(
                "the report's derived resolution names artifact {}, not {}",
                assumption["derived_from"]["artifact_sha256"], artifact.sha256
            ));
        }
    }
    Ok(json!({
        "derived": derived,
        "import_phase_refusals": refusals,
        "imported": assumption["imported"].as_array().map(Vec::len),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact() -> Artifact {
        Artifact {
            path: PathBuf::from("/p/link-resolution.json"),
            sha256: "a".repeat(64),
        }
    }

    #[test]
    fn a_run_is_accepted_only_when_it_derived_from_the_artifact() {
        let line = format!(
            "{DERIVED_LINE}/p/link-resolution.json (sha256 {}): 3 home(s); refused {{}}",
            "a".repeat(64)
        );
        let derived = json!({"assumptions": {"cross_unit_link_resolution": {
            "evidence": "derived", "derived_from": {"artifact_sha256": "a".repeat(64)},
            "imported": [{"function": "f"}]}}});
        let checked = check_run(&line, &derived, &artifact()).unwrap();
        assert_eq!(checked["imported"], 1);
        // No assumption: no home was imported (none exported, or the census
        // refused the import phase), which the console says.
        let census = format!("{line}\n{REFUSED_LINE}incomplete definition census");
        let checked = check_run(&census, &json!({}), &artifact()).unwrap();
        assert_eq!(
            checked["import_phase_refusals"].as_array().unwrap().len(),
            1
        );
        // Another artifact's line, no line, or a whole-run refusal: refused.
        let other = line.replace(&"a".repeat(64), &"b".repeat(64));
        assert!(check_run(&other, &json!({}), &artifact()).is_err());
        let whole = format!("{REFUSED_LINE}compile database digest differs: x");
        let error = check_run(&whole, &json!({}), &artifact()).unwrap_err();
        assert!(error.contains("compile database digest differs"), "{error}");
        let refused = json!({"assumptions": {"cross_unit_link_resolution": {
            "evidence": "derived", "refused": "x"}}});
        assert!(check_run(&line, &refused, &artifact()).is_err());
        let stated = json!({"assumptions": {"cross_unit_link_resolution": {
            "premise": "stated"}}});
        assert!(check_run(&line, &stated, &artifact()).is_err());
        let elsewhere = json!({"assumptions": {"cross_unit_link_resolution": {
            "evidence": "derived", "derived_from": {"artifact_sha256": "c".repeat(64)}}}});
        assert!(check_run(&line, &elsewhere, &artifact()).is_err());
    }
}
