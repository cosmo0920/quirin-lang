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

//! Card **MEASURE-CONTRACT-ONLY-SIDE-WIRING**: the tip side of a
//! contract-only diff measured by the base side's analyzer, resuming the base
//! side's pinned group checkpoints (`--reuse-contract-only`).
//!
//! Card MEASURE-LOOKUP-RECORD-REUSE built the acceptance
//! (`quirin-audit pinned --accept-base-analyzer BASE --repository DIR`): the
//! base binary measures the tip's contract data, so every identity field is
//! the base side's and a base-side group checkpoint can stand for a tip-side
//! group whose identity is otherwise equal. This module wires it into a
//! measurement, fail-closed:
//!
//! * [`candidate`]: only the first comparison, only two `rev:` sides, only
//!   with a pinned corpus route, without a rule switch and without an explicit
//!   build commit; anything else measures separate sides as before.
//! * [`prepare_tip`]: the tip is checked out (a shared clone, so the run can
//!   check its `HEAD` and contract cleanliness), its diff from the base is
//!   classified by the **base side's binary** (`pinned --classify-only`, the
//!   classifier the tip run is then checked by), and the base binary is
//!   copied into the tip tree in place of a build. A refusal of any step
//!   builds the tip as a separate side, exactly as without the option.
//! * The base side's pinned route seals its groups into
//!   `<output>/checkpoints/<route>`; the tip side's waits for it and resumes
//!   from that directory under the acceptance
//!   ([`crate::sides::Checkpoint`]). Both run from the fixed working
//!   directory every snapshot route runs from
//!   ([`crate::sides::WORK_DIRECTORY`]), which the checkpoint identity
//!   records.
//! * `--verify-reuse` also measures `<tip>-full` (the same tree and binary,
//!   no checkpoints) and fails unless every report artifact of the tip equals
//!   the full side's byte for byte and the first comparison equals the
//!   comparison of the base with the full side.
//!
//! The analysis decides nothing here: a group is reused only when the
//! analyzer's own identity check passes (whole-environment scope, until
//! `GATEWAY-WHOLE-TABLE-READS` lets a group seal restricted).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::inputs::{Route, RouteKind};
use crate::sides::{self, Built, Context, SideSpec};
use crate::util::{capture, now, sha_file};

/// The line `quirin-audit pinned --classify-only` prints on acceptance.
pub const CLASSIFICATION_SCHEMA: &str = "quirin-contract-only-classification/1";

/// The suffix of the verification side's name.
pub const FULL_SUFFIX: &str = "-full";

/// The text a tree's `scripts/run_pinned_audit.py` holds when it forwards
/// the acceptance.
const FORWARDS_ACCEPTANCE: &str = "--accept-base-analyzer";

/// The two sides of the first comparison, when the measurement may take the
/// reuse path; otherwise why not (it then measures separate sides).
///
/// # Errors
///
/// The reason the path is not taken.
pub fn candidate(
    specs: &[(String, SideSpec)],
    comparisons: &[(String, String)],
    routes: &[Route],
    rules_off: &[String],
    build_commit: Option<&str>,
) -> Result<(String, String), String> {
    let (base, tip) = comparisons.first().ok_or("no comparison")?.clone();
    for name in [&base, &tip] {
        let spec = specs
            .iter()
            .find(|(known, _)| known == name)
            .map(|(_, spec)| spec)
            .ok_or_else(|| format!("side {name} is not defined"))?;
        if !matches!(spec, SideSpec::Rev(_)) {
            return Err(format!(
                "side {name} is a {} side; only two `rev:` sides are measured at one fixed path \
                 from one checkout of each revision",
                spec.kind()
            ));
        }
    }
    if !routes
        .iter()
        .any(|route| matches!(route.kind, RouteKind::PinnedCorpus { .. }))
    {
        return Err("no pinned corpus route: only the pinned route has group checkpoints".into());
    }
    if !rules_off.is_empty() {
        return Err("--rules-off: switched sides are measured separately".into());
    }
    if build_commit.is_some() {
        return Err(
            "--build-commit: the accepted analyzer must record the base commit as its build commit"
                .into(),
        );
    }
    Ok((base, tip))
}

