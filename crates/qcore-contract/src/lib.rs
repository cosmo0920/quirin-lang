//! Declarative `.qcore` contract machinery (layer L1 of
//! `docs/GENERALIZATION_PLAN.md` §3.1, the W1 target crate): the
//! total-or-reject resource-contract loader and the contract-observation
//! interpreter. Contracts are trusted assumptions, never verified facts.
//!
//! `chunk_contract` remains in `qcore-compiler` until the ChunkIO summary
//! types it consumes move to `qcore-schemas` (the W5 task 3 split); moving
//! it now would invert the layering through the provider modules.

pub mod contract_loader;
pub mod contract_observation;
