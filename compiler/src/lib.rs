//! Executable implementation of the QCore-C static semantics.
//!
//! The implemented vertical slices cover lexical/index foundations,
//! interval-graded usage analysis, ownership, static resource-flow bounds,
//! path-sensitive typed recoverable errors, checked error-region cleanup, and
//! cleanup-explicit CFG lowering to C11.

pub mod ast;
pub mod c_emit;
pub mod cfg;
pub mod check;
pub mod cleanup_lowering;
pub mod error_region;
pub mod index;
pub mod lexer;
pub mod ownership;
pub mod parser;
pub mod resource_flow;
pub mod typed_errors;
pub mod usage;

pub use ast::{AxisDecl, AxisMode, Declaration, DimensionDecl, DimensionRef, Program};
pub use c_emit::{CEmitError, emit_c11, parse_c_observation};
pub use cfg::{
    BasicBlock, BlockId, CfgError, CfgInstruction, CfgTerminator, ErrorAbi, ErrorTag, ErrorVariant,
    ExecutionObservation, ExecutionOutcome, TraceEvent, TypedCfg, interpret_cfg,
};
pub use check::{CheckError, StaticEnvironment};
pub use cleanup_lowering::{
    LoweringError, SourceOp, SourceProgram, interpret_source, lower_cleanup_cfg,
};
pub use error_region::{
    CleanupAction, CleanupBinding, CleanupEvent, CleanupPack, ErrorRegionError, QuantOpen,
    RegionCatchClause, RegionExitMap, RegionExitState, RegionExitTag, RegionName, RegionOp,
    analyze_error_regions,
};
pub use index::{IndexError, IndexExpression, IndexTerm};
pub use lexer::{LexError, Span, Token, TokenKind, lex};
pub use ownership::{
    Footprint, OwnershipArithmeticError, OwnershipError, OwnershipOp, OwnershipState,
    ResourceEntry, ResourceId, check_ownership, execute_ownership,
};
pub use parser::{ParseError, parse_program};
pub use resource_flow::{
    Flow, FlowError, FunctionSummary, NetChange, Quantity, ResourceBounds, interpret_flow_trace,
};
pub use typed_errors::{
    CallExit, CallSummary, CatchClause, Effect, EffectRow, ErrorDeclarations, ErrorLedger,
    ErrorName, ErrorOp, ExitMap, ExitState, ExitTag, LedgerTransition, TypedErrorError,
    analyze_typed_errors, check_no_unhandled_errors,
};
pub use usage::{
    DemandContext, DemandError, Usage, UsageArithmeticError, UsageCheckError, UsageExpr,
    UsageUpper, UsageViolation, check_usage,
};
