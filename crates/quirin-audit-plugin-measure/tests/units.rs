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

//! Card MEASURE-TOOL: the mirror, the profile adapter, the baseline digest
//! formula, the shared composition vectors and the superseded-script
//! registry.

#![cfg(unix)]

mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use qcore_schemas::composition::{
    Component, Contribution, Provenance, RowKey, RuleSet, UnitRef, VerdictTuple,
};
use qcore_schemas::verdict::Verdict;
use quirin_audit_plugin_measure::inputs::{self, PinnedFile, Route, RouteKind};
use quirin_audit_plugin_measure::{baseline, compare, mirror, sides};
use serde_json::{Value, json};
use support::scratch;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

// ---- the re-spelled mirror -------------------------------------------------

fn corpus_route() -> Route {
    Route {
        name: "corpus".into(),
        profile: "eight-home".into(),
        kind: RouteKind::PinnedCorpus {
            source_commit: "66910c10".into(),
            inputs_root: "build/cycle19".into(),
            compile_databases: vec![
                PinnedFile {
                    path: "c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json"
                        .into(),
                    sha256: None,
                },
                PinnedFile {
                    path: "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json".into(),
                    sha256: None,
                },
            ],
            mirror_trees: vec![
                "c-gateway-wave-pin/fluent-bit-clang-source".into(),
                "c-gateway-wave-pin/monkey".into(),
                "provider-packages-wave20".into(),
            ],
        },
        link_map: PinnedFile {
            path: "link-map.json".into(),
            sha256: None,
        },
        clang: "clang".into(),
        tracked: Vec::new(),
        required_inputs: Vec::new(),
    }
}

/// A checkout whose pinned databases spell `old`, a symbolic link to `real`
/// (the 2026-10-23 migration), as `mirror_inputs.py` found it.
fn moved_checkout(directory: &Path) -> (PathBuf, PathBuf) {
    let real = directory.join("real");
    let old = directory.join("old");
    let pin = real.join("build/cycle19/c-gateway-wave-pin");
    for tree in [
        "fluent-bit-clang-source/build",
        "monkey/monkey-build",
        "fluent-bit-clang-production",
    ] {
        std::fs::create_dir_all(pin.join(tree)).unwrap();
    }
    std::fs::create_dir_all(real.join("build/cycle19/provider-packages-wave20/qcore.chunkio"))
        .unwrap();
    std::fs::write(pin.join("fluent-bit-clang-source/a.c"), "int a;\n").unwrap();
    std::os::unix::fs::symlink(&real, &old).unwrap();
    let spelled = |relative: &str| old.join("build/cycle19/c-gateway-wave-pin").join(relative);
    let fluent = json!([{
        "directory": spelled("fluent-bit-clang-source/build").display().to_string(),
        "file": spelled("fluent-bit-clang-source/a.c").display().to_string(),
        "command": format!("cc -I{} -I{} -c a.c",
            spelled("monkey/include").display(), spelled("fluent-bit-clang-production/x").display()),
    }]);
    std::fs::write(
        pin.join("fluent-bit-clang-source/build/compile_commands.json"),
        serde_json::to_string_pretty(&fluent).unwrap(),
    )
    .unwrap();
    let monkey = json!([{
        "directory": spelled("monkey/monkey-build").display().to_string(),
        "file": spelled("monkey/m.c").display().to_string(),
        "command": "cc -c m.c",
    }]);
    std::fs::write(
        pin.join("monkey/monkey-build/compile_commands.json"),
        serde_json::to_string_pretty(&monkey).unwrap(),
    )
    .unwrap();
    (real, old)
}

