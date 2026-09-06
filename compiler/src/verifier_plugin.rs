//! Compatibility facade for the extracted verifier host.
//!
//! Generic registry, transport, firewall and composition live in
//! `qcore-verifier-host`. First-party adapters remain here until their
//! configured package loader migration lands.

pub use qcore_verifier_host::*;

use std::collections::BTreeSet;
use std::sync::Arc;

pub fn first_party_manifests() -> Vec<PluginManifest> {
    let mut manifests = vec![
        crate::verifier_plugins::libevent::manifest(),
        crate::verifier_plugins::chunkio::manifest(),
    ];
    manifests.extend(unextracted_first_party_manifests());
    manifests
}

fn unextracted_first_party_manifests() -> Vec<PluginManifest> {
    vec![
        first_party_manifest(
            "qcore.cmetrics",
            "cmetrics",
            &[VerifierCapability::ResourceContracts],
            "QCore.Resource",
        ),
        first_party_manifest(
            "qcore.ctraces",
            "ctraces",
            &[VerifierCapability::ResourceContracts],
            "QCore.Resource",
        ),
        first_party_manifest(
            "qcore.librdkafka",
            "librdkafka",
            &[
                VerifierCapability::ResourceContracts,
                VerifierCapability::Concurrency,
            ],
            "QCore.Concurrency",
        ),
    ]
}

pub(crate) fn first_party_manifest(
    id: &str,
    library: &str,
    capabilities: &[VerifierCapability],
    theorem_family: &str,
) -> PluginManifest {
    let plugin_id = PluginId::new(id).expect("first-party plugin IDs are valid");
    PluginManifest {
        package_version: env!("CARGO_PKG_VERSION").into(),
        protocol_versions: BTreeSet::from([PLUGIN_PROTOCOL_VERSION]),
        evidence_versions: BTreeSet::from([PLUGIN_EVIDENCE_VERSION]),
        entrypoint: PluginEntrypoint::BuiltIn,
        package_digest: format!("builtin:{plugin_id}"),
        targets: vec![SupportedTarget {
            library: library.into(),
            revisions: BTreeSet::from(["*".into()]),
            profiles: BTreeSet::from(["*".into()]),
            dialects: BTreeSet::from(["c11".into(), "qcore".into()]),
            platforms: BTreeSet::from(["*".into()]),
            backends: BTreeSet::from(["*".into()]),
        }],
        capabilities: capabilities.iter().copied().collect(),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "qcore.unsupported_assertions.v1".into(),
        lean_theorem_family: theorem_family.into(),
        plugin_id,
    }
}

pub fn first_party_registry() -> VerifierRegistry {
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(crate::verifier_plugins::libevent::provider()))
        .expect("Libevent plugin ID is unique");
    registry
        .register(Arc::new(crate::verifier_plugins::chunkio::provider()))
        .expect("ChunkIO plugin ID is unique");
    for manifest in unextracted_first_party_manifests() {
        registry
            .register(Arc::new(
                BuiltInProvider::new(manifest).expect("first-party manifest is valid"),
            ))
            .expect("first-party plugin IDs are unique");
    }
    registry
}
