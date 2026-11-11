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

//! W3 obligation inventory and versioned verification task.
//!
//! Implements `docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` §1. These are serde
//! types only: the *derivation* that fills them is trusted core code, and a
//! provider never proposes the list of obligations whose absence would make
//! its own proof succeed (`docs/REEVALUATION_SPEC.md` §6).
//!
//! Canonicalization rule (design open question 6, accepted): every type in
//! this module uses `BTreeMap`, `BTreeSet` or `Vec`, and no floating-point
//! field, so `serde_json` produces a deterministic byte string for the task
//! digest. `tests/schema_canonicalization.rs` pins the rule.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::digest::sha256_hex;
use crate::plugin::{TargetProfile, UnitIdentity, VerifierCapability};
use crate::reason::ReasonCategory;

/// Version of the *task semantics*, not of the protocol frame. Bumped when
/// the meaning of a derived inventory changes (SV-LIB direction, design §1.1).
pub const VERIFICATION_TASK_VERSION: u16 = 1;

/// How much of the program's path space a claim covers. Bounds are part of
/// the claim, never metadata (design §1.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PathBounds {
    /// Every checked function contributes exactly one path. The v2.0 bound
    /// for chunk-lifecycle schedules (design §2.3 step 4).
    SinglePath,
    /// A checked function contributes exactly this many complete paths. The
    /// nullable-field destructor route is the first derivation to emit it; a
    /// task recording a different path count is a different task.
    BoundedPaths { paths: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskBounds {
    pub path_exploration: PathBounds,
    /// Concurrency schedule count; `None` means "not applicable", never "all".
    pub schedules_explored: Option<u64>,
    /// `true` marks a claim as checked for the recorded runs only. Such a
    /// claim is never presented as static source safety.
    pub observed_runs_only: bool,
    /// Trace-length bound for replay; `None` means the bound is structural.
    pub max_operations: Option<u64>,
}

impl TaskBounds {
    #[must_use]
    pub const fn single_path() -> Self {
        Self {
            path_exploration: PathBounds::SinglePath,
            schedules_explored: None,
            observed_runs_only: false,
            max_operations: None,
        }
    }

    #[must_use]
    pub const fn observed_runs() -> Self {
        Self {
            path_exploration: PathBounds::SinglePath,
            schedules_explored: None,
            observed_runs_only: true,
            max_operations: None,
        }
    }
}

/// The property an obligation demands. Variants are grouped by the capability
/// that derives them; the event/concurrency shapes are reserved now so the
/// Cycle 18 / W5 extraction needs no evidence-version bump (design §1.4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationProperty {
    // resource_contracts
    CustodyDischarged,
    ParameterModeRespected,
    NoUnknownCallEffect,
    /// Both Boolean prestates satisfy a source-derived borrowed-field contract.
    BooleanFieldContract,
    /// One source-derived constant-write frame helper writes only ordinary
    /// declared metadata fields and preserves the record's modeled Boolean
    /// fields, as its pinned contract requires. Live storage, exclusive access
    /// and cross-unit record ABI equality remain the contract's own declared
    /// premises; no custody, lifecycle or whole-unit claim is made.
    FrameHelperContract,
    /// One source-derived unsigned unit counter update satisfies its pinned
    /// contract. Caller storage, exclusivity and the unsigned/usize ABI
    /// binding remain the contract's own declared premises.
    CounterUpdateContract,
    /// One source-derived intrusive-list unlink, together with the complete
    /// body of the single helper it calls, satisfies its pinned contract.
    /// Ring membership and exclusive link access remain declared premises.
    ListUnlinkContract,
    /// One source-derived nullable-field destructor releases exactly its
    /// declared owned fields in every realizable nullable state. Allocation
    /// provenance and the `free`/libc binding remain declared premises.
    FieldDestructorContract,
    /// One source-derived filesystem cleanup-attempt body has the call-result
    /// def/use shape its pinned policy requires. Successful OS cleanup,
    /// descriptor and mapping custody remain outside this property entirely.
    CallResultUseContract,
    /// One source-derived native cleanup or guarded-delete body performs the
    /// return value, call count and record field transition its pinned contract
    /// requires, in every sampled prestate of its family, under the symbolic
    /// value lattice. Successful OS cleanup, descriptor, mapping and file
    /// custody remain outside this property entirely.
    CleanupReplayContract,
    /// One source-derived wrapper body composes separately checked
    /// prerequisite capabilities into the return value, call order, counter
    /// decrement and field writes its pinned contract requires, in every
    /// sampled entry state. Every prerequisite is re-established by the core;
    /// successful OS cleanup and mapping custody remain outside this property.
    CompositionContract,
    // chunk_lifecycle
    ContextDischarged,
    StreamDischarged,
    ChunkDischarged,
    TransactionSettled,
    LifecycleProgramComplete,
    // observation_hooks
    ObservationChecked,
    ObservationCoverageComplete,
    // translation_unit_projection
    ProjectionAccounted,
    // event_traces (reserved: no derivation, no witness kind)
    EventGenerationDischarged,
    CallbackQuiescentAtTeardown,
    // concurrency (reserved: no derivation, no witness kind)
    MutexGuardReleased,
    JoinBeforeExit,
}

