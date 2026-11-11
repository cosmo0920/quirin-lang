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

//! Card W45-USER-CONTRACT-PLUGINS, end to end: a user contract crate outside
//! the repository on a user load path that extends the shipped one, a checker
//! plugin attached to its contracts running as a real process package, and
//! the four-value verdict through `quirin-audit contracts`.
//!
//! The user crate is the SDK's example (`providers/quirin-provider-plugin/
//! tests/fixtures/example-user-crate/quirin-provider-acme`), copied into a
//! scratch tree with the audited C (`tests/fixtures/user_plugins/`). The
//! checker is a shell package that answers a response frame the test
//! computes; its manifest template follows an `exit`, so it is bound to the
//! executable exactly as a built provider's is. Every scenario reads what the
//! command line wrote; the certified scenario runs the library route with a
//! small test double of a trusted-core witness checker, because the command
//! line has none.

// The checker fixtures execute /bin/sh scripts and require Unix permissions.
#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_audit::no_build::{AssumedConfiguration, NoBuildAudit, select_units};
use qcore_audit::providers::{ContractInput, SelectionSource, resolve_providers};
use qcore_audit::user_plugins::{PluginCheckers, checker_request, checker_task};
use qcore_contract::load_path_manifest::load_contract_load_path_manifest;
use qcore_schemas::verdict::Verdict;
use qcore_verifier_host::*;
use serde_json::{Value, json};

// The checker executable is written by a separate process, so a sibling
// test's concurrent spawn cannot make its `execve` fail with `ETXTBSY`
// (`tests/support/process_fixture.rs`). Data files stay `fs::write`.
#[path = "../../../tests/support/process_fixture.rs"]
mod process_fixture;
use process_fixture::write_executable;

static NEXT: AtomicUsize = AtomicUsize::new(0);
const PLUGIN: &str = "acme.pool_check";

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
    eprintln!("skipping Clang-dependent user-contract plugin test");
    false
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::write(&target, fs::read(entry.path()).unwrap()).unwrap();
        }
    }
}

/// What the checker package does.
enum Checker {
    /// Answer with this proposal (computed against the core's own task).
    Answer(ProposedVerdict, bool),
    /// Sleep past its declared deadline.
    Sleep,
    /// Exit unsuccessfully without answering.
    Crash,
}

struct Tree(PathBuf);

impl Tree {
    /// `src/` (the audited C), `user/quirin-provider-acme/` (the SDK's example
    /// crate) and `user/user-load-path.json`, extending the shipped load path.
    fn new(checkers: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-user-plugins-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        copy_tree(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/user_plugins/src"),
            &root.join("src"),
        );
        copy_tree(
            &repository()
                .join("providers/quirin-provider-plugin/tests/fixtures/example-user-crate"),
            &root.join("user"),
        );
        let tree = Self(root);
        succeeded(&tree.write_load_path(checkers, &[]));
        tree
    }

    /// The user load path `quirin-plugin load-path` writes for the example
    /// crate, with or without its checker binding, and extra overrides.
    /// Returns `quirin-audit plugin views --write`'s output.
    fn write_load_path(&self, checkers: bool, overrides: &[Value]) -> Output {
        let crate_root = self.0.join("user/quirin-provider-acme");
        let mut modules = Vec::new();
        for entry in fs::read_dir(crate_root.join("contracts/acme")).unwrap() {
            modules.push(format!(
                "acme/{}",
                entry.unwrap().file_name().to_string_lossy()
            ));
        }
        modules.sort();
        let load_path = json!({
            "schema_version": 1,
            "extends": repository().join("providers/contract-load-path.json"),
            "crates": [{
                "name": "quirin-provider-acme", "namespace": "acme", "version": "0.1.0",
                "contracts": "quirin-provider-acme/contracts", "dependencies": [],
                "modules": modules, "published": {},
            }],
            "overrides": overrides,
            "checkers": if checkers {
                json!([{"crate": "quirin-provider-acme", "module": "acme/pool.qv",
                        "functions": ["acme_pool_get", "acme_pool_put"],
                        "package": "quirin-provider-acme/checkers/pool-check"}])
            } else {
                json!([])
            },
        });
        fs::create_dir_all(crate_root.join("checkers/pool-check")).unwrap();
        fs::write(
            self.load_path(),
            serde_json::to_vec_pretty(&load_path).unwrap(),
        )
        .unwrap();
        self.quirin(&[
            "plugin",
            "views",
            "--load-path",
            "user/user-load-path.json",
            "--write",
        ])
    }

