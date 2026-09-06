//! `qcore.chunkio`: bounded source-derived chunk field summaries.
//!
//! Lifecycle and contract certificate checking remain in the core during this
//! extraction. Source summaries retain their private construction boundary.

pub mod chunk_source;
pub mod file_attempts;
pub mod file_close;
pub mod file_delete;
pub mod file_outcomes;
pub mod filesystem_chunk_close;
pub mod memory_close;
pub mod nested_cleanup;
pub mod nested_memory;
pub mod parent_teardown;
pub mod teardown_source;
pub mod traversal;
pub mod unmap_wrapper;
pub mod unsupported_close;
pub use chunk_source::*;

pub const PINNED_FLUENT_BIT_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";

use crate::verifier_plugin::{BuiltInProvider, PackageLoadError, PluginManifest};

/// Read the provider's declared manifest through validated profile discovery.
pub fn manifest() -> Result<PluginManifest, PackageLoadError> {
    crate::first_party::load_first_party_manifests(&crate::source_profiles::profiles_root())?
        .into_iter()
        .find(|manifest| manifest.plugin_id.as_str() == "qcore.chunkio")
        .ok_or_else(|| PackageLoadError {
            path: "qcore.chunkio".into(),
            message: "provider is absent from profile packages".into(),
        })
}

/// The v1 adapter stays Unknown; loading data does not add semantic evidence.
pub fn provider() -> Result<BuiltInProvider, PackageLoadError> {
    BuiltInProvider::new(manifest()?).map_err(|error| PackageLoadError {
        path: "qcore.chunkio".into(),
        message: error.to_string(),
    })
}
