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

//! Card LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING (rule 53, contract spec
//! C06.2): on every route that publishes rows, a call whose ranged
//! declaration is held is described and classified by the reason it is held
//! for, with a checker binding present (the libcurl user crate's shape: a
//! flexible declaration, a version range, a checker whose uncertified `Safe`
//! is `Pending`). Out of range, unreadable and ambiguous publish `Unknown`
//! with the library-version reason; a version that only differs from the
//! pinned one publishes `Pending` (`contract-pin-mismatch`, review and
//! re-pin); in force, the row is the binding's `Pending`. A binding's
//! `Pending` claim never lifts a held call above `Unknown`.
//!
//! The fixture is `library_version_route_database.rs`'s, with a checker
//! package (`user_contract_plugins.rs`'s technique) answering `Safe`
//! without evidence.

// The checker fixture executes a /bin/sh script.
#![cfg(unix)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::user_plugins::{checker_request, checker_task};
use qcore_c_gateway::legacy_c::pin_hold::{hold_description, library_version_hold_description};
use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_contract::profile_loader::sha256_hex;
use qcore_verifier_host::*;
use serde_json::{Value, json};

#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;
use process_fixture::write_executable;

static NEXT: AtomicUsize = AtomicUsize::new(0);

const VARIABLE: &str = "QCORE_LIBRARY_VERSION_COMPILE_DATABASE";
const PLUGIN: &str = "fakelib.reset_check";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_ok() {
        return true;
    }
    assert!(
        std::env::var_os("QCORE_CLANG").is_none(),
        "explicit QCORE_CLANG is invalid"
    );
    eprintln!("skipping Clang-dependent library version hold test");
    false
}

/// The ranged function's caller, and a second unit so a database can
/// configure two units differently. The caller's unit also declares
/// `fakelib_size` with a result its contract disagrees with, so the unit's
/// ABI gate withdraws that declaration (rule 12-series LOADER-ABI) and the
/// unit is classified under the gate's reduced environment, as the
/// librdkafka unit of the registering card's run was.
const UNITS: [(&str, &str); 2] = [
    (
        "src/reset_user.c",
        "void fakelib_reset(void);\nlong fakelib_size(void);\n\nvoid use_reset(void) {\n    \
         fakelib_reset();\n}\n",
    ),
    ("src/other.c", "int other(int x) {\n    return x + 1;\n}\n"),
];

fn driver() -> String {
    qcore_c_gateway::clang_command()
        .unwrap()
        .get_program()
        .to_string_lossy()
        .into_owned()
}

fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// One way the audited headers stand against the lock (pinned at 2.5.0, the
/// `bundled` set; the declaration's range is `since "2.0"`).
#[derive(Clone, Copy, Debug)]
enum Standing {
    InForce,
    Changed,
    OutOfRange,
    Unreadable,
    Ambiguous,
}

impl Standing {
    const ALL: [Self; 5] = [
        Self::InForce,
        Self::Changed,
        Self::OutOfRange,
        Self::Unreadable,
        Self::Ambiguous,
    ];

