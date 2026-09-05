//! `qcore.libevent`: bounded event semantics, profiles, and observation refinement.

pub mod event_driven;
pub use event_driven::*;

use crate::verifier_plugin::{
    BuiltInProvider, PluginManifest, VerifierCapability, first_party_manifest,
};

pub fn manifest() -> PluginManifest {
    first_party_manifest(
        "qcore.libevent",
        "libevent",
        &[
            VerifierCapability::ResourceContracts,
            VerifierCapability::EventTraces,
            VerifierCapability::Concurrency,
            VerifierCapability::ObservationHooks,
        ],
        "QCore.EventDriven",
    )
}

/// The v1 adapter cannot transport event programs or observation witnesses.
/// Use the typed APIs in this module for the existing bounded analysis.
pub fn provider() -> BuiltInProvider {
    BuiltInProvider::new(manifest()).expect("Libevent plugin manifest is valid")
}
