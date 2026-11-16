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

//! Card W48-PLUGIN-VERDICT-PRESERVATION, the checker-plugin route end to end:
//! a test plugin's answer, through the firewall, to the published function
//! and unit rows, the report, the contract sidecar's `plugin_checks`, and the
//! `.qo` checkpoints of `quirin-audit current --checkpoint-dir` and their
//! composition. Every published row is projected back to its contributors
//! (`qcore_audit::plugin_verdicts`), and each plugin's own answer, verdict and
//! reasons come back out.
//!
//! The fixture is `user_contract_plugins`'s (the SDK's example user crate, the
//! audited C, a shell checker package answering a precomputed frame) in a git
//! checkout with a source profile and a compilation database, so the
//! configured route runs. `Pending` and `Unknown` are traced on the command
//! line; `Safe` and `Unsafe` need a trusted-core checker, which only a library
//! caller can supply, so they are traced through the same route's library
//! entry with test doubles of a witness and a counterexample checker — and the
//! same answers on the command line stay `Pending`.
//!
//! Included as a module of `user_contract_plugins`, so it runs in that
//! binary's exclusive slot (`scripts/run_test_binaries.py`).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::checkpoint::{Checkpoint, CheckpointReader, TextFormat, decompose};
use qcore_audit::checkpoint_run::CheckpointOptions;
use qcore_audit::compose::{ComposeOutputs, Console, compose};
use qcore_audit::current_route::{
    CurrentRoute, historical_reference_revision, run_current_route_with_checkpoints,
};
use qcore_audit::evidence::EvidenceMode;
use qcore_audit::plugin_verdicts::{
    Recovered, Source, SourceKind, of_source, project_function, project_unit,
};
use qcore_audit::providers::{ContractInput, SelectionSource, resolve_providers};
use qcore_audit::user_plugins::{PluginCheckers, checker_request, checker_task};
use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::verdict::Verdict;
use qcore_verifier_host::*;
use serde_json::{Value, json};