#[test]
fn the_mirror_respells_exactly_as_mirror_inputs_did() {
    let directory = scratch("mirror");
    let (real, old) = moved_checkout(&directory);
    let fixed = PathBuf::from("/fixed/mirror");
    let mirror_dir = directory.join("mirror");
    let (spelled, record) = mirror::prepare(
        &[corpus_route()],
        &real,
        &mirror_dir,
        &fixed,
        mirror::Mode::Auto,
    )
    .unwrap();
    assert_eq!(record["spelling"], "mirror");
    assert_eq!(record["spelled_prefix"], old.display().to_string());
    assert_eq!(record["canonical_prefix"], real.display().to_string());
    assert_eq!(
        spelled["corpus"].inputs_root.as_deref(),
        Some(fixed.as_path())
    );
    // mirror_inputs.py's rules: the database's own tree names the mirror copy,
    // every other spelled path the real checkout.
    let fluent = std::fs::read_to_string(
        mirror_dir.join("c-gateway-wave-pin/fluent-bit-clang-source/build/compile_commands.json"),
    )
    .unwrap();
    let old_text = old.display().to_string();
    let real_text = real.display().to_string();
    assert!(!fluent.contains(&old_text));
    assert!(fluent.contains("/fixed/mirror/c-gateway-wave-pin/fluent-bit-clang-source/a.c"));
    assert!(fluent.contains(&format!(
        "{real_text}/build/cycle19/c-gateway-wave-pin/monkey/include"
    )));
    assert!(fluent.contains(&format!(
        "{real_text}/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-production/x"
    )));
    let monkey = std::fs::read_to_string(
        mirror_dir.join("c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
    )
    .unwrap();
    assert!(monkey.contains("/fixed/mirror/c-gateway-wave-pin/monkey/m.c"));
    // The copies are byte for byte and the manifest records both digests.
    assert_eq!(
        std::fs::read(mirror_dir.join("c-gateway-wave-pin/fluent-bit-clang-source/a.c")).unwrap(),
        b"int a;\n"
    );
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(mirror_dir.join("manifest.json")).unwrap()).unwrap();
    for tree in manifest["trees"].as_object().unwrap().values() {
        assert_eq!(tree["original_digest"], tree["copy_digest"]);
    }
    assert_eq!(manifest["databases"].as_object().unwrap().len(), 2);
    assert_eq!(manifest["mirror_as_run"], "/fixed/mirror");

    // `never` reads the originals in place; a stale pinned digest refuses.
    let (spelled, record) = mirror::prepare(
        &[corpus_route()],
        &real,
        &directory.join("unused"),
        &fixed,
        mirror::Mode::Never,
    )
    .unwrap();
    assert_eq!(record["spelling"], "original");
    assert_eq!(
        spelled["corpus"].inputs_root,
        Some(real.join("build/cycle19"))
    );
    assert!(!directory.join("unused").exists());
    let mut stale = corpus_route();
    if let RouteKind::PinnedCorpus {
        compile_databases, ..
    } = &mut stale.kind
    {
        compile_databases[0].sha256 = Some("0".repeat(64));
    }
    let error = mirror::prepare(
        &[stale],
        &real,
        &directory.join("x"),
        &fixed,
        mirror::Mode::Auto,
    )
    .unwrap_err();
    assert!(error.contains("not the pinned"), "{error}");
}

// ---- the profile adapter ---------------------------------------------------

/// A committed evaluation-profile record (card EVALUATION-PROFILES).
fn record(name: &str) -> PathBuf {
    repository()
        .join("contract/evaluations")
        .join(name)
        .join("evaluation.json")
}

fn live(name: &str) -> Route {
    inputs::read_manifest(&Path::new(env!("CARGO_MANIFEST_DIR")).join("inputs/live.json"))
        .unwrap()
        .into_iter()
        .find(|route| route.name == name)
        .unwrap()
}

