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

//! Card ONIGMO-PROVIDER-PLUGIN: this crate through the SDK, as a user crate
//! is taken — read and validated (`quirin-plugin check`), listed in a user
//! load path (`quirin-plugin load-path`), its checker packaged and verified
//! (`quirin-plugin package` / `verify`) — and the packaged checker answering
//! real evidence-v2 frames.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use qcore_schemas::PLUGIN_EVIDENCE_VERSION_V2;
use qcore_schemas::obligation::{
    Obligation, ObligationProperty, ObligationScope, ObligationSubject, TaskBounds,
};
use qcore_schemas::plugin::{
    PLUGIN_PROTOCOL_VERSION, PluginId, ProposedVerdict, TargetProfile, UnitIdentity,
    VerificationRequestV2, VerificationResponseV2, VerifierCapability,
};
use quirin_provider_plugin::{load_path, package, user_crate::UserCrate};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "quirin-onigmo-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn the_crate_reads_as_a_user_crate_and_lists_in_a_user_load_path() {
    let krate = UserCrate::read(&crate_dir()).unwrap();
    assert_eq!(krate.name, "quirin-provider-onigmo");
    assert_eq!(krate.namespace, "onigmo");
    // The SDK it depends on is not a contract dependency.
    assert!(krate.dependencies.is_empty(), "{:?}", krate.dependencies);
    assert_eq!(krate.modules.len(), quirin_provider_onigmo::MODULES.len());
    for (path, text) in quirin_provider_onigmo::MODULES {
        assert_eq!(&krate.modules[*path], text, "{path}");
    }
    assert_eq!(krate.manifest.checkers.len(), 9);
    let providers = crate_dir().parent().unwrap().to_path_buf();
    let manifest =
        load_path::user_load_path("contract-load-path.json", &providers, &[krate]).unwrap();
    assert_eq!(
        manifest["crates"][0]["contracts"],
        "quirin-provider-onigmo/contracts"
    );
    assert_eq!(manifest["crates"][0]["dependencies"], serde_json::json!([]));
    assert_eq!(manifest["checkers"].as_array().unwrap().len(), 9);
    assert!(
        manifest["checkers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|binding| {
                binding["package"] == "quirin-provider-onigmo/checkers/onigmo-check"
                    && binding["functions"].as_array().unwrap().len() == 1
            })
    );
}

fn request(module: &str, text: &str, function: &str) -> VerificationRequestV2 {
    let translation_unit = format!("quirin-provider-onigmo:{module}");
    VerificationRequestV2 {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        request_id: format!("onigmo.contract_check:{function}"),
        plugin_id: PluginId::new("onigmo.contract_check").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "onigmo".into(),
            revision: "0.1.0".into(),
            profile: "user-contract".into(),
            dialect: "qcore-resource-v1".into(),
            platform: "any".into(),
            backend: "quirin-audit".into(),
        },
        unit: UnitIdentity {
            source_digest: quirin_provider_plugin::sha256_hex(text.as_bytes()),
            build_digest: "b".repeat(64),
            translation_unit: translation_unit.clone(),
        },
        facts: Vec::new(),
        assumptions: Default::default(),
        task_digest: "c".repeat(64),
        obligations: vec![Obligation {
            obligation_id: format!("resource_contracts:{function}"),
            capability: VerifierCapability::ResourceContracts,
            property: ObligationProperty::ParameterModeRespected,
            subject: ObligationSubject::Function {
                symbol: function.into(),
                translation_unit,
            },
            scope: ObligationScope::WholeUnit,
            bounds: TaskBounds::single_path(),
            assumptions: Default::default(),
            evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        }],
        source_effect: None,
        counter_effect: None,
        reactor_program: None,
        teardown_source: None,
        call_use_source: None,
        cleanup_replay_source: None,
        composition_source: None,
    }
}

fn ask(package: &Path, request: &VerificationRequestV2) -> VerificationResponseV2 {
    let payload = serde_json::to_vec(request).unwrap();
    let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
    frame.extend(payload);
    let mut child = Command::new(package.join("quirin-onigmo-check"))
        .current_dir(package)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&frame).unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout[4..]).unwrap()
}

