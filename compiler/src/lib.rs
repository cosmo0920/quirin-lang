//! Executable implementation of the QCore-C static semantics.
//!
//! The implemented vertical slices cover lexical/index foundations,
//! interval-graded usage analysis, ownership, static resource-flow bounds,
//! path-sensitive typed recoverable errors, checked error-region cleanup, and
//! cleanup-explicit CFG lowering to C11, plus a reduced executable model tying
//! those safety mechanisms together for the Cycle 8 metatheory, and a
//! conservative annotated legacy-C resource analyzer for Cycle 9. Cycles
//! 10–12 add the source front end, whole-program checking, and compiler driver.
//! Cycles 13–16 add translation validation, restricted concurrency, advanced
//! control fragments, and a bounded combined research-closure certificate.

pub mod advanced_control;
pub mod ast;
pub mod c_emit;
pub mod cfg;
pub mod check;
pub mod cleanup_lowering;
pub mod concurrency;
pub mod driver;
pub mod error_region;
pub mod executable_model;
pub mod frontend;
pub mod index;
pub mod legacy_c;
pub mod lexer;
pub mod ownership;
pub mod parser;
pub mod research_closure;
pub mod resource_flow;
pub mod translation_validation;
pub mod typed_errors;
pub mod usage;
pub mod whole_program;

pub use advanced_control::{
    AdvancedControlError, AdvancedExecution, AdvancedOp, AdvancedOutcome, AdvancedProgram,
    AdvancedResourceId, AdvancedTraceEvent, Capture, CaptureMode, DestructorId, DestructorOutcome,
    DestructorSpec, HandlerId, HandlerKind, HandlerSpec, PrimaryFailure, run_advanced,
};
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
pub use concurrency::{
    AccessFootprint, AccessKind, AccessPermission, ConcurrencyCertificate, ConcurrencyDiagnostic,
    ConcurrentEvent, ConcurrentProgram, FULL_ACCESS, LockRank, MutexId, SharedResourceId, TaskId,
    check_concurrent,
};
pub use driver::{
    CompileOptions, CompiledModule, EmittedSourceMap, FileDiagnostic, SourceFile, SourceLocation,
    compile_files, format_diagnostic,
};
pub use error_region::{
    CleanupAction, CleanupBinding, CleanupEvent, CleanupPack, ErrorRegionError, QuantOpen,
    RegionCatchClause, RegionExitMap, RegionExitState, RegionExitTag, RegionName, RegionOp,
    analyze_error_regions,
};
pub use executable_model::{
    ErrorIdentity as ModelErrorIdentity, ExecutableModel, ModelBuildError, ModelCleanupPack,
    ModelControl, ModelOutcome, ModelProgram, ModelState, ModelTraceEvent, ModelTypedError,
    NaturalIndex, Operation as ModelOperation, ResourceIdentity as ModelResourceIdentity,
    UsageBinding as ModelUsageBinding, UsageGrade as ModelUsageGrade, Variable as ModelVariable,
};
pub use frontend::{
    DeclarationKind as SourceDeclarationKind, Located, SemanticPass, SourceBlock,
    SourceDeclaration, SourceDiagnostic, SourceParameter, SourceParseError,
    SourceProgram as CompleteSourceProgram, SourceTypeKind, StatementKind as SourceStatementKind,
    SymbolId as SourceSymbolId, TypedBinding, TypedSourceDeclaration, TypedSourceProgram,
    TypedStatement, covered_semantic_passes, elaborate_source, parse_source_program,
    source_diagnostic,
};
pub use index::{IndexError, IndexExpression, IndexTerm};
pub use legacy_c::{
    AnalysisStatus, LegacyCError, LegacyCfg, LegacyCfgNode, LegacyDiagnostic, LegacyDiagnosticKind,
    LegacyFunctionSummary, LegacyInstruction, LegacyNodeId, LegacyProgramSummary, ResourceApiKind,
    ResourceContracts, analyze_legacy_c, analyze_legacy_c_translation_unit, analyze_legacy_c11,
};
pub use lexer::{LexError, Span, Token, TokenKind, lex};
pub use ownership::{
    Footprint, OwnershipArithmeticError, OwnershipError, OwnershipOp, OwnershipState,
    ResourceEntry, ResourceId, check_ownership, execute_ownership,
};
pub use parser::{ParseError, parse_program};
pub use research_closure::{
    OPEN_RESEARCH_QUESTIONS, OpenResearchQuestion, Progress, ResearchClosureCertificate,
    ResearchClosureError, ResearchProgram, ResourceSafetyCertificate, close_research_program,
};
pub use resource_flow::{
    Flow, FlowError, FunctionSummary, NetChange, Quantity, ResourceBounds, interpret_flow_trace,
};
pub use translation_validation::{
    TranslationCertificate, TranslationValidationError, validate_cfg_boundary, validate_translation,
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
pub use whole_program::{
    CallSite, CheckedFunction, CheckedProgram, FunctionSemantics,
    FunctionSummary as WholeFunctionSummary, ParameterOwnership, check_whole_program,
};
