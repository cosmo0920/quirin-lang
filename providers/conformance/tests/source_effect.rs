//! Real typed C import → packaged ChunkIO process → trusted contract replay.
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
    forged.source_effect.as_mut().unwrap().summary.paths[0].return_value = 7;
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
        &CONTRACT.replace("result == old_lock", "result == 7"),
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
