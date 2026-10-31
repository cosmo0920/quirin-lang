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
//! Cycle 20 adds a versioned, fail-closed verifier-provider registry and makes
//! first-party library verification use the same certificate firewall.

pub use qcore_verification::source_imports;

pub use qcore_core::advanced_control;
pub use qcore_core::ast;
pub use qcore_core::c_emit;
pub use qcore_core::cfg;
pub use qcore_frontend::check;
pub mod chunk_contract;
pub use qcore_core::chunk_lifecycle;
pub use qcore_core::cleanup_lowering;
/// Typed diagnostic adapters for native verification harnesses using this facade.
pub use qcore_core::diagnostic_log_c_emit;
pub use qcore_core::scalar_floats;
pub use qcore_core::scalar_types;
pub use qcore_core::std_io;
pub mod cleanup_outcome;
pub use qcore_contract::contract_loader;
pub use qcore_contract::contract_observation;
pub use qcore_contract::profile_loader;
pub use qcore_core::prelude::concurrency::concurrency;
#[path = "call_use_source_capability.rs"]
pub mod call_use_source_capability;
#[path = "cleanup_replay_source_capability.rs"]
pub mod cleanup_replay_source_capability;
#[path = "composition_source_capability.rs"]
pub mod composition_source_capability;
pub mod counter_contract;
pub mod counter_source;
pub mod driver;
#[path = "teardown_source_capability.rs"]
pub mod teardown_source_capability;
pub use qcore_core::error_region;
pub use qcore_core::executable_model;
pub mod external_integration;
pub mod external_source;
pub mod field_destructor;
pub use qcore_c_gateway::legacy_c;
// Card W48-QRLIB-LINKAGE: the symbol readers, the `.qrlib` generator and the
// link plan, for the `quirin qrlib` and `quirin build` commands.
pub use qcore_c_gateway::linkage;
// Card RULE-SWITCH-ENTRY-POINTS: the rule switch, for `quirin`'s admission.
pub use qcore_c_gateway::gateway_rules;
pub use qcore_core::index;
pub use qcore_core::inference;
pub use qcore_frontend::frontend;
pub use qcore_frontend::lexer;
pub mod list_unlink_contract;
pub use qcore_core::ownership;
pub use qcore_core::prelude;
pub use qcore_core::prelude::concurrency::native_concurrency;
pub use qcore_frontend::parser;
pub use qcore_frontend::quant_index;
pub use qcore_verification::obligation_derivation;
pub mod first_party;
pub use qcore_core::research_closure;
pub mod research_extensions;
pub use qcore_core::resource_flow;
pub mod source_call_use;
mod source_contract;
pub mod source_profiles;
pub use qcore_core::translation_validation;
pub use qcore_core::typed_errors;
pub mod typed_source;
pub mod typed_source_match;
pub use qcore_core::usage;
pub mod verifier_plugin;
pub mod verifier_plugins;
pub use qcore_core::prelude::concurrency::weak_memory;
pub use qcore_frontend::whole_program;
pub use qcore_verification::verify_dispatch;
pub use qcore_verification::witness_replay;

