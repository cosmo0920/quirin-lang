//! `qcore.libevent`: bounded event semantics, profiles, and observation refinement.

pub mod event_driven;
pub mod monkey_binding;
pub mod monkey_observation;
pub mod monkey_source;
pub use event_driven::*;

use crate::verifier_plugin::{BuiltInProvider, PackageLoadError, PluginManifest};

/// Read the provider's declared manifest through validated profile discovery.
pub fn manifest() -> Result<PluginManifest, PackageLoadError> {
    crate::first_party::load_first_party_manifests(&crate::source_profiles::profiles_root())?
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.libevent")
        .ok_or_else(|| PackageLoadError {
            path: "qcore.libevent".into(),
            message: "provider is absent from profile packages".into(),
        })
}

/// The v1 adapter stays Unknown; loading data does not add semantic evidence.
pub fn provider() -> Result<BuiltInProvider, PackageLoadError> {
    BuiltInProvider::new(manifest()?).map_err(|error| PackageLoadError {
        path: "qcore.libevent".into(),
        message: error.to_string(),
    })
}