    /// The compile database the run probes.
    fn database(self) -> &'static str {
        match self {
            Self::InForce => "in-force.json",
            Self::Changed => "changed.json",
            Self::OutOfRange => "out-of-range.json",
            Self::Unreadable => "unreadable.json",
            Self::Ambiguous => "ambiguous.json",
        }
    }

    /// The version record's reason, `None` in force.
    fn reason(self) -> Option<&'static str> {
        match self {
            Self::InForce => None,
            Self::Changed => Some("library-version-changed"),
            Self::OutOfRange => Some("library-version-out-of-range"),
            Self::Unreadable => Some("library-version-unreadable"),
            Self::Ambiguous => Some("library-version-ambiguous"),
        }
    }

    /// The published status of `use_reset` (with the binding's `Pending`).
    fn status(self) -> &'static str {
        match self {
            Self::InForce | Self::Changed => "Pending",
            Self::OutOfRange | Self::Unreadable | Self::Ambiguous => "Unknown",
        }
    }

    /// The refusal the held call carries, `None` in force.
    fn hold(self) -> Option<String> {
        match self {
            Self::InForce => None,
            Self::Changed => Some(hold_description("fakelib_reset")),
            other => Some(library_version_hold_description(
                "fakelib_reset",
                other.reason().unwrap(),
            )),
        }
    }
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-library-version-hold-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let fixture = Self(root.clone());
        let checkout = root.join("checkout");
        fs::create_dir_all(checkout.join("src")).unwrap();
        for (path, text) in UNITS {
            fs::write(checkout.join(path), text).unwrap();
        }
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "src"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Library Version Fixture",
                "-c",
                "user.email=library-version@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Library version hold fixture",
            ],
        );
        let commit = git(&checkout, &["rev-parse", "HEAD"]);
        // The pinned set, a changed one inside the range, one below `since`,
        // and a disagreeing one.
        fixture.header("bundled", 2, 5, 0);
        fixture.header("changed", 2, 6, 0);
        fixture.header("old", 1, 9, 0);
        fixture.header("other", 3, 0, 0);
        let krate = root.join("user/quirin-provider-fakelib");
        fs::create_dir_all(krate.join("src")).unwrap();
        fs::create_dir_all(krate.join("contracts/fakelib")).unwrap();
        fs::write(
            krate.join("Cargo.toml"),
            "[package]\nname = \"quirin-provider-fakelib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(
            krate.join("src/lib.rs"),
            "pub const NAMESPACE: &str = \"fakelib\";\npub const PUBLISHED: &[&str] = &[];\n",
        )
        .unwrap();
        fs::write(
            krate.join("library-versions.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "libraries": {"fakelib": {"header": "fakelib/fakelibv.h",
                    "major": "FAKELIB_VERSION_MAJOR", "minor": "FAKELIB_VERSION_MINOR",
                    "patch": "FAKELIB_VERSION_PATCH"}},
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            krate.join("contracts/fakelib/reset.qv"),
            "module contract;\n\nextern fn fakelib_reset() -> unit\ncontract {\n    \
             requires resource_contract_v1;\n    requires resource_revision(flexible);\n    \
             requires resource_library_version(fakelib, since \"2.0\");\n    \
             ensures normal resource_return_none();\n};\n",
        )
        .unwrap();
        // Unranged, unbound, and contradicted by the caller's unit.
        fs::write(
            krate.join("contracts/fakelib/size.qv"),
            "module contract;\n\nextern fn fakelib_size() -> u32\ncontract {\n    \
             requires resource_contract_v1;\n    requires resource_revision(flexible);\n    \
             ensures normal resource_return_none();\n};\n",
        )
        .unwrap();
        fs::create_dir_all(krate.join("checkers/reset-check")).unwrap();
        fs::write(
            fixture.load_path(),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "extends": repository().join("providers/contract-load-path.json"),
                "crates": [{
                    "name": "quirin-provider-fakelib", "namespace": "fakelib",
                    "version": "0.1.0", "contracts": "quirin-provider-fakelib/contracts",
                    "dependencies": [], "modules": ["fakelib/reset.qv", "fakelib/size.qv"], "published": {},
                }],
                "overrides": [],
                "checkers": [{"crate": "quirin-provider-fakelib", "module": "fakelib/reset.qv",
                              "functions": ["fakelib_reset"],
                              "package": "quirin-provider-fakelib/checkers/reset-check"}],
            }))
            .unwrap(),
        )
        .unwrap();
        succeeded(&fixture.quirin(&[
            "plugin",
            "views",
            "--load-path",
            "user/user-load-path.json",
            "--write",
        ]));
        fixture.install_checker();
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        fs::write(
            profile.join("profile.json"),
            json!({
                "schema_version": 1, "source_set": "sample",
                "pins": {"sample": {"git_commit": commit, "clean_paths": [],
                                    "description": "library version hold fixture"}},
                "libraries": {"lib": {
                    "pin": "sample", "source_roots": [{"root": "src", "selection": "top_level"}],
                    "audited_files": UNITS.iter().map(|(path, text)| json!({
                        "path": path, "sha256": sha256_hex(text.as_bytes())})).collect::<Vec<_>>(),
                    "source_witnesses": [], "permitted_mechanisms": [], "operation_summaries": {},
                    "api_dispositions": {}, "accepted_builds": [], "required_evidence": [],
                    "typed_import": null, "callback_summaries": {}, "boolean_field_models": [],
                    "provider": null,
                }},
                "resource_contract_manifest": null,
                "provider_manifests": [],
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version": 1, "libraries": [{"library": "lib", "label": null}]})
                .to_string(),
        )
        .unwrap();
        fixture.database(Standing::InForce.database(), &[&["-Ibundled"]]);
        fixture.database(Standing::Changed.database(), &[&["-Ichanged"]]);
        fixture.database(Standing::OutOfRange.database(), &[&["-Iold"]]);
        // No configuration finds the version header (the registering card's
        // unreadable runs: the libcurl headers off every searched path).
        fixture.database(Standing::Unreadable.database(), &[&[]]);
        fixture.database(
            Standing::Ambiguous.database(),
            &[&["-Ibundled"], &["-Iother"]],
        );
        fixture
    }

    fn load_path(&self) -> PathBuf {
        self.0.join("user/user-load-path.json")
    }

    fn header(&self, directory: &str, major: u32, minor: u32, patch: u32) {
        let path = self.0.join(directory).join("fakelib");
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("fakelibv.h"),
            format!(
                "#define FAKELIB_VERSION_MAJOR {major}\n#define FAKELIB_VERSION_MINOR \
                 {minor}\n#define FAKELIB_VERSION_PATCH {patch}\n"
            ),
        )
        .unwrap();
    }

    /// A compile database of both units: unit `i` takes flag list `i`, or the
    /// last one.
    fn database(&self, name: &str, configurations: &[&[&str]]) {
        let driver = driver();
        let entries = UNITS
            .iter()
            .enumerate()
            .map(|(index, (path, _))| {
                let flags = configurations[index.min(configurations.len() - 1)];
                let file = self.0.join("checkout").join(path);
                let mut arguments = vec![driver.clone(), "-std=c11".to_owned()];
                arguments.extend(flags.iter().map(|flag| (*flag).to_owned()));
                arguments.extend(["-c".to_owned(), file.display().to_string()]);
                json!({"directory": self.0, "file": file, "arguments": arguments})
            })
            .collect::<Vec<_>>();
        fs::write(self.0.join(name), serde_json::to_vec(&entries).unwrap()).unwrap();
    }

    /// The checker package: answers `Safe` without evidence, which the
    /// firewall classifies `Pending` (`plugin-claim-uncertified`).
    fn install_checker(&self) {
        let package = self
            .0
            .join("user/quirin-provider-fakelib/checkers/reset-check");
        let template = manifest();
        let mut bytes =
            b"#!/bin/sh\n/bin/cat > /dev/null\n/bin/cat response.bin\nexit $?\n".to_vec();
        bytes.extend(envelope(&serde_json::to_string(&template).unwrap()).into_bytes());
        bytes.push(b'\n');
        write_executable(&package.join("check.sh"), &bytes);
        let mut installed = template;
        installed.package_digest = sha256_hex(&bytes);
        fs::write(
            package.join("verifier-plugin.json"),
            serde_json::to_vec(&installed).unwrap(),
        )
        .unwrap();
        let loaded = load_contract_load_path_manifest(&self.load_path()).unwrap();
        let task = checker_task(&loaded.extension.unwrap().checkers[0]);
        let request = checker_request(&installed.plugin_id, &task);
        let response = VerificationResponseV2 {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Safe,
            diagnostics: Vec::new(),
            evidence: None,
        };
        fs::write(
            package.join("response.bin"),
            encode_protocol_frame(&response, DEFAULT_MAX_FRAME_BYTES).unwrap(),
        )
        .unwrap();
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quirin-audit"));
        command
            .current_dir(&self.0)
            .env_remove(VARIABLE)
            .env_remove("C_INCLUDE_PATH");
        command
    }

    fn quirin(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn json(&self, relative: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(relative)).unwrap()).unwrap()
    }

    fn pin(&self) {
        succeeded(&self.quirin(&[
            "pin",
            "--load-path",
            "user/user-load-path.json",
            "--source-root",
            "checkout",
            "--compile-database",
            Standing::InForce.database(),
        ]));
        assert_eq!(
            self.json("user/quirin-provider-fakelib/contracts.lock")["library_versions"]["fakelib"]
                ["version"],
            "2.5.0"
        );
    }

    /// The current route over the standing's database.
    fn current(&self, standing: Standing) -> Run {
        let out = format!("current-{standing:?}");
        succeeded(
            &self
                .command()
                .args([
                    "current",
                    "--checkout",
                    "checkout",
                    "--profile-directory",
                    "profiles/sample",
                    "--compile-database",
                    standing.database(),
                    "--load-path",
                    "user/user-load-path.json",
                    "--provider",
                    "quirin-provider-fakelib",
                    "--report",
                    &format!("{out}/report.json"),
                    "--monkey-source-report",
                    &format!("{out}/monkey-source.json"),
                    "--jobs",
                    "1",
                ])
                .output()
                .unwrap(),
        );
        Run::read(self, &out, "report.contracts.json")
    }

    /// The configured route over the standing's database.
    fn configured(&self, standing: Standing) -> Run {
        self.configured_as("configured", standing, |_| {})
    }

    /// The configured route over the standing's database, its configuration
    /// edited by `edit` (card CONFIGURED-ROUTE-PLUGIN-BINDINGS: the other
    /// ways a configuration resolves a user load path).
    fn configured_as(&self, name: &str, standing: Standing, edit: impl FnOnce(&mut Value)) -> Run {
        let out = format!("{name}-{standing:?}");
        fs::create_dir_all(self.0.join(&out)).unwrap();
        let config = format!("{out}/audit.json");
        let mut configuration = json!({
            "schema_version": 1,
            "profile_directory": "../profiles/sample",
            "checkout": "../checkout",
            "compile_database": format!("../{}", standing.database()),
            "libraries": ["lib"],
            "report": "report.json",
            "source_revision_policy": "checkout_head",
            "load_path": "../user/user-load-path.json",
            "providers": ["quirin-provider-fakelib"],
            "jobs": 1,
        });
        edit(&mut configuration);
        fs::write(self.0.join(&config), configuration.to_string()).unwrap();
        succeeded(&self.quirin(&[&config]));
        Run::read(self, &out, "report.assumptions.json")
    }

    /// Card CONFIGURED-ROUTE-PLUGIN-BINDINGS: a hand-written schema-2
    /// manifest naming the user load path (the selection the provider route
    /// lowers to), and a profile whose default it is.
    fn write_manifest(&self) {
        fs::write(
            self.manifest(),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 2,
                "load_path": "user-load-path.json",
                "modules": ["quirin-provider-fakelib:fakelib/reset.qv",
                            "quirin-provider-fakelib:fakelib/size.qv"],
            }))
            .unwrap(),
        )
        .unwrap();
        let sample = self.0.join("profiles/sample");
        let profile = self.0.join("profiles-manifest/sample");
        fs::create_dir_all(&profile).unwrap();
        let mut document = self.json("profiles/sample/profile.json");
        document["resource_contract_manifest"] = json!("../../user/manifest.json");
        fs::write(profile.join("profile.json"), document.to_string()).unwrap();
        fs::copy(sample.join("audit.json"), profile.join("audit.json")).unwrap();
    }

    fn manifest(&self) -> PathBuf {
        self.0.join("user/manifest.json")
    }

    /// The current route with `--contract-manifest`.
    fn current_manifest(&self, standing: Standing) -> Run {
        let out = format!("current-manifest-{standing:?}");
        succeeded(
            &self
                .command()
                .args([
                    "current",
                    "--checkout",
                    "checkout",
                    "--profile-directory",
                    "profiles/sample",
                    "--compile-database",
                    standing.database(),
                    "--contract-manifest",
                    "user/manifest.json",
                    "--report",
                    &format!("{out}/report.json"),
                    "--monkey-source-report",
                    &format!("{out}/monkey-source.json"),
                    "--jobs",
                    "1",
                ])
                .output()
                .unwrap(),
        );
        Run::read(self, &out, "report.contracts.json")
    }

    /// The contracts route with `--contract-manifest`.
    fn contracts_manifest(&self, standing: Standing) -> Run {
        let out = format!("contracts-manifest-{standing:?}");
        fs::create_dir_all(self.0.join(&out)).unwrap();
        succeeded(
            &self
                .command()
                .env(VARIABLE, self.0.join(standing.database()))
                .args([
                    "contracts",
                    "--source-root",
                    "checkout/src",
                    "--contract-manifest",
                    "user/manifest.json",
                    "--report",
                    &format!("{out}/report.json"),
                ])
                .output()
                .unwrap(),
        );
        Run::read(self, &out, "report.contracts.json")
    }

    /// The contracts route (no database of its own), probing the standing's
    /// database through the explicit override.
    fn contracts(&self, standing: Standing) -> Run {
        let out = format!("contracts-{standing:?}");
        fs::create_dir_all(self.0.join(&out)).unwrap();
        succeeded(
            &self
                .command()
                .env(VARIABLE, self.0.join(standing.database()))
                .args([
                    "contracts",
                    "--source-root",
                    "checkout/src",
                    "--load-path",
                    "user/user-load-path.json",
                    "--provider",
                    "quirin-provider-fakelib",
                    "--report",
                    &format!("{out}/report.json"),
                ])
                .output()
                .unwrap(),
        );
        Run::read(self, &out, "report.contracts.json")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// What one run published for `use_reset`, and the declaration's record.
struct Run {
    status: String,
    diagnostics: BTreeSet<String>,
    links: Value,
    record: Value,
    plugin_checks: Value,
}

impl Run {
    fn read(fixture: &Fixture, out: &str, sidecar: &str) -> Self {
        let report = fixture.json(&format!("{out}/report.json"));
        let row = report["units"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|unit| unit["functions"].as_array().unwrap())
            .find(|row| row["function"] == "use_reset")
            .unwrap_or_else(|| panic!("no use_reset row: {report}"))
            .clone();
        let sidecar = fixture.json(&format!("{out}/{sidecar}"));
        Self {
            status: row["status"].as_str().unwrap().to_owned(),
            diagnostics: row["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|diagnostic| diagnostic["description"].as_str())
                .map(str::to_owned)
                .collect(),
            links: row["plugin_links"].clone(),
            record: sidecar["contract_revision"]["library_versions"]["declarations"][0].clone(),
            plugin_checks: sidecar["plugin_checks"]["counts"].clone(),
        }
    }

    /// The run published what rule 53 says for `standing`.
    fn check(&self, route: &str, standing: Standing) {
        let context = format!(
            "{route} {standing:?}: status {} diagnostics {:?} links {} record {}",
            self.status, self.diagnostics, self.links, self.record
        );
        assert_eq!(self.record["function"], "fakelib_reset", "{context}");
        assert_eq!(
            self.record["in_force"],
            standing.reason().is_none(),
            "{context}"
        );
        assert_eq!(
            self.record["reason"].as_str(),
            standing.reason(),
            "{context}"
        );
        // Every route runs the binding (card CONFIGURED-ROUTE-PLUGIN-BINDINGS
        // brought the configured route to parity: it published the in-force
        // row `Safe`, with no `plugin_checks` section and no link), and it
        // answered an uncertified `Safe`.
        assert_eq!(self.plugin_checks["Pending"], 1, "{context}");
        assert_eq!(self.status, standing.status(), "{context}");
        match standing.hold() {
            None => {
                // In force: the row applies the bound contract, and the
                // binding's `Pending` is the row's.
                assert!(self.diagnostics.is_empty(), "{context}");
                assert_eq!(self.links[0]["verdict"], "Pending", "{context}");
            }
            Some(hold) => {
                // Held: the call applies no contract, so no link; the
                // refusal names the reason it is held for, and the
                // human-resolvable text only for a changed version.
                assert!(self.diagnostics.contains(&hold), "{context}");
                let human = hold_description("fakelib_reset");
                assert_eq!(
                    self.diagnostics.contains(&human),
                    matches!(standing, Standing::Changed),
                    "{context}"
                );
                assert!(self.links.is_null(), "{context}");
            }
        }
    }
}

fn succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn manifest() -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new(PLUGIN).unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([PLUGIN_PROTOCOL_VERSION]),
        evidence_versions: BTreeSet::from([PLUGIN_EVIDENCE_VERSION_V2]),
        entrypoint: PluginEntrypoint::Executable {
            path: "check.sh".into(),
        },
        package_digest: "0".repeat(64),
        targets: vec![SupportedTarget {
            library: "fakelib".into(),
            revisions: BTreeSet::from(["*".into()]),
            profiles: BTreeSet::from(["user-contract".into()]),
            dialects: BTreeSet::from(["qcore-resource-v1".into()]),
            platforms: BTreeSet::from(["*".into()]),
            backends: BTreeSet::from(["quirin-audit".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ResourceContracts]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::ThirdParty,
        certificate_checker: "fakelib.reset_check.replay".into(),
        lean_theorem_family: "QCore.VerdictSemigroup".into(),
    }
}

/// The current route, the registering card's reproduction in fixture form:
/// at every standing, with the checker binding present.
#[test]
fn the_current_route_holds_by_reason_with_a_checker_binding() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.pin();
    for standing in Standing::ALL {
        fixture.current(standing).check("current", standing);
    }
}

/// The configured route, the same standings.
#[test]
fn the_configured_route_holds_by_reason_with_a_checker_binding() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.pin();
    for standing in Standing::ALL {
        fixture.configured(standing).check("configured", standing);
    }
}

/// The contracts route, the same standings.
#[test]
fn the_contracts_route_holds_by_reason_with_a_checker_binding() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.pin();
    for standing in Standing::ALL {
        fixture.contracts(standing).check("contracts", standing);
    }
}

