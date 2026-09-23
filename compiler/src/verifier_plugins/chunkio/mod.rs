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
pub mod lifecycle_coverage;
pub mod memory_close;
pub mod nested_cleanup;
pub mod nested_memory;
pub mod parent_teardown;
pub mod pinned_lifecycle;
pub mod teardown_source;
pub mod traversal;
pub mod unmap_wrapper;
pub mod unsupported_close;
pub use chunk_source::*;

pub const PINNED_FLUENT_BIT_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";

// The per-plugin `manifest()`/`provider()` selection route was removed by P09
// (`docs/reports/remaining-p09/README.md`). Use
// `crate::first_party::{load_first_party_manifests, load_first_party_registry}`,
// which reads the same validated profile packages and honours each manifest's
// declared entrypoint instead of forcing a built-in adapter.