    fn load_path(&self) -> PathBuf {
        self.0.join("user/user-load-path.json")
    }

    fn package(&self) -> PathBuf {
        self.0.join("user/quirin-provider-acme/checkers/pool-check")
    }

    /// Install the checker package, answering as `checker` says.
    fn install(&self, checker: &Checker) {
        let package = self.package();
        let _ = fs::remove_dir_all(&package);
        fs::create_dir_all(&package).unwrap();
        let mut template = manifest();
        let body = match checker {
            Checker::Answer(..) => "/bin/cat > /dev/null\n/bin/cat response.bin",
            Checker::Sleep => {
                template.limits.timeout_ms = 300;
                "/bin/cat > /dev/null\nsleep 5"
            }
            Checker::Crash => "/bin/cat > /dev/null\nexit 3",
        };
        let mut bytes = format!("#!/bin/sh\n{body}\nexit $?\n").into_bytes();
        bytes.extend(envelope(&serde_json::to_string(&template).unwrap()).into_bytes());
        bytes.push(b'\n');
        let executable = package.join("check.sh");
        write_executable(&executable, &bytes);
        let mut installed = template;
        installed.package_digest = sha256_hex(&bytes);
        fs::write(
            package.join("verifier-plugin.json"),
            serde_json::to_vec(&installed).unwrap(),
        )
        .unwrap();
        // The answer is data beside the executable, outside its digest, so
        // evidence can bind the installed package's real identity.
        if let Checker::Answer(verdict, evidence) = checker {
            let manifest = load_contract_load_path_manifest(&self.load_path()).unwrap();
            let task = checker_task(&manifest.extension.unwrap().checkers[0]);
            let request = checker_request(&installed.plugin_id, &task);
            let response = VerificationResponseV2 {
                protocol_version: request.protocol_version,
                request_id: request.request_id.clone(),
                plugin_id: request.plugin_id.clone(),
                verdict: *verdict,
                diagnostics: vec![],
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
            .args(args)
            .output()
            .unwrap()
    }

    fn audit(&self, report: &str) -> Output {
        self.quirin(&[
            "contracts",
            "--source-root",
            "src",
            "--load-path",
            "user/user-load-path.json",
            "--provider",
            "quirin-provider-acme",
            "--report",
            report,
        ])
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }

    fn unit<'a>(report: &'a Value, file: &str) -> &'a Value {
        report["units"]
            .as_array()
            .unwrap()
            .iter()
            .find(|unit| unit["file"] == file)
            .unwrap()
    }

