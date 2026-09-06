//! Trusted semantic kernel seed of QCore-C (layer L1 of
//! `docs/GENERALIZATION_PLAN.md` §3.1).
//!
//! Started at the W1 relocation with the dependency-free vocabulary the
//! front end and contract crates build on: interval usage grades, the
//! axis/dimension AST, and index environments. The remaining semantic
//! kernel migrates here from `qcore-compiler` at the W5 consolidation.

pub mod ast;
pub mod index;
pub mod usage;