use super::{
    Replay, copy_example_user_crate, manifest, proposal, repository, succeeded, write_executable,
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const CHECK: &str = "acme.pool_check";
const AUDIT: &str = "acme.pool_audit";

/// The audited units: a caller that takes and returns an item, one that
/// leaks it (the host's own `Unsafe`), and one that applies no contract.
fn units() -> Vec<(&'static str, String)> {
    let header = "void *acme_pool_get(void);\nvoid acme_pool_put(void *item);\n";
    vec![
        (
            "lib/pool_user.c",
            format!(
                "{header}void use_pool(void) {{ void *item = acme_pool_get(); if (!item) return; \
                 acme_pool_put(item); }}\nint no_pool(int x) {{ return x + 1; }}\n"
            ),
        ),
        (
            "lib/leak_user.c",
            format!(
                "{header}void leak_pool(void) {{ void *item = acme_pool_get(); (void)item; }}\n"
            ),
        ),
        ("lib/plain.c", "int plain(int x) { return x * 2; }\n".into()),
    ]
}

/// What a checker package does.
#[derive(Clone, Copy)]
enum Answer {
    /// Propose this verdict, with or without evidence, and one diagnostic of
    /// its own.
    Says(ProposedVerdict, bool),
    /// Exit unsuccessfully without answering.
    Crash,
}

fn git(directory: &Path, args: &[&str]) {
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
}

struct Fixture(PathBuf);

impl Fixture {
    /// A checkout, the example user crate with one checker binding per
    /// plugin id, a profile and a compilation database.
    fn new(driver: &Path, plugins: &[&str]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-plugin-verdicts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let checkout = root.join("checkout");
        for (path, text) in units() {
            let path = checkout.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        git(&checkout, &["init", "--quiet"]);
        git(&checkout, &["add", "lib"]);
        git(
            &checkout,
            &[
                "-c",
                "user.name=Plugin Verdict Fixture",
                "-c",
                "user.email=plugin-verdicts@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "Plugin verdict fixture",
            ],
        );
        let commit = String::from_utf8(
            Command::new("git")
                .current_dir(&checkout)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        copy_example_user_crate(&root.join("user"));
        let crate_root = root.join("user/quirin-provider-acme");
        let checkers = plugins
            .iter()
            .map(|plugin| {
                let package = format!("quirin-provider-acme/checkers/{plugin}");
                fs::create_dir_all(root.join("user").join(&package)).unwrap();
                json!({"crate": "quirin-provider-acme", "module": "acme/pool.qv",
                       "functions": ["acme_pool_get", "acme_pool_put"], "package": package})
            })
            .collect::<Vec<_>>();
        let mut modules = fs::read_dir(crate_root.join("contracts/acme"))
            .unwrap()
            .map(|entry| format!("acme/{}", entry.unwrap().file_name().to_string_lossy()))
            .collect::<Vec<_>>();
        modules.sort();
        fs::write(
            root.join("user/user-load-path.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": 1,
                "extends": repository().join("providers/contract-load-path.json"),
                "crates": [{
                    "name": "quirin-provider-acme", "namespace": "acme", "version": "0.1.0",
                    "contracts": "quirin-provider-acme/contracts", "dependencies": [],
                    "modules": modules, "published": {},
                }],
                "overrides": [],
                "checkers": checkers,
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("contracts.json"),
            json!({"schema_version": 1, "modules": []}).to_string(),
        )
        .unwrap();
        let profile = root.join("profiles/sample");
        fs::create_dir_all(&profile).unwrap();
        fs::write(profile.join("profile.json"), json!({
            "schema_version": 1, "source_set": "sample",
            "pins": {"sample": {"git_commit": commit, "clean_paths": [], "description": "plugin verdict fixture"}},
            "libraries": {"lib": {
                "pin": "sample", "source_roots": [{"root": "lib", "selection": "top_level"}],
                "audited_files": units().iter().map(|(path, text)| json!({"path": path, "sha256": sha256_hex(text.as_bytes())})).collect::<Vec<_>>(),
                "source_witnesses": [], "permitted_mechanisms": [], "operation_summaries": {},
                "api_dispositions": {}, "accepted_builds": [], "required_evidence": [], "typed_import": null,
                "callback_summaries": {}, "boolean_field_models": [], "provider": null,
            }},
            "resource_contract_manifest": "../../contracts.json",
            "provider_manifests": [],
        }).to_string()).unwrap();
        fs::write(
            profile.join("audit.json"),
            json!({"schema_version": 1, "libraries": [{"library": "lib", "label": null}]})
                .to_string(),
        )
        .unwrap();
        fs::write(
            root.join("compile_commands.json"),
            json!(
                units()
                    .iter()
                    .map(|(path, _)| json!({
                        "directory": checkout, "file": path,
                        "arguments": [driver, "-std=c11", "-c", path, "-o", path.replace(".c", ".o")],
                    }))
                    .collect::<Vec<_>>()
            )
            .to_string(),
        )
        .unwrap();
        let fixture = Self(root);
        succeeded(&fixture.quirin(&[
            "plugin",
            "views",
            "--load-path",
            "user/user-load-path.json",
            "--write",
        ]));
        fixture
    }

    fn load_path(&self) -> PathBuf {
        self.0.join("user/user-load-path.json")
    }

    /// Install binding `index`'s package as plugin `plugin`, answering
    /// `answer` against the core's own task.
    fn install(&self, index: usize, plugin: &str, answer: Answer) {
        let package = self
            .0
            .join("user/quirin-provider-acme/checkers")
            .join(plugin);
        let _ = fs::remove_dir_all(&package);
        fs::create_dir_all(&package).unwrap();
        let mut template = manifest();
        template.plugin_id = PluginId::new(plugin).unwrap();
        template.certificate_checker = format!("{plugin}.replay");
        let body = match answer {
            Answer::Says(..) => "/bin/cat > /dev/null\n/bin/cat response.bin",
            Answer::Crash => "/bin/cat > /dev/null\nexit 3",
        };
        let mut bytes = format!("#!/bin/sh\n{body}\nexit $?\n").into_bytes();
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
        if let Answer::Says(verdict, evidence) = answer {
            let manifest = load_contract_load_path_manifest(&self.load_path()).unwrap();
            let task = checker_task(&manifest.extension.unwrap().checkers[index]);
            let request = checker_request(&installed.plugin_id, &task);
            let response = VerificationResponseV2 {
                protocol_version: request.protocol_version,
                request_id: request.request_id.clone(),
                plugin_id: request.plugin_id.clone(),
                verdict,
                diagnostics: vec![PluginDiagnostic {
                    code: format!("{plugin}.observed"),
                    message: format!("{plugin} observed both pool calls"),
                    fact_id: None,
                }],
                evidence: evidence.then(|| proposal(&request, &installed)),
            };
            fs::write(
                package.join("response.bin"),
                encode_protocol_frame(&response, DEFAULT_MAX_FRAME_BYTES).unwrap(),
            )
            .unwrap();
        }
    }

    fn quirin(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quirin-audit"))
            .current_dir(&self.0)
            .env_remove("QCORE_BOUNDS_PROPERTY")
            .env_remove("QCORE_NULL_PROPERTY")
            .env_remove("QCORE_MEMBER_PREPASS_SECTION")
            .args(args)
            .output()
            .unwrap()
    }

    /// `quirin-audit current` over the fixture, its outputs under `out`.
    fn current(&self, out: &str, extra: &[&str]) -> Output {
        let report = format!("{out}/report.json");
        let monkey = format!("{out}/monkey-source.json");
        let mut args = vec![
            "current",
            "--checkout",
            "checkout",
            "--profile-directory",
            "profiles/sample",
            "--compile-database",
            "compile_commands.json",
            "--load-path",
            "user/user-load-path.json",
            "--provider",
            "quirin-provider-acme",
            "--report",
            &report,
            "--monkey-source-report",
            &monkey,
            "--jobs",
            "2",
        ];
        args.extend_from_slice(extra);
        self.quirin(&args)
    }

    /// The same route through its library entry, with trusted-core checkers.
    fn current_library(&self, out: &str, checkpoints: Option<&str>, checkers: PluginCheckers<'_>) {
        let profile = load_profile_directory(&self.0.join("profiles/sample")).unwrap();
        let resolved = resolve_providers(
            &self.load_path(),
            &["quirin-provider-acme".to_owned()],
            false,
            SelectionSource::CommandLine,
        )
        .unwrap();
        let manifest = profile.resource_contract_manifest_path();
        let contracts = ContractInput::providers(&resolved, manifest.as_deref());
        let historical = historical_reference_revision(&profile).unwrap();
        let report = self.0.join(out).join("report.json");
        let monkey = self.0.join(out).join("monkey-source.json");
        let compile_database = self.0.join("compile_commands.json");
        let checkout = self.0.join("checkout");
        let options = checkpoints.map(|directory| CheckpointOptions {
            directory: self.0.join(directory),
            resume: false,
        });
        run_current_route_with_checkpoints(
            &CurrentRoute {
                checkout: &checkout,
                profile: &profile,
                contracts: &contracts,
                mode: EvidenceMode::Configured,
                compile_database: Some(&compile_database),
                monkey_database: None,
                monkey_linked: false,
                monkey_source_report: Some(&monkey),
                diagnostic_report: Some(&report),
                binding_directory: None,
                label: "sample",
                historical_note: "plugin verdict fixture",
                historical_reference_revision: &historical,
                jobs: 2,
                selection: None,
                library_providers: None,
                plugin_checkers: checkers,
            },
            options.as_ref(),
        )
        .unwrap();
    }

    /// The checkpoints a run wrote, from its run record.
    fn checkpoints(&self, directory: &str) -> Vec<PathBuf> {
        self.json(&format!("{directory}/checkpoint-run.json"))["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|group| {
                self.0.join(format!(
                    "{directory}/{}.qo",
                    group["group"].as_str().unwrap()
                ))
            })
            .collect()
    }

    /// Compose a run's checkpoints into `out` (the library composer, which
    /// `quirin-audit checkpoint compose` calls).
    fn compose(&self, directory: &str, out: &str) {
        let report = self.0.join(out).join("report.json");
        let monkey = self.0.join(out).join("monkey-source.json");
        fs::create_dir_all(self.0.join(out)).unwrap();
        compose(
            &self.checkpoints(directory),
            &ComposeOutputs {
                report: Some(&report),
                monkey_source_report: Some(&monkey),
                binding_directory: None,
            },
            Console::TotalsOnly,
        )
        .unwrap();
    }

    fn json(&self, relative: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(relative)).unwrap()).unwrap()
    }

    fn bytes(&self, relative: &str) -> Vec<u8> {
        fs::read(self.0.join(relative)).unwrap()
    }

    /// The report and its contract sidecar are byte-identical in `a` and `b`.
    fn assert_same_outputs(&self, a: &str, b: &str) {
        for name in ["report.json", "report.contracts.json"] {
            assert!(
                self.bytes(&format!("{a}/{name}")) == self.bytes(&format!("{b}/{name}")),
                "{a}/{name} and {b}/{name} differ"
            );
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn clang() -> Option<PathBuf> {
    match qcore_c_gateway::clang_command() {
        Ok(command) => Some(PathBuf::from(command.get_program())),
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent plugin verdict test: {reason}");
            None
        }
    }
}

fn unit<'a>(report: &'a Value, file: &str) -> &'a Value {
    report["units"]
        .as_array()
        .unwrap()
        .iter()
        .find(|unit| {
            unit["file"]
                .as_str()
                .is_some_and(|path| path.ends_with(file))
        })
        .unwrap_or_else(|| panic!("no unit {file}"))
}

fn function<'a>(unit: &'a Value, name: &str) -> &'a Value {
    unit["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["function"] == name)
        .unwrap_or_else(|| panic!("no function {name}"))
}

