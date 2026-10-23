//! Declarative `.qcore` contract machinery (layer L1 of
//! `docs/GENERALIZATION_PLAN.md` §3.1, the W1 target crate): the
//! total-or-reject resource-contract loader and the contract-observation
//! interpreter. Contracts are trusted assumptions, never verified facts.
//!
//! `chunk_contract` evaluates declared schema paths; source import remains a
//! separately checked boundary in the caller.

pub mod bounds_contract;
pub mod bounds_hypotheses;
pub mod bounds_quant;
pub mod bundle;
pub mod contract_inclusion;
pub mod contract_lifecycle;
pub mod contract_loader;
pub mod contract_observation;
pub mod contract_revision;
pub mod contract_views;
pub mod deref_parts;
pub mod descriptor_contract_binding;
pub mod descriptor_observation;
mod extension_compatibility;
pub mod extern_bridge;
pub mod load_path_manifest;
pub mod local_condition;
pub mod outer_contract;
pub mod profile_loader;

mod profile_json;
pub mod typed_profile;

pub mod chunk_contract;

pub mod hypothesis;
pub mod hypothesis_bindings;

pub mod premise_flow;

pub mod premise_metadata_bridge;

pub mod guarded_decomposition;

pub mod guarded_metadata_bridge;

pub mod logger_effects;

/// Card W48-RK-LOG-CALLER-FRAME (rule 32): the caller-frame projection of a
/// reviewed logger effect model and its declarations' exact premise sets.
pub mod caller_frame;

/// Finite source-bound Monkey scheduler lifetime obligations.
pub mod scheduler_lifetime;