#[test]
fn the_profile_adapter_reads_the_committed_evaluation_profiles() {
    let corpus = inputs::read_profile(&record("eight-home")).unwrap();
    let expected = live("corpus");
    assert_eq!(
        (corpus.name.as_str(), corpus.profile.as_str()),
        ("corpus", "eight-home")
    );
    assert_eq!(corpus.link_map.sha256, expected.link_map.sha256);
    assert_eq!(
        corpus.link_map.path,
        "contract/evaluations/eight-home/link-map.json"
    );
    assert!(
        corpus
            .required_inputs
            .contains(&"build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source".to_owned())
    );
    match (&corpus.kind, &expected.kind) {
        (
            RouteKind::PinnedCorpus {
                source_commit,
                inputs_root,
                compile_databases,
                mirror_trees,
            },
            RouteKind::PinnedCorpus {
                source_commit: live_commit,
                inputs_root: live_root,
                compile_databases: live_databases,
                ..
            },
        ) => {
            assert_eq!(source_commit, live_commit);
            assert_eq!(inputs_root, live_root);
            assert_eq!(
                compile_databases, live_databases,
                "same databases and digests"
            );
            assert!(
                mirror_trees.contains(&"c-gateway-wave-pin/fluent-bit-clang-source".to_owned())
            );
        }
        other => panic!("{other:?}"),
    }
    let system = inputs::read_profile(&record("fluent-bit-system")).unwrap();
    let expected = live("system");
    assert_eq!(system.name, "system");
    assert_eq!(
        system.kind, expected.kind,
        "the adapter places every configured input"
    );
    let error = inputs::read_profile(&record("cycle-19-fluent-bit")).unwrap_err();
    assert!(error.contains("frozen"), "{error}");
}

// ---- the baseline digest formula ---------------------------------------------

#[test]
fn bundle_digests_of_a_snapshot_equal_those_read_from_its_git_tree() {
    let repository = repository();
    let directory = scratch("bundles");
    let archive = directory.join("tree.tar");
    let status = Command::new("git")
        .args(["archive", "--format=tar", "-o"])
        .arg(&archive)
        .args(["HEAD", "providers", "contract"])
        .current_dir(&repository)
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&directory)
        .status()
        .unwrap();
    assert!(status.success());
    let from_files = sides::bundle_digests(&directory).unwrap();
    let from_git = baseline::git_bundle_digests(&repository, "HEAD").unwrap();
    assert!(from_files.contains_key("contract"));
    assert!(
        from_files
            .keys()
            .any(|bundle| bundle.starts_with("providers/"))
    );
    assert_eq!(from_files, from_git);
}

// ---- the shared composition vectors ----------------------------------------

const LEAN: &str = include_str!("../../../proof/QCore/AuditComposition.lean");

/// One Lean `Component` literal, `⟨row, prov, body, .verdict, [assumptions]⟩`.
fn lean_component(text: &str) -> Component {
    let inner = text.trim().trim_start_matches('⟨').trim_end_matches('⟩');
    let (head, assumptions) = inner.split_once('[').unwrap();
    let fields: Vec<&str> = head
        .split(',')
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .collect();
    let number = |text: &str| text.parse::<u64>().unwrap();
    let verdict = match fields[3] {
        ".safe" => Verdict::Safe,
        ".pending" => Verdict::Pending,
        ".unknown" => Verdict::Unknown,
        ".provisionalUnsafe" => Verdict::Unsafe,
        other => panic!("{other}"),
    };
    let assumptions: BTreeSet<String> = assumptions
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .map(|a| format!("p{a}"))
        .collect();
    let key = vector_row(number(fields[0]));
    Component {
        unit: key.unit.clone(),
        row: key.row.clone(),
        property: key.property.clone(),
        profile: key.profile.clone(),
        provenance: Provenance {
            analyzer_revision: format!("a{}", number(fields[1])),
            rule_set: RuleSet {
                registry: "r".into(),
                off: BTreeSet::new(),
            },
            contract_bundles: Default::default(),
            external_checker: None,
        },
        verdict: VerdictTuple {
            verdict,
            scope: "s".into(),
            bounds: "b".into(),
            assumptions,
            evidence_version: "e".into(),
        },
        reasons: Vec::new(),
        detail: json!(number(fields[2])),
    }
}

fn vector_row(row: u64) -> RowKey {
    RowKey {
        unit: UnitRef {
            library: "l".into(),
            file: format!("f{row}.c"),
        },
        row: "unit#0".into(),
        property: "p".into(),
        profile: "q".into(),
    }
}

