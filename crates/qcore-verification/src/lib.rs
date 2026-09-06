//! Trusted verification adapters above the dependency-free semantic kernel.
//! Provider artifacts depend on schemas only and never link this crate.
//! Relocation preserves all existing derivation-incomplete and replay gates.

pub(crate) use qcore_core::chunk_lifecycle;
pub(crate) use qcore_verifier_host as verifier_plugin;

pub mod obligation_derivation;
pub mod verify_dispatch;
pub mod witness_replay;