fn plugin(id: &str) -> Source {
    Source::new(SourceKind::Plugin, id)
}

/// A plugin's recovered records all say the same: its proposal, the
/// firewall's class and code, its own diagnostic, and the published value.
fn assert_plugin(
    records: &[Recovered],
    id: &str,
    proposed: Option<&str>,
    claim: &str,
    code: Option<&str>,
    verdict: Verdict,
) {
    let own = of_source(records, &plugin(id));
    assert!(!own.is_empty(), "no record of {id}: {records:#?}");
    for record in own {
        let answer = record.answer.as_ref().unwrap();
        assert_eq!(answer.proposed.as_deref(), proposed, "{record:#?}");
        assert_eq!(answer.claim, claim, "{record:#?}");
        assert_eq!(answer.code.as_deref(), code, "{record:#?}");
        assert_eq!(record.verdict, verdict, "{record:#?}");
        assert!(!record.reasons.is_empty() || verdict == Verdict::Safe);
        if proposed.is_some() {
            assert_eq!(
                answer.reasons[0].code,
                format!("{id}.observed"),
                "{record:#?}"
            );
        } else {
            assert!(answer.reasons.is_empty());
        }
    }
}

/// Every unit and function row of a report projects to the join of its
/// records (the projection refuses otherwise), and a row no plugin linked
/// has none.
fn assert_every_row_projects(report: &Value) {
    for unit in report["units"].as_array().unwrap() {
        let records = project_unit(unit).unwrap();
        assert_eq!(records.is_empty(), unit.get("host_verdict").is_none());
        for row in unit["functions"].as_array().into_iter().flatten() {
            project_function(row).unwrap();
        }
    }
}