/// Split a Lean vector line into its component literals and trailing fields.
fn vector(line: &str) -> (Vec<Component>, u64, Option<Verdict>, bool) {
    let line = line.trim().trim_end_matches(',');
    let inner = &line[line.find('[').unwrap() + 1..];
    let close = inner.rfind("], ").unwrap();
    let (components, rest) = (&inner[..close], &inner[close + 3..]);
    let mut literals = Vec::new();
    let mut depth = 0;
    let mut start = None;
    for (index, ch) in components.char_indices() {
        match ch {
            '⟨' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '⟩' => {
                depth -= 1;
                if depth == 0 {
                    literals.push(lean_component(
                        &components[start.unwrap()..index + ch.len_utf8()],
                    ));
                }
            }
            _ => {}
        }
    }
    let fields: Vec<&str> = rest.splitn(3, ", ").collect();
    let row = fields[0].trim().parse().unwrap();
    let expected = match fields[1].trim() {
        "none" => None,
        some => Some(match some.trim_start_matches("some ").trim() {
            ".safe" => Verdict::Safe,
            ".pending" => Verdict::Pending,
            ".unknown" => Verdict::Unknown,
            ".provisionalUnsafe" => Verdict::Unsafe,
            other => panic!("{other}"),
        }),
    };
    let conflict = fields[2].trim().starts_with("true");
    (literals, row, expected, conflict)
}

/// The tool composes two sides with `⊕` ([`compare::union`]). Over every
/// shared vector of `proof/QCore/AuditComposition.lean`, split between two
/// sides in either order, the union's derivation and conflict are the
/// vector's: the tool's composition is the audited one.
#[test]
fn the_shared_composition_vectors_hold_through_the_tools_union() {
    let header = "def vectors : List Vector := [";
    let start = LEAN.find(header).unwrap() + header.len();
    let body = &LEAN[start..];
    let body = &body[..body.find("\n]").unwrap()];
    let lines: Vec<&str> = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(lines.len(), 14, "every shared vector is read");
    for (index, line) in lines.iter().enumerate() {
        let (components, row, expected, conflict) = vector(line);
        for flip in [false, true] {
            let (mut before, mut after) = (Contribution::new(), Contribution::new());
            for (position, component) in components.iter().enumerate() {
                if (position % 2 == 0) != flip {
                    before.insert(component.clone());
                } else {
                    after.insert(component.clone());
                }
            }
            let (union, record) = compare::union(&before, &after);
            let derived = union.derive(&vector_row(row));
            assert_eq!(
                derived.as_ref().map(|d| d.verdict),
                expected,
                "vector {index}"
            );
            assert_eq!(
                derived.as_ref().is_some_and(|d| d.is_conflicted()),
                conflict,
                "vector {index}"
            );
            let named = record["conflicts"].as_array().unwrap().iter().any(|entry| {
                entry["row"]
                    .as_str()
                    .unwrap()
                    .starts_with(&format!("l::f{row}.c::"))
            });
            assert_eq!(
                named, conflict,
                "vector {index}: a conflict is named in the record"
            );
        }
    }
}

// ---- the superseded scripts ------------------------------------------------

/// Every copied measurement script the tool replaces is listed with the
/// digest it had when it was superseded, and is never edited (each report
/// owns its evidence; drivers record their own digest).
#[test]
fn superseded_scripts_are_listed_and_frozen() {
    let repository = repository();
    let path = repository.join("docs/reports/measure-tool/superseded-scripts.json");
    let registry: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let scripts = registry["scripts"].as_array().unwrap();
    assert!(scripts.len() >= 10);
    for script in scripts {
        let relative = script["path"].as_str().unwrap();
        let bytes = std::fs::read(repository.join(relative)).unwrap();
        assert_eq!(
            script["sha256"].as_str().unwrap(),
            qcore_schemas::sha256_hex(&bytes),
            "{relative} was edited after it was superseded"
        );
        assert!(
            !script["superseded_by"].as_str().unwrap().is_empty(),
            "{relative}"
        );
    }
}
