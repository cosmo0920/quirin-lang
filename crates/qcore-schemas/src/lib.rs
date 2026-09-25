//! Published, versioned schema surface of the QCore verifier ecosystem.
//!
//! This crate is layer L0 of the pluggability target in
//! `docs/GENERALIZATION_PLAN.md` §3.1: the serde data types that cross the
//! core/provider boundary. It carries no analysis logic and must never
//! depend on any other workspace crate. Providers depend on this crate
//! only; the trusted core re-exports these types for compatibility.

pub mod contract;
pub mod digest;
pub mod identity;
pub mod mechanism;
pub mod obligation;
pub mod plugin;
pub mod profile;
pub mod reactor;
pub mod reason;

pub mod witness;

pub use contract::{
    ContractTrust, DerivedExtent, PendingHumanReview, ResourceApiKind, ResourceContracts,
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord,
    ReviewDeferral, ReviewScope,
};
pub use digest::{CANONICAL_SERIALIZER, sha256_hex};
pub use identity::ExternalSourceBundle;
pub use mechanism::LibraryMechanismSummary;
pub use obligation::{
    ChunkEntityRef, DerivationIncomplete, Obligation, ObligationProperty, ObligationScope,
    ObligationSubject, PathBounds, TaskBounds, TaskError, VERIFICATION_TASK_VERSION,
    VerificationTask,
};
pub use plugin::{
    DEFAULT_MAX_FRAME_BYTES, EvidenceClaim, FactPredicate, ManifestError, NormalizedFact,
    PLUGIN_EVIDENCE_VERSION, PLUGIN_PROTOCOL_VERSION, PluginDiagnostic, PluginEntrypoint, PluginId,
    PluginLimits, PluginManifest, PluginTrustClass, ProposedEvidence, ProposedEvidenceV2,
    ProposedVerdict, Reproducibility, RequestError, SupportedTarget, TargetProfile, UnitIdentity,
    VerificationRequest, VerificationRequestV2, VerificationResponse, VerificationResponseV2,
    VerifierCapability,
};
pub use profile::{
    ApiDisposition, AuditedFile, BooleanFieldModel, BuildProfileSelector,
    CHECKOUT_ROOT_SOURCE_ROOT, CallbackEffectSummary, ImportRole, LibraryProfile, OperationKind,
    PROFILE_MANIFEST_VERSION, ProfileManifest, ProviderBinding, RequiredEvidenceKind,
    SUPPORTED_PROFILE_MANIFEST_VERSIONS, SignatureConstraint, SourcePin, SourceRootSelection,
    SourceWitness, TypedImportProfile, UnitSelection, WitnessCheck,
};
pub use reason::{ReasonCategory, ReasonSubject, UnclassifiedReason, UnknownReasonCategory};
pub use witness::{
    BoundOperation, ChunkBackendRecord, ChunkLifecycleWitness, ChunkOperationRecord,
    ChunkResidencyRecord, ContractObservationRecord, ContractObservationWitness, NativeRunBinding,
    ObservedRun, PLUGIN_EVIDENCE_VERSION_V2, SemanticWitness, SourceSite,
};

pub mod call_use_summary;
pub mod chunk_summary;
pub mod cleanup_replay_summary;
pub mod composition_summary;
pub mod counter_summary;
pub mod reactor_program;
pub mod teardown_summary;