// Compatibility paths for callers predating the in-process plugin extraction.
pub use verifier_plugins::chunkio::chunk_source;
pub use verifier_plugins::libevent::event_driven;

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
    AccessFootprint, AccessKind, AccessPermission, ConcurrencyAssumption, ConcurrencyCertificate,
    ConcurrencyDiagnostic, ConcurrencyProfile, ConcurrentEvent, ConcurrentProgram, FULL_ACCESS,
    LockRank, MutexId, SharedResourceId, TaskId, check_concurrent,
};
pub use driver::{
    BitfieldTaskAssumption, CheckedComposedModule, CheckedEpochModule, CheckedExternModule,
    CheckedMutexGuardModule, CheckedSourceModule, CheckedStdIoModule, CompileOptions,
    CompiledModule, ComposedEpochModule, ComposedRegionEvidence, CustodyPayloadProfile,
    EmittedSourceMap, FileDiagnostic, RecordedTarget, SourceExecutionProfile, SourceFile,
    SourceLocation, SourceResultProfile, compile_files, compile_files_for_target,
    compile_files_with_filesystem, compile_files_with_filesystem_for_target,
    compile_files_with_filesystem_with_registry,
    compile_files_with_filesystem_with_registry_for_target, compile_files_with_registry,
    compile_files_with_registry_for_target, compile_source_bundle_with_registry, format_diagnostic,
};
pub use error_region::{
    CleanupAction, CleanupBinding, CleanupEvent, CleanupPack, ErrorRegionError, QuantOpen,
    RegionCatchClause, RegionExitMap, RegionExitState, RegionExitTag, RegionName, RegionOp,
    analyze_error_regions,
};
pub use event_driven::{
    ActivationReason, ArgumentMode, BuffereventId, BuffereventKey, CallbackArgumentId,
    CallbackContext, CallbackSummary, ClassifiedCloseFailure, ClosePremiseDisposition,
    ClosePremiseRecheckError, DeclaredCloseFailure, DeclaredClosePremise, DeclaredCloseRefusal,
    DeleteMode, DescriptorId, EventBackend, EventBaseId, EventCertificate, EventDiagnostic,
    EventFlags, EventId, EventKey, EventLifecycle, EventOperation, EventProgram,
    ExecutionContextId, Generation, InvalidOrderingEvidence, LIBEVENT_SUBMODULE_PATH,
    LibeventBuildProfile, LibeventSourceError, LibeventSourceEvidence, LoopControl, ObservedEvent,
    ObservedKind, PayloadAccessKind, PayloadId, ProfileDisposition, ThreadMode,
    check_event_program, check_observation_refinement, classify_declared_close_failure,
    classify_libevent_symbol, explore_activation_orders, recheck_declared_close_premises,
    verify_pinned_libevent_source,
};
pub use executable_model::{
    ErrorIdentity as ModelErrorIdentity, ExecutableModel, ModelBuildError, ModelCleanupPack,
    ModelControl, ModelOutcome, ModelProgram, ModelState, ModelTraceEvent, ModelTypedError,
    NaturalIndex, Operation as ModelOperation, ResourceIdentity as ModelResourceIdentity,
    UsageBinding as ModelUsageBinding, UsageGrade as ModelUsageGrade, Variable as ModelVariable,
};
pub use external_integration::{
    ChunkImportCertificate, ChunkImportDiagnostic, ChunkImportProfile, DispatcherObservation,
    ExternalContainerId, ExternalContainerKind, ExternalContextId, ExternalEvidenceId,
    ExternalHolder, ExternalResourceId, ImportedChunkProgram, ImportedLibraryCustodyProgram,
    LibraryCustodyCertificate, LibraryCustodyDiagnostic, LibraryCustodyOperation,
    LibraryCustodyProgram, LibraryImportCertificate, LibraryImportDiagnostic,
    LibraryMechanismSummary, LibrarySummaryProfile, MonkeyCallback, MonkeyCallbackSummary,
    MonkeyCompositionCertificate, MonkeyCompositionDiagnostic, MonkeyCompositionProfile,
    MonkeyCompositionProgram, PINNED_FLUENT_BIT_REVISION, SynchronizationEvidenceKind,
    check_imported_chunk_program, check_imported_library_custody, check_library_custody,
    check_monkey_composition, check_monkey_composition_with_profile, chunk_operation_kind,
    required_chunk_summary,
};
pub use external_source::{
    ExternalSourceBundle, LibrarySourceCertificate, LibrarySourceDiagnostic,
    derive_library_summary_profile,
};
pub use frontend::{
    DeclarationKind as SourceDeclarationKind, Located, SemanticPass, SourceBlock,
    SourceContractClause, SourceContractClauseKind, SourceContractExitSelector, SourceDeclaration,
    SourceDiagnostic, SourceFunctionContract, SourceModuleMode, SourceParameter, SourceParseError,
    SourceProgram as CompleteSourceProgram, SourceTypeKind, StatementKind as SourceStatementKind,
    SymbolId as SourceSymbolId, TypedBinding, TypedSourceDeclaration, TypedSourceProgram,
    TypedStatement, VerifierProviderUse, VerifierScopeDeclaration, covered_semantic_passes,
    elaborate_source, parse_source_program, source_diagnostic,
};
pub use index::{IndexError, IndexExpression, IndexTerm};
pub use inference::{
    InferenceCertificate, InferenceConstraint, InferenceError, PrincipalSolution, UsageInterval,
    check_inference_certificate, infer_principal,
};
pub use legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, ContractTrust, DerivedExtent,
    EnumerationRefusal, LegacyCError, LegacyCertificateCoverage, LegacyCertificateDefinition,
    LegacyCertificateExpectedContext, LegacyCertificateInputIdentity, LegacyCertificateRequest,
    LegacyCertificateScope, LegacyCertificateScopeEvidence, LegacyCfg, LegacyCfgNode,
    LegacyDiagnostic, LegacyDiagnosticKind, LegacyFunctionScope, LegacyFunctionSummary,
    LegacyInstruction, LegacyNodeId, LegacyProgramSummary, LegacySafetyCertificate,
    ResourceApiKind, ResourceContracts, ResourceFunctionContract, ResourceKind,
    ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord, SourceDefinitionInventory,
    UnanalyzableDefinition, UnanalyzableForm, UnrestrictedLegacyResult, analyze_legacy_c,
    analyze_legacy_c_translation_unit, analyze_legacy_c11, analyze_unrestricted_c11,
    analyze_unrestricted_c11_with_inventory, check_legacy_safety_certificate,
    check_legacy_safety_certificate_for, enumerate_source_definitions, legacy_c_source_functions,
    legacy_certificate_expected_context, legacy_certificate_expected_context_from_prepared,
    validate_unrestricted_c11,
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
/// Card `source.extern-call.v1`: the bounded extern-call model, its C11
/// backend and its reference execution.
pub use qcore_core::{
    composed_std_io, composed_std_io_c_emit, extern_c_emit, extern_call, ffi, native_library,
    native_library_header,
};
/// Card `NATIVE-LIBRARY-HEADER`: the canonical digest a manifest's exported
/// header binding is measured with, re-exported so a consumer of this facade
/// measures with the same function the driver does rather than a second one.
pub use qcore_schemas::digest;
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
pub use verifier_plugin::{
    BoundOperation, BuiltInProvider, ChunkBackendRecord, ChunkEntityRef, ChunkLifecycleWitness,
    ChunkOperationRecord, ChunkResidencyRecord, CompositeCertificate, ContractObservationRecord,
    ContractObservationWitness, CoreCertificate, DerivationIncomplete, EvidenceClaim,
    FactPredicate, ManifestError, NativeRunBinding, NormalizedFact, Obligation, ObligationProperty,
    ObligationScope, ObligationSubject, ObservedRun, PLUGIN_EVIDENCE_VERSION,
    PLUGIN_EVIDENCE_VERSION_V2, PLUGIN_PROTOCOL_VERSION, PackageLoadError, PathBounds,
    PluginDiagnostic, PluginEntrypoint, PluginId, PluginLimits, PluginManifest, PluginTrustClass,
    ProcessProvider, ProposedEvidence, ProposedEvidenceV2, ProposedVerdict, ProviderFailure,
    RegistryError, Reproducibility, ResolvedVerifierScope, ResolvedVerifierUse,
    ScopeResolutionError, SemanticWitness, SourceSite, SupportedTarget, TargetProfile, TaskBounds,
    UnitIdentity, UnverifiedProvider, VERIFICATION_TASK_VERSION, VerificationOutcome,
    VerificationRequest, VerificationRequestV2, VerificationResponse, VerificationResponseV2,
    VerificationTask, VerifierCapability, VerifierProvider, VerifierRegistry, WitnessChecker,
    compose_certificates, compose_verification, compose_verification_with_inventory,
    decode_protocol_frame, encode_protocol_frame, first_party_manifests, first_party_registry,
    load_first_party_manifests, load_first_party_registry,
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

#[cfg(test)]
pub(crate) mod test_support {
    use std::io::Write;
    use std::sync::{Once, OnceLock};

    static CLANG_AVAILABILITY: OnceLock<Result<(), String>> = OnceLock::new();
    static CLANG_WARNING: Once = Once::new();

    /// Normalize compile-time text fixtures before syntax-preserving assembly
    /// or comparison, independently of Git's platform checkout policy.
    pub(crate) fn normalize_newlines(source: &str) -> String {
        source.replace("\r\n", "\n")
    }

    /// Execute syntax-only POSIX LP64 fixtures with their declared target, even
    /// when the installed Clang defaults to Windows LLP64. Child-only settings
    /// keep concurrent native-target tests and the user's override untouched.
    #[allow(dead_code)] // Remaining compiler fixtures do not call this after adapter extraction.
    pub(crate) fn posix_clang_test() -> bool {
        if !clang_available() {
            assert!(
                std::env::var_os("QCORE_TEST_POSIX_CHILD").is_none(),
                "the isolated POSIX child must execute its Clang-backed test"
            );
            return false;
        }
        #[cfg(windows)]
        if std::env::var_os("QCORE_TEST_POSIX_CHILD").is_none() {
            use std::path::PathBuf;
            use std::process::Command;
            static LAUNCHER: OnceLock<PathBuf> = OnceLock::new();
            let clang = qcore_c_gateway::clang_command().expect("POSIX fixtures require Clang");
            let launcher = LAUNCHER.get_or_init(|| {
                let directory = std::env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join(format!("qcore-posix-clang-{}", std::process::id()));
                std::fs::create_dir_all(&directory).unwrap();
                let source = directory.join("launcher.rs");
                let executable = directory.join("clang-posix-fixture.exe");
                std::fs::write(
                    &source,
                    r#"
fn main() {
    let status = std::process::Command::new(std::env::var_os("QCORE_TEST_REAL_CLANG").unwrap())
        .arg("--target=x86_64-unknown-linux-gnu")
        .args(std::env::args_os().skip(1))
        .status().expect("launch discovered Clang");
    std::process::exit(status.code().unwrap_or(1));
}
"#,
                )
                .unwrap();
                let output = Command::new("rustc")
                    .arg(&source)
                    .arg("-o")
                    .arg(&executable)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                executable
            });
            let thread = std::thread::current();
            let test = thread.name().expect("libtest names its test threads");
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", test, "--nocapture"])
                .env("QCORE_TEST_POSIX_CHILD", "1")
                .env("QCORE_TEST_REAL_CLANG", clang.get_program())
                .env("QCORE_CLANG", launcher)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "POSIX target test {test}:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
                "child must execute the selected test: {output:?}"
            );
            return false;
        }
        true
    }

    /// Report whether the Clang-backed tests can run, emitting one visible
    /// warning when the executable is absent or unusable.
    pub(crate) fn clang_available() -> bool {
        let availability =
            CLANG_AVAILABILITY.get_or_init(|| qcore_c_gateway::clang_command().map(|_| ()));
        match availability {
            Ok(()) => true,
            Err(reason) => {
                CLANG_WARNING.call_once(|| {
                    // Write directly so libtest's successful-test capture does
                    // not hide the reason these tests did not execute.
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "warning: skipping Clang-dependent tests: {reason}"
                    );
                });
                false
            }
        }
    }

    #[test]
    fn newline_normalization_preserves_lf_syntax() {
        let source = "int f(void) {\n    return 0;\n}\n";
        assert_eq!(normalize_newlines(source), source);
    }

    #[test]
    fn newline_normalization_maps_crlf_and_mixed_syntax_to_lf() {
        assert_eq!(
            normalize_newlines("int f(void) {\r\n    return 0;\n}\r\n"),
            "int f(void) {\n    return 0;\n}\n"
        );
    }

    #[test]
    fn newline_normalization_is_idempotent() {
        let once = normalize_newlines("a\r\nb\nc\r\n");
        assert_eq!(normalize_newlines(&once), once);
    }
}
