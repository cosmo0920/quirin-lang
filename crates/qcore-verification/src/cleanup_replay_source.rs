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

//! Function-scoped source-derived symbolic-replay evidence for the two bounded
//! ChunkIO cleanup-outcome families.
//!
//! Only the opaque typed-C cleanup-replay import can construct this replay
//! context. Task identity binds the actual input/preprocessing/build snapshots,
//! the pinned contract, the selected policy, the contract-declared record
//! profile and the identity of the call-result def/use **prerequisite** this
//! reading depends on. OS cleanup success, descriptor, mapping and file
//! custody, and any discharge of a resource remain outside every claim here: a
//! recorded `close`, `munmap` or `unlink` call is a recorded call.
//!
//! Three things are load-bearing, and each is work the trusted core does:
//!
//! 1. **The profile is contract-derived.** The core reads the pinned contract's
//!    own `cleanup_vocabulary_v1` declaration (card P04-B1), builds the
//!    [`CleanupProfile`] from it, and refuses unless the transported profile
//!    record is exactly that. A producer cannot choose the record shape it
//!    answers over.
//! 2. **The prerequisite is re-established, never read.** The projection
//!    carries a [`PrerequisiteIdentity`] triple and nothing else about the
//!    prerequisite. The core re-derives the prerequisite task from its own
//!    import, refuses unless the claimed triple *is* the identity it derives,
//!    and re-runs the prerequisite's own contract evaluation. A provider's copy
//!    of a prerequisite certificate is never trusted because nothing it sends
//!    is ever read as one.
//! 3. **Acceptance terminates in the core's own replay.** The core runs the
//!    unedited [`crate::cleanup_outcome`] checker over its own import and its
//!    own contract, and only then requires the proposal to be that same
//!    reading. Agreement alone discharges nothing.

use qcore_c_gateway::boolean_source::SourceInputIdentity;
use qcore_c_gateway::cleanup_replay_source::{CleanupReplayFamily, CleanupReplaySourceImport};
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::cleanup_replay_summary::{
    CLEANUP_REPLAY_SEMANTIC_PROFILE, CLEANUP_REPLAY_SOURCE_INPUT_KEY, CleanupEntryRecord,
    CleanupFieldRecord, CleanupLatticeValue, CleanupProfileRecord, CleanupReplayCaseRecord,
    CleanupReplayProposalRecord, CleanupReplayRecord, CleanupResetRecord,
};
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;
use std::collections::{BTreeMap, BTreeSet};

use crate::call_use_source::FileAttemptSourceTask;
use crate::cleanup_outcome::{
    CleanupCertificate, CleanupEntry, CleanupField, CleanupProfile, GuardedDeleteCertificate,
    ResetValue, check_cleanup_outcomes, check_guarded_delete_outcomes,
};
use crate::cleanup_vocabulary::declared_cleanup_vocabulary;

/// Digest keys this route binds beyond the shared input key.
pub const CLEANUP_REPLAY_IMPORT_KEY: &str = "qcore:cleanup-replay-import:v1";
pub const CLEANUP_REPLAY_CONTRACT_KEY: &str = "qcore:cleanup-replay-contract:v1";
pub const CLEANUP_REPLAY_POLICY_KEY: &str = "qcore:cleanup-replay-policy:v1";
pub const CLEANUP_REPLAY_PROFILE_KEY: &str = "qcore:cleanup-replay-profile:v1";

/// The certificate the core's own evaluation produces, per family. The two
/// arms are exactly the two certificates the unedited checker publishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CleanupReplayCertificate {
    NativeCleanupOutcome(Box<CleanupCertificate>),
    GuardedDelete(Box<GuardedDeleteCertificate>),
}

fn reset_record(reset: ResetValue) -> CleanupResetRecord {
    match reset {
        ResetValue::Null => CleanupResetRecord::Null,
        ResetValue::Integer(value) => CleanupResetRecord::Integer { value },
    }
}

fn reset_lattice(reset: ResetValue) -> CleanupLatticeValue {
    match reset {
        ResetValue::Null => CleanupLatticeValue::Null,
        ResetValue::Integer(value) => CleanupLatticeValue::Integer { value },
    }
}