    fn status(unit: &Value, function: &str) -> String {
        unit["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["function"] == function)
            .unwrap()["status"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
            library: "acme".into(),
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
        certificate_checker: "acme.pool_check.replay".into(),
        lean_theorem_family: "QCore.VerdictSemigroup".into(),
    }
}

/// Evidence observing both bound functions, bound to the request's task.
fn proposal(request: &VerificationRequestV2, template: &PluginManifest) -> ProposedEvidenceV2 {
    ProposedEvidenceV2 {
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        unit: request.unit.clone(),
        capability: request.capability,
        coverage: request
            .obligations
            .iter()
            .map(|obligation| obligation.obligation_id.clone())
            .collect(),
        assumptions_used: request.assumptions.clone(),
        reproducibility: Reproducibility {
            provider_version: template.package_version.clone(),
            package_digest: template.package_digest.clone(),
            deterministic: true,
        },
        witness: SemanticWitness::ContractObservation(ContractObservationWitness {
            task_digest: request.task_digest.clone(),
            runs: vec![ObservedRun {
                run_id: "run".into(),
                binding: NativeRunBinding {
                    emitted_source_digest: "a".repeat(64),
                    executable_digest: "b".repeat(64),
                    compiler_invocation: "cc pool.c".into(),
                    exit_status: 0,
                    seed_or_input: "fixture".into(),
                    profile: "default".into(),
                    instrumentation_version: "v1".into(),
                },
                observations: ["acme_pool_get", "acme_pool_put"]
                    .into_iter()
                    .map(|function| ContractObservationRecord {
                        function: function.into(),
                        hook: function.into(),
                        values: BTreeMap::from([("ok".into(), 1)]),
                    })
                    .collect(),
            }],
        }),
    }
}

/// Every analysed unit's status is the fold of its function statuses joined
/// with its own (the callee chain roll-up).
fn assert_rollup(report: &Value) {
    for unit in report["units"].as_array().unwrap() {
        let Ok(status) = unit["status"]
            .as_str()
            .unwrap_or_default()
            .parse::<Verdict>()
        else {
            continue;
        };
        let functions = Verdict::fold(
            unit["functions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row["status"].as_str().unwrap().parse::<Verdict>().unwrap()),
        );
        assert!(status >= functions, "{unit}");
        if unit["reason"].is_null() || unit["reason_code"] == "plugin-claim-uncertified" {
            assert_eq!(status, functions, "{unit}");
        }
    }
}

#[test]
fn without_a_checker_the_user_contracts_audit_as_any_contracts_do() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(false);
    succeeded(&tree.audit("r.json"));
    let report = tree.read("r.json");
    let unit = Tree::unit(&report, "pool_user.c");
    assert_eq!(unit["status"], "Safe");
    assert_eq!(Tree::status(unit, "use_pool"), "Safe");
    let sidecar = tree.read("r.contracts.json");
    let user = &sidecar["provider_selection"]["user_load_path"];
    assert_eq!(user["user_crates"], json!(["quirin-provider-acme"]));
    assert_eq!(user["overrides_applied"], json!([]));
    assert!(sidecar.get("plugin_checks").is_none());
    assert_rollup(&report);
}

#[test]
fn an_uncertified_safe_claim_is_pending_and_the_chain_folds() {
    if !clang_available() {
        return;
    }
    for evidence in [false, true] {
        let tree = Tree::new(true);
        tree.install(&Checker::Answer(ProposedVerdict::Safe, evidence));
        let output = tree.audit("r.json");
        succeeded(&output);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("checker plugins: 1 binding(s): 0 Safe / 1 Pending / 0 Unknown"),
            "{stdout}"
        );
        let sidecar = tree.read("r.contracts.json");
        let binding = &sidecar["plugin_checks"]["bindings"][0];
        assert_eq!(binding["claim"], "uncertified-safe");
        assert_eq!(binding["code"], if evidence { "QP016" } else { "QP008" });
        assert_eq!(binding["verdict"], "Pending");
        assert_eq!(binding["reasons"][0]["code"], "plugin-claim-uncertified");
        let report = tree.read("r.json");
        let unit = Tree::unit(&report, "pool_user.c");
        assert_eq!(Tree::status(unit, "use_pool"), "Pending");
        assert_eq!(Tree::status(unit, "no_pool"), "Safe");
        assert_eq!(unit["status"], "Pending");
        assert_eq!(unit["reason_code"], "plugin-claim-uncertified");
        let links = &unit["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["function"] == "use_pool")
            .unwrap()["plugin_links"];
        assert_eq!(links.as_array().unwrap().len(), 2, "{links}");
        assert_eq!(links[0]["plugin"], PLUGIN);
        // A unit that applies no bound contract is untouched.
        let other = Tree::unit(&report, "plain.c");
        assert_eq!(other["status"], "Safe");
        assert!(other["functions"][0].get("plugin_links").is_none());
        assert_rollup(&report);
    }
}

#[test]
fn an_uncertified_unsafe_claim_is_pending_and_never_unsafe() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(true);
    tree.install(&Checker::Answer(ProposedVerdict::Unsafe, false));
    succeeded(&tree.audit("r.json"));
    let binding = &tree.read("r.contracts.json")["plugin_checks"]["bindings"][0];
    assert_eq!(binding["claim"], "unvalidated-unsafe");
    assert_eq!(binding["verdict"], "Pending");
    let report = tree.read("r.json");
    let unit = Tree::unit(&report, "pool_user.c");
    assert_eq!(Tree::status(unit, "use_pool"), "Pending");
    assert_eq!(unit["status"], "Pending");
    assert_rollup(&report);
}

#[test]
fn a_timeout_or_a_crash_is_unknown() {
    if !clang_available() {
        return;
    }
    for checker in [Checker::Sleep, Checker::Crash] {
        let tree = Tree::new(true);
        tree.install(&checker);
        succeeded(&tree.audit("r.json"));
        let binding = &tree.read("r.contracts.json")["plugin_checks"]["bindings"][0];
        assert_eq!(binding["claim"], "failure");
        assert_eq!(binding["code"], "QP006");
        assert_eq!(binding["verdict"], "Unknown");
        let report = tree.read("r.json");
        let unit = Tree::unit(&report, "pool_user.c");
        assert_eq!(Tree::status(unit, "use_pool"), "Unknown");
        assert_eq!(unit["status"], "Unknown");
        assert_eq!(unit["reason_code"], "provider-failure");
        assert_rollup(&report);
    }
    // A package that does not verify is a failure too, and never runs.
    let tree = Tree::new(true);
    tree.install(&Checker::Crash);
    write_executable(&tree.package().join("check.sh"), b"#!/bin/sh\nexit 0\n");
    succeeded(&tree.audit("r.json"));
    let binding = &tree.read("r.contracts.json")["plugin_checks"]["bindings"][0];
    assert_eq!(binding["code"], "package");
    assert_eq!(binding["verdict"], "Unknown");
}

/// A test double of a trusted-core witness checker: it discharges exactly the
/// obligations whose function the witness observed, decided against the
/// core's own obligation list.
struct Replay;

impl WitnessChecker for Replay {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        let SemanticWitness::ContractObservation(witness) = witness else {
            return Err("not an observation witness".into());
        };
        let observed: BTreeSet<&str> = witness
            .runs
            .iter()
            .flat_map(|run| run.observations.iter().map(|o| o.function.as_str()))
            .collect();
        Ok(task
            .obligations
            .iter()
            .filter(|obligation| {
                matches!(&obligation.subject, ObligationSubject::Function { symbol, .. }
                    if observed.contains(symbol.as_str()))
            })
            .map(|obligation| obligation.obligation_id.clone())
            .collect())
    }
}