// Card CONFIGURED-ROUTE-PLUGIN-BINDINGS: the route inventory. Every
// verdict-publishing route that resolves a user load path runs its checker
// bindings through the same code (`user_plugins::run`, the fragments' links,
// `fold_unit`, `fold_dependencies`), or refuses it. The tests below drive each
// way a route resolves one, with the declaration in force (the standing where
// a route that skipped the binding would publish `Safe`).

/// The configured route under per-library provider scoping
/// (`library_providers`, resolved to its union on the user load path).
#[test]
fn the_configured_route_runs_bindings_under_per_library_providers() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.pin();
    fixture
        .configured_as("configured-scoped", Standing::InForce, |configuration| {
            let object = configuration.as_object_mut().unwrap();
            object.remove("providers");
            object.insert(
                "library_providers".into(),
                json!({"lib": {"providers": ["quirin-provider-fakelib"],
                               "dependent_libraries": []}}),
            );
        })
        .check("configured-scoped", Standing::InForce);
}

/// A schema-2 manifest naming the user load path: `current
/// --contract-manifest`, `contracts --contract-manifest`, and the configured
/// route reading it as the profile's default manifest.
#[test]
fn the_manifest_routes_run_the_bindings_of_the_load_path_they_name() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.pin();
    fixture.write_manifest();
    fixture
        .current_manifest(Standing::InForce)
        .check("current-manifest", Standing::InForce);
    fixture
        .contracts_manifest(Standing::InForce)
        .check("contracts-manifest", Standing::InForce);
    fixture
        .configured_as("configured-manifest", Standing::InForce, |configuration| {
            let object = configuration.as_object_mut().unwrap();
            object.remove("providers");
            object.remove("load_path");
            object.insert(
                "profile_directory".into(),
                json!("../profiles-manifest/sample"),
            );
        })
        .check("configured-manifest", Standing::InForce);
}

