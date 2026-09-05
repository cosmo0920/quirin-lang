//! `qcore.chunkio`: bounded source-derived chunk field summaries.
//!
//! Lifecycle and contract certificate checking remain in the core during this
//! extraction. Source summaries retain their private construction boundary.

pub mod chunk_source;
pub use chunk_source::*;

pub const PINNED_FLUENT_BIT_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";

use crate::verifier_plugin::{
    BuiltInProvider, PluginManifest, VerifierCapability, first_party_manifest,
};

pub fn manifest() -> PluginManifest {
    first_party_manifest(
        "qcore.chunkio",
        "chunkio",
        &[
            VerifierCapability::ResourceContracts,
            VerifierCapability::ChunkLifecycle,
            VerifierCapability::TranslationUnitProjection,
        ],
        "QCore.ChunkLifecycle",
    )
}

/// The v1 adapter cannot transport source summaries or lifecycle witnesses.
/// Use the typed APIs in this module for the existing bounded analysis.
pub fn provider() -> BuiltInProvider {
    BuiltInProvider::new(manifest()).expect("ChunkIO plugin manifest is valid")
}