#[test]
fn a_certified_safe_claim_is_accepted() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(true);
    // The same package and answer the command line reads as `Pending` (QP016:
    // it has no trusted checker); here the library route supplies one.
    tree.install(&Checker::Answer(ProposedVerdict::Safe, true));
    let resolved = resolve_providers(
        &tree.load_path(),
        &["quirin-provider-acme".to_owned()],
        false,
        SelectionSource::CommandLine,
    )
    .unwrap();
    let input = ContractInput::providers(&resolved, None);
    let root = tree.0.join("src");
    let (units, selection) = select_units(&root, &[]).unwrap();
    let assumed = AssumedConfiguration::new(None, Vec::new(), Vec::new()).unwrap();
    let mut audit = NoBuildAudit::with_plugin_checkers(
        &root,
        &selection,
        &input,
        assumed,
        PluginCheckers {
            witness: Some(&Replay),
            counterexample: None,
        },
    )
    .unwrap();
    for unit in &units {
        let outcome = audit.evaluate(unit);
        audit.absorb(outcome.result.unwrap()).unwrap();
    }
    let report_path = tree.0.join("certified.json");
    audit.finish(Some(&report_path)).unwrap();
    let sidecar = tree.read("certified.contracts.json");
    let binding = &sidecar["plugin_checks"]["bindings"][0];
    assert_eq!(binding["claim"], "certified-safe", "{binding}");
    assert_eq!(binding["verdict"], "Safe");
    let report = tree.read("certified.json");
    let unit = Tree::unit(&report, "pool_user.c");
    assert_eq!(Tree::status(unit, "use_pool"), "Safe");
    assert_eq!(unit["status"], "Safe");
    assert_rollup(&report);
}