impl ObligationProperty {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CustodyDischarged => "custody_discharged",
            Self::ParameterModeRespected => "parameter_mode_respected",
            Self::NoUnknownCallEffect => "no_unknown_call_effect",
            Self::BooleanFieldContract => "boolean_field_contract",
            Self::FrameHelperContract => "frame_helper_contract",
            Self::CounterUpdateContract => "counter_update_contract",
            Self::ListUnlinkContract => "list_unlink_contract",
            Self::FieldDestructorContract => "field_destructor_contract",
            Self::CallResultUseContract => "call_result_use_contract",
            Self::CleanupReplayContract => "cleanup_replay_contract",
            Self::CompositionContract => "composition_contract",
            Self::ContextDischarged => "context_discharged",
            Self::StreamDischarged => "stream_discharged",
            Self::ChunkDischarged => "chunk_discharged",
            Self::TransactionSettled => "transaction_settled",
            Self::LifecycleProgramComplete => "lifecycle_program_complete",
            Self::ObservationChecked => "observation_checked",
            Self::ObservationCoverageComplete => "observation_coverage_complete",
            Self::ProjectionAccounted => "projection_accounted",
            Self::EventGenerationDischarged => "event_generation_discharged",
            Self::CallbackQuiescentAtTeardown => "callback_quiescent_at_teardown",
            Self::MutexGuardReleased => "mutex_guard_released",
            Self::JoinBeforeExit => "join_before_exit",
        }
    }
}

