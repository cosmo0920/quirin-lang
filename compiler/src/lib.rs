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
//! Cycle 17 adds independently checked inference and loop certificates, a
//! conservative unrestricted-C gateway, explicit C11 candidate executions,
//! native threaded execution, and a mechanically interpreted ledger category.
//! Cycle 18 adds a fail-closed Libevent profile, generation-indexed event and
//! bufferevent lifecycles, callback authority, and observed-trace refinement.
//! Cycle 19 begins the pinned Fluent Bit integration with rich external-call
//! resource contracts and path-sensitive nullable/reallocation custody.

pub mod advanced_control;
pub mod ast;
pub mod c_emit;
pub mod cfg;
pub mod check;
pub mod chunk_lifecycle;
pub mod cleanup_lowering;
pub mod concurrency;
pub mod driver;
pub mod error_region;
pub mod event_driven;
pub mod executable_model;
pub mod frontend;
pub mod index;
pub mod inference;
pub mod legacy_c;
pub mod lexer;
pub mod native_concurrency;
pub mod ownership;
pub mod parser;
pub mod research_closure;
pub mod research_extensions;
pub mod resource_flow;
pub mod translation_validation;
pub mod typed_errors;
pub mod usage;
pub mod weak_memory;
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
pub use chunk_lifecycle::{
    BackingDeleteOutcome, ChunkBackend, ChunkCertificate, ChunkContextId, ChunkContextRecord,
    ChunkDiagnostic, ChunkId, ChunkOpenFailure, ChunkOperation, ChunkProgram, ChunkRecord,
    ChunkResidency, ChunkRouteId, ChunkState, ChunkStreamId, ChunkStreamRecord, ChunkTaskId,
    ChunkTraceEvent, check_chunk_program,
};
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
pub use event_driven::{
    ActivationReason, ArgumentMode, BuffereventId, BuffereventKey, CallbackArgumentId,
    CallbackContext, CallbackSummary, DeleteMode, DescriptorId, EventBackend, EventBaseId,
    EventCertificate, EventDiagnostic, EventFlags, EventId, EventKey, EventLifecycle,
    EventOperation, EventProgram, ExecutionContextId, Generation, InvalidOrderingEvidence,
    LIBEVENT_API_PROFILE, LibeventBuildProfile, LibeventSourceError, LibeventSourceEvidence,
    LoopControl, ObservedEvent, ObservedKind, PINNED_LIBEVENT_COMMIT, PINNED_LIBEVENT_DESCRIPTION,
    PayloadAccessKind, PayloadId, ProfileDisposition, ThreadMode, check_event_program,
    check_observation_refinement, classify_libevent_symbol, explore_activation_orders,
    verify_pinned_libevent_source,
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
pub use inference::{
    InferenceCertificate, InferenceConstraint, InferenceError, PrincipalSolution, UsageInterval,
    check_inference_certificate, infer_principal,
};
pub use legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, LegacyCError, LegacyCfg, LegacyCfgNode,
    LegacyDiagnostic, LegacyDiagnosticKind, LegacyFunctionScope, LegacyFunctionSummary,
    LegacyInstruction, LegacyNodeId, LegacyProgramSummary, LegacySafetyCertificate,
    ResourceApiKind, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode, UnrestrictedLegacyResult, analyze_legacy_c,
    analyze_legacy_c_translation_unit, analyze_legacy_c11, analyze_unrestricted_c11,
    check_legacy_safety_certificate, legacy_c_source_functions, validate_unrestricted_c11,
};
pub use lexer::{LexError, Span, Token, TokenKind, lex};
pub use native_concurrency::{
    FeatureDisposition, NativeBackendError, NativeCellId, NativeExecutionCertificate,
    NativeOperation, NativeProgram, NativeResourceId, NativeScheduleStep, NativeTaskId,
    PROMOTED_FEATURE_PROFILE, PlatformMatrixCertificate, PlatformMatrixError, PlatformRun,
    PromotedConcurrencyFeature, RequiredPlatform, check_platform_matrix, run_native_backend,
};
pub use ownership::{
    Footprint, OwnershipArithmeticError, OwnershipError, OwnershipOp, OwnershipState,
    ResourceEntry, ResourceId, check_ownership, execute_ownership,
};
pub use parser::{ParseError, parse_program};
pub use research_closure::{
    OPEN_RESEARCH_QUESTIONS, OpenResearchQuestion, PROMOTED_RESEARCH_QUESTIONS, Progress,
    ResearchClosureCertificate, ResearchClosureError, ResearchProgram, ResourceSafetyCertificate,
    close_research_program,
};
pub use research_extensions::{
    CategoricalCertificate, CategoryError, Cycle17Certificate, Cycle17Error, Cycle17Input, Ledger,
    LedgerMorphism, LoopBoundEvidence, LoopSummaryCertificate, LoopSummaryError, ResearchExtension,
    check_categorical_account, check_loop_summary, close_cycle17, loop_ledger_morphism,
    native_ledger_morphism, summarize_general_loop,
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
pub use weak_memory::{
    C11Execution, C11ExecutionCertificate, MemoryEvent, MemoryEventId, MemoryEventKind,
    MemoryLocation, MemoryOrder, MemoryThreadId, RelationEdge, WeakMemoryError,
    check_c11_execution,
};
pub use whole_program::{
    CallSite, CheckedFunction, CheckedProgram, FunctionSemantics,
    FunctionSummary as WholeFunctionSummary, ParameterOwnership, check_whole_program,
};
