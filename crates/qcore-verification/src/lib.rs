//! Trusted verification adapters above the dependency-free semantic kernel.
//! Provider artifacts depend on schemas only and never link this crate.
//! Relocation preserves all existing derivation-incomplete and replay gates.

pub(crate) use qcore_core::chunk_lifecycle;
pub(crate) use qcore_verifier_host as verifier_plugin;

pub mod atomic_publication_source;
pub mod obligation_derivation;
pub mod verify_dispatch;
pub mod witness_replay;

pub mod observed_wait_composition;
pub mod reactor_callback;
pub mod reactor_source;

pub mod call_use_source;
pub mod callback_coverage;
pub mod callback_task_handoff;
pub mod chunk_lifecycle_coverage;
pub mod cleanup_replay_source;
pub mod composition_source;
pub mod counter_source;
pub mod cycle18_callback_obligations;
pub mod cycle18_composition;
pub mod event_composition_context;
pub mod event_payload;
pub mod file_close_composition;
pub mod filesystem_chunk_close_composition;
pub mod frame_source;
pub mod memory_close_composition;
pub mod monkey_wait_projection;
pub mod native_platform_matrix;
pub mod source_effect;
pub mod teardown_source;
pub mod timed_wait_observation_wire;
pub mod timeout_teardown;
pub mod unmap_composition;
pub mod unsupported_close_composition;

pub mod call_use_contract;
pub mod cleanup_outcome;
pub mod cleanup_vocabulary;
pub mod counter_contract;
pub mod field_destructor;
pub mod list_unlink_contract;

pub mod driver;
pub mod external_source;
pub mod library_import;
pub mod research_extensions;