/// `Pending` and `Unknown` on the command line, through checkpoints and their
/// composition; two plugins disagreeing; a failing plugin; a plugin beside a
/// host `Unsafe`.
#[test]
fn pending_and_unknown_survive_checkpoints_and_composition() {
    let Some(clang) = clang() else { return };
    let fixture = Fixture::new(&clang, &[CHECK, AUDIT]);
    fixture.install(0, CHECK, Answer::Says(ProposedVerdict::Safe, false));
    fixture.install(1, AUDIT, Answer::Crash);
    succeeded(&fixture.current("whole", &[]));
    succeeded(&fixture.current("ck", &["--checkpoint-dir", "qo"]));
    fixture.assert_same_outputs("whole", "ck");
    let mut compose = vec!["checkpoint".to_owned(), "compose".to_owned()];
    compose.extend(
        fixture
            .checkpoints("qo")
            .iter()
            .map(|path| path.to_str().unwrap().to_owned()),
    );
    compose.extend(
        [
            "--report",
            "composed/report.json",
            "--monkey-source-report",
            "composed/monkey-source.json",
        ]
        .map(str::to_owned),
    );
    succeeded(&fixture.quirin(&compose.iter().map(String::as_str).collect::<Vec<_>>()));
    fixture.assert_same_outputs("whole", "composed");
    // The checkpoints (one per configuration) hold the records themselves,
    // not only the verdicts: the linked units' verdict members carry them.
    let mut linked = 0;
    for path in fixture.checkpoints("qo") {
        let mut reader = CheckpointReader::open(&path).unwrap();
        reader.verify().unwrap();
        let checkpoint = Checkpoint::from_bytes(&fs::read(&path).unwrap()).unwrap();
        let text = decompose(&checkpoint, TextFormat::Yaml).unwrap();
        let verdicts = &text[text.find("\nverdicts:\n").unwrap()..];
        let verdicts = &verdicts[..verdicts.find("\ncertificates:\n").unwrap()];
        if verdicts.contains("plugin_links") {
            linked += 1;
            for key in [
                "host_status",
                "host_verdict",
                "answer",
                "proposed",
                "QP008",
                "QP006",
            ] {
                assert!(verdicts.contains(key), "{} lacks {key}", path.display());
            }
        }
    }
    assert_eq!(linked, 2, "pool_user.c and leak_user.c");

    for out in ["whole", "composed"] {
        let report = fixture.json(&format!("{out}/report.json"));
        assert_every_row_projects(&report);
        // Two plugins disagreeing beside a host Safe: the uncertified Safe is
        // Pending, the crash Unknown, the function their join; each plugin's
        // own record comes back out.
        let pool = unit(&report, "pool_user.c");
        let use_pool = function(pool, "use_pool");
        assert_eq!(use_pool["host_status"], "Safe");
        assert_eq!(use_pool["status"], "Unknown");
        let records = project_function(use_pool).unwrap();
        assert_eq!(records.len(), 5, "{records:#?}");
        assert_plugin(
            &records,
            CHECK,
            Some("Safe"),
            "uncertified-safe",
            Some("QP008"),
            Verdict::Pending,
        );
        assert_plugin(
            &records,
            AUDIT,
            None,
            "failure",
            Some("QP006"),
            Verdict::Unknown,
        );
        assert!(function(pool, "no_pool").get("plugin_links").is_none());
        // The unit names the deciding (Unknown) link's reason, never the
        // lower Pending link's, and keeps its own verdict.
        assert_eq!(pool["status"], "Unknown");
        assert_eq!(pool["reason_code"], "provider-failure");
        assert_eq!(
            pool["host_verdict"],
            json!({"status": "Safe", "reason": null, "reason_code": null})
        );
        let unit_records = project_unit(pool).unwrap();
        assert_eq!(
            of_source(&unit_records, &plugin(CHECK)).len(),
            2,
            "one per applied callee"
        );
        // A plugin beside a host Unsafe never lowers it.
        let leak = unit(&report, "leak_user.c");
        let leak_pool = function(leak, "leak_pool");
        assert_eq!(leak_pool["host_status"], "Unsafe");
        assert_eq!(leak_pool["status"], "Unsafe");
        assert_eq!(leak["status"], "Unsafe");
        assert_eq!(leak["host_verdict"]["status"], "Unsafe");
        assert_eq!(leak["reason"], leak["host_verdict"]["reason"]);
        assert_plugin(
            &project_unit(leak).unwrap(),
            AUDIT,
            None,
            "failure",
            Some("QP006"),
            Verdict::Unknown,
        );
        // A unit no plugin linked carries nothing new.
        let plain = unit(&report, "plain.c");
        assert_eq!(plain["status"], "Safe");
        assert!(plain.get("host_verdict").is_none());
        // The sidecar's bindings keep each plugin's own answer.
        let sidecar = fixture.json(&format!("{out}/report.contracts.json"));
        let bindings = sidecar["plugin_checks"]["bindings"].as_array().unwrap();
        assert_eq!(bindings[0]["plugin_id"], CHECK);
        assert_eq!(bindings[0]["verdict"], "Pending");
        assert_eq!(bindings[0]["answer"]["proposed"], "Safe");
        assert_eq!(
            bindings[0]["answer"]["reasons"][0]["code"],
            format!("{CHECK}.observed")
        );
        assert_eq!(bindings[1]["plugin_id"], AUDIT);
        assert_eq!(bindings[1]["verdict"], "Unknown");
        assert_eq!(bindings[1]["answer"]["proposed"], Value::Null);
        assert_eq!(bindings[1]["answer"]["claim"], "failure");
    }
}

