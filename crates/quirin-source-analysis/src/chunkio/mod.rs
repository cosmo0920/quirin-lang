//! Source adapters for the pinned ChunkIO audit.
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
pub mod teardown_source;
pub mod traversal;
pub mod unmap_wrapper;
pub mod unsupported_close;
pub use chunk_source::*;
pub const PINNED_FLUENT_BIT_REVISION: &str = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97";
