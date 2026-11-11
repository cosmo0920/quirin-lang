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

//! Real CMetrics package, generic void-field source and hostile proposal controls.
#![cfg(unix)]
use qcore_c_gateway::boolean_source::{ChunkSourceResult, import_chunk_source_with_profile};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::chunk_summary::BooleanFieldResult;
use qcore_schemas::{PluginId, ProposedVerdict, TargetProfile};
use qcore_verification::source_effect::{BooleanFieldSourceTask, semantic_profile_for};
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

#[test]
fn cmetrics_process_checks_unit_paths_and_rejects_scalar_or_unbound_claims() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::temp_dir().join(format!(
        "qcore-cmetrics-unit-package-{}",
        std::process::id()
    ));
    let built = Command::new("python3")
        .arg(root.join("providers/package.py"))
        .args(["build", "--output"])
        .arg(&output)
        .arg("--target-dir")
        .arg(root.join("providers/target/conformance-artifacts"))
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(
            ProcessProvider::from_package(output.join("qcore.cmetrics")).unwrap(),
        ))
        .unwrap();
    let profile = load_profile_directory(&root.join("contract/profiles/cmetrics-counter")).unwrap();
    let source = "struct cmt_counter {int allow_reset; int untouched;}; void cmt_counter_allow_reset(struct cmt_counter *counter) { if (counter->allow_reset) {return;} counter->allow_reset=1; }";
    let mut imports = import_chunk_source_with_profile(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        &profile.pin_for("cmetrics").unwrap().git_commit,
        &profile,
        &["cmetrics"],
        &[],
        include_str!(
            "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
        ),
    )
    .unwrap();
    assert_eq!(imports.len(), 1);
    let ChunkSourceResult::Summary(summary) = imports.remove(0) else {
        panic!("missing unit import")
    };
    let contract =
        std::fs::read_to_string(output.join("qcore.cmetrics/contracts/cmetrics/counter_reset.qv"))
            .unwrap();
    let target = TargetProfile {
        library: "cmetrics".into(),
        revision: summary.source_revision().into(),
        profile: semantic_profile_for(&summary).into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    };
    let replay =
        BooleanFieldSourceTask::derive(&summary, "counter.c", &contract, target.clone()).unwrap();
    let request = replay.request(PluginId::new("qcore.cmetrics").unwrap());
    let result = registry.verify_v2(&request, replay.task(), Some(&replay));
    assert_eq!(
        result.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        result.diagnostics
    );
    assert!(result.certificate.is_some());
    assert_eq!(replay.evaluate().unwrap().checked_states(), 2);
    for mutation in 0..5 {
        let mut changed = request.clone();
        let input = changed.source_effect.as_mut().unwrap();
        match mutation {
            0 => input.result_kind = BooleanFieldResult::I32,
            1 => input.schema_version = 2,
            2 => input.program = qcore_schemas::chunk_summary::BooleanFieldStatement::ReturnUnit,
            3 => {
                input.profile_assumptions.clear();
            }
            4 => changed.target.profile = "qcore.c_boolean_field_contract.v2".into(),
            _ => unreachable!(),
        }
        let rejected = registry.verify_v2(&changed, replay.task(), Some(&replay));
        assert_eq!(
            rejected.verdict,
            ProposedVerdict::Unknown,
            "mutation {mutation}"
        );
        assert!(rejected.certificate.is_none());
    }
    let mut wrong_key = replay.task().clone();
    let digest = wrong_key
        .contract_sources
        .remove(request.source_effect.as_ref().unwrap().digest_key())
        .unwrap();
    wrong_key.contract_sources.insert(
        qcore_schemas::chunk_summary::BOOLEAN_FIELD_SOURCE_INPUT_KEY.into(),
        digest,
    );
    let mut rebound = request.clone();
    rebound.task_digest = wrong_key.task_digest();
    let rejected = registry.verify_v2(&rebound, &wrong_key, Some(&replay));
    assert_eq!(rejected.verdict, ProposedVerdict::Unknown);
    // Card LANDING-REPAIR: since the quant migration (6bcda2983) the contract
    // also states the entry domain `(#old_field == 0) || (#old_field == 1)`,
    // so a blanket `field == 1` rewrite corrupts that entry condition too.
    // The derivation refuses such a domain outright; the false policy proper
    // changes only the normal exit condition and must still be refused by
    // the verifier.
    let corrupted_domain = contract.replace("field == 1", "field == 0");
    assert_eq!(contract.matches("field == 1").count(), 2);
    assert!(
        BooleanFieldSourceTask::derive(&summary, "counter.c", &corrupted_domain, target.clone())
            .is_err()
    );
    assert_eq!(contract.matches("(#field == 1)").count(), 1);
    let false_policy = contract.replace("(#field == 1)", "(#field == 0)");
    let false_replay =
        BooleanFieldSourceTask::derive(&summary, "counter.c", &false_policy, target).unwrap();
    assert_eq!(
        registry
            .verify_v2(
                &false_replay.request(request.plugin_id),
                false_replay.task(),
                Some(&false_replay)
            )
            .verdict,
        ProposedVerdict::Unknown
    );
    std::fs::remove_dir_all(output).unwrap();
}
