//! Trusted verification adapters above the dependency-free semantic kernel.
//! Provider artifacts depend on schemas only and never link this crate.
//! Relocation preserves all existing derivation-incomplete and replay gates.

pub(crate) use qcore_core::chunk_lifecycle;
pub(crate) use qcore_verifier_host as verifier_plugin;

pub mod obligation_derivation;
pub mod verify_dispatch;
pub mod witness_replay;

pub mod reactor_callback;

pub mod source_effect;

pub mod cleanup_outcome;
pub mod counter_contract;
pub mod field_destructor;
pub mod list_unlink_contract;
