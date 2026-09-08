//! Provider-independent semantic kernel of QCore-C (layer L1).
//!
//! Includes interval usage and index vocabulary, ownership and resource flow,
//! typed recoverable errors and cleanup, CFG interpretation and C11 emission,
//! executable models, advanced control, principal inference, bounded concurrency,
//! explicit weak-memory candidates, native model execution and translation validation.
//! Source parsing/driving, external C analysis and library adapters live in higher
//! layers. This crate has no runtime dependencies and no workspace dependencies.

pub mod advanced_control;
pub mod aggregate_composition;
pub mod ast;
pub mod c_emit;
pub mod capacity_boundary;
pub mod cfg;
pub mod chunk_integrity;
pub mod cleanup_lowering;
pub mod concurrency;
pub mod disjoint_footprint;
pub mod error_region;
pub mod executable_model;
pub mod extern_c_emit;
pub mod extern_call;
pub mod index;
pub mod inference;
pub mod library_custody;
pub mod module_imports;
pub mod mutex_c_emit;
pub mod mutex_invariant;
pub mod native_concurrency;
pub mod os_endpoint;
pub mod ownership;
pub mod prelude;
pub mod research_closure;
pub mod resource_flow;
pub mod translation_validation;
pub mod typed_errors;
pub mod usage;
pub mod weak_memory;

pub use advanced_control::{
    AdvancedControlError, AdvancedExecution, AdvancedOp, AdvancedOutcome, AdvancedProgram,
    AdvancedResourceId, AdvancedTraceEvent, Capture, CaptureMode, DestructorId, DestructorOutcome,
    DestructorSpec, HandlerId, HandlerKind, HandlerSpec, PrimaryFailure, run_advanced,
};
pub use c_emit::{CEmitError, emit_c11, parse_c_observation};
pub use cfg::{
    BasicBlock, BlockId, CfgError, CfgInstruction, CfgTerminator, ErrorAbi, ErrorTag, ErrorVariant,
    ExecutionObservation, ExecutionOutcome, TraceEvent, TypedCfg, interpret_cfg,
};
pub use chunk_integrity::{
    AllocationLedger, CaseRecord, CaseVerdict, ConstructorOutcome, ContentClass, EndpointLedger,
    Injection, IntegrityRefusal, Verdict as IntegrityVerdict, classify_content, crc32,
    expected_outcome, parse_records as parse_integrity_records,
    policy_values as integrity_policy_values, validate_case as validate_integrity_case,
    validate_family as validate_integrity_family,
};
pub use cleanup_lowering::{
    LoweringError, SourceOp, SourceProgram, interpret_source, lower_cleanup_cfg,
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
pub use inference::{
    InferenceCertificate, InferenceConstraint, InferenceError, PrincipalSolution, UsageInterval,
    check_inference_certificate, infer_principal,
};
pub use os_endpoint::{
    DescriptionState, DescriptorState, Duty, EndpointKind, Epoch, Errno,
    Outcome as EndpointOutcome, PostState, PostStateClaim, Premise, Premises, Recovery, Refusal,
    RelationError, Target, WriteBackState, check_claim as check_endpoint_claim, close_poststate,
    join as join_endpoint_poststates, poisoned_duties,
};
pub use ownership::{
    Footprint, OwnershipArithmeticError, OwnershipError, OwnershipOp, OwnershipState,
    ResourceEntry, ResourceId, check_ownership, execute_ownership,
};
pub use resource_flow::{
    Flow, FlowError, FunctionSummary, NetChange, Quantity, ResourceBounds, interpret_flow_trace,
};
pub use typed_errors::{
    CallExit, CallSummary, CatchClause, Effect, EffectRow, ErrorDeclarations, ErrorLedger,
    ErrorName, ErrorOp, ExitMap, ExitState, ExitTag, LedgerTransition, TypedErrorError,
    analyze_typed_errors, check_no_unhandled_errors,
};

pub use concurrency::{
    AccessFootprint, AccessKind, AccessPermission, ConcurrencyAssumption, ConcurrencyCertificate,
    ConcurrencyDiagnostic, ConcurrencyProfile, ConcurrentEvent, ConcurrentProgram, FULL_ACCESS,
    LockRank, MutexId, SharedResourceId, TaskId, check_concurrent,
};
pub use disjoint_footprint::{
    AllocationId, DerivedFootprint, DescriptorId, DisjointnessJudgement, ElementShape, FieldId,
    FieldLayout, FieldStorage, FootprintAssumption, FootprintCertificate, FootprintDiagnostic,
    FootprintEvent, FootprintExport, FootprintProfile, FootprintProgram, FootprintRecheckError,
    Generation, HandleId, LayoutContract, LayoutDefect, ProtocolCellId, RecordId,
    UnsupportedFootprint, check_disjoint_footprint_program, recheck_disjoint_footprint_certificate,
};
pub use mutex_invariant::{
    BorrowId, CertificateRecheckError, DerivedSyncEdge, FactId, GuardId, IdentityKind, InvariantId,
    InvariantPhase, InvariantTransition, InvariantWitness, MutexInvariantCertificate,
    MutexInvariantDiagnostic, MutexInvariantEvent, MutexInvariantProgram, ShareGrant, SyncWitness,
    UnsupportedSynchronization, check_mutex_invariant_program, recheck_mutex_invariant_certificate,
};
pub use native_concurrency::{
    FeatureDisposition, NativeBackendError, NativeCellId, NativeExecutionCertificate,
    NativeOperation, NativeProgram, NativeResourceId, NativeScheduleStep, NativeTaskId,
    PROMOTED_FEATURE_PROFILE, PlatformMatrixCertificate, PlatformMatrixError, PlatformRun,
    PromotedConcurrencyFeature, RequiredPlatform, check_platform_matrix, run_native_backend,
};
pub use research_closure::{
    OPEN_RESEARCH_QUESTIONS, OpenResearchQuestion, PROMOTED_RESEARCH_QUESTIONS, Progress,
    ResearchClosureCertificate, ResearchClosureError, ResearchProgram, ResourceSafetyCertificate,
    close_research_program,
};
pub use translation_validation::{
    TranslationCertificate, TranslationValidationError, validate_cfg_boundary, validate_translation,
};
pub use weak_memory::{
    C11Execution, C11ExecutionCertificate, MemoryEvent, MemoryEventId, MemoryEventKind,
    MemoryLocation, MemoryOrder, MemoryThreadId, RelationEdge, WeakMemoryError,
    check_c11_execution,
};

pub mod chunk_lifecycle;

pub mod resource_accounting;

pub mod epoch_accounting;

pub mod event_reactor;

pub mod region_task_credit;

pub mod scoped_tasks;

pub mod task_c_emit;
pub mod task_execution;
pub mod task_trace_relation;
