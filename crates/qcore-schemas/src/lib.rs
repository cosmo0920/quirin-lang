//! Published, versioned schema surface of the QCore verifier ecosystem.
//!
//! This crate is layer L0 of the pluggability target in
//! `docs/GENERALIZATION_PLAN.md` §3.1: the serde data types that cross the
//! core/provider boundary. It carries no analysis logic and must never
//! depend on any other workspace crate. Providers depend on this crate
//! only; the trusted core re-exports these types for compatibility.

pub mod contract;
pub mod identity;
pub mod mechanism;
pub mod plugin;

pub use contract::{
    ContractTrust, ResourceApiKind, ResourceContracts, ResourceFunctionContract,
    ResourceParameterMode, ResourceReturnMode, ResourceSummaryRecord,
};
pub use identity::ExternalSourceBundle;
pub use mechanism::LibraryMechanismSummary;
pub use plugin::{
    DEFAULT_MAX_FRAME_BYTES, EvidenceClaim, FactPredicate, ManifestError, NormalizedFact,
    PLUGIN_EVIDENCE_VERSION, PLUGIN_PROTOCOL_VERSION, PluginDiagnostic, PluginEntrypoint, PluginId,
    PluginLimits, PluginManifest, PluginTrustClass, ProposedEvidence, ProposedVerdict,
    Reproducibility, RequestError, SupportedTarget, TargetProfile, UnitIdentity,
    VerificationRequest, VerificationResponse, VerifierCapability,
};
