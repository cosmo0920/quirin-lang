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

//! Real typed C import → packaged ChunkIO process → trusted contract replay.
#![cfg(unix)]
#[path = "../../../crates/qcore-verification/tests/fixtures/boolean_source.rs"]
mod fixture;
use fixture::*;
use qcore_schemas::{PluginId, ProposedVerdict};
use qcore_verifier_host::{ProcessProvider, VerifierRegistry};
use std::process::Command;
use std::sync::Arc;

#[test]
fn real_package_proposes_source_effects_without_admitting_false_contracts() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let output = std::env::temp_dir().join(format!(
        "qcore-source-effect-package-{}",
        std::process::id()
    ));
    let built = Command::new("python3")
        .arg(root.join("package.py"))
        .args(["build", "--output"])
        .arg(&output)
        .arg("--target-dir")
        .arg(root.join("target/conformance-artifacts"))
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
            ProcessProvider::from_package(output.join("qcore.chunkio")).unwrap(),
        ))
        .unwrap();
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    let request = replay.request(PluginId::new("qcore.chunkio").unwrap());
    let result = registry.verify_v2(&request, replay.task(), Some(&replay));
    assert_eq!(
        result.verdict,
        ProposedVerdict::Safe,
        "{:?}",
        result.diagnostics
    );
    let certificate = result.certificate.unwrap();
    assert_eq!(certificate.task(), Some(replay.task()));
    assert_eq!(
        certificate.witness().unwrap().kind(),
        "boolean_field_contract"
    );

    // The process receives the typed statement program, never expected paths.
    let input_json = serde_json::to_value(request.source_effect.as_ref().unwrap()).unwrap();
    assert!(input_json.get("summary").is_none());
    assert!(input_json.get("paths").is_none());
    assert!(input_json.get("program").is_some());
    let mut old_shape = input_json.clone();
    old_shape["summary"] = serde_json::to_value(&*summary).unwrap();
    assert!(
        serde_json::from_value::<qcore_schemas::chunk_summary::BooleanFieldSourceInput>(old_shape)
            .is_err()
    );
    let mut wrong_version = request.clone();
    wrong_version.source_effect.as_mut().unwrap().schema_version = 1;
    assert!(wrong_version.validate().is_err());

    // Independent process execution agrees with the retained trusted evaluator
    // for each supported expression and ordered branch/write behavior.
    for body in [
        "return !!ch->lock;",
        "return -(-ch->lock);",
        "return ch->lock != 0;",
        "return !(ch->lock == 0);",
        "if (ch->lock == 0) { return 0; } return 1;",
        "if (ch->lock) { ch->lock = 1; } else { ch->lock = 0; } return ch->lock;",
        "ch->lock = !ch->lock; ch->lock = !ch->lock; return ch->lock;",
        "return ch->lock; ch->lock = 0;",
    ] {
        let source = format!(
            "struct cio_chunk {{ int lock; }}; int cio_chunk_is_locked(struct cio_chunk *ch) {{ {body} }}"
        );
        let imported = checked(&source, vec![]);
        let mutable_contract =
            CONTRACT.replace("borrow<'call, const, Chunk>", "borrow<'call, mut, Chunk>");
        let context = task(&imported, &mutable_contract);
        if imported.paths.iter().any(|path| !path.writes.is_empty()) {
            let readonly = task(&imported, CONTRACT);
            let rejected = registry.verify_v2(
                &readonly.request(PluginId::new("qcore.chunkio").unwrap()),
                readonly.task(),
                Some(&readonly),
            );
            assert_eq!(rejected.verdict, ProposedVerdict::Unknown);
            assert!(
                rejected
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "QP009"
                        && diagnostic.message.contains("const QCore borrow"))
            );
        }
        let response = registry.verify_v2(
            &context.request(PluginId::new("qcore.chunkio").unwrap()),
            context.task(),
            Some(&context),
        );
        assert_eq!(
            response.verdict,
            ProposedVerdict::Safe,
            "{body}: {:?}",
            response.diagnostics
        );
        let qcore_schemas::SemanticWitness::BooleanFieldContract(witness) =
            response.certificate.unwrap().witness().unwrap().clone()
        else {
            panic!("wrong witness");
        };
        assert_eq!(witness.summary, *imported, "{body}");
    }

    let no_checker = registry.verify_v2(&request, replay.task(), None);
    assert_eq!(no_checker.verdict, ProposedVerdict::Unknown);
    assert!(no_checker.diagnostics.iter().any(|d| d.code == "QP016"));
    assert_eq!(
        VerifierRegistry::default()
            .verify_v2(&request, replay.task(), Some(&replay))
            .verdict,
        ProposedVerdict::Unknown
    );

    let mut forged = request.clone();
    forged.source_effect.as_mut().unwrap().program =
        qcore_schemas::chunk_summary::BooleanFieldStatement::Return(
            qcore_schemas::chunk_summary::BooleanFieldExpression::Integer(7),
        );
    let rejection = registry.verify_v2(&forged, replay.task(), Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP001"));
    let mut forged_task = replay.task().clone();
    forged_task.contract_sources.insert(
        qcore_schemas::chunk_summary::BOOLEAN_FIELD_SOURCE_INPUT_KEY.into(),
        forged.source_effect.as_ref().unwrap().digest(),
    );
    forged.task_digest = forged_task.task_digest();
    let rejection = registry.verify_v2(&forged, &forged_task, Some(&replay));
    assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    let false_contract = task(
        &summary,
        &CONTRACT.replace("#result == #old_lock", "#result == 7"),
    );
    let rejection = registry.verify_v2(
        &false_contract.request(PluginId::new("qcore.chunkio").unwrap()),
        false_contract.task(),
        Some(&false_contract),
    );
    assert_eq!(
        rejection.verdict,
        ProposedVerdict::Unknown,
        "provider's Safe proposal cannot bypass contract evaluation"
    );
    assert!(rejection.diagnostics.iter().any(|d| d.code == "QP009"));

    for changed in [
        checked(&format!("// changed actual source\n{SOURCE}"), vec![]),
        checked(SOURCE, vec!["QCORE_OTHER_BUILD=1".into()]),
    ] {
        let other = task(&changed, CONTRACT);
        let rejection = registry.verify_v2(
            &other.request(PluginId::new("qcore.chunkio").unwrap()),
            other.task(),
            Some(&replay),
        );
        assert_eq!(rejection.verdict, ProposedVerdict::Unknown);
    }
    registry
        .set_enabled(&PluginId::new("qcore.chunkio").unwrap(), false)
        .unwrap();
    assert_eq!(
        registry
            .verify_v2(&request, replay.task(), Some(&replay))
            .verdict,
        ProposedVerdict::Unknown
    );
    std::fs::remove_dir_all(output).unwrap();
}
