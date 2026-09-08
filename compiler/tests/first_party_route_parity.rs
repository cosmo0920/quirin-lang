//! P09 parity harness for the per-plugin hardcoded first-party provider route.
//!
//! Temporary: this file exists only to re-measure, in this worktree, that
//! `verifier_plugins::{libevent,chunkio}::{manifest, provider}` answer
//! identically to the generic profile-package discovery route
//! (`first_party::{load_first_party_manifests, load_first_party_registry}`)
//! before the hardcoded route is deleted. It is removed by the deletion
//! commit; the surviving route keeps its own pinned tests.
//!
//! Set `QCORE_P09_PARITY_OUT` to a path to archive the measurement as JSON.

use std::collections::BTreeMap;
use std::sync::Arc;

use qcore_compiler::source_profiles::profiles_root;
use qcore_compiler::verifier_plugins::{chunkio, libevent};
use qcore_compiler::{
    BuiltInProvider, FactPredicate, NormalizedFact, PLUGIN_EVIDENCE_VERSION,
    PLUGIN_PROTOCOL_VERSION, PluginEntrypoint, PluginId, PluginManifest, ProposedVerdict,
    TargetProfile, UnitIdentity, VerificationRequest, VerifierCapability, VerifierProvider,
    VerifierRegistry, load_first_party_manifests, load_first_party_registry,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn request(plugin: &str, library: &str, capability: VerifierCapability) -> VerificationRequest {
    VerificationRequest {
        protocol_version: PLUGIN_PROTOCOL_VERSION,
        evidence_version: PLUGIN_EVIDENCE_VERSION,
        request_id: "p09-parity".into(),
        plugin_id: PluginId::new(plugin).unwrap(),
        capability,
        target: TargetProfile {
            library: library.into(),
            revision: "audited-revision".into(),
            profile: "default".into(),
            dialect: "c11".into(),
            platform: "linux".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            source_digest: digest('a'),
            build_digest: digest('b'),
            translation_unit: "src/input.c".into(),
        },
        facts: vec![NormalizedFact {
            fact_id: "resource-1".into(),
            subject: "ctx".into(),
            predicate: FactPredicate::ResourceBalanced,
            holds: true,
        }],
        assumptions: BTreeMap::from([("callbacks_bounded".into(), true)]),
    }
}

/// A registry holding exactly the one provider the hardcoded route builds.
fn hardcoded_registry(provider: BuiltInProvider) -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    registry.register(Arc::new(provider)).unwrap();
    registry
}

fn discovered_manifest(plugin: &str) -> PluginManifest {
    load_first_party_manifests(&profiles_root())
        .unwrap()
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == plugin)
        .unwrap()
}