/// A test double of a trusted-core counterexample checker: it shows violated
/// exactly the obligations whose function the witness observed.
struct Violation;

impl CounterexampleChecker for Violation {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        Replay.check(task, witness)
    }
}

/// `Safe` and `Unsafe` need a trusted checker: through the library entry
/// with test doubles they reach the rows, the sidecar and the checkpoints'
/// composition as `Safe` and provisional `Unsafe`; the same answers on the
/// command line, which has no checker, stay `Pending`.
#[test]
fn certified_safe_and_validated_unsafe_survive_checkpoints_and_composition() {
    let Some(clang) = clang() else { return };
    let checkers = PluginCheckers {
        witness: Some(&Replay),
        counterexample: Some(&Violation),
    };
    for (proposed, published, claim) in [
        (ProposedVerdict::Safe, Verdict::Safe, "certified-safe"),
        (ProposedVerdict::Unsafe, Verdict::Unsafe, "validated-unsafe"),
    ] {
        let fixture = Fixture::new(&clang, &[CHECK]);
        fixture.install(0, CHECK, Answer::Says(proposed, true));
        fixture.current_library("whole", None, checkers);
        fixture.current_library("ck", Some("qo"), checkers);
        fixture.assert_same_outputs("whole", "ck");
        fixture.compose("qo", "composed");
        fixture.assert_same_outputs("whole", "composed");
        let spelling = match proposed {
            ProposedVerdict::Safe => "Safe",
            _ => "Unsafe",
        };
        for out in ["whole", "composed"] {
            let report = fixture.json(&format!("{out}/report.json"));
            assert_every_row_projects(&report);
            let pool = unit(&report, "pool_user.c");
            let use_pool = function(pool, "use_pool");
            assert_eq!(use_pool["host_status"], "Safe");
            assert_eq!(use_pool["status"], published.as_str());
            assert_eq!(pool["status"], published.as_str());
            let records = project_unit(pool).unwrap();
            assert_plugin(&records, CHECK, Some(spelling), claim, None, published);
            if published == Verdict::Unsafe {
                // A validated plugin Unsafe beside a host Safe: the unit
                // names the plugin's violation, under no §L21.1 category.
                assert!(
                    pool["reason"]
                        .as_str()
                        .unwrap()
                        .contains("counterexample the audit validated"),
                    "{pool:#}"
                );
                assert_eq!(pool["reason_code"], Value::Null);
            }
            let sidecar = fixture.json(&format!("{out}/report.contracts.json"));
            let binding = &sidecar["plugin_checks"]["bindings"][0];
            assert_eq!(binding["claim"], claim);
            assert_eq!(binding["answer"]["proposed"], spelling);
        }
        // The command line has no trusted checker: the same package and
        // answer are Pending there, never Safe or Unsafe.
        succeeded(&fixture.current("cli", &[]));
        let report = fixture.json("cli/report.json");
        assert_every_row_projects(&report);
        let pool = unit(&report, "pool_user.c");
        assert_eq!(pool["status"], "Pending");
        assert_eq!(pool["reason_code"], "plugin-claim-uncertified");
        let uncertified = match proposed {
            ProposedVerdict::Safe => ("uncertified-safe", "QP016"),
            _ => ("unvalidated-unsafe", "QP010"),
        };
        assert_plugin(
            &project_function(function(pool, "use_pool")).unwrap(),
            CHECK,
            Some(spelling),
            uncertified.0,
            Some(uncertified.1),
            Verdict::Pending,
        );
    }
}