#[test]
fn the_packaged_checker_answers_each_binding() {
    let root = scratch("package");
    let output = root.join("onigmo-check");
    let installed = package::build(
        Path::new(env!("CARGO_BIN_EXE_quirin-onigmo-check")),
        &output,
    )
    .unwrap();
    assert_eq!(installed.plugin_id.as_str(), "onigmo.contract_check");
    assert_eq!(package::verify(&output).unwrap(), installed);
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../quirin-plugin.json")).unwrap();
    for binding in manifest["checkers"].as_array().unwrap() {
        let module = binding["module"].as_str().unwrap();
        let function = binding["functions"][0].as_str().unwrap();
        let text = quirin_provider_onigmo::MODULES
            .iter()
            .find(|(path, _)| *path == module)
            .unwrap()
            .1;
        let response = ask(&output, &request(module, text, function));
        assert_eq!(
            response.verdict,
            ProposedVerdict::Safe,
            "{function}: {response:?}"
        );
        assert!(response.evidence.is_none(), "a claim, never a certificate");
        // Every diagnostic is a formula: the note a conjunction, never a
        // refusal.
        assert!(
            response.diagnostics.iter().any(|d| {
                d.message.starts_with(&format!("{function}: #prototype("))
                    && d.message.contains(" && ")
                    && !d.message.ends_with("-> #false")
            }),
            "{response:?}"
        );
    }
    // A module text the checker was not built with is Unknown: here the
    // range is a `since` the documentation never gives, so the drift never
    // reaches the checks it would fail.
    let (path, text) = quirin_provider_onigmo::MODULES
        .iter()
        .find(|(path, _)| *path == "onigmo/regex.qv")
        .unwrap();
    let unknown = |response: &VerificationResponseV2| {
        assert_eq!(response.verdict, ProposedVerdict::Unknown, "{response:?}");
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.message.ends_with("-> #false")),
            "{response:?}"
        );
    };
    let edited = text.replace("documented \"6.1.2\"", "since \"6.1.2\"");
    unknown(&ask(&output, &request(path, &edited, "onig_new")));
    // A range excluding the pinned version, likewise.
    let removed = text.replace(
        "documented \"6.1.2\")",
        "documented \"6.1.2\", removed \"6.2.0\")",
    );
    unknown(&ask(&output, &request(path, &removed, "onig_free")));
    // A function the module does not declare (onig_end is never declared).
    unknown(&ask(&output, &request(path, text, "onig_end")));
    fs::remove_dir_all(&root).unwrap();
}

/// Rule 53: the crate reaches a run only through the generated user load
/// path, where every declaration is behind its checker, and never through
/// the shipped one, which carries no checker bindings.
#[test]
fn the_crate_is_on_the_user_load_path_only() {
    let providers = crate_dir().parent().unwrap().to_path_buf();
    let read = |name: &str| -> serde_json::Value {
        serde_json::from_slice(&fs::read(providers.join(name)).unwrap()).unwrap()
    };
    let shipped = read("contract-load-path.json");
    let shipped_text = serde_json::to_string(&shipped).unwrap();
    assert!(!shipped_text.contains("quirin-provider-onigmo"));
    assert!(!shipped_text.contains("onigmo/"));
    let user = read("user-crates-load-path.json");
    assert_eq!(user["extends"], "contract-load-path.json");
    assert_eq!(user["overrides"], serde_json::json!([]));
    let record = user["crates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["name"] == "quirin-provider-onigmo")
        .expect("the user load path lists the crate");
    assert_eq!(record["namespace"], "onigmo");
    assert_eq!(
        record["modules"].as_array().unwrap().len(),
        quirin_provider_onigmo::MODULES.len()
    );
    let bindings = user["checkers"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|binding| binding["crate"] == "quirin-provider-onigmo")
        .collect::<Vec<_>>();
    let mut bound = bindings
        .iter()
        .flat_map(|binding| binding["functions"].as_array().unwrap().clone())
        .map(|function| function.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    bound.sort();
    let mut documented = quirin_provider_onigmo::check::DOCUMENTED
        .iter()
        .map(|entry| entry.function.to_owned())
        .collect::<Vec<_>>();
    documented.sort();
    assert_eq!(bound, documented, "every declaration is bound exactly once");
    assert!(
        bindings.iter().all(|binding| {
            binding["package"] == "quirin-provider-onigmo/checkers/onigmo-check"
        })
    );
    // The lock pins the version the audited build's headers report.
    let lock: serde_json::Value =
        serde_json::from_slice(&fs::read(crate_dir().join("contracts.lock")).unwrap()).unwrap();
    assert_eq!(lock["library_versions"]["onigmo"]["version"], "6.2.0");
    let pinned: serde_json::Value =
        serde_json::from_str(quirin_provider_onigmo::PINNED_PROTOTYPES).unwrap();
    assert_eq!(pinned["version"], "6.2.0");
    // Every declaration is pinned to its vendored sites (the bodies are in
    // the audited tree), none flexible.
    assert_eq!(lock["flexible"], serde_json::json!([]));
    let mut pinned_functions = lock["pinned"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["function"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    pinned_functions.sort();
    assert_eq!(pinned_functions, documented);
    for entry in lock["pinned"].as_array().unwrap() {
        let sites = entry["identities"][0]["sites"].as_array().unwrap();
        assert!(
            sites
                .iter()
                .any(|site| site["path"] == "lib/onigmo/onigmo.h" && site["kind"] == "declaration"),
            "{entry}"
        );
        // The pin scanner (`crates/qcore-audit/src/c_identity.rs`) finds no
        // definition of the three `regexec.c` functions after
        // `onig_region_new`: that file's conditional groups do not balance
        // (`regexec.c:2439-2447`), so its first wanted body runs to the end of
        // the file and the scan resumes after it
        // (C-IDENTITY-SITES-AFTER-UNBALANCED-BODY). Pinned as found; this
        // assertion moves when the scanner is repaired.
        let declaration_only = matches!(
            entry["function"].as_str().unwrap(),
            "onig_match" | "onig_region_free" | "onig_search"
        );
        let defined = sites.iter().any(|site| {
            site["kind"] == "definition"
                && site["path"].as_str().unwrap().starts_with("lib/onigmo/")
        });
        assert_eq!(defined, !declaration_only, "{entry}");
    }
}