impl fmt::Display for ObligationProperty {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A chunk-model entity, numbered by the core's own call-site enumeration. A
/// provider can never name an entity the core did not already number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "entity", rename_all = "snake_case")]
pub enum ChunkEntityRef {
    Context { id: u32 },
    Stream { id: u32 },
    Chunk { id: u32 },
}

impl fmt::Display for ChunkEntityRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Context { id } => write!(formatter, "context#{id}"),
            Self::Stream { id } => write!(formatter, "stream#{id}"),
            Self::Chunk { id } => write!(formatter, "chunk#{id}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "subject", rename_all = "snake_case")]
pub enum ObligationSubject {
    Function {
        symbol: String,
        translation_unit: String,
    },
    AllocationSite {
        function: String,
        site_index: u32,
        resource_kind: String,
    },
    ChunkEntity {
        entity: ChunkEntityRef,
    },
    ObservedContract {
        function: String,
        contract_module: String,
    },
    /// Reserved for the concurrency capability (design §1.4.4).
    MutexIdentity {
        name: String,
    },
    /// Reserved for the concurrency capability (design §1.4.4).
    TaskIdentity {
        name: String,
    },
    Unit,
}

impl ObligationSubject {
    /// The canonical rendering used inside the obligation identifier. Distinct
    /// subjects render distinctly; the rendering is stable across runs.
    #[must_use]
    pub fn canonical(&self) -> String {
        match self {
            Self::Function {
                symbol,
                translation_unit,
            } => format!("function|{translation_unit}|{symbol}"),
            Self::AllocationSite {
                function,
                site_index,
                resource_kind,
            } => format!("allocation_site|{function}|{site_index}|{resource_kind}"),
            Self::ChunkEntity { entity } => format!("chunk_entity|{entity}"),
            Self::ObservedContract {
                function,
                contract_module,
            } => format!("observed_contract|{contract_module}|{function}"),
            Self::MutexIdentity { name } => format!("mutex|{name}"),
            Self::TaskIdentity { name } => format!("task|{name}"),
            Self::Unit => "unit".to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum ObligationScope {
    WholeUnit,
    Functions {
        symbols: BTreeSet<String>,
    },
    /// Observation obligations are scoped to the run identities the core
    /// actually replayed; the set is filled in at check time.
    ObservedRuns {
        run_ids: BTreeSet<String>,
    },
}

/// One core-derived proof obligation. It carries the whole verdict-contract
/// tuple: `subject` + `scope` (artifact and scope), `property`, `bounds`,
/// `assumptions`, and `evidence_version`; the task supplies the semantic
/// profile.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    pub obligation_id: String,
    pub capability: VerifierCapability,
    pub property: ObligationProperty,
    pub subject: ObligationSubject,
    pub scope: ObligationScope,
    pub bounds: TaskBounds,
    pub assumptions: BTreeMap<String, bool>,
    pub evidence_version: u16,
}

/// A construct the deriver could not classify. Recorded on the task rather
/// than silently omitted: a task carrying one of these can never reach `Safe`
/// for the affected capability (design §1.4).
///
/// `category` is the closed L21.1 classification of `reason`. It refines the
/// row and replaces nothing: `capability` and `reason` keep their exact meaning
/// and bytes. The field is **required**, not optional with a default — a record
/// that omits it is refused at deserialization, which is what makes an
/// evidence-v2 witness recorded before this increment fail against a task
/// derived after it, and vice versa.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivationIncomplete {
    pub capability: VerifierCapability,
    /// The closed L21.1 category. Named explicitly at every construction site;
    /// never defaulted, and never inferred from `reason` in production.
    pub category: ReasonCategory,
    pub reason: String,
}

impl DerivationIncomplete {
    /// `true` when this row makes a claim about the analysed source. An
    /// environment failure is structurally excluded (§L21.1 rule 1).
    #[must_use]
    pub const fn describes_analysed_source(&self) -> bool {
        self.category.describes_analysed_source()
    }
}

/// The complete, versioned verification task (design §1.1). The digest of
/// this record travels in every v2 request and every witness.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationTask {
    pub task_version: u16,
    pub unit: UnitIdentity,
    pub target: TargetProfile,
    pub semantic_profile: String,
    /// Path -> SHA-256 of every trusted contract/profile input assumed. A
    /// missing input identity is `Unknown`, never an empty-string default.
    pub contract_sources: BTreeMap<String, String>,
    pub bounds: TaskBounds,
    pub assumptions: BTreeMap<String, bool>,
    pub evidence_version: u16,
    pub obligations: Vec<Obligation>,
    /// Non-empty means the derivation is incomplete for the listed
    /// capabilities; those capabilities stay `Unknown` regardless of any
    /// provider response.
    pub derivation_incomplete: Vec<DerivationIncomplete>,
}

impl VerificationTask {
    /// SHA-256 over the canonical JSON serialization (design §1.1).
    #[must_use]
    pub fn task_digest(&self) -> String {
        let canonical = serde_json::to_vec(self).expect("task types are serializable");
        sha256_hex(&canonical)
    }

    /// Obligation identifiers are computed by the core, never accepted from a
    /// provider: `capability:sha256(identity ++ capability ++ property ++
    /// canonical subject)` truncated to a fixed hex length.
    #[must_use]
    pub fn obligation_id(
        &self,
        capability: VerifierCapability,
        property: ObligationProperty,
        subject: &ObligationSubject,
    ) -> String {
        let identity = format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.task_version,
            self.unit.source_digest,
            self.unit.build_digest,
            self.unit.translation_unit,
            self.semantic_profile,
            capability.as_str(),
            property.as_str(),
            subject.canonical(),
        );
        format!(
            "{}:{}",
            capability.as_str(),
            &sha256_hex(identity.as_bytes())[..24]
        )
    }

    /// Identifiers of every obligation for one capability.
    #[must_use]
    pub fn coverage_for(&self, capability: VerifierCapability) -> BTreeSet<String> {
        self.obligations
            .iter()
            .filter(|obligation| obligation.capability == capability)
            .map(|obligation| obligation.obligation_id.clone())
            .collect()
    }

    #[must_use]
    pub fn obligations_for(&self, capability: VerifierCapability) -> Vec<&Obligation> {
        self.obligations
            .iter()
            .filter(|obligation| obligation.capability == capability)
            .collect()
    }

    /// `true` when the derivation could not classify some construct for this
    /// capability. Such a capability can never reach `Safe`.
    #[must_use]
    pub fn is_incomplete_for(&self, capability: VerifierCapability) -> bool {
        self.derivation_incomplete
            .iter()
            .any(|gap| gap.capability == capability)
    }

    /// Mirrors `VerificationRequest::validate`'s duplicate check: obligation
    /// identifiers are unique within a task, and every field that identifies
    /// the task is present.
    pub fn validate(&self) -> Result<(), TaskError> {
        if self.task_version != VERIFICATION_TASK_VERSION
            || self.evidence_version != crate::witness::PLUGIN_EVIDENCE_VERSION_V2
            || self.semantic_profile.is_empty()
            || self.unit.translation_unit.is_empty()
        {
            return Err(TaskError::InvalidIdentity);
        }
        if self
            .contract_sources
            .iter()
            .any(|(path, digest)| path.is_empty() || digest.len() != 64)
        {
            return Err(TaskError::InvalidContractSource);
        }
        let ids = self
            .obligations
            .iter()
            .map(|obligation| obligation.obligation_id.as_str())
            .collect::<BTreeSet<_>>();
        if ids.len() != self.obligations.len()
            || self
                .obligations
                .iter()
                .any(|obligation| obligation.obligation_id.is_empty())
        {
            return Err(TaskError::DuplicateObligation);
        }
        if self
            .obligations
            .iter()
            .any(|obligation| obligation.evidence_version != self.evidence_version)
        {
            return Err(TaskError::EvidenceVersionMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskError {
    InvalidIdentity,
    InvalidContractSource,
    DuplicateObligation,
    EvidenceVersionMismatch,
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity => formatter.write_str("invalid verification-task identity"),
            Self::InvalidContractSource => {
                formatter.write_str("a contract source identity is missing or not a SHA-256")
            }
            Self::DuplicateObligation => {
                formatter.write_str("obligation identifiers are duplicated or empty")
            }
            Self::EvidenceVersionMismatch => {
                formatter.write_str("an obligation demands a different evidence version")
            }
        }
    }
}

impl std::error::Error for TaskError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> VerificationTask {
        VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                source_digest: "a".repeat(64),
                build_digest: "b".repeat(64),
                translation_unit: "fixture.qc".into(),
            },
            target: TargetProfile {
                library: "chunkio".into(),
                revision: "audit".into(),
                profile: "default".into(),
                dialect: "qcore".into(),
                platform: "linux".into(),
                backend: "native".into(),
            },
            semantic_profile: "qcore.chunk_lifecycle.v1".into(),
            contract_sources: BTreeMap::new(),
            bounds: TaskBounds::single_path(),
            assumptions: BTreeMap::new(),
            evidence_version: 2,
            obligations: Vec::new(),
            derivation_incomplete: Vec::new(),
        }
    }

