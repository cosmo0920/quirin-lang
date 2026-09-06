//! QCore language front end (layer L1 of `docs/GENERALIZATION_PLAN.md`
//! §3.1): lexer, axis/dimension parser and static environment, quant-index
//! parsing/resolution, and the source front end with elaboration.
//!
//! Moved from `qcore-compiler` at the W1 relocation; the compiler crate
//! re-exports every module here at its previous path.

pub mod check;
pub mod frontend;
pub mod lexer;
pub mod parser;
pub mod quant_index;

pub use parser::parse_program;