/// Ask the base side's binary to classify `BASE..HEAD` of the tip checkout.
///
/// # Errors
///
/// A refusal (the binary's reason), a binary without `--classify-only`, or an
/// answer that does not name exactly the two revisions.
pub fn classify(base: &Built, tip_source: &Path, tip: &str) -> Result<Value, String> {
    let revision = base
        .revision
        .as_deref()
        .ok_or("the base side has no revision")?;
    let output = std::process::Command::new(&base.binary_real)
        .args([
            "pinned",
            "--classify-only",
            "--accept-base-analyzer",
            revision,
        ])
        .arg("--repository")
        .arg(tip_source)
        .env_clear()
        .envs(sides::base_environment())
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| format!("{}: {error}", base.binary_real.display()))?;
    if !output.status.success() {
        return Err(format!(
            "the base side's analyzer refused the classification ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let line: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("the classification is not one JSON line: {error}"))?;
    if line["schema"] != json!(CLASSIFICATION_SCHEMA) || line["accepted"] != json!(true) {
        return Err(format!("an unrecognised classification: {line}"));
    }
    if line["acceptance"]["base"] != json!(revision) || line["acceptance"]["tip"] != json!(tip) {
        return Err(format!(
            "the classification names {}..{}, not {revision}..{tip}",
            line["acceptance"]["base"], line["acceptance"]["tip"]
        ));
    }
    Ok(line)
}

/// Prepare the tip side under the base side's analyzer.
///
/// # Errors
///
/// Any step that fails, with its reason; the caller then builds the tip as a
/// separate side.
pub fn prepare_tip(
    name: &str,
    spec: &SideSpec,
    base: &Built,
    context: &Context,
) -> Result<Built, String> {
    let SideSpec::Rev(revision) = spec else {
        return Err(format!("side {name} is not a `rev:` side"));
    };
    let directory = context.output.join("sides").join(name);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let started = now();
    let full = sides::resolve_revision(&context.repository, revision)?;
    let source = directory.join("src");
    let source_text = source.display().to_string();
    let repository = context.repository.display().to_string();
    capture(
        "git",
        &[
            "clone",
            "--quiet",
            "--shared",
            "--no-checkout",
            &repository,
            &source_text,
        ],
        None,
    )?;
    capture(
        "git",
        &["checkout", "--quiet", "--detach", &full],
        Some(&source),
    )?;
    let head = capture("git", &["rev-parse", "HEAD"], Some(&source))?;
    if head.trim() != full {
        return Err(format!(
            "the tip checkout is at {}, not {full}",
            head.trim()
        ));
    }
    let submodules = sides::clone_submodules(&context.repository, &full, &source)?;
    let runner = source.join("scripts/run_pinned_audit.py");
    let forwards = fs::read_to_string(&runner)
        .map_err(|error| format!("{}: {error}", runner.display()))?
        .contains(FORWARDS_ACCEPTANCE);
    if !forwards {
        return Err(format!(
            "the tip's {} does not forward the analyzer acceptance (a tree before card \
             MEASURE-CONTRACT-ONLY-SIDE-WIRING)",
            runner.display()
        ));
    }
    let classification = classify(base, &source, &full)?;
    let release = source.join("target/release");
    fs::create_dir_all(&release).map_err(|error| format!("{}: {error}", release.display()))?;
    let binary = release.join("quirin-audit");
    fs::copy(&base.binary_real, &binary).map_err(|error| {
        format!(
            "{} -> {}: {error}",
            base.binary_real.display(),
            binary.display()
        )
    })?;
    let base_sha = sha_file(&base.binary_real)?;
    if sha_file(&binary)? != base_sha {
        return Err("the copied base binary differs from the base side's".into());
    }
    let fixed = context.mount_root.join("src");
    let record = json!({
        "side": name,
        "kind": spec.kind(),
        "build_started": started,
        "build": "not built: the base side's analyzer is accepted for a contract-only diff",
        "submodules": submodules,
        "contract_only_reuse": {
            "analyzer_from_side": base.name,
            "base": base.revision,
            "tip": full,
            "binary_sha256": base_sha,
            "classification": classification,
        },
    });
    sides::finish(Built {
        name: name.into(),
        spec: spec.clone(),
        directory,
        binary_real: binary,
        binary_run: fixed.join("target/release/quirin-audit"),
        source_real: source,
        source_run: fixed,
        snapshot: true,
        revision: Some(full),
        build_commit: base.build_commit.clone(),
        record,
    })
}

/// Undo a refused [`prepare_tip`] so the side can be built in the same
/// directory: only the checkout it made is removed.
///
/// # Errors
///
/// A checkout that cannot be removed.
pub fn discard(context: &Context, name: &str) -> Result<(), String> {
    let source = context.output.join("sides").join(name).join("src");
    if source.exists() {
        fs::remove_dir_all(&source).map_err(|error| format!("{}: {error}", source.display()))?;
    }
    Ok(())
}

/// The checkpoint directory of one pinned route.
#[must_use]
pub fn checkpoint_directory(output: &Path, route: &str) -> PathBuf {
    output.join("checkpoints").join(route)
}

/// The verification of one route: the tip's report artifacts against the
/// full side's, byte for byte (no substitution), and the first comparison's
/// route record against the base-to-full comparison's.
#[must_use]
pub fn verify_route(artifacts: &Value, reused: &Value, full: &Value) -> Value {
    let strip = |record: &Value| {
        let mut record = record.clone();
        if let Some(object) = record.as_object_mut() {
            object.remove("after_directory");
        }
        record
    };
    let empty = |key: &str| {
        artifacts[key].as_array().is_some_and(Vec::is_empty)
            || artifacts[key]
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
    };
    let byte_identical = empty("differing")
        && empty("only_before")
        && empty("only_after")
        && empty("identical_after_named_substitution")
        && artifacts["identical"]
            .as_array()
            .is_some_and(|files| !files.is_empty());
    let comparison_equal = strip(reused) == strip(full);
    json!({
        "artifacts_byte_identical": byte_identical,
        "artifacts_compared": artifacts["identical"].as_array().map_or(0, Vec::len),
        "comparison_equal": comparison_equal,
        "artifacts": artifacts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(kind: &str) -> Route {
        let definition = if kind == "pinned-corpus" {
            json!({"name": "corpus", "profile": "p", "kind": "pinned-corpus",
                   "source_commit": "66910c10", "inputs_root": "in",
                   "compile_databases": [], "mirror_trees": [],
                   "link_map": {"path": "l.json"}, "clang": "clang", "tracked": []})
        } else {
            json!({"name": "slice", "profile": "p", "kind": "configured-selection",
                   "checkout": "c", "compile_database": {"path": "d.json"},
                   "selection": "s.json", "profile_directory": "p",
                   "link_map": {"path": "l.json"}, "clang": "clang", "tracked": []})
        };
        serde_json::from_value(definition).unwrap()
    }

    fn rev(name: &str) -> (String, SideSpec) {
        (name.into(), SideSpec::Rev(format!("{name}-rev")))
    }

    #[test]
    fn only_two_rev_sides_with_a_pinned_route_and_no_switch_are_candidates() {
        let pair = vec![("base".to_owned(), "tip".to_owned())];
        let corpus = vec![route("pinned-corpus"), route("configured-selection")];
        let specs = vec![rev("base"), rev("tip")];
        assert_eq!(
            candidate(&specs, &pair, &corpus, &[], None).unwrap(),
            ("base".to_owned(), "tip".to_owned())
        );
        let no_pinned = vec![route("configured-selection")];
        assert!(
            candidate(&specs, &pair, &no_pinned, &[], None)
                .unwrap_err()
                .contains("no pinned corpus route")
        );
        assert!(
            candidate(&specs, &pair, &corpus, &["33".into()], None)
                .unwrap_err()
                .contains("--rules-off")
        );
        assert!(
            candidate(&specs, &pair, &corpus, &[], Some("x"))
                .unwrap_err()
                .contains("--build-commit")
        );
        for other in [
            SideSpec::Baseline("abc".into()),
            SideSpec::Run(PathBuf::from("/r")),
            SideSpec::Checkout(PathBuf::from("/c")),
            SideSpec::Binary(PathBuf::from("/b")),
        ] {
            let mixed = vec![("base".to_owned(), other.clone()), rev("tip")];
            assert!(
                candidate(&mixed, &pair, &corpus, &[], None)
                    .unwrap_err()
                    .contains("only two `rev:` sides"),
                "{other:?}"
            );
            let mixed = vec![rev("base"), ("tip".to_owned(), other)];
            assert!(candidate(&mixed, &pair, &corpus, &[], None).is_err());
        }
    }

    #[test]
    fn a_verification_needs_identical_artifacts_and_an_equal_comparison() {
        let identical = json!({"identical": [{"file": "corpus/r.json", "sha256": "a"}],
            "identical_after_named_substitution": {}, "differing": [],
            "only_before": [], "only_after": []});
        let record = json!({"moves": [], "after_directory": "/x/tip/corpus"});
        let full = json!({"moves": [], "after_directory": "/x/tip-full/corpus"});
        let verdict = verify_route(&identical, &record, &full);
        assert_eq!(verdict["artifacts_byte_identical"], true);
        assert_eq!(verdict["comparison_equal"], true);
        let moved = json!({"moves": [{"row": "r"}], "after_directory": "/x"});
        assert_eq!(
            verify_route(&identical, &record, &moved)["comparison_equal"],
            false
        );
        for (key, value) in [
            ("differing", json!([{"file": "f"}])),
            ("only_before", json!(["f"])),
            ("only_after", json!(["f"])),
            ("identical_after_named_substitution", json!({"f": {}})),
            ("identical", json!([])),
        ] {
            let mut artifacts = identical.clone();
            artifacts[key] = value;
            assert_eq!(
                verify_route(&artifacts, &record, &full)["artifacts_byte_identical"],
                false,
                "{key}"
            );
        }
    }

    #[test]
    fn the_tip_resumes_under_the_base_analyzer_and_the_base_seals() {
        let seal = sides::Checkpoint {
            directory: PathBuf::from("/o/checkpoints/corpus"),
            accept_base: None,
        };
        assert_eq!(
            seal.arguments(),
            ["--checkpoint-dir", "/o/checkpoints/corpus"]
        );
        let resume = sides::Checkpoint {
            directory: PathBuf::from("/o/checkpoints/corpus"),
            accept_base: Some(("b".repeat(40), PathBuf::from("/tmp/quirin-measure/src"))),
        };
        assert_eq!(
            resume.arguments(),
            [
                "--checkpoint-dir".to_owned(),
                "/o/checkpoints/corpus".to_owned(),
                "--resume".to_owned(),
                "--accept-base-analyzer".to_owned(),
                "b".repeat(40),
                "--repository".to_owned(),
                "/tmp/quirin-measure/src".to_owned(),
            ]
        );
        assert_eq!(resume.record()["role"], "resume-under-base-analyzer");
        assert_eq!(seal.record()["role"], "seal");
    }

    #[test]
    fn the_checkpoint_summary_counts_decisions_per_stage() {
        let run = json!({"groups": [
            {"group": "a", "stage": "first", "decision": "reused", "reason": "identity equal",
             "environment_scope": "whole"},
            {"group": "b", "stage": "first", "decision": "recomputed", "reason": "no checkpoint"},
            {"group": "b", "stage": "first", "decision": "sealed", "environment_scope": "whole"},
            {"stage": "first", "wall_time_ms": 5, "stopped": false},
        ]});
        assert_eq!(
            sides::checkpoint_summary(&run),
            json!({"first": {"reused (whole)": 1, "recomputed: no checkpoint": 1,
                             "sealed (whole)": 1}})
        );
        assert_eq!(sides::checkpoint_summary(&json!({})), Value::Null);
    }
}