    #[test]
    fn digests_are_deterministic_and_input_sensitive() {
        let first = task();
        assert_eq!(first.task_digest(), task().task_digest());
        let mut changed = task();
        changed.semantic_profile = "qcore.chunk_lifecycle.v2".into();
        assert_ne!(first.task_digest(), changed.task_digest());
        let mut changed = task();
        changed.unit.source_digest = "c".repeat(64);
        assert_ne!(first.task_digest(), changed.task_digest());
        let mut changed = task();
        changed
            .contract_sources
            .insert("contract/qcore/x.qcore".into(), "d".repeat(64));
        assert_ne!(first.task_digest(), changed.task_digest());
        let mut changed = task();
        changed.bounds.observed_runs_only = true;
        assert_ne!(first.task_digest(), changed.task_digest());
    }

    #[test]
    fn obligation_identifiers_are_deterministic_and_distinct() {
        let task = task();
        let subject = ObligationSubject::ChunkEntity {
            entity: ChunkEntityRef::Chunk { id: 0 },
        };
        let first = task.obligation_id(
            VerifierCapability::ChunkLifecycle,
            ObligationProperty::ChunkDischarged,
            &subject,
        );
        assert_eq!(
            first,
            task.obligation_id(
                VerifierCapability::ChunkLifecycle,
                ObligationProperty::ChunkDischarged,
                &subject,
            )
        );
        assert!(first.starts_with("chunk_lifecycle:"));
        let other = ObligationSubject::ChunkEntity {
            entity: ChunkEntityRef::Chunk { id: 1 },
        };
        assert_ne!(
            first,
            task.obligation_id(
                VerifierCapability::ChunkLifecycle,
                ObligationProperty::ChunkDischarged,
                &other,
            )
        );
    }

    #[test]
    fn duplicate_obligation_identifiers_are_rejected() {
        let mut task = task();
        let obligation = Obligation {
            obligation_id: "chunk_lifecycle:deadbeef".into(),
            capability: VerifierCapability::ChunkLifecycle,
            property: ObligationProperty::LifecycleProgramComplete,
            subject: ObligationSubject::Unit,
            scope: ObligationScope::WholeUnit,
            bounds: TaskBounds::single_path(),
            assumptions: BTreeMap::new(),
            evidence_version: 2,
        };
        task.obligations = vec![obligation.clone(), obligation];
        assert_eq!(task.validate(), Err(TaskError::DuplicateObligation));
    }
}
