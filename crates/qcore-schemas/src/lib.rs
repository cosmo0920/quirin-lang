// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Published, versioned schema surface of the QCore verifier ecosystem.
//!
//! This crate is layer L0 of the pluggability target in
//! `docs/GENERALIZATION_PLAN.md` §3.1: the serde data types that cross the
//! core/provider boundary. It carries no analysis logic and must never
//! depend on any other workspace crate. Providers depend on this crate
//! only; the trusted core re-exports these types for compatibility.

// Card CG22-CALLBACK-REGISTRY-CONTRACT: callback contracts.
pub mod arm_release;
pub mod callback;
// Card AUDIT-COMPOSITION: the integration semigroup across results.
pub mod composition;
pub mod constructed_state;
pub mod contract;
pub mod descriptor_cell;
// Card VERDICT-CONTRACT-DEPENDENCIES (gateway rule 13f): the per-thread log of
// the names a resource environment was asked for; evidence only.
pub mod contract_consultation;
pub mod contract_refutation;
pub mod contract_revision;
pub mod contract_supersession;
pub mod digest;
pub mod handoff;
pub mod identity;
pub mod loop_lifetime;
pub mod mechanism;
pub mod member_effect;
pub mod member_path;
pub mod obligation;
pub mod plugin;
pub mod profile;
pub mod reactor;
pub mod reason;
// Card REFCOUNT-RESOURCE-KIND: reference counts.
pub mod reference_count;
// Card CG23-RELATIONAL-POSTCONDITIONS: return-code dispositions.
pub mod relational_post;
// Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4, gateway rule 38e).
pub mod result_range;
pub mod retained_by;
// Card SLOT-MOVE-AND-INTERIOR-BORROW: slot moves and derived out slots.
pub mod slot_move;
pub mod verdict;

pub mod witness;

pub use contract::{
    ContainerEffect, ContractTrust, CountedUserClauses, DerivedExtent, PendingHumanReview,
    ResourceApiKind, ResourceContracts, ResourceFunctionContract, ResourceParameterMode,
    ResourceReturnMode, ResourceSummaryRecord, ReviewDeferral, ReviewDisposition, ReviewScope,
    UserEffect, UserExitClause, UserOperation,
};
pub use contract_revision::{
    DeclaredRevision, PinMismatch, PinMismatchReason, RevisionMarker, RevisionMode, RevisionScope,
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
pub use verdict::{
    Chain, ChainKind, ChainLink, PendingReason, ReasonedVerdict, Verdict, VerdictReason,
};
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

pub mod hypothesis;

pub mod premise_flow;

pub mod quant_proposition;

pub mod quant_effect;

pub mod quant_theorem;

pub mod quant_condition;