/// The record shape one pinned contract policy declares, together with the
/// guarded-delete eligibility field it declares (or `None` for the two native
/// cleanup policies, which must declare none).
///
/// This is the only supported way to obtain the profile a projection may carry:
/// it is read out of the contract's own `cleanup_vocabulary_v1` clauses, so a
/// caller cannot invent a record shape, and [`CleanupReplaySourceTask::derive`]
/// re-reads it and refuses a transported profile that is not exactly this.
pub fn declared_profile(
    contract: &str,
    policy: &str,
) -> Result<(CleanupProfile, Option<String>), String> {
    let vocabulary = declared_cleanup_vocabulary(contract, policy)?;
    let shape = vocabulary.shape();
    Ok((
        CleanupProfile {
            function: shape.function.clone(),
            root_type: shape.root_type.clone(),
            syscall: shape.syscall.clone(),
            syscall_signature: shape.syscall_signature.clone(),
            presence_field: shape.presence_field.clone(),
            fields: shape
                .fields
                .iter()
                .map(|field| CleanupField {
                    name: field.name.clone(),
                    c_type: field.c_type.clone(),
                    reset: field.reset,
                })
                .collect(),
            argument_fields: shape.argument_fields.clone(),
            policy: policy.to_owned(),
            errno_function: shape.errno_function.clone(),
            report_function: shape.report_function.clone(),
        },
        shape.mapping_field.clone(),
    ))
}

/// The portable record of a `CleanupProfile`, less its policy name — which
/// declaration the core reads never travels.
#[must_use]
pub fn profile_record(profile: &CleanupProfile) -> CleanupProfileRecord {
    CleanupProfileRecord {
        function: profile.function.clone(),
        root_type: profile.root_type.clone(),
        syscall: profile.syscall.clone(),
        syscall_signature: profile.syscall_signature.clone(),
        presence_field: profile.presence_field.clone(),
        fields: profile
            .fields
            .iter()
            .map(|field| CleanupFieldRecord {
                name: field.name.clone(),
                c_type: field.c_type.clone(),
                reset: reset_record(field.reset),
            })
            .collect(),
        argument_fields: profile.argument_fields.clone(),
        errno_function: profile.errno_function.clone(),
        report_function: profile.report_function.clone(),
    }
}

/// The lattice state the replay begins one prestate from.
///
/// This mirrors the private `Replay::new` rule of [`crate::cleanup_outcome`]
/// and the guarded-delete case loop's two eligibility overrides. It is a
/// derivation of an *input*, not a decision, and it is an **additional**
/// agreement requirement: every published figure of a case still comes from
/// the unedited checker, so this function can only make the route refuse more,
/// never admit more. The duplication is recorded as boundary
/// `P04-B2-INITIAL-MIRROR`.
fn initial_lattice_state(
    profile: &CleanupProfile,
    entry: CleanupEntry,
    overrides: &[(&str, CleanupLatticeValue)],
) -> BTreeMap<String, CleanupLatticeValue> {
    let mut state: BTreeMap<String, CleanupLatticeValue> = profile
        .fields
        .iter()
        .map(|field| {
            let value = if field.name == profile.presence_field && entry == CleanupEntry::Absent {
                reset_lattice(field.reset)
            } else if field.c_type.ends_with('*') {
                CleanupLatticeValue::Pointer {
                    field: field.name.clone(),
                }
            } else {
                CleanupLatticeValue::Symbol {
                    field: field.name.clone(),
                }
            };
            (field.name.clone(), value)
        })
        .collect();
    for (name, value) in overrides {
        state.insert((*name).to_owned(), value.clone());
    }
    state
}

fn entry_record(entry: CleanupEntry) -> CleanupEntryRecord {
    match entry {
        CleanupEntry::Null => CleanupEntryRecord::Null,
        CleanupEntry::Absent => CleanupEntryRecord::Absent,
        CleanupEntry::Present => CleanupEntryRecord::Present,
    }
}