/// The pinned corpus route runs no checker binding and checkpoints fragments
/// that cannot carry links, so it refuses a manifest whose load path carries
/// one, before any work: nothing is written.
#[test]
fn the_pinned_route_refuses_a_manifest_whose_load_path_carries_bindings() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    fixture.write_manifest();
    let output = fixture.quirin(&[
        "pinned",
        "checkout",
        "absent-monkey.json",
        "absent-monkey-source.json",
        "pinned/report.json",
        "absent-observations",
        "user/manifest.json",
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains(qcore_audit::user_plugins::UNRUN_BINDINGS)
            && stderr.contains("quirin-provider-fakelib:fakelib/reset.qv"),
        "{stderr}"
    );
    assert!(!fixture.0.join("pinned").exists());
}

/// The pinned route's own inputs, and every shipped profile's manifest, name
/// the shipped load path, which carries no binding: the refusal never fires
/// there, and no manifest input carries an extension, so no measured route
/// runs a binding.
#[test]
fn shipped_manifests_carry_no_checker_binding() {
    let mut manifests = vec![repository().join("cycle19-resource-contracts.json")];
    for entry in fs::read_dir(repository().join("contract/profiles")).unwrap() {
        let directory = entry.unwrap().path();
        let Ok(text) = fs::read_to_string(directory.join("profile.json")) else {
            continue;
        };
        let document: Value = serde_json::from_str(&text).unwrap();
        if let Some(relative) = document["resource_contract_manifest"].as_str() {
            manifests.push(directory.join(relative));
        }
    }
    assert!(manifests.len() > 1);
    for manifest in manifests {
        qcore_audit::user_plugins::refuse_unrun_bindings("pinned", &manifest)
            .unwrap_or_else(|error| panic!("{}: {error}", manifest.display()));
        let input = qcore_audit::providers::ContractInput::manifest(&manifest).unwrap();
        assert!(input.extension().is_none(), "{}", manifest.display());
    }
}

/// A manifest naming a user load path that cannot be read for its bindings
/// is never loaded without them: the input refuses to load.
#[test]
fn a_manifest_whose_load_path_cannot_be_read_does_not_load() {
    if !clang_available() {
        return;
    }
    let fixture = Fixture::new();
    let broken = fixture.0.join("user/broken.json");
    fs::write(
        &broken,
        json!({"schema_version": 2, "load_path": "absent-load-path.json",
               "modules": ["quirin-provider-fakelib:fakelib/reset.qv"]})
        .to_string(),
    )
    .unwrap();
    let input = qcore_audit::providers::ContractInput::manifest(&broken).unwrap();
    assert!(input.load().is_err());
    assert!(qcore_audit::user_plugins::refuse_unrun_bindings("pinned", &broken).is_err());
}