#[test]
fn hardcoded_and_discovered_first_party_routes_agree() {
    let discovery = load_first_party_registry(&profiles_root()).unwrap();
    let mut record = serde_json::Map::new();
    record.insert(
        "measurement".into(),
        serde_json::json!({
            "card": "P09",
            "old_route": "qcore_compiler::verifier_plugins::{libevent,chunkio}::{manifest, provider}",
            "new_route": "qcore_compiler::first_party::{load_first_party_manifests, load_first_party_registry}",
            "profiles_root": profiles_root().display().to_string(),
            "protocol_version": PLUGIN_PROTOCOL_VERSION,
            "evidence_version": PLUGIN_EVIDENCE_VERSION,
        }),
    );

    for (plugin, hardcoded_manifest, hardcoded_provider) in [
        (
            "qcore.libevent",
            libevent::manifest().unwrap(),
            libevent::provider().unwrap(),
        ),
        (
            "qcore.chunkio",
            chunkio::manifest().unwrap(),
            chunkio::provider().unwrap(),
        ),
    ] {
        let discovered = discovered_manifest(plugin);

        // (A) manifest identity: equal as records and byte-identical as JSON.
        assert_eq!(hardcoded_manifest, discovered, "{plugin} manifest record");
        let hardcoded_bytes = serde_json::to_vec(&hardcoded_manifest).unwrap();
        let discovered_bytes = serde_json::to_vec(&discovered).unwrap();
        assert_eq!(hardcoded_bytes, discovered_bytes, "{plugin} manifest bytes");

        // The hardcoded provider carries exactly the discovered manifest.
        assert_eq!(hardcoded_provider.manifest(), &discovered);

        let library = discovered.targets[0].library.clone();
        let registry = hardcoded_registry(hardcoded_provider);

        // (B) declared capabilities: identical outcomes on both routes.
        let mut capabilities = Vec::new();
        for capability in &discovered.capabilities {
            let request = request(plugin, &library, *capability);
            let left = registry.verify(&request);
            let right = discovery.verify(&request);
            assert_eq!(left, right, "{plugin} {capability:?} outcome");
            assert_eq!(left.verdict, ProposedVerdict::Unknown);
            assert!(left.certificate.is_none());
            capabilities.push(serde_json::json!({
                "capability": format!("{capability:?}"),
                "verdict": format!("{:?}", left.verdict),
                "certificate": left.certificate.is_some(),
                "diagnostics": left
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.clone())
                    .collect::<Vec<_>>(),
                "routes_agree": true,
            }));
        }

        // (C) refusal parity: a foreign library and an undeclared capability
        // are refused identically on both routes, with no certificate.
        let foreign = request(
            plugin,
            "not-a-declared-library",
            *discovered.capabilities.iter().next().unwrap(),
        );
        assert_eq!(registry.verify(&foreign), discovery.verify(&foreign));
        assert_eq!(registry.verify(&foreign).verdict, ProposedVerdict::Unknown);
        let undeclared = [
            VerifierCapability::ResourceContracts,
            VerifierCapability::EventTraces,
            VerifierCapability::Concurrency,
            VerifierCapability::ObservationHooks,
            VerifierCapability::ChunkLifecycle,
            VerifierCapability::TranslationUnitProjection,
        ]
        .into_iter()
        .find(|capability| !discovered.capabilities.contains(capability));
        if let Some(capability) = undeclared {
            let request = request(plugin, &library, capability);
            assert_eq!(registry.verify(&request), discovery.verify(&request));
            assert_eq!(registry.verify(&request).verdict, ProposedVerdict::Unknown);
        }

        // (D) entrypoint coverage: the hardcoded route can only ever build a
        // built-in adapter. Given the same manifest with an executable
        // entrypoint it refuses outright, where discovery builds a
        // `ProcessProvider`. This is a capability gap, never a promotion.
        let mut executable = discovered.clone();
        executable.entrypoint = PluginEntrypoint::Executable {
            path: "provider".into(),
        };
        executable.package_digest = digest('c');
        let refusal = format!("{:?}", BuiltInProvider::new(executable).err().unwrap());

        record.insert(
            plugin.to_owned(),
            serde_json::json!({
                "manifest_records_equal": true,
                "manifest_bytes_equal": true,
                "manifest_bytes": hardcoded_bytes.len(),
                "package_version": discovered.package_version,
                "entrypoint": format!("{:?}", discovered.entrypoint),
                "library": library,
                "capabilities": capabilities,
                "foreign_target_routes_agree": true,
                "undeclared_capability_routes_agree": undeclared.is_some(),
                "hardcoded_route_refuses_executable_entrypoint": refusal,
            }),
        );
    }

    // (E) absence parity: an invalid profile root yields no manifests and no
    // registry entries on the surviving route, so every scope stays Unknown.
    let missing = profiles_root().join("qcore-p09-absent");
    assert!(load_first_party_manifests(&missing).is_err());
    assert!(load_first_party_registry(&missing).is_err());
    assert!(libevent::manifest().is_ok()); // real root still resolves
    record.insert(
        "absent_profile_root".into(),
        serde_json::json!({
            "load_first_party_manifests": "PackageLoadError",
            "load_first_party_registry": "PackageLoadError",
            "fallback": "none",
        }),
    );

    if let Some(path) = std::env::var_os("QCORE_P09_PARITY_OUT") {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&serde_json::Value::Object(record)).unwrap(),
        )
        .unwrap();
    }
}
