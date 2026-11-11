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

//! G03-F: a source set whose C sources sit at the checkout root, and the
//! build options a real multi-target CMake database carries.
//!
//! These are generic regressions over a hermetic pinned repository with the
//! shape Libevent has: `.c` files directly at the checkout root, one command
//! per (source, target) pair, and the warning / aliasing / optimization
//! options a project's own `CMakeLists.txt` appends unconditionally.
//!
//! They pin two things and certify nothing: that the checkout-root spelling is
//! admissible only at manifest version 2 and only as the exact literal `"."`,
//! and that the transported build-option table is an allowlist whose default
//! is still refusal. `Unknown` stays in the denominator throughout.

use qcore_contract::profile_loader::sha256_hex;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// The build options Libevent's own `CMakeLists.txt` appends under a GNU-like
/// compiler, beyond the four the gateway already transported. Every entry has
/// a measured preservation argument recorded in
/// `docs/reports/remaining-g03-f/README.md`.
const ADDED_FLAGS: [&str; 17] = [
    "-Wextra",
    "-Wno-unused-parameter",
    "-Wstrict-aliasing",
    "-Wstrict-prototypes",
    "-Wundef",
    "-fno-strict-aliasing",
    "-Wmissing-prototypes",
    "-Winit-self",
    "-Wmissing-field-initializers",
    "-Wdeclaration-after-statement",
    "-Waddress",
    "-Woverride-init",
    "-Wwrite-strings",
    "-Wno-unused-function",
    "-Wno-pragmas",
    "-Wvla",
    "-Wno-void-pointer-to-enum-cast",
];