#[test]
fn a_user_contract_colliding_with_a_shipped_one_is_refused_unless_overridden() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(false);
    fs::write(
        tree.0
            .join("user/quirin-provider-acme/contracts/acme/alloc.qcore"),
        "module contract;\n\nextern fn malloc(size: usize) -> ptr<mut, u8>\ncontract {\n    \
         requires resource_contract_v1;\n    requires resource_revision(flexible);\n    \
         ensures normal resource_borrowed(size);\n    \
         ensures normal resource_return_owned(nullable);\n};\n",
    )
    .unwrap();
    let collision = "user contract `malloc` in `quirin-provider-acme:acme/alloc.qcore` collides \
                     with the shipped contract in `quirin-provider-libc:libc/memory.qv`";
    // Even the view refuses to form while the collision stands.
    let refused = tree.write_load_path(false, &[]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains(collision));
    let override_record = json!({"function": "malloc", "crate": "quirin-provider-acme",
                                 "shipped_crate": "quirin-provider-libc",
                                 "reason": "our allocator wrapper replaces malloc in this build"});
    succeeded(&tree.write_load_path(false, std::slice::from_ref(&override_record)));
    succeeded(&tree.audit("r.json"));
    let applied =
        &tree.read("r.contracts.json")["provider_selection"]["user_load_path"]["overrides_applied"];
    assert_eq!(applied.as_array().unwrap().len(), 1, "{applied}");
    assert_eq!(applied[0]["function"], "malloc");
    assert_eq!(applied[0]["shipped_crate"], "quirin-provider-libc");
    assert_eq!(applied[0]["shipped_module"], "libc/memory.qv");
    // Without the override the audit refuses, whatever the view says.
    let text = fs::read_to_string(tree.load_path()).unwrap();
    let mut without: Value = serde_json::from_str(&text).unwrap();
    without["overrides"] = json!([]);
    fs::write(
        tree.load_path(),
        serde_json::to_vec_pretty(&without).unwrap(),
    )
    .unwrap();
    let refused = tree.audit("r1.json");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains(collision));
    // A user crate may not take a shipped crate's name.
    fs::write(
        tree.load_path(),
        text.replace("\"quirin-provider-acme\"", "\"quirin-provider-libc\""),
    )
    .unwrap();
    assert!(!tree.audit("r2.json").status.success());
}

#[test]
fn a_user_lock_mismatch_is_pending() {
    if !clang_available() {
        return;
    }
    let tree = Tree::new(false);
    fs::write(
        tree.0
            .join("user/quirin-provider-acme/contracts/acme/queue.qcore"),
        "module contract;\n\n// Pinned (the default): in force only where the audited source \
         matches its lock.\nextern fn acme_queue_take(item: ptr<mut, u8>) -> unit\ncontract {\n    \
         requires resource_contract_v1;\n    ensures normal resource_released(item, nonnull);\n    \
         ensures normal resource_return_none();\n};\n",
    )
    .unwrap();
    succeeded(&tree.write_load_path(false, &[]));
    // `pin` writes the user crate's lock only; the shipped locks are untouched.
    let pin = tree.quirin(&[
        "pin",
        "--load-path",
        "user/user-load-path.json",
        "--source-root",
        "src",
    ]);
    succeeded(&pin);
    assert!(
        tree.0
            .join("user/quirin-provider-acme/contracts.lock")
            .is_file()
    );
    succeeded(&tree.audit("before.json"));
    let before = tree.read("before.json");
    assert_eq!(
        Tree::status(Tree::unit(&before, "queue_user.c"), "use_queue"),
        "Safe"
    );
    // The reviewed body changes: the pin is out of force and its caller is
    // Pending, which a re-pin after review resolves.
    let body = tree.0.join("src/queue.c");
    let text = fs::read_to_string(&body).unwrap();
    fs::write(
        &body,
        text.replace("acme_pool_put(item);", "acme_pool_put(item); ;"),
    )
    .unwrap();
    succeeded(&tree.audit("after.json"));
    let after = tree.read("after.json");
    let unit = Tree::unit(&after, "queue_user.c");
    assert_eq!(Tree::status(unit, "use_queue"), "Pending");
    assert_eq!(unit["status"], "Pending");
    assert_eq!(unit["reason_code"], "contract-pin-mismatch");
    let compare = tree.quirin(&[
        "compare",
        "--before",
        "before.json",
        "--after",
        "after.json",
        "--output",
        "compare.json",
    ]);
    succeeded(&compare);
    let report = tree.read("compare.json");
    assert_eq!(report["counts"]["coverage_lost"], 1);
    assert_eq!(
        report["verdict_moves"]["coverage_lost"][0]["after"],
        "Pending"
    );
    assert_eq!(report["verdicts"]["after"]["functions"]["Pending"], 1);
    assert_eq!(report["verdicts"]["before"]["functions"]["Pending"], 0);
    assert_rollup(&after);
}
