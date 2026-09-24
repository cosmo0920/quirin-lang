//! Declarative `.qcore` contract machinery (layer L1 of
//! `docs/GENERALIZATION_PLAN.md` §3.1, the W1 target crate): the
//! total-or-reject resource-contract loader and the contract-observation
//! interpreter. Contracts are trusted assumptions, never verified facts.
//!
//! `chunk_contract` evaluates declared schema paths; source import remains a
//! separately checked boundary in the caller.

pub mod bundle;
pub mod contract_inclusion;
pub mod contract_loader;
pub mod contract_observation;
pub mod extern_bridge;
pub mod load_path_manifest;
pub mod profile_loader;

mod profile_json;
pub mod typed_profile;

pub mod chunk_contract;