/// Identity and target checks. The declared revision is an outer-audit
/// assumption in every route; what is checked here is that the task, the
/// target and the importer's own captured identity agree.
fn check_target(
    target: &TargetProfile,
    import_revision: &str,
    identity: &SourceInputIdentity,
    translation_unit: &str,
    contract_source: &str,
) -> Result<(), String> {
    if translation_unit.is_empty()
        || contract_source.is_empty()
        || target.library.is_empty()
        || target.profile != CLEANUP_REPLAY_SEMANTIC_PROFILE
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible cleanup replay target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native cleanup replay platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the cleanup replay route imports no helper bodies".into());
    }
    Ok(())
}

/// Sealed replay context. A raw wire projection cannot construct one.
pub struct CleanupReplaySourceTask {
    import: CleanupReplaySourceImport,
    contract_source: String,
    policy: String,
    /// The profile the *core* built from its own contract declaration. The
    /// transported record is required to equal this, and this — never the
    /// transported one — is what the checker is handed.
    profile: CleanupProfile,
    mapping_field: Option<String>,
    prerequisite: FileAttemptSourceTask,
    task: VerificationTask,
}

impl CleanupReplaySourceTask {
    /// Build the task. `prerequisite` is the core's **own** call-result def/use
    /// context for the same body; the projection's claimed triple is required
    /// to be the identity that context derives.
    pub fn derive(
        import: &CleanupReplaySourceImport,
        prerequisite: FileAttemptSourceTask,
        translation_unit: &str,
        contract_source: &str,
        policy: &str,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_target(
            &target,
            import.source_revision(),
            import.input_identity(),
            translation_unit,
            contract_source,
        )?;
        if policy.is_empty() {
            return Err("cleanup replay policy is incomplete".into());
        }
        // 1. The profile is the contract's own declaration, not the wire's.
        let (profile, declared_mapping) = declared_profile(contract_source, policy)?;
        if *import.profile() != profile_record(&profile) {
            return Err(
                "transported cleanup profile is not the vocabulary the contract declares".into(),
            );
        }
        if profile.function != import.function() {
            return Err("declared cleanup profile names a different function".into());
        }
        // The eligibility field is not a profile member, so the family must
        // agree with the declaration in both directions: the delete policy
        // declares one and names it, the outcome policies declare none.
        let mapping_field = match (import.family(), declared_mapping.as_deref()) {
            (CleanupReplayFamily::NativeCleanupOutcome, None) => None,
            (CleanupReplayFamily::GuardedDelete { mapping_field }, Some(declared))
                if mapping_field == declared =>
            {
                Some(declared.to_owned())
            }
            _ => {
                return Err(
                    "cleanup replay family does not match the declared eligibility vocabulary"
                        .into(),
                );
            }
        };
        // 2. The prerequisite claim is a lookup key, checked against the
        //    identity the core's own prerequisite context derives.
        let claimed = &import.source_input().prerequisite;
        if *claimed != prerequisite.prerequisite_identity() {
            return Err(
                "cleanup replay prerequisite identity is not the one the core derives".into(),
            );
        }
        let paths = u64::try_from(import.summary().paths().len())
            .map_err(|_| "cleanup replay path count out of range")?;
        if paths == 0 {
            return Err("cleanup replay import has no complete path".into());
        }
        let input = import.source_input();
        let serialized_identity =
            serde_json::to_vec(import.input_identity()).map_err(|e| e.to_string())?;
        let serialized_profile =
            serde_json::to_vec(&profile_record(&profile)).map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (CLEANUP_REPLAY_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                CLEANUP_REPLAY_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_identity),
            ),
            (
                CLEANUP_REPLAY_CONTRACT_KEY.to_owned(),
                sha256_hex(contract_source.as_bytes()),
            ),
            // Which declaration the core reads is part of the checked question.
            (
                CLEANUP_REPLAY_POLICY_KEY.to_owned(),
                sha256_hex(policy.as_bytes()),
            ),
            // The record shape the replay evolves, as the core derived it.
            (
                CLEANUP_REPLAY_PROFILE_KEY.to_owned(),
                sha256_hex(&serialized_profile),
            ),
            // A dependent whose prerequisite moved is a different task.
            (
                PrerequisiteIdentity::DIGEST_KEY.to_owned(),
                claimed.digest(),
            ),
        ]);
        let assumptions = BTreeMap::from([
            ("record_is_live".into(), true),
            ("exclusive_access".into(), true),
            ("cleanup_callees_are_declared_external".into(), true),
            ("os_cleanup_success_is_not_inferred".into(), true),
            ("clang_typed_import_is_trusted".into(), true),
            ("source_projection_policy_is_declared_trusted".into(), true),
            (
                "declared_source_revision_is_outer_audit_assumption".into(),
                true,
            ),
        ]);
        let bounds = TaskBounds {
            path_exploration: PathBounds::BoundedPaths { paths },
            schedules_explored: None,
            observed_runs_only: false,
            max_operations: None,
        };
        let mut task = VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                translation_unit: translation_unit.into(),
                source_digest: import.input_identity().raw_source_digest().into(),
                build_digest: sha256_hex(&serialized_identity),
            },
            target,
            semantic_profile: CLEANUP_REPLAY_SEMANTIC_PROFILE.into(),
            contract_sources,
            bounds: bounds.clone(),
            assumptions: assumptions.clone(),
            evidence_version: 2,
            obligations: Vec::new(),
            derivation_incomplete: Vec::new(),
        };
        let subject = ObligationSubject::Function {
            symbol: import.function().into(),
            translation_unit: translation_unit.into(),
        };
        let property = ObligationProperty::CleanupReplayContract;
        let capability = VerifierCapability::ResourceContracts;
        task.obligations.push(Obligation {
            obligation_id: task.obligation_id(capability, property, &subject),
            capability,
            property,
            subject,
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([import.function().to_owned()]),
            },
            bounds,
            assumptions,
            evidence_version: 2,
        });
        task.validate().map_err(|e| e.to_string())?;
        Ok(Self {
            import: import.clone(),
            contract_source: contract_source.into(),
            policy: policy.to_owned(),
            profile,
            mapping_field,
            prerequisite,
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    /// The profile the core derived from its own contract declaration.
    pub fn profile(&self) -> &CleanupProfile {
        &self.profile
    }

    pub fn policy(&self) -> &str {
        &self.policy
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("cleanup-replay:{}", self.task.task_digest()),
            plugin_id,
            capability: VerifierCapability::ResourceContracts,
            target: self.task.target.clone(),
            unit: self.task.unit.clone(),
            facts: Vec::new(),
            assumptions: self.task.assumptions.clone(),
            task_digest: self.task.task_digest(),
            obligations: self.task.obligations.clone(),
            source_effect: None,
            counter_effect: None,
            reactor_program: None,
            teardown_source: None,
            call_use_source: None,
            cleanup_replay_source: Some(self.import.source_input()),
            composition_source: None,
        }
    }

    /// Re-establish the prerequisite: the core re-derives its identity and
    /// re-runs the prerequisite's own contract evaluation. Nothing a provider
    /// sends reaches this.
    pub fn revalidate_prerequisite(&self) -> Result<(), String> {
        self.prerequisite
            .revalidate_prerequisite(&self.import.source_input().prerequisite)
            .map(|_| ())
    }

    /// This task's own identity, for a *dependent* capability to name it by.
    /// It carries no verdict, certificate or case data — only the semantic
    /// profile, the task digest and the single obligation identifier, each of
    /// which the dependent's core re-derives for itself.
    pub fn prerequisite_identity(&self) -> PrerequisiteIdentity {
        PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// Re-run this task's own evaluation for a dependent and confirm the
    /// claimed identity is the one this context derives. The dependent calls
    /// this before it looks at any dependent proposal.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<CleanupReplayCertificate, String> {
        if !claimed.is_complete() {
            return Err("cleanup replay prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err(
                "cleanup replay prerequisite identity is not the one the core derives".into(),
            );
        }
        // A dependent inherits this route's own prerequisite obligation too:
        // re-establishing a prerequisite that has not itself re-established its
        // prerequisite would admit a chain by its weakest link.
        self.revalidate_prerequisite()?;
        self.evaluate()
    }

    /// The typed summary this task imported, so a dependent can require that
    /// the body it composes over is the one this prerequisite checked.
    pub fn summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.summary()
    }

    /// Re-check the imported body against the pinned contract with the
    /// unedited replay. This is the core's own evaluation; no provider input
    /// reaches it.
    pub fn evaluate(&self) -> Result<CleanupReplayCertificate, String> {
        match &self.mapping_field {
            None => {
                check_cleanup_outcomes(self.import.summary(), &self.profile, &self.contract_source)
                    .map(|c| CleanupReplayCertificate::NativeCleanupOutcome(Box::new(c)))
            }
            Some(mapping_field) => check_guarded_delete_outcomes(
                self.import.summary(),
                &self.profile,
                mapping_field,
                &self.contract_source,
            )
            .map(|c| CleanupReplayCertificate::GuardedDelete(Box::new(c))),
        }
    }

    /// The symbolic replay the core reads out of its own checked certificate.
    fn record(
        &self,
        certificate: &CleanupReplayCertificate,
    ) -> Result<CleanupReplayProposalRecord, String> {
        let index =
            |value: usize| u32::try_from(value).map_err(|_| "index out of range".to_owned());
        match certificate {
            CleanupReplayCertificate::NativeCleanupOutcome(certificate) => {
                let mut cases = Vec::new();
                for case in certificate.cases() {
                    cases.push(CleanupReplayCaseRecord::NativeCleanupOutcome {
                        entry: entry_record(case.entry),
                        syscall_result: case.syscall_result,
                        initial_fields: initial_lattice_state(&self.profile, case.entry, &[]),
                        source_path: index(case.source_path)?,
                        returned: case.returned,
                        cleanup_calls: index(case.cleanup_calls)?,
                        field_writes: case.field_writes.clone(),
                        fields_preserved: case.fields_preserved,
                        fields_reset: case.fields_reset,
                    });
                }
                Ok(CleanupReplayProposalRecord::NativeCleanupOutcome(
                    self.replay_record(certificate.source(), cases),
                ))
            }
            CleanupReplayCertificate::GuardedDelete(certificate) => {
                let mapping_field = self
                    .mapping_field
                    .as_deref()
                    .ok_or("guarded delete certificate without a declared mapping field")?;
                let mut cases = Vec::new();
                for case in certificate.cases() {
                    let mut overrides: Vec<(&str, CleanupLatticeValue)> = Vec::new();
                    if !case.descriptor_field_active {
                        overrides.push((
                            self.profile.presence_field.as_str(),
                            CleanupLatticeValue::Integer { value: -1 },
                        ));
                    }
                    if !case.mapping_field_active {
                        overrides.push((mapping_field, CleanupLatticeValue::Null));
                    }
                    cases.push(CleanupReplayCaseRecord::GuardedDelete {
                        descriptor_field_active: case.descriptor_field_active,
                        mapping_field_active: case.mapping_field_active,
                        syscall_result: case.syscall_result,
                        initial_fields: initial_lattice_state(
                            &self.profile,
                            CleanupEntry::Present,
                            &overrides,
                        ),
                        source_path: index(case.source_path)?,
                        returned: case.returned,
                        syscall_calls: index(case.syscall_calls)?,
                        diagnostic_calls: index(case.diagnostic_calls)?,
                        fields_preserved: case.fields_preserved,
                    });
                }
                Ok(CleanupReplayProposalRecord::GuardedDelete(
                    self.replay_record(certificate.source(), cases),
                ))
            }
        }
    }

    fn replay_record(
        &self,
        source: &qcore_c_gateway::typed_source::SourceSummary,
        cases: Vec<CleanupReplayCaseRecord>,
    ) -> CleanupReplayRecord {
        CleanupReplayRecord {
            function: source.function().into(),
            source_revision: source.source_revision().into(),
            signature: source.signature().into(),
            cases,
            profile_assumptions: source.profile_assumptions().clone(),
        }
    }
}

impl WitnessChecker for CleanupReplaySourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("cleanup replay task differs from its checked import context".into());
        }
        let SemanticWitness::CleanupReplay(proposed) = witness else {
            return Err("expected cleanup replay witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("cleanup replay witness is bound to a different task".into());
        }
        // The prerequisite is re-established by the core before the dependent
        // proposal is looked at at all.
        self.revalidate_prerequisite()?;
        // Acceptance terminates in the core's own unedited replay and its own
        // contract evaluation; the proposal is then required to be that same
        // reading.
        let certificate = self.evaluate()?;
        if proposed.summary != self.record(&certificate)? {
            return Err("cleanup replay witness differs from the checked source replay".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}
