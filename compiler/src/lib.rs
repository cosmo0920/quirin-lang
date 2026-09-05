//! Executable implementation of the QCore-C static semantics.
//!
//! The implemented vertical slices cover lexical/index foundations,
//! interval-graded usage analysis, and ownership without exceptional exits.

pub mod ast;
pub mod check;
pub mod index;
pub mod lexer;
pub mod ownership;
pub mod parser;
pub mod usage;

pub use ast::{AxisDecl, AxisMode, Declaration, DimensionDecl, DimensionRef, Program};
pub use check::{CheckError, StaticEnvironment};
pub use index::{IndexError, IndexExpression, IndexTerm};
pub use lexer::{LexError, Span, Token, TokenKind, lex};
pub use ownership::{
    Footprint, OwnershipArithmeticError, OwnershipError, OwnershipOp, OwnershipState,
    ResourceEntry, ResourceId, check_ownership, execute_ownership,
};
pub use parser::{ParseError, parse_program};
pub use usage::{
    DemandContext, DemandError, Usage, UsageArithmeticError, UsageCheckError, UsageExpr,
    UsageUpper, UsageViolation, check_usage,
};
