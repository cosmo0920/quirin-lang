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
pub mod atomic_publication;
pub mod c_emit;
pub mod c_program;
pub mod callback_task_handoff;
pub mod capacity_boundary;
pub mod cfg;
pub mod chunk_integrity;
pub mod cleanup_lowering;
pub mod composed_std_io;
pub mod composed_std_io_c_emit;
pub mod composed_tasks;
pub mod concurrency;
pub mod diagnostic_log_c_emit;
pub mod disjoint_footprint;
pub mod error_region;
pub mod executable_model;
// Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z).
pub mod bounds_body_standing;
pub mod execution_coverage;
pub mod extern_c_emit;
pub mod extern_call;
pub mod ffi;
pub mod ffi_c_emit;
pub mod footprint_mutex;
pub mod index;
pub mod index_extent;
pub mod index_extent_frames;
pub mod index_quant;
pub mod inference;
pub mod io_emit;
pub mod library_custody;
pub mod local_condition;
pub mod module_imports;
pub mod mutex_c_emit;
// Card NULL-SAFETY-PROPERTY (gateway rules 14k-14p).
pub mod mutex_invariant;
pub mod native_concurrency;
pub mod native_library;
pub mod native_library_header;
pub mod null_deref;
// Card NULL-MEMBER-INVARIANT-SCOPE (gateway rules 14x-14z).
pub mod null_member_scope;
pub mod null_paths;
pub mod null_triage;
pub mod obs_reordering;
pub mod os_endpoint;
pub mod ownership;
pub mod prelude;
// Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z).
pub mod reclamation;
// Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4, G05j, gateway rule 38e).
pub mod research_closure;
pub mod resource_calls;
pub mod resource_calls_c_emit;
pub mod resource_flow;
pub mod result_set;
pub mod socket_address;
pub mod socket_event;
pub mod socket_footprint;
pub mod source_loop;
pub mod source_record;
pub mod std_buffer;
pub mod std_buffer_c_emit;
pub mod std_event;
pub mod std_event_c_emit;
pub mod std_http;
pub mod std_http_c_emit;
pub mod std_io;
pub mod std_io_c_emit;
pub mod std_socket;
pub mod std_socket_c_emit;
pub mod std_string;
pub mod std_string_c_emit;
pub mod translation_validation;
pub mod typed_errors;
pub mod usage;
pub mod weak_memory;
// Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION (C09.4, G05j, gateway rule 38e).
pub mod written_set;

pub use advanced_control::{
    AdvancedControlError, AdvancedExecution, AdvancedOp, AdvancedOutcome, AdvancedProgram,
    AdvancedResourceId, AdvancedTraceEvent, Capture, CaptureMode, DestructorId, DestructorOutcome,
    DestructorSpec, HandlerId, HandlerKind, HandlerSpec, PrimaryFailure, run_advanced,
};
pub use atomic_publication::{
    AtomicPublicationCertificate, AtomicPublicationProgram, AtomicPublicationRefusal,
    check_atomic_publication,
};
pub use c_emit::{CEmitError, emit_c11, emit_c11_with_log_level, parse_c_observation};
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
    CloseRecord, CloseRecordCertificate, CloseRecordRefusal, DescriptionState, DescriptorState,
    Duty, EndpointKind, Epoch, Errno, Outcome as EndpointOutcome, PostState, PostStateClaim,
    Premise, Premises, Recovery, Refusal, RelationError, Target, WriteBackState,
    check_claim as check_endpoint_claim, check_close_record, close_poststate,
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
    AdmittedGuardLocalJoin, BorrowId, CertificateRecheckError, DerivedSyncEdge, FactId, GuardId,
    IdentityKind, InvariantId, InvariantPhase, InvariantTransition, InvariantWitness,
    MutexInvariantCertificate, MutexInvariantDiagnostic, MutexInvariantEvent,
    MutexInvariantProgram, ShareGrant, SyncWitness, UnsupportedSynchronization,
    check_mutex_invariant_program, recheck_mutex_invariant_certificate,
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
    C11Execution, C11ExecutionCertificate, CompareExchangeStrength, FenceScope, MemoryEvent,
    MemoryEventId, MemoryEventKind, MemoryLocation, MemoryOrder, MemoryThreadId, RelationEdge,
    WeakMemoryError, check_c11_execution, compare_exchange_failure_admitted,
    event_order_represented,
};

pub mod chunk_lifecycle;

pub mod resource_accounting;

pub mod epoch_accounting;
pub mod epoch_credit_recovery;

pub mod event_observation;
pub mod event_reactor;
// Card W47-LOOP-OWNER-CONTRACTS (gateway rule 10p).
pub mod loop_owner;

pub mod region_task_credit;

pub mod scoped_tasks;

pub mod task_c_emit;
pub mod task_execution;
pub mod task_trace_relation;

pub mod transfer;

// Card W47-CONDVAR-JOIN-PUBLICATION (gateway rule 10q).
pub mod condvar_join;

// Card W47-FLB-LIB-LIFECYCLE (gateway rule 10q as amended, C04.11).
pub mod paired_lifecycle;

pub mod scalar_calls;

// Card W48-QR-SCALAR-TYPES: the fixed-width scalar table of `.qr` sources.
pub mod scalar_types;

// Card W48-QR-FLOATING-POINT: IEEE binary32/binary64 values of the table.
pub mod scalar_floats;

// Card W48-QR-SCALAR-TYPES-R4: the value statement's decision, shared by the
// extern and task routes.
pub mod scalar_values;

pub mod scalar_calls_c_emit;

pub mod hypothesis;

pub mod premise_flow;
