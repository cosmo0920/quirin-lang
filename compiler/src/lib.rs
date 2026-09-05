//! Executable implementation of the QCore-C static semantics.
//!
//! The initial vertical slice covers lexical analysis, dimension declarations,
//! quantitative-axis declarations, pretty printing, and name validation.

pub mod ast;
pub mod check;
pub mod index;
pub mod lexer;
pub mod parser;

pub use ast::{AxisDecl, AxisMode, Declaration, DimensionDecl, DimensionRef, Program};
pub use check::{CheckError, StaticEnvironment};
pub use index::{IndexError, IndexExpression, IndexTerm};
pub use lexer::{LexError, Span, Token, TokenKind, lex};
pub use parser::{ParseError, parse_program};
