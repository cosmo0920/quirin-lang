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

//! Card OPENSSL-PROVIDER-PLUGIN: this crate through the SDK, as a user crate
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
        "quirin-openssl-{tag}-{}-{}",
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
    assert_eq!(krate.name, "quirin-provider-openssl");
    assert_eq!(krate.namespace, "openssl");
    // The SDK it depends on is not a contract dependency.
    assert!(krate.dependencies.is_empty(), "{:?}", krate.dependencies);
    assert_eq!(krate.modules.len(), quirin_provider_openssl::MODULES.len());
    for (path, text) in quirin_provider_openssl::MODULES {
        assert_eq!(&krate.modules[*path], text, "{path}");
    }
    assert_eq!(krate.manifest.checkers.len(), 12);
    let providers = crate_dir().parent().unwrap().to_path_buf();
    let manifest =
        load_path::user_load_path("contract-load-path.json", &providers, &[krate]).unwrap();
    assert_eq!(
        manifest["crates"][0]["contracts"],
        "quirin-provider-openssl/contracts"
    );
    assert_eq!(manifest["crates"][0]["dependencies"], serde_json::json!([]));
    assert_eq!(manifest["checkers"].as_array().unwrap().len(), 12);
    assert!(
        manifest["checkers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|binding| {
                binding["package"] == "quirin-provider-openssl/checkers/openssl-check"
                    && binding["functions"].as_array().unwrap().len() == 1
            })
    );
}

fn request(module: &str, text: &str, function: &str) -> VerificationRequestV2 {
    let translation_unit = format!("quirin-provider-openssl:{module}");
    VerificationRequestV2 {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION_V2,
        request_id: format!("openssl.contract_check:{function}"),
        plugin_id: PluginId::new("openssl.contract_check").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "openssl".into(),
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
    let mut child = Command::new(package.join("quirin-openssl-check"))
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
    let output = root.join("openssl-check");
    let installed = package::build(
        Path::new(env!("CARGO_BIN_EXE_quirin-openssl-check")),
        &output,
    )
    .unwrap();
    assert_eq!(installed.plugin_id.as_str(), "openssl.contract_check");
    assert_eq!(package::verify(&output).unwrap(), installed);
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../quirin-plugin.json")).unwrap();
    for binding in manifest["checkers"].as_array().unwrap() {
        let module = binding["module"].as_str().unwrap();
        let function = binding["functions"][0].as_str().unwrap();
        let text = quirin_provider_openssl::MODULES
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
        assert!(
            response
                .diagnostics
                .iter()
                .any(|d| d.message.starts_with(&format!("{function}: prototype"))),
            "{response:?}"
        );
    }
    // A module text the checker was not built with is Unknown.
    let edited = quirin_provider_openssl::MODULES[0]
        .1
        .replace("BIO_free", "BIO_free ");
    let response = ask(&output, &request("openssl/bio.qv", &edited, "BIO_free"));
    assert_eq!(response.verdict, ProposedVerdict::Unknown);
    // A function the module does not declare is Unknown.
    let text = quirin_provider_openssl::MODULES[0].1;
    let response = ask(&output, &request("openssl/bio.qv", text, "BIO_new"));
    assert_eq!(response.verdict, ProposedVerdict::Unknown);
    fs::remove_dir_all(&root).unwrap();
}