/// Root-level sources reproducing the three resource-projection outcomes.
fn sources() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (
            "balanced.c",
            "extern void *alloc(void); extern void dispose(void *p);\nvoid balanced(void) { void *p = alloc(); dispose(p); }\n",
        ),
        (
            "leak.c",
            "extern void *alloc(void);\nvoid leak(void) { void *p = alloc(); }\n",
        ),
        (
            "opaque.c",
            "extern void opaque(void);\nvoid unknown_effect(void) { opaque(); }\n",
        ),
    ])
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-root-source-set-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let checkout = root.join("checkout");
        fs::create_dir_all(&checkout).unwrap();
        for (name, source) in sources() {
            fs::write(checkout.join(name), source).unwrap();
        }
        for args in [
            vec!["init", "--quiet"],
            vec!["add", "."],
            vec![
                "-c",
                "user.name=Root Source Set",
                "-c",
                "user.email=audit@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Pin a checkout-root source set",
            ],
        ] {
            let output = Command::new("git")
                .current_dir(&checkout)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let commit = Command::new("git")
            .current_dir(&checkout)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        let commit = String::from_utf8(commit.stdout).unwrap().trim().to_owned();

        fs::write(
            root.join("memory.qcore"),
            include_str!("fixtures/alloc_release.qv"),
        )
        .unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": ["memory.qcore"]}).to_string(),
        )
        .unwrap();

        let audited = sources()
            .into_iter()
            .map(|(name, source)| json!({"path": name, "sha256": sha256_hex(source.as_bytes())}))
            .collect::<Vec<_>>();
        let library = json!({
            "pin": "example",
            "source_roots": [{"root": ".", "selection": "top_level"}],
            "audited_files": audited,
            "source_witnesses": [], "permitted_mechanisms": [], "operation_summaries": {},
            "api_dispositions": {}, "accepted_builds": [], "required_evidence": [],
            "typed_import": null, "callback_summaries": {}, "boolean_field_models": [],
            "provider": null,
        });
        fs::create_dir_all(root.join("profiles/example")).unwrap();
        fs::write(
            root.join("profiles/example/profile.json"),
            json!({
                "schema_version": 2, "source_set": "example",
                "pins": {"example": {
                    "git_commit": commit, "clean_paths": [],
                    "description": "checkout-root source set fixture",
                }},
                "libraries": {"example": library},
                "resource_contract_manifest": "../../contracts.json",
                "provider_manifests": [],
            })
            .to_string(),
        )
        .unwrap();

        // Match the gateway's selected compiler, including QCORE_CLANG and
        // Visual Studio installations whose Clang is not on PATH.
        let clang = qcore_c_gateway::clang_command().expect("Clang is required for audit fixtures");
        let compiler = clang.get_program().to_str().unwrap().to_owned();
        // Two targets compile every source, exactly as Libevent's own build
        // does: the commands differ only in `-o`.
        let mut database = Vec::new();
        for target in ["core", "full"] {
            for name in sources().into_keys() {
                let mut arguments = vec![compiler.clone(), "-std=gnu17".into(), "-Wall".into()];
                arguments.extend(ADDED_FLAGS.iter().map(|flag| (*flag).to_owned()));
                arguments.push("-O3".into());
                arguments.extend([
                    "-c".to_owned(),
                    name.to_owned(),
                    "-o".to_owned(),
                    format!("objects/{target}.dir/{name}.o"),
                ]);
                database.push(json!({
                    "directory": checkout,
                    "file": name,
                    "arguments": arguments,
                }));
            }
        }
        fs::write(
            root.join("compile_commands.json"),
            serde_json::to_string(&database).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("audit.json"),
            json!({
                "schema_version": 1, "profile_directory": "profiles/example",
                "checkout": "checkout", "compile_database": "compile_commands.json",
                "libraries": ["example"], "report": "output/report.json",
                "compile_target": {"object_prefix": "objects/core.dir/"},
            })
            .to_string(),
        )
        .unwrap();
        Self(root)
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .arg(self.0.join("audit.json"))
            .output()
            .unwrap()
    }

    fn mutate(&self, path: &str, change: impl FnOnce(&mut Value)) {
        let path = self.0.join(path);
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        change(&mut value);
        fs::write(path, value.to_string()).unwrap();
    }

    fn stderr(output: &Output) -> String {
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    fn report(&self) -> Value {
        serde_json::from_slice(&fs::read(self.0.join("output/report.json")).unwrap()).unwrap()
    }

    fn assumptions(&self) -> Value {
        serde_json::from_slice(&fs::read(self.0.join("output/report.assumptions.json")).unwrap())
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// F1 + F2 together: a checkout-root source set audits end to end, with every
/// transported option carried and no verdict promoted.
#[test]
fn a_checkout_root_source_set_audits_from_configuration_alone() {
    let fixture = Fixture::new();
    let output = fixture.run();
    assert!(output.status.success(), "{}", Fixture::stderr(&output));
    let report = fixture.report();
    let units = report["units"].as_array().unwrap();
    assert_eq!(units.len(), 3, "{units:#?}");
    let verdicts = units
        .iter()
        .map(|unit| {
            (
                unit["file"].as_str().unwrap().to_owned(),
                unit["status"].as_str().unwrap().to_owned(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(verdicts["balanced.c"], "Safe");
    assert_eq!(verdicts["leak.c"], "Unsafe");
    assert_eq!(verdicts["opaque.c"], "Unknown");
}

/// Every one of the seventeen added options plus the optimization level is
/// carried into the recorded compile-task identity, in the order the database
/// spelled them. A transported option is not a dropped one.
#[test]
fn every_added_build_option_reaches_the_recorded_compile_task() {
    let fixture = Fixture::new();
    let output = fixture.run();
    assert!(output.status.success(), "{}", Fixture::stderr(&output));
    let assumptions = fixture.assumptions();
    let tasks = assumptions["compile_tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 3);
    for task in tasks {
        let options = serde_json::to_string(&task["identity"]["preprocessor_options"]).unwrap();
        for flag in ADDED_FLAGS {
            let variant = expected_variant(flag);
            assert!(
                options.contains(variant),
                "{flag} (as {variant}) is missing from {options}"
            );
        }
        assert!(options.contains("\"Optimization\":\"Level3\""), "{options}");
        // The whole argument vector is recorded, so nothing was filtered away.
        let arguments = serde_json::to_string(&task["identity"]["arguments"]).unwrap();
        for flag in ADDED_FLAGS {
            assert!(arguments.contains(flag), "{flag} missing from {arguments}");
        }
    }
}

/// The serialized name each admitted option carries.
fn expected_variant(flag: &str) -> &'static str {
    match flag {
        "-Wextra" => "\"ExtraWarnings\"",
        "-Wno-unused-parameter" => "\"NoUnusedParameter\"",
        "-Wstrict-aliasing" => "\"StrictAliasingWarnings\"",
        "-Wstrict-prototypes" => "\"StrictPrototypes\"",
        "-Wundef" => "\"UndefinedIdentifierWarnings\"",
        "-fno-strict-aliasing" => "\"NoStrictAliasing\"",
        "-Wmissing-prototypes" => "\"MissingPrototypes\"",
        "-Winit-self" => "\"InitSelf\"",
        "-Wmissing-field-initializers" => "\"MissingFieldInitializers\"",
        "-Wdeclaration-after-statement" => "\"DeclarationAfterStatement\"",
        "-Waddress" => "\"AddressComparisons\"",
        "-Woverride-init" => "\"OverrideInit\"",
        "-Wwrite-strings" => "\"WriteStrings\"",
        "-Wno-unused-function" => "\"NoUnusedFunction\"",
        "-Wno-pragmas" => "\"NoPragmas\"",
        "-Wvla" => "\"VariableLengthArrays\"",
        "-Wno-void-pointer-to-enum-cast" => "\"NoVoidPointerToEnumCast\"",
        other => panic!("no recorded variant for {other}"),
    }
}

/// The added table did not widen the default: an option outside it still
/// rejects the whole database with no report.
#[test]
fn an_option_outside_the_transported_table_still_rejects() {
    for probe in [
        "-ffast-math",
        "-Ofast",
        "-O",
        "-fvisibility=hidden",
        "-flto",
    ] {
        let fixture = Fixture::new();
        fixture.mutate("compile_commands.json", |database| {
            for entry in database.as_array_mut().unwrap() {
                entry["arguments"]
                    .as_array_mut()
                    .unwrap()
                    .insert(2, json!(probe));
            }
        });
        let output = fixture.run();
        assert!(!output.status.success(), "{probe} was accepted");
        assert!(
            Fixture::stderr(&output).contains(probe),
            "{probe}: {}",
            Fixture::stderr(&output)
        );
        assert!(!fixture.0.join("output/report.json").exists());
    }
}

/// The checkout-root spelling is a version-2 admission, not a v1 one: the same
/// manifest bytes under `"schema_version": 1` reject.
#[test]
fn a_version_one_manifest_still_rejects_the_checkout_root() {
    let fixture = Fixture::new();
    fixture.mutate("profiles/example/profile.json", |profile| {
        profile["schema_version"] = json!(1);
    });
    let output = fixture.run();
    assert!(!output.status.success());
    let stderr = Fixture::stderr(&output);
    assert!(stderr.contains("source root"), "{stderr}");
    assert!(!fixture.0.join("output/report.json").exists());
}

/// `.` is admitted as an exact spelling, not as a path-normalisation rule.
/// Traversal, absolute and mixed spellings keep their rejection at every
/// supported version.
///
/// `"src/."` is deliberately absent: `Path::components` normalises a trailing
/// `.` away, so it was already an ordinary relative root under version 1 and
/// resolves inside the checkout. Version 2 does not change it.
#[test]
fn traversal_and_absolute_roots_reject_at_every_version() {
    for version in [1, 2] {
        for root in ["..", "./src", "src/..", "/", "/abs", "", "../checkout"] {
            let fixture = Fixture::new();
            fixture.mutate("profiles/example/profile.json", |profile| {
                profile["schema_version"] = json!(version);
                profile["libraries"]["example"]["source_roots"] =
                    json!([{"root": root, "selection": "top_level"}]);
            });
            let output = fixture.run();
            assert!(
                !output.status.success(),
                "schema {version} accepted root `{root}`"
            );
            assert!(
                Fixture::stderr(&output).contains("source root"),
                "schema {version}, root `{root}`: {}",
                Fixture::stderr(&output)
            );
            assert!(!fixture.0.join("output/report.json").exists());
        }
    }
}

/// A manifest version this loader does not define still rejects wholesale.
#[test]
fn an_unsupported_manifest_version_rejects() {
    let fixture = Fixture::new();
    fixture.mutate("profiles/example/profile.json", |profile| {
        profile["schema_version"] = json!(3);
    });
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(!fixture.0.join("output/report.json").exists());
}

/// Without the declared target selection, the two per-target commands are two
/// distinct configurations and the audit refuses to choose between them. The
/// selection narrows; it does not merge.
#[test]
fn a_multi_target_database_still_refuses_to_choose_without_a_declared_target() {
    let fixture = Fixture::new();
    fixture.mutate("audit.json", |config| {
        config.as_object_mut().unwrap().remove("compile_target");
    });
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(
        Fixture::stderr(&output).contains("conflicting compilation commands"),
        "{}",
        Fixture::stderr(&output)
    );
    assert!(!fixture.0.join("output/report.json").exists());
}

/// Residual ambiguity inside the selected target still refuses: the filter
/// keeps records, it never merges two commands that differ elsewhere.
#[test]
fn residual_ambiguity_inside_the_selected_target_still_refuses() {
    let fixture = Fixture::new();
    fixture.mutate("compile_commands.json", |database| {
        let entries = database.as_array_mut().unwrap();
        let mut extra = entries[0].clone();
        // Same target object directory, a different macro: a genuinely
        // different configuration for the same file.
        extra["arguments"]
            .as_array_mut()
            .unwrap()
            .insert(2, json!("-DVARIANT=1"));
        entries.push(extra);
    });
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(
        Fixture::stderr(&output).contains("conflicting compilation commands"),
        "{}",
        Fixture::stderr(&output)
    );
    assert!(!fixture.0.join("output/report.json").exists());
}

/// A declared target that names nothing rejects rather than auditing an empty
/// or silently widened selection.
#[test]
fn a_declared_target_that_selects_nothing_rejects() {
    for prefix in ["objects/absent.dir/", "-o", "$TARGET/", "   "] {
        let fixture = Fixture::new();
        fixture.mutate("audit.json", |config| {
            config["compile_target"] = json!({"object_prefix": prefix});
        });
        let output = fixture.run();
        assert!(!output.status.success(), "prefix `{prefix}` was accepted");
        assert!(!fixture.0.join("output/report.json").exists());
    }
}

/// The declared narrowing is recorded as an assumption, so no report presents
/// a target-selected result without naming the selection it read.
#[test]
fn the_declared_target_selection_is_recorded_in_the_assumptions() {
    let fixture = Fixture::new();
    let output = fixture.run();
    assert!(output.status.success(), "{}", Fixture::stderr(&output));
    let assumptions = fixture.assumptions();
    assert_eq!(
        assumptions["compile_target_selection"]["object_prefix"],
        "objects/core.dir/"
    );
    assert!(
        assumptions["compile_target_selection"]["residual_ambiguity"]
            .as_str()
            .unwrap()
            .contains("still refused")
    );
}
