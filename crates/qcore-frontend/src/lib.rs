//! QCore language front end (layer L1 of `docs/GENERALIZATION_PLAN.md`
//! §3.1): lexer, axis/dimension parser and static environment, quant-index
//! parsing/resolution, source elaboration and whole-program semantic integration.
//!
//! Moved from `qcore-compiler` at the W1 relocation; the compiler crate
//! re-exports every module here at its previous path.

pub mod check;
pub mod frontend;
pub mod lexer;
pub mod parser;
pub mod quant_index;
pub mod source_tasks;

pub use parser::parse_program;

// Internal aliases preserve the relocated semantic integration's imports without
// introducing a dependency on compiler orchestration or external verification.
#[cfg(test)]
use qcore_core::Effect;
pub(crate) use qcore_core::{
    ResourceId, cfg, cleanup_lowering, error_region, ownership, resource_flow, typed_errors, usage,
};

pub mod whole_program;
pub use whole_program::{
    CallSite, CheckedFunction, CheckedProgram, FunctionSemantics,
    FunctionSummary as WholeFunctionSummary, ParameterOwnership, check_whole_program,
};
