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

//! Function-scoped source-derived **composition** evidence: a reading over
//! separately checked prerequisite capabilities.
//!
//! Only the opaque typed-C composition import can construct this context. Task
//! identity binds the actual input/preprocessing/build snapshots, the pinned
//! composition contract, both prerequisite contracts, the selected prerequisite
//! policy, the record profile the **core** derived from that policy's own
//! declaration, and the identity of each of the three prerequisites this
//! reading depends on. OS cleanup success and mapping custody remain outside
//! every claim here: a recorded `munmap` call is a recorded call.
//!
//! Four things are load-bearing, and each is work the trusted core does:
//!
//! 1. **No composed result is transported, in either direction.** The
//!    projection carries the wrapper's paths, the prerequisite body's paths and
//!    the prerequisite record profile — the *question* — plus three
//!    [`PrerequisiteIdentity`] triples, which are lookup keys. A producer must
//!    recompute each prerequisite's reading itself; it is never handed one, and
//!    a certificate has no wire representation on this route at all.
//! 2. **Every prerequisite is re-established, never read.** For each of the
//!    three, the core re-derives the prerequisite task from its own import,
//!    refuses unless the claimed triple *is* the identity it derives, and
//!    re-runs that prerequisite's own contract evaluation. The cleanup
//!    prerequisite in turn re-establishes *its* prerequisite, so a chain cannot
//!    be admitted by its weakest link.
//! 3. **The bodies are tied to the prerequisites.** The wrapper summary this
//!    route imported must be the summary the call-result def/use prerequisite
//!    checked, and the prerequisite body summary must be the one the cleanup
//!    prerequisite checked. Otherwise a projection could compose one body's
//!    paths with another body's re-established certificate.
//! 4. **Acceptance terminates in the core's own composition.** The core runs
//!    the unedited [`crate::unmap_composition`] procedure over its own imports,
//!    its own contracts and its own contract-derived profile, and only then
//!    requires the proposal to be that same reading. Agreement alone discharges
//!    nothing.

use qcore_c_gateway::boolean_source::SourceInputIdentity;
use qcore_c_gateway::composition_source::{
    CompositionFamily, CompositionSourceImport, FilesystemChunkCloseSourceImport,
    MemoryCloseSourceImport, UnsupportedCloseSourceImport, memory_close_callees,
};
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionFamilyInput, CompositionProposalRecord,
    CompositionRecord, CompositionSourceInput, CompositionWitness, DeletingCloseCaseRecord,
    DeletionOutcomeRecord, FILE_CLOSE_DELETE_SEMANTIC_PROFILE, FILE_CLOSE_SEMANTIC_PROFILE,
    FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE, FileCloseCaseRecord, FileCloseCompositionRecord,
    FileClosePrerequisites, FilesystemChunkCloseCaseRecord, FilesystemChunkCloseCompositionRecord,
    FilesystemChunkClosePrerequisites, MEMORY_CLOSE_SEMANTIC_PROFILE, MemoryCloseCaseRecord,
    MemoryCloseCompositionRecord, MemoryClosePrerequisites, ResidualCustodyRecord,
    UNMAP_WRAPPER_SEMANTIC_PROFILE, UNSUPPORTED_CLOSE_SEMANTIC_PROFILE, UnmapCaseRecord,
    UnmapWrapperPrerequisites, UnsupportedCloseCompositionRecord, UnsupportedClosePrerequisites,
};
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;
use std::collections::{BTreeMap, BTreeSet};

use crate::call_use_source::FileAttemptSourceTask;
use crate::cleanup_outcome::{CleanupCertificate, CleanupProfile};
use crate::cleanup_replay_source::{
    CleanupReplayCertificate, CleanupReplaySourceTask, declared_profile, profile_record,
};
use crate::counter_source::CounterSourceTask;
use crate::file_close_composition::{
    DeletionOutcome, FileCloseContracts, FileCloseDeleteContracts, FileCloseDeletionEvidence,
    FileCloseEvidence, ResidualCustody, check_file_close_composition, check_file_close_deletion,
};
use crate::filesystem_chunk_close_composition::{
    FilesystemChunkCloseContracts, FilesystemChunkCloseEvidence,
    OwnedFilesystemChunkCloseContracts, check_filesystem_chunk_close_bodies,
};
use crate::frame_source::FrameHelperSourceTask;
use crate::memory_close_composition::{
    MemoryCloseCertificate, MemoryCloseContracts, MemoryCloseDependencies, MemoryCloseReading,
    OwnedMemoryCloseContracts, RESET_SIGNATURE, read_memory_close_bodies,
};
use crate::teardown_source::{FieldDestructorSourceTask, ListUnlinkSourceTask};
use crate::unmap_composition::{
    UnmapCertificate, UnmapCompositionContracts, check_unmap_composition,
};
use crate::unsupported_close_composition::{
    OwnedUnsupportedCloseContracts, UnsupportedCloseContracts, UnsupportedCloseEvidence,
    check_unsupported_close_bodies,
};

/// Digest keys this route binds beyond the shared input key.
pub const COMPOSITION_IMPORT_KEY: &str = "qcore:composition-import:v1";
pub const COMPOSITION_CONTRACT_KEY: &str = "qcore:composition-contract:v1";
pub const COMPOSITION_NATIVE_CONTRACT_KEY: &str = "qcore:composition-native-contract:v1";
pub const COMPOSITION_NATIVE_POLICY_KEY: &str = "qcore:composition-native-policy:v1";
pub const COMPOSITION_NATIVE_PROFILE_KEY: &str = "qcore:composition-native-profile:v1";
pub const COMPOSITION_COUNTER_CONTRACT_KEY: &str = "qcore:composition-counter-contract:v1";

/// The pinned contract texts and declared policy this composition is taken
/// under. Every one of them is the core's own selection; none travels.
pub struct CompositionContracts<'a> {
    /// The composition's own pinned contract.
    pub contract_source: &'a str,
    /// The prerequisite cleanup family's pinned contract, and the policy that
    /// selects the declaration the record profile is read out of.
    pub native_contract_source: &'a str,
    pub native_policy: &'a str,
    /// The counter prerequisite's pinned contract and declared root types.
    pub counter_contract_source: &'a str,
    pub counter_root_c_type: &'a str,
    pub counter_root_qcore_type: &'a str,
}

/// The three prerequisite contexts, each the core's **own**.
pub struct CompositionPrerequisites {
    /// The call-result def/use context for the wrapper body itself.
    pub wrapper_call_use: FileAttemptSourceTask,
    /// The symbolic cleanup replay context for the body the wrapper calls.
    pub native_cleanup: CleanupReplaySourceTask,
    /// The unsigned counter context for the decrement the wrapper performs.
    pub counter: CounterSourceTask,
}

/// Merge premise sets the way `compose_verification` merges a composite's
/// (card E03-B): the published set is the **union** of every component's, never
/// one component's, and a key two components disagree on **refuses** rather than
/// being silently resolved.
///
/// A composition that published only its own premises would be claiming its
/// prerequisites' obligations under fewer assumptions than those prerequisites
/// were checked under, which is exactly the failure E03-B repaired at the host
/// level. This route is a single-provider capability, so the merge happens here
/// rather than in `compose_verification`; the semantics are the same and
/// `composition_premise_union` is public so the disagreement direction can be
/// driven directly.
pub fn composition_premise_union(
    component_sets: &[&BTreeMap<String, bool>],
) -> Result<BTreeMap<String, bool>, String> {
    let mut merged: BTreeMap<String, bool> = BTreeMap::new();
    for set in component_sets {
        for (name, value) in *set {
            match merged.get(name) {
                Some(existing) if existing != value => {
                    return Err(format!(
                        "components disagree on the premise `{name}`; a composition                          publishes the union of its components' premises and refuses                          a conflicting value"
                    ));
                }
                _ => {
                    merged.insert(name.clone(), *value);
                }
            }
        }
    }
    Ok(merged)
}

impl CompositionPrerequisites {
    /// The three components' own premise sets, in the order the core
    /// re-establishes them.
    fn premise_sets(&self) -> [&BTreeMap<String, bool>; 3] {
        [
            &self.wrapper_call_use.task().assumptions,
            &self.native_cleanup.task().assumptions,
            &self.counter.task().assumptions,
        ]
    }

    fn identities(&self) -> UnmapWrapperPrerequisites {
        UnmapWrapperPrerequisites {
            wrapper_call_use: self.wrapper_call_use.prerequisite_identity(),
            native_cleanup: self.native_cleanup.prerequisite_identity(),
            counter: self.counter.prerequisite_identity(),
        }
    }
}

/// The leaf composition's claimed triple, read out of the tagged set. A
/// projection that names another family's set has no triple here and is refused
/// before anything else is looked at.
fn claimed_unmap(input: &CompositionSourceInput) -> Result<&UnmapWrapperPrerequisites, String> {
    input
        .prerequisites
        .unmap_wrapper()
        .ok_or_else(|| "composition projection names another family's prerequisite set".to_owned())
}

/// Identity and target checks, in the shape every source route uses.
fn check_target(
    target: &TargetProfile,
    import_revision: &str,
    identity: &SourceInputIdentity,
    translation_unit: &str,
    contracts: &CompositionContracts<'_>,
) -> Result<(), String> {
    if translation_unit.is_empty()
        || contracts.contract_source.is_empty()
        || contracts.native_contract_source.is_empty()
        || contracts.native_policy.is_empty()
        || contracts.counter_contract_source.is_empty()
        || contracts.counter_root_c_type.is_empty()
        || contracts.counter_root_qcore_type.is_empty()
        || target.library.is_empty()
        || target.profile != UNMAP_WRAPPER_SEMANTIC_PROFILE
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible composition target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native composition platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the composition route imports no helper bodies".into());
    }
    Ok(())
}

/// Sealed composition context. A raw wire projection cannot construct one.
pub struct CompositionSourceTask {
    import: CompositionSourceImport,
    contract_source: String,
    native_contract_source: String,
    native_policy: String,
    counter_contract_source: String,
    counter_root_c_type: String,
    counter_root_qcore_type: String,
    /// The profile the *core* built from its own contract declaration. The
    /// transported record is required to equal this, and this — never the
    /// transported one — is what the composition is handed.
    native_profile: CleanupProfile,
    prerequisites: CompositionPrerequisites,
    task: VerificationTask,
}

impl CompositionSourceTask {
    /// Build the task. `prerequisites` are the core's **own** contexts; the
    /// projection's claimed triples are required to be the identities they
    /// derive.
    pub fn derive(
        import: &CompositionSourceImport,
        prerequisites: CompositionPrerequisites,
        translation_unit: &str,
        contracts: &CompositionContracts<'_>,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_target(
            &target,
            import.source_revision(),
            import.input_identity(),
            translation_unit,
            contracts,
        )?;
        // 1. The prerequisite record profile is the contract's own declaration,
        //    not the wire's.
        let (native_profile, declared_mapping) =
            declared_profile(contracts.native_contract_source, contracts.native_policy)?;
        if declared_mapping.is_some() {
            return Err(
                "the composition's prerequisite policy declares a guarded-delete vocabulary".into(),
            );
        }
        if *import.native_profile() != profile_record(&native_profile) {
            return Err(
                "transported prerequisite profile is not the vocabulary the contract declares"
                    .into(),
            );
        }
        if native_profile.function != import.native_summary().function() {
            return Err("declared prerequisite profile names a different function".into());
        }
        // 2. Each claimed triple is a lookup key, checked against the identity
        //    the core's own prerequisite context derives.
        let input = import.source_input();
        if !input.version_matches() {
            return Err("unsupported composition projection version".into());
        }
        if !input.family.matches(&input.prerequisites) {
            return Err("composition family and prerequisite set disagree".into());
        }
        let claimed = claimed_unmap(&input)?.clone();
        if !claimed.is_complete() || !claimed.are_distinct() {
            return Err("composition prerequisite identities are incomplete or repeated".into());
        }
        if claimed != prerequisites.identities() {
            return Err(
                "composition prerequisite identities are not the ones the core derives".into(),
            );
        }
        // 3. The bodies this route imported must be the bodies those
        //    prerequisites checked, or a projection could compose one body's
        //    paths with another body's re-established certificate.
        if prerequisites.wrapper_call_use.summary() != import.wrapper_summary() {
            return Err(
                "the composed wrapper body is not the one its def/use prerequisite checked".into(),
            );
        }
        if prerequisites.native_cleanup.summary() != import.native_summary() {
            return Err(
                "the composed prerequisite body is not the one its cleanup prerequisite checked"
                    .into(),
            );
        }
        if !matches!(import.family(), CompositionFamily::UnmapWrapper) {
            return Err("this context checks the leaf unmap-wrapper composition only".into());
        }
        let paths = u64::try_from(import.wrapper_summary().paths().len())
            .map_err(|_| "composition path count out of range")?;
        if paths == 0 {
            return Err("composition import has no complete wrapper path".into());
        }
        let serialized_identity =
            serde_json::to_vec(import.input_identity()).map_err(|e| e.to_string())?;
        let serialized_profile =
            serde_json::to_vec(&profile_record(&native_profile)).map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (COMPOSITION_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                COMPOSITION_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_identity),
            ),
            (
                COMPOSITION_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.contract_source.as_bytes()),
            ),
            // A composition binds every contract it composes under, not only
            // its own: a moved prerequisite contract is a different question.
            (
                COMPOSITION_NATIVE_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.native_contract_source.as_bytes()),
            ),
            (
                COMPOSITION_NATIVE_POLICY_KEY.to_owned(),
                sha256_hex(contracts.native_policy.as_bytes()),
            ),
            (
                COMPOSITION_NATIVE_PROFILE_KEY.to_owned(),
                sha256_hex(&serialized_profile),
            ),
            (
                COMPOSITION_COUNTER_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.counter_contract_source.as_bytes()),
            ),
            // Three distinct keys, so a dependent whose *any* prerequisite
            // moved is a different task and one key cannot absorb another.
            (
                UnmapWrapperPrerequisites::WRAPPER_CALL_USE_KEY.to_owned(),
                claimed.wrapper_call_use.digest(),
            ),
            (
                UnmapWrapperPrerequisites::NATIVE_CLEANUP_KEY.to_owned(),
                claimed.native_cleanup.digest(),
            ),
            (
                UnmapWrapperPrerequisites::COUNTER_KEY.to_owned(),
                claimed.counter.digest(),
            ),
        ]);
        // The composition's own premises, over and above what its components
        // already assume: the wrapper's external sync and logging frames.
        let own = BTreeMap::from([
            ("sync_and_logging_frames_are_declared_premises".into(), true),
            ("os_cleanup_success_is_not_inferred".into(), true),
        ]);
        // E03-B semantics: the published set is the union of every component's
        // premises and this route's own, never one component's.
        let mut sets = prerequisites.premise_sets().to_vec();
        sets.push(&own);
        let assumptions = composition_premise_union(&sets)?;
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
            semantic_profile: UNMAP_WRAPPER_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::CompositionContract;
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
            contract_source: contracts.contract_source.into(),
            native_contract_source: contracts.native_contract_source.into(),
            native_policy: contracts.native_policy.into(),
            counter_contract_source: contracts.counter_contract_source.into(),
            counter_root_c_type: contracts.counter_root_c_type.into(),
            counter_root_qcore_type: contracts.counter_root_qcore_type.into(),
            native_profile,
            prerequisites,
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    /// The prerequisite record profile the core derived from its own contract.
    pub fn native_profile(&self) -> &CleanupProfile {
        &self.native_profile
    }

    pub fn native_policy(&self) -> &str {
        &self.native_policy
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("composition:{}", self.task.task_digest()),
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
            cleanup_replay_source: None,
            composition_source: Some(self.import.source_input()),
        }
    }

    /// This task's own identity, for a *dependent* capability to name it by. A
    /// composition is itself a prerequisite on the `file_close` route, so it
    /// carries the same three-field lookup key every other capability does — no
    /// verdict, no certificate, no case vector.
    pub fn prerequisite_identity(&self) -> PrerequisiteIdentity {
        PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// Re-run this whole composition for a dependent and confirm the claimed
    /// identity is the one this context derives.
    ///
    /// This is the **inner** level of the two-level re-establishment: the
    /// dependent's core calls it, and it re-establishes all three of *this*
    /// composition's prerequisites (one of which re-establishes its own) before
    /// re-running the unedited composition. No level is taken on trust and no
    /// level is skipped.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<UnmapCertificate, String> {
        if !claimed.is_complete() {
            return Err("composition prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err("composition prerequisite identity is not the one the core derives".into());
        }
        self.evaluate()
    }

    /// The wrapper body this task imported, so a dependent can require that the
    /// nested question it transports is the one this prerequisite checked.
    pub fn wrapper_summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.wrapper_summary()
    }

    /// The transported family input this task imported, so a dependent can
    /// require its nested question to be exactly this one.
    pub fn family_input(&self) -> qcore_schemas::composition_summary::CompositionFamilyInput {
        self.import.source_input().family
    }

    /// Re-establish all three prerequisites and return the two certificates the
    /// composition consumes. Each one is re-derived and re-evaluated by the
    /// core; nothing a provider sends reaches this.
    pub fn revalidate_prerequisites(
        &self,
    ) -> Result<
        (
            CleanupCertificate,
            crate::counter_contract::CounterCertificate,
        ),
        String,
    > {
        let input = self.import.source_input();
        let claimed = claimed_unmap(&input)?.clone();
        // The wrapper's own def/use contract must still hold: on the retained
        // in-process route the composition is unobtainable without it.
        self.prerequisites
            .wrapper_call_use
            .revalidate_prerequisite(&claimed.wrapper_call_use)?;
        let native = match self
            .prerequisites
            .native_cleanup
            .revalidate_as_prerequisite(&claimed.native_cleanup)?
        {
            CleanupReplayCertificate::NativeCleanupOutcome(certificate) => *certificate,
            CleanupReplayCertificate::GuardedDelete(_) => {
                return Err(
                    "the composition's cleanup prerequisite answered the wrong family".into(),
                );
            }
        };
        let counter = self
            .prerequisites
            .counter
            .revalidate_as_prerequisite(&claimed.counter)?;
        Ok((native, counter))
    }

    /// The pinned inputs this composition is taken under, all of them the core's
    /// own. A dependent that re-runs this composition inside its own reading
    /// takes them from here rather than restating them, so the two levels cannot
    /// be composed under different contracts.
    pub fn composition_contracts(&self) -> UnmapCompositionContracts<'_> {
        UnmapCompositionContracts {
            pinned_revision: self.import.source_revision(),
            native_function: self.native_profile.function.as_str(),
            native_profile: &self.native_profile,
            native_contract: &self.native_contract_source,
            counter_contract: &self.counter_contract_source,
            counter_root_c_type: &self.counter_root_c_type,
            counter_root_qcore_type: &self.counter_root_qcore_type,
            contract: &self.contract_source,
        }
    }

    /// Re-establish every prerequisite and re-run the unedited composition over
    /// the core's own imports, contracts and contract-derived profile. This is
    /// the core's own evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<UnmapCertificate, String> {
        let (native, counter) = self.revalidate_prerequisites()?;
        check_unmap_composition(
            self.import.wrapper_summary(),
            &native,
            &counter,
            &self.composition_contracts(),
        )
    }

    /// The composed reading the core reads out of its own checked certificate.
    fn record(&self, certificate: &UnmapCertificate) -> Result<CompositionProposalRecord, String> {
        let index =
            |value: usize| u32::try_from(value).map_err(|_| "index out of range".to_owned());
        let mut cases = Vec::new();
        for case in certificate.cases() {
            cases.push(UnmapCaseRecord {
                source_path: index(case.source_path)?,
                backend_present: case.backend_present,
                initially_mapped: case.initially_mapped,
                initially_synced: case.initially_synced,
                sync_result: case.sync_result,
                native_result: case.native_result,
                native_source_path: case.native_source_path.map(index).transpose()?,
                returned: case.returned,
                counter_decrements: index(case.counter_decrements)?,
                wrapper_writes: case.wrapper_writes.clone(),
            });
        }
        let source = self.import.wrapper_summary();
        Ok(CompositionProposalRecord::UnmapWrapper(CompositionRecord {
            function: source.function().into(),
            source_revision: source.source_revision().into(),
            signature: source.signature().into(),
            cases,
            profile_assumptions: source.profile_assumptions().clone(),
        }))
    }
}

impl WitnessChecker for CompositionSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("composition task differs from its checked import context".into());
        }
        let SemanticWitness::Composition(proposed) = witness else {
            return Err("expected composition witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("composition witness is bound to a different task".into());
        }
        // Every prerequisite is re-established and the composition re-run by the
        // core before the proposal is looked at at all.
        let certificate = self.evaluate()?;
        if proposed.summary != self.record(&certificate)? {
            return Err("composition witness differs from the checked source composition".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}

// ---------------------------------------------------------------------------
// The `cio_file_close` compositions: the first route whose own prerequisite is
// another composition.
// ---------------------------------------------------------------------------

/// Digest keys the `file_close` route binds beyond the shared input key.
pub const FILE_CLOSE_CONTRACT_KEY: &str = "qcore:composition-file-close-contract:v1";
pub const FILE_CLOSE_DELETE_CONTRACT_KEY: &str = "qcore:composition-file-close-delete-contract:v1";
pub const FILE_CLOSE_CLOSE_CONTRACT_KEY: &str = "qcore:composition-close-contract:v1";
pub const FILE_CLOSE_CLOSE_POLICY_KEY: &str = "qcore:composition-close-policy:v1";
pub const FILE_CLOSE_CLOSE_PROFILE_KEY: &str = "qcore:composition-close-profile:v1";
pub const FILE_CLOSE_DELETE_PREREQ_CONTRACT_KEY: &str =
    "qcore:composition-delete-prerequisite-contract:v1";
pub const FILE_CLOSE_DELETE_PREREQ_POLICY_KEY: &str =
    "qcore:composition-delete-prerequisite-policy:v1";
pub const FILE_CLOSE_DELETE_PREREQ_PROFILE_KEY: &str =
    "qcore:composition-delete-prerequisite-profile:v1";
/// The nested composition's own input digest, bound separately from the
/// prerequisite identity so a moved nested *question* and a moved nested *task*
/// are both visible to this task's identity.
pub const FILE_CLOSE_NESTED_INPUT_KEY: &str = "qcore:composition-nested-question:v1";

/// The pinned contract texts and declared policies a `file_close` composition is
/// taken under. Every one of them is the core's own selection; none travels.
pub struct FileCloseCompositionContracts<'a> {
    /// The non-deleting composition's own pinned contract. Always present: the
    /// deleting arm is computed on top of the non-deleting reading, exactly as
    /// the retained in-process entry point does.
    pub no_delete_contract_source: &'a str,
    /// The deleting arm's own pinned contract, present **exactly** on the
    /// deleting task.
    pub delete_contract_source: Option<&'a str>,
    /// The native close prerequisite's pinned contract and the policy that
    /// selects the declaration its record profile is read out of.
    pub close_contract_source: &'a str,
    pub close_policy: &'a str,
    /// The guarded-delete prerequisite's pinned contract and policy, present
    /// exactly on the deleting task.
    pub delete_prerequisite_contract_source: Option<&'a str>,
    pub delete_prerequisite_policy: Option<&'a str>,
}

/// The `file_close` route's prerequisite contexts, each the core's **own**.
/// `unmap_composition` is itself a composition, which is what makes this the
/// first two-level route.
pub struct FileClosePrerequisiteTasks {
    pub wrapper_call_use: FileAttemptSourceTask,
    pub unmap_composition: CompositionSourceTask,
    pub native_cleanup: CleanupReplaySourceTask,
    /// Present exactly on the deleting task.
    pub native_delete: Option<CleanupReplaySourceTask>,
}

impl FileClosePrerequisiteTasks {
    fn premise_sets(&self) -> Vec<&BTreeMap<String, bool>> {
        let mut sets = vec![
            &self.wrapper_call_use.task().assumptions,
            &self.unmap_composition.task().assumptions,
            &self.native_cleanup.task().assumptions,
        ];
        if let Some(delete) = &self.native_delete {
            sets.push(&delete.task().assumptions);
        }
        sets
    }

    fn identities(&self) -> FileClosePrerequisites {
        FileClosePrerequisites {
            wrapper_call_use: self.wrapper_call_use.prerequisite_identity(),
            unmap_composition: self.unmap_composition.prerequisite_identity(),
            native_cleanup: self.native_cleanup.prerequisite_identity(),
            native_delete: self
                .native_delete
                .as_ref()
                .map(CleanupReplaySourceTask::prerequisite_identity),
        }
    }
}

/// What a `file_close` composition decides: the non-deleting reading, or the
/// deleting reading which embeds it.
pub enum FileCloseCertificate {
    NoDelete(Box<FileCloseEvidence>),
    Deleting(Box<FileCloseDeletionEvidence>),
}

impl FileCloseCertificate {
    #[must_use]
    pub fn no_delete(&self) -> &FileCloseEvidence {
        match self {
            Self::NoDelete(evidence) => evidence,
            Self::Deleting(evidence) => evidence.no_delete(),
        }
    }
}

/// Sealed `file_close` composition context. A raw wire projection cannot
/// construct one.
pub struct FileCloseCompositionSourceTask {
    import: CompositionSourceImport,
    no_delete_contract_source: String,
    delete_contract_source: Option<String>,
    close_contract_source: String,
    close_policy: String,
    /// The profile the *core* built from the close contract's own declaration.
    close_profile: CleanupProfile,
    delete_prerequisite_contract_source: Option<String>,
    delete_prerequisite_policy: Option<String>,
    /// The profile and eligibility field the *core* built from the delete
    /// contract's own declaration.
    delete_profile: Option<CleanupProfile>,
    delete_mapping_field: Option<String>,
    prerequisites: FileClosePrerequisiteTasks,
    task: VerificationTask,
}

impl FileCloseCompositionSourceTask {
    /// Build the task. `prerequisites` are the core's **own** contexts,
    /// including its own nested composition context; the projection's claimed
    /// identities are required to be the ones they derive, and the transported
    /// nested question is required to be the nested context's own.
    pub fn derive(
        import: &CompositionSourceImport,
        prerequisites: FileClosePrerequisiteTasks,
        translation_unit: &str,
        contracts: &FileCloseCompositionContracts<'_>,
        target: TargetProfile,
    ) -> Result<Self, String> {
        let deleting = match import.family() {
            CompositionFamily::FileClose => false,
            CompositionFamily::FileCloseDeleting => true,
            CompositionFamily::UnmapWrapper
            | CompositionFamily::MemoryClose
            | CompositionFamily::FilesystemChunkClose
            | CompositionFamily::UnsupportedClose => {
                return Err("this context checks the file_close compositions only".into());
            }
        };
        let expected_profile = if deleting {
            FILE_CLOSE_DELETE_SEMANTIC_PROFILE
        } else {
            FILE_CLOSE_SEMANTIC_PROFILE
        };
        check_file_close_target(
            &target,
            expected_profile,
            import.source_revision(),
            import.input_identity(),
            translation_unit,
            contracts,
            deleting,
        )?;
        let input = import.source_input();
        if !input.version_matches() {
            return Err("unsupported composition projection version".into());
        }
        if input.family.semantic_profile() != expected_profile {
            return Err("the transported question answers another semantic profile".into());
        }
        if !input.family.matches(&input.prerequisites) {
            return Err("composition family and prerequisite set disagree".into());
        }
        // 1. Both prerequisite record profiles are the contracts' own
        //    declarations, never the wire's.
        let (close_profile, close_mapping) =
            declared_profile(contracts.close_contract_source, contracts.close_policy)?;
        if close_mapping.is_some() {
            return Err(
                "the close prerequisite policy declares a guarded-delete vocabulary".into(),
            );
        }
        if input.family.native_profile() != Some(&profile_record(&close_profile)) {
            return Err(
                "transported close profile is not the vocabulary the contract declares".into(),
            );
        }
        if close_profile.function != import.native_summary().function() {
            return Err("declared close profile names a different function".into());
        }
        let (delete_profile, delete_mapping_field) = match (
            deleting,
            contracts.delete_prerequisite_contract_source,
            contracts.delete_prerequisite_policy,
        ) {
            (false, None, None) => (None, None),
            (true, Some(contract), Some(policy)) => {
                let (profile, declared) = declared_profile(contract, policy)?;
                let declared = declared.ok_or(
                    "the delete prerequisite policy declares no guarded-delete vocabulary",
                )?;
                let CompositionFamilyInput::FileClose {
                    delete: Some(transported),
                    ..
                } = &input.family
                else {
                    return Err("the deleting question carries no delete body".into());
                };
                if transported.profile != profile_record(&profile) {
                    return Err(
                        "transported delete profile is not the vocabulary the contract declares"
                            .into(),
                    );
                }
                if transported.mapping_field != declared {
                    return Err(
                        "transported eligibility field is not the one the contract declares".into(),
                    );
                }
                if Some(profile.function.as_str()) != import.delete_summary().map(|s| s.function())
                {
                    return Err("declared delete profile names a different function".into());
                }
                (Some(profile), Some(declared))
            }
            _ => {
                return Err("the deleting arm and its delete contract selection must agree".into());
            }
        };
        // 2. Each claimed identity is a lookup key, checked against the identity
        //    the core's own prerequisite context derives.
        let claimed = input
            .prerequisites
            .file_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        if !claimed.is_complete() || !claimed.are_distinct() {
            return Err("file_close prerequisite identities are incomplete or repeated".into());
        }
        if claimed.native_delete.is_some() != deleting
            || prerequisites.native_delete.is_some() != deleting
        {
            return Err("the deleting arm and its delete prerequisite must agree".into());
        }
        if claimed != prerequisites.identities() {
            return Err(
                "file_close prerequisite identities are not the ones the core derives".into(),
            );
        }
        // 3. Every transported body is tied to the prerequisite that checked it,
        //    including the nested composition's own two bodies, or a projection
        //    could compose one body's paths with another body's re-established
        //    certificate.
        if prerequisites.wrapper_call_use.summary() != import.wrapper_summary() {
            return Err(
                "the composed wrapper body is not the one its def/use prerequisite checked".into(),
            );
        }
        if prerequisites.native_cleanup.summary() != import.native_summary() {
            return Err(
                "the composed close body is not the one its cleanup prerequisite checked".into(),
            );
        }
        match (&prerequisites.native_delete, import.delete_summary()) {
            (None, None) => (),
            (Some(task), Some(summary)) if task.summary() == summary => (),
            _ => {
                return Err(
                    "the composed delete body is not the one its delete prerequisite checked"
                        .into(),
                );
            }
        }
        if prerequisites.unmap_composition.wrapper_summary()
            != import
                .nested_wrapper_summary()
                .ok_or("the file_close question carries no nested composition")?
        {
            return Err(
                "the nested composed body is not the one the nested composition checked".into(),
            );
        }
        // The strongest tie of the two-level route: the transported nested
        // *question* must be, verbatim, the question the core's own nested
        // composition task imported. A nested question that is merely
        // well-formed, or that belongs to another unit, is refused here.
        let [nested] = input.family.nested()[..] else {
            return Err("the file_close question carries no single nested composition".into());
        };
        if *nested != prerequisites.unmap_composition.family_input() {
            return Err(
                "the transported nested question is not the one the nested composition imported"
                    .into(),
            );
        }
        let paths = u64::try_from(import.wrapper_summary().paths().len())
            .map_err(|_| "composition path count out of range")?;
        if paths == 0 {
            return Err("composition import has no complete wrapper path".into());
        }
        let serialized_identity =
            serde_json::to_vec(import.input_identity()).map_err(|e| e.to_string())?;
        let serialized_close_profile =
            serde_json::to_vec(&profile_record(&close_profile)).map_err(|e| e.to_string())?;
        let serialized_nested = serde_json::to_vec(nested).map_err(|e| e.to_string())?;
        let mut contract_sources = BTreeMap::from([
            (COMPOSITION_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                COMPOSITION_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_identity),
            ),
            (
                FILE_CLOSE_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.no_delete_contract_source.as_bytes()),
            ),
            (
                FILE_CLOSE_CLOSE_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.close_contract_source.as_bytes()),
            ),
            (
                FILE_CLOSE_CLOSE_POLICY_KEY.to_owned(),
                sha256_hex(contracts.close_policy.as_bytes()),
            ),
            (
                FILE_CLOSE_CLOSE_PROFILE_KEY.to_owned(),
                sha256_hex(&serialized_close_profile),
            ),
            (
                FILE_CLOSE_NESTED_INPUT_KEY.to_owned(),
                sha256_hex(&serialized_nested),
            ),
            // Four distinct keys, so a dependent whose *any* prerequisite moved
            // is a different task and one key cannot absorb another.
            (
                FileClosePrerequisites::WRAPPER_CALL_USE_KEY.to_owned(),
                claimed.wrapper_call_use.digest(),
            ),
            (
                FileClosePrerequisites::UNMAP_COMPOSITION_KEY.to_owned(),
                claimed.unmap_composition.digest(),
            ),
            (
                FileClosePrerequisites::NATIVE_CLEANUP_KEY.to_owned(),
                claimed.native_cleanup.digest(),
            ),
        ]);
        if let Some(contract) = contracts.delete_contract_source {
            contract_sources.insert(
                FILE_CLOSE_DELETE_CONTRACT_KEY.to_owned(),
                sha256_hex(contract.as_bytes()),
            );
        }
        if let (Some(contract), Some(policy), Some(profile)) = (
            contracts.delete_prerequisite_contract_source,
            contracts.delete_prerequisite_policy,
            delete_profile.as_ref(),
        ) {
            let serialized =
                serde_json::to_vec(&profile_record(profile)).map_err(|e| e.to_string())?;
            contract_sources.insert(
                FILE_CLOSE_DELETE_PREREQ_CONTRACT_KEY.to_owned(),
                sha256_hex(contract.as_bytes()),
            );
            contract_sources.insert(
                FILE_CLOSE_DELETE_PREREQ_POLICY_KEY.to_owned(),
                sha256_hex(policy.as_bytes()),
            );
            contract_sources.insert(
                FILE_CLOSE_DELETE_PREREQ_PROFILE_KEY.to_owned(),
                sha256_hex(&serialized),
            );
        }
        if let Some(delete) = &claimed.native_delete {
            contract_sources.insert(
                FileClosePrerequisites::NATIVE_DELETE_KEY.to_owned(),
                delete.digest(),
            );
        }
        // This route's own premises, over and above what its components already
        // assume: the wrapper's `free` bindings, and the fact that releasing the
        // backend's C allocation never discharges an OS cleanup obligation.
        let own = BTreeMap::from([
            ("libc_free_bindings_are_declared_premises".into(), true),
            ("os_cleanup_success_is_not_inferred".into(), true),
            ("backend_release_does_not_discharge_os_custody".into(), true),
        ]);
        let mut sets = prerequisites.premise_sets();
        sets.push(&own);
        let assumptions = composition_premise_union(&sets)?;
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
            semantic_profile: expected_profile.into(),
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
        let property = ObligationProperty::CompositionContract;
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
            no_delete_contract_source: contracts.no_delete_contract_source.into(),
            delete_contract_source: contracts.delete_contract_source.map(str::to_owned),
            close_contract_source: contracts.close_contract_source.into(),
            close_policy: contracts.close_policy.into(),
            close_profile,
            delete_prerequisite_contract_source: contracts
                .delete_prerequisite_contract_source
                .map(str::to_owned),
            delete_prerequisite_policy: contracts.delete_prerequisite_policy.map(str::to_owned),
            delete_profile,
            delete_mapping_field,
            prerequisites,
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    /// Whether this task answers the deleting entry point.
    #[must_use]
    pub fn is_deleting(&self) -> bool {
        self.delete_profile.is_some()
    }

    /// The wrapper body this task imported.
    pub fn wrapper_summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.wrapper_summary()
    }

    /// The record profile the core derived from the close contract's own
    /// declaration.
    pub fn close_profile(&self) -> &CleanupProfile {
        &self.close_profile
    }

    pub fn close_policy(&self) -> &str {
        &self.close_policy
    }

    /// The record profile and eligibility field the core derived from the delete
    /// contract's own declaration, on the deleting task.
    pub fn delete_profile(&self) -> Option<&CleanupProfile> {
        self.delete_profile.as_ref()
    }

    pub fn delete_policy(&self) -> Option<&str> {
        self.delete_prerequisite_policy.as_deref()
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("composition:{}", self.task.task_digest()),
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
            cleanup_replay_source: None,
            composition_source: Some(self.import.source_input()),
        }
    }

    /// How many composition levels this task re-establishes, counting itself.
    /// Measured from the transported question's own shape rather than assumed,
    /// and pinned by test.
    #[must_use]
    pub fn nesting_depth(&self) -> u32 {
        self.import.source_input().family.nesting_depth()
    }

    /// The whole question this context imported, so a *dependent* can require
    /// its transported nested question to be exactly this one.
    pub fn family_input(&self) -> CompositionFamilyInput {
        self.import.source_input().family
    }

    /// Re-establish every prerequisite, at every level. The **outer** level of
    /// the two-level re-establishment: the nested composition is re-derived and
    /// re-run here, and it re-establishes its own three prerequisites in turn,
    /// one of which re-establishes its own. Nothing a provider sends reaches any
    /// level.
    pub fn revalidate_prerequisites(
        &self,
    ) -> Result<
        (
            UnmapCertificate,
            CleanupCertificate,
            Option<crate::cleanup_outcome::GuardedDeleteCertificate>,
        ),
        String,
    > {
        let input = self.import.source_input();
        let claimed = input
            .prerequisites
            .file_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        // The wrapper's own def/use contract must still hold: on the retained
        // in-process route the composition is unobtainable without it.
        self.prerequisites
            .wrapper_call_use
            .revalidate_prerequisite(&claimed.wrapper_call_use)?;
        // Level two: the nested composition re-establishes its own three.
        let unmap = self
            .prerequisites
            .unmap_composition
            .revalidate_as_prerequisite(&claimed.unmap_composition)?;
        let close = match self
            .prerequisites
            .native_cleanup
            .revalidate_as_prerequisite(&claimed.native_cleanup)?
        {
            CleanupReplayCertificate::NativeCleanupOutcome(certificate) => *certificate,
            CleanupReplayCertificate::GuardedDelete(_) => {
                return Err("the close prerequisite answered the wrong family".into());
            }
        };
        let delete = match (&self.prerequisites.native_delete, &claimed.native_delete) {
            (None, None) => None,
            (Some(task), Some(identity)) => match task.revalidate_as_prerequisite(identity)? {
                CleanupReplayCertificate::GuardedDelete(certificate) => Some(*certificate),
                CleanupReplayCertificate::NativeCleanupOutcome(_) => {
                    return Err("the delete prerequisite answered the wrong family".into());
                }
            },
            _ => return Err("the deleting arm and its delete prerequisite must agree".into()),
        };
        Ok((unmap, close, delete))
    }

    /// The pinned inputs the non-deleting arm is composed under, all of them the
    /// core's own — including the nested composition's, taken from the nested
    /// context rather than restated, so the two levels cannot diverge.
    fn no_delete_contracts<'a>(
        &'a self,
        unmap: &'a UnmapCompositionContracts<'a>,
    ) -> FileCloseContracts<'a> {
        FileCloseContracts {
            pinned_revision: self.import.source_revision(),
            unmap,
            close_function: self.close_profile.function.as_str(),
            close_profile: &self.close_profile,
            close_contract: &self.close_contract_source,
            contract: &self.no_delete_contract_source,
        }
    }

    /// Re-establish every prerequisite at every level and re-run the unedited
    /// composition over the core's own imports, contracts and contract-derived
    /// profiles. This is the core's own evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<FileCloseCertificate, String> {
        let (unmap, close, delete) = self.revalidate_prerequisites()?;
        let nested = self.prerequisites.unmap_composition.composition_contracts();
        let no_delete = check_file_close_composition(
            self.import.wrapper_summary(),
            &unmap,
            &close,
            &self.no_delete_contracts(&nested),
        )?;
        match (delete, &self.delete_profile, &self.delete_mapping_field) {
            (None, None, None) => Ok(FileCloseCertificate::NoDelete(Box::new(no_delete))),
            (Some(delete), Some(profile), Some(field)) => {
                let contract = self
                    .delete_contract_source
                    .as_deref()
                    .ok_or("the deleting arm has no pinned contract")?;
                let delete_function = self
                    .import
                    .delete_summary()
                    .ok_or("the deleting arm imported no delete body")?
                    .function();
                let evidence = check_file_close_deletion(
                    no_delete,
                    &delete,
                    &self.no_delete_contracts(&nested),
                    &FileCloseDeleteContracts {
                        delete_function,
                        delete_profile: profile,
                        delete_mapping_field: field,
                        delete_contract: self
                            .delete_prerequisite_contract_source
                            .as_deref()
                            .ok_or("the deleting arm has no pinned prerequisite contract")?,
                        contract,
                    },
                )?;
                Ok(FileCloseCertificate::Deleting(Box::new(evidence)))
            }
            _ => Err("the deleting arm's certificate and contracts disagree".into()),
        }
    }

    /// This task's own identity, for a *dependent* capability to name it by.
    /// Added by card `P04-COMPOSE-FS`, whose backend prerequisite is this task.
    pub fn prerequisite_identity(&self) -> PrerequisiteIdentity {
        PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// Re-run this whole two-level composition for a dependent and confirm the
    /// claimed identity is the one this context derives. Only the **deleting**
    /// entry point can serve as a prerequisite: a dependent that crosses both
    /// case vectors needs both, and the non-deleting arm publishes only one.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<Box<FileCloseDeletionEvidence>, String> {
        if !claimed.is_complete() {
            return Err("file_close prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err("file_close prerequisite identity is not the one the core derives".into());
        }
        match self.evaluate()? {
            FileCloseCertificate::Deleting(evidence) => Ok(evidence),
            FileCloseCertificate::NoDelete(_) => Err(
                "only the deleting file_close composition can serve as a backend prerequisite"
                    .into(),
            ),
        }
    }

    /// The composed reading the core reads out of its own checked certificate.
    fn record(
        &self,
        certificate: &FileCloseCertificate,
    ) -> Result<CompositionProposalRecord, String> {
        let index =
            |value: usize| u32::try_from(value).map_err(|_| "index out of range".to_owned());
        let residual = |value: ResidualCustody| match value {
            ResidualCustody::Absent => ResidualCustodyRecord::Absent,
            ResidualCustody::Unresolved => ResidualCustodyRecord::Unresolved,
        };
        let no_delete = certificate.no_delete();
        let mut cases = Vec::new();
        for case in no_delete.cases() {
            cases.push(FileCloseCaseRecord {
                source_path: index(case.source_path)?,
                backend_present: case.backend_present,
                path_allocation_present: case.path_allocation_present,
                unmap_case: case.unmap_case.map(index).transpose()?,
                close_case: case.close_case.map(index).transpose()?,
                mapping: residual(case.mapping),
                descriptor: residual(case.descriptor),
                path_released: case.path_released,
                backend_released: case.backend_released,
                up_counter_decrements: index(case.up_counter_decrements)?,
            });
        }
        let uncovered_paths = no_delete
            .uncovered_paths()
            .iter()
            .map(|value| index(*value))
            .collect::<Result<Vec<_>, _>>()?;
        let (deleting_cases, deleting_uncovered_paths) = match certificate {
            FileCloseCertificate::NoDelete(_) => (None, None),
            FileCloseCertificate::Deleting(evidence) => {
                let mut deleting = Vec::new();
                for case in evidence.deleting_cases() {
                    deleting.push(DeletingCloseCaseRecord {
                        source_path: index(case.source_path)?,
                        unmap_case: case.unmap_case.map(index).transpose()?,
                        close_case: case.close_case.map(index).transpose()?,
                        delete_case: case.delete_case.map(index).transpose()?,
                        mapping: residual(case.mapping),
                        descriptor: residual(case.descriptor),
                        deletion: match case.deletion {
                            DeletionOutcome::NoBackend => DeletionOutcomeRecord::NoBackend,
                            DeletionOutcome::BlockedByFields => {
                                DeletionOutcomeRecord::BlockedByFields
                            }
                            DeletionOutcome::SyscallFailed => DeletionOutcomeRecord::SyscallFailed,
                            DeletionOutcome::SyscallSucceeded => {
                                DeletionOutcomeRecord::SyscallSucceeded
                            }
                        },
                        backend_released: case.backend_released,
                        path_released: case.path_released,
                        diagnostic_calls: index(case.diagnostic_calls)?,
                        up_counter_decrements: index(case.up_counter_decrements)?,
                    });
                }
                let uncovered = evidence
                    .uncovered_paths()
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<Vec<_>, _>>()?;
                (Some(deleting), Some(uncovered))
            }
        };
        let source = self.import.wrapper_summary();
        Ok(CompositionProposalRecord::FileClose(
            FileCloseCompositionRecord {
                function: source.function().into(),
                source_revision: source.source_revision().into(),
                signature: source.signature().into(),
                cases,
                uncovered_paths,
                deleting_cases,
                deleting_uncovered_paths,
                profile_assumptions: source.profile_assumptions().clone(),
            },
        ))
    }
}

/// Identity and target checks for the `file_close` route.
fn check_file_close_target(
    target: &TargetProfile,
    expected_profile: &str,
    import_revision: &str,
    identity: &SourceInputIdentity,
    translation_unit: &str,
    contracts: &FileCloseCompositionContracts<'_>,
    deleting: bool,
) -> Result<(), String> {
    let delete_selection_present = contracts.delete_contract_source.is_some()
        && contracts.delete_prerequisite_contract_source.is_some()
        && contracts.delete_prerequisite_policy.is_some();
    let delete_selection_absent = contracts.delete_contract_source.is_none()
        && contracts.delete_prerequisite_contract_source.is_none()
        && contracts.delete_prerequisite_policy.is_none();
    if translation_unit.is_empty()
        || contracts.no_delete_contract_source.is_empty()
        || contracts.close_contract_source.is_empty()
        || contracts.close_policy.is_empty()
        || (deleting && !delete_selection_present)
        || (!deleting && !delete_selection_absent)
        || contracts.delete_contract_source.is_some_and(str::is_empty)
        || contracts
            .delete_prerequisite_contract_source
            .is_some_and(str::is_empty)
        || contracts
            .delete_prerequisite_policy
            .is_some_and(str::is_empty)
        || target.library.is_empty()
        || target.profile != expected_profile
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible file_close composition target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native composition platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the composition route imports no helper bodies".into());
    }
    Ok(())
}

impl WitnessChecker for FileCloseCompositionSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("composition task differs from its checked import context".into());
        }
        let SemanticWitness::Composition(proposed) = witness else {
            return Err("expected composition witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("composition witness is bound to a different task".into());
        }
        // Every prerequisite at every level is re-established and both
        // compositions re-run by the core before the proposal is looked at.
        let certificate = self.evaluate()?;
        if proposed.summary != self.record(&certificate)? {
            return Err("composition witness differs from the checked source composition".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}

// ---------------------------------------------------------------------------
// The memory-close composition: four prerequisites over a **derived** unit.
// ---------------------------------------------------------------------------

/// Digest keys the memory-close route binds beyond the shared input key.
pub const MEMORY_CLOSE_BASE_IMPORT_KEY: &str = "qcore:composition-memory-base-import:v1";
pub const MEMORY_CLOSE_DERIVED_IMPORT_KEY: &str = "qcore:composition-memory-derived-import:v1";
/// The derivation itself — recipe, recipe function, both measured text digests
/// and both input identity classes — bound separately from either identity so a
/// moved *recipe* and a moved *text* are two different retirements.
pub const MEMORY_CLOSE_DERIVATION_KEY: &str = "qcore:composition-memory-derivation:v1";
pub const MEMORY_CLOSE_CONTRACT_KEY: &str = "qcore:composition-memory-close-contract:v1";
pub const MEMORY_CLOSE_FRAME_CONTRACT_KEY: &str = "qcore:composition-memory-frame-contract:v1";
pub const MEMORY_CLOSE_BACKEND_CONTRACT_KEY: &str = "qcore:composition-memory-backend-contract:v1";
pub const MEMORY_CLOSE_UNLINK_CONTRACT_KEY: &str = "qcore:composition-memory-unlink-contract:v1";
pub const MEMORY_CLOSE_COUNTER_CONTRACT_KEY: &str = "qcore:composition-memory-counter-contract:v1";
/// The exact counter field and delta this composition requires of its counter
/// prerequisite. Neither is in the counter contract, so without this key two
/// different required decrements over one contract would be one task.
pub const MEMORY_CLOSE_COUNTER_BINDING_KEY: &str = "qcore:composition-memory-counter-binding:v1";
/// The four prerequisite bodies' callee identities, as the core derived them.
pub const MEMORY_CLOSE_CALLEE_KEY: &str = "qcore:composition-memory-callees:v1";

/// The memory-close route's prerequisite contexts, each the core's **own**.
///
/// Three of these four had no `PrerequisiteIdentity` before wave 14 and one —
/// the frame helper — had no verification task at all. That is what blocked
/// this family for two waves; `P04-FRAME` supplied all four.
pub struct MemoryClosePrerequisiteTasks {
    pub reset: FrameHelperSourceTask,
    pub backend: FieldDestructorSourceTask,
    pub unlink: ListUnlinkSourceTask,
    pub counter: CounterSourceTask,
}

impl MemoryClosePrerequisiteTasks {
    fn premise_sets(&self) -> Vec<&BTreeMap<String, bool>> {
        vec![
            &self.reset.task().assumptions,
            &self.backend.task().assumptions,
            &self.unlink.task().assumptions,
            &self.counter.task().assumptions,
        ]
    }

    fn identities(&self) -> MemoryClosePrerequisites {
        MemoryClosePrerequisites {
            reset: self.reset.prerequisite_identity(),
            backend: self.backend.prerequisite_identity(),
            unlink: self.unlink.prerequisite_identity(),
            counter: self.counter.prerequisite_identity(),
        }
    }

    /// The unit each prerequisite was checked over, as its own task records it.
    fn source_digests(&self) -> [&str; 4] {
        [
            self.reset.task().unit.source_digest.as_str(),
            self.backend.task().unit.source_digest.as_str(),
            self.unlink.task().unit.source_digest.as_str(),
            self.counter.task().unit.source_digest.as_str(),
        ]
    }
}

/// Sealed memory-close composition context. A raw wire projection cannot
/// construct one.
pub struct MemoryCloseCompositionSourceTask {
    import: MemoryCloseSourceImport,
    contracts: OwnedMemoryCloseContracts,
    prerequisites: MemoryClosePrerequisiteTasks,
    task: VerificationTask,
}

impl MemoryCloseCompositionSourceTask {
    /// Build the task. `prerequisites` are the core's **own** contexts; the
    /// projection's claimed identities are required to be the ones they derive,
    /// the transported derivation is required to be the one the core's own
    /// importer measured under the core's own recipe, and every prerequisite is
    /// required to have been checked over the very unit that derivation derives
    /// from.
    pub fn derive(
        import: &MemoryCloseSourceImport,
        prerequisites: MemoryClosePrerequisiteTasks,
        translation_unit: &str,
        contracts: &MemoryCloseContracts<'_>,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_memory_close_target(
            &target,
            import.source_revision(),
            import.derived_identity(),
            translation_unit,
            contracts,
        )?;
        let input = import.source_input();
        if !input.version_matches() {
            return Err("unsupported composition projection version".into());
        }
        if input.family.semantic_profile() != MEMORY_CLOSE_SEMANTIC_PROFILE {
            return Err("the transported question answers another semantic profile".into());
        }
        if !input.family.matches(&input.prerequisites) {
            return Err("composition family and prerequisite set disagree".into());
        }
        // 1. The derivation is the core's own. The transported record must be
        //    the one this import measured, and that record must in turn be the
        //    one the core's own recipe produces. Nothing about the derived text
        //    is taken on a producer's word — the derived text does not travel at
        //    all, and its account of the derivation is compared, not believed.
        let CompositionFamilyInput::MemoryClose {
            derivation,
            callees,
            ..
        } = &input.family
        else {
            return Err("this context checks the memory close composition only".into());
        };
        if derivation != import.derivation() {
            return Err(
                "the transported derivation is not the one the core's own importer measured".into(),
            );
        }
        if !derivation.is_complete() || !derivation.is_a_derivation() {
            return Err("the transported derivation is incomplete or derives no new text".into());
        }
        if derivation.recipe != contracts.probe_recipe
            || derivation.recipe_function != contracts.probe_function
        {
            return Err("the transported recipe is not the core's own derivation recipe".into());
        }
        if derivation.base_source_digest != import.base_identity().raw_source_digest()
            || derivation.derived_source_digest != import.derived_identity().raw_source_digest()
        {
            return Err("the transported derivation names text the core did not derive".into());
        }
        // 2. Each claimed identity is a lookup key, checked against the identity
        //    the core's own prerequisite context derives.
        let claimed = input
            .prerequisites
            .memory_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        if !claimed.is_complete() || !claimed.are_distinct() {
            return Err("memory close prerequisite identities are incomplete or repeated".into());
        }
        if claimed != prerequisites.identities() {
            return Err(
                "memory close prerequisite identities are not the ones the core derives".into(),
            );
        }
        // 3. Every prerequisite must have been checked over the unit this
        //    derivation derives from. Without this a prerequisite task over an
        //    unrelated unit could supply a frame identity the wrapper resolves.
        let base = import.base_identity().raw_source_digest();
        if prerequisites
            .source_digests()
            .iter()
            .any(|digest| *digest != base)
        {
            return Err(
                "a memory close prerequisite was checked over a different unit than the one the \
                 derivation derives from"
                    .into(),
            );
        }
        // 4. The transported callee identities must be the ones the core reads
        //    off its own prerequisites' bodies, so the wrapper cannot be
        //    projected against frames no prerequisite established.
        let derived_callees = memory_close_callees(
            (prerequisites.reset.summary().function(), RESET_SIGNATURE),
            prerequisites.backend.summary(),
            prerequisites.unlink.summary(),
            prerequisites.counter.summary(),
        );
        if *callees != derived_callees || *import.callees() != derived_callees {
            return Err(
                "the transported callee identities are not the ones the core's prerequisites \
                 establish"
                    .into(),
            );
        }
        let paths = u64::try_from(import.wrapper_summary().paths().len())
            .map_err(|_| "composition path count out of range")?;
        if paths == 0 {
            return Err("composition import has no complete wrapper path".into());
        }
        let serialized_base =
            serde_json::to_vec(import.base_identity()).map_err(|e| e.to_string())?;
        let serialized_derived =
            serde_json::to_vec(import.derived_identity()).map_err(|e| e.to_string())?;
        let serialized_derivation = serde_json::to_vec(derivation).map_err(|e| e.to_string())?;
        let serialized_callees = serde_json::to_vec(&derived_callees).map_err(|e| e.to_string())?;
        let serialized_counter_binding =
            serde_json::to_vec(&(contracts.counter_field, contracts.counter_delta))
                .map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (COMPOSITION_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            // Two import keys, not one. `for_derived_text` erases a declared
            // file, so the derived identity records neither the spelled input
            // nor the directory: two base units with equal bytes and different
            // declared identities derive byte-identical derived identities.
            // Binding only the derived one would make them a single task.
            (
                MEMORY_CLOSE_BASE_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_base),
            ),
            (
                MEMORY_CLOSE_DERIVED_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_derived),
            ),
            (
                MEMORY_CLOSE_DERIVATION_KEY.to_owned(),
                sha256_hex(&serialized_derivation),
            ),
            (
                MEMORY_CLOSE_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.contract.as_bytes()),
            ),
            // A composition binds every contract it composes under, not only
            // its own: a moved prerequisite contract is a different question.
            (
                MEMORY_CLOSE_FRAME_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.frame_contract.as_bytes()),
            ),
            (
                MEMORY_CLOSE_BACKEND_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.backend_contract.as_bytes()),
            ),
            (
                MEMORY_CLOSE_UNLINK_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.unlink_contract.as_bytes()),
            ),
            (
                MEMORY_CLOSE_COUNTER_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.counter_contract.as_bytes()),
            ),
            (
                MEMORY_CLOSE_COUNTER_BINDING_KEY.to_owned(),
                sha256_hex(&serialized_counter_binding),
            ),
            (
                MEMORY_CLOSE_CALLEE_KEY.to_owned(),
                sha256_hex(&serialized_callees),
            ),
            // Four distinct keys, so a dependent whose *any* prerequisite moved
            // is a different task and one key cannot absorb another.
            (
                MemoryClosePrerequisites::RESET_KEY.to_owned(),
                claimed.reset.digest(),
            ),
            (
                MemoryClosePrerequisites::BACKEND_KEY.to_owned(),
                claimed.backend.digest(),
            ),
            (
                MemoryClosePrerequisites::UNLINK_KEY.to_owned(),
                claimed.unlink.digest(),
            ),
            (
                MemoryClosePrerequisites::COUNTER_KEY.to_owned(),
                claimed.counter.digest(),
            ),
        ]);
        // This route's own premises, over and above what its components already
        // assume. The last one is the card's own: the bodies were read out of
        // text this repository derived, not out of the file the audit selected,
        // and that derivation is a declared step rather than a proved one.
        let own = BTreeMap::from([
            ("libc_free_bindings_are_declared_premises".into(), true),
            (
                "derived_probe_text_preprocesses_as_the_base_unit_does".into(),
                true,
            ),
            (
                "backend_discriminant_is_read_from_a_source_probe".into(),
                true,
            ),
        ]);
        let mut sets = prerequisites.premise_sets();
        sets.push(&own);
        let assumptions = composition_premise_union(&sets)?;
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
                // The unit the audit selected, not the text the recipe made.
                source_digest: import.base_identity().raw_source_digest().into(),
                // Both identities, because both are load-bearing here.
                build_digest: sha256_hex(
                    &serde_json::to_vec(&(&serialized_base, &serialized_derived))
                        .map_err(|e| e.to_string())?,
                ),
            },
            target,
            semantic_profile: MEMORY_CLOSE_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::CompositionContract;
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
            contracts: OwnedMemoryCloseContracts::of(contracts),
            prerequisites,
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    /// The wrapper body this task imported, from the derived unit.
    pub fn wrapper_summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.wrapper_summary()
    }

    /// The derivation the core's own importer measured.
    pub fn derivation(&self) -> &qcore_schemas::composition_summary::DerivedUnitRecord {
        self.import.derivation()
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("composition:{}", self.task.task_digest()),
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
            cleanup_replay_source: None,
            composition_source: Some(self.import.source_input()),
        }
    }

    /// Re-establish all four prerequisites and assemble the dependency bundle
    /// the composition consumes. Every member is re-derived and re-evaluated by
    /// the core; nothing a provider sends reaches this.
    pub fn revalidate_prerequisites(&self) -> Result<MemoryCloseDependencies, String> {
        let input = self.import.source_input();
        let claimed = input
            .prerequisites
            .memory_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        self.prerequisites
            .reset
            .revalidate_as_prerequisite(&claimed.reset)?;
        let backend = self
            .prerequisites
            .backend
            .revalidate_as_prerequisite(&claimed.backend)?;
        let unlink = self
            .prerequisites
            .unlink
            .revalidate_as_prerequisite(&claimed.unlink)?;
        let counter = self
            .prerequisites
            .counter
            .revalidate_as_prerequisite(&claimed.counter)?;
        Ok(MemoryCloseDependencies {
            // The frame helper has no transported witness form at all, so its
            // reading is the core's own summary, re-evaluated above.
            reset: self.prerequisites.reset.summary().clone(),
            backend,
            unlink,
            counter,
        })
    }

    /// Re-establish every prerequisite and re-run the unedited composition over
    /// the core's own imports and its own contracts. This is the core's own
    /// evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<MemoryCloseCertificate, String> {
        Ok(self.reading()?.certificate)
    }

    /// The whole reading the core decides: the certificate the retained route
    /// publishes, the probed discriminant and the per-entry replay detail.
    pub fn reading(&self) -> Result<MemoryCloseReading, String> {
        let dependencies = self.revalidate_prerequisites()?;
        read_memory_close_bodies(
            self.import.wrapper_summary(),
            self.import.probe_summary(),
            &dependencies,
            &self.contracts.borrowed(),
        )
    }

    /// The whole question this context imported, so a *dependent* can require
    /// its transported nested question to be exactly this one.
    pub fn family_input(&self) -> CompositionFamilyInput {
        self.import.source_input().family
    }

    /// This task's own identity, for a *dependent* capability to name it by.
    pub fn prerequisite_identity(&self) -> PrerequisiteIdentity {
        PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// Re-run this whole composition for a dependent and confirm the claimed
    /// identity is the one this context derives.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<MemoryCloseCertificate, String> {
        if !claimed.is_complete() {
            return Err("memory close prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err(
                "memory close prerequisite identity is not the one the core derives".into(),
            );
        }
        self.evaluate()
    }

    /// The composed reading the core reads out of its own checked certificate.
    fn record(&self, reading: &MemoryCloseReading) -> Result<CompositionProposalRecord, String> {
        let index =
            |value: usize| u32::try_from(value).map_err(|_| "index out of range".to_owned());
        let mut cases = Vec::new();
        for entry in &reading.entries {
            cases.push(MemoryCloseCaseRecord {
                nonnull: entry.nonnull,
                source_path: index(entry.source_path)?,
                calls: index(entry.calls)?,
                backend_discharged: entry.backend_discharged,
                primary_unlinked: entry.primary_unlinked,
                state_unlinked: entry.state_unlinked,
                name_released: entry.name_released,
                chunk_released: entry.chunk_released,
                counter_decremented: entry.counter_decremented,
            });
        }
        let source = self.import.wrapper_summary();
        Ok(CompositionProposalRecord::MemoryClose(
            MemoryCloseCompositionRecord {
                function: source.function().into(),
                source_revision: source.source_revision().into(),
                signature: source.signature().into(),
                memory_kind: reading.memory_kind,
                cases,
                checked_paths: reading
                    .certificate
                    .checked_paths()
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<Vec<_>, _>>()?,
                uncovered_paths: reading
                    .certificate
                    .uncovered_paths()
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<Vec<_>, _>>()?,
                profile_assumptions: source.profile_assumptions().clone(),
            },
        ))
    }
}

/// Identity and target checks for the memory-close route. The dialect is
/// compared against the **derived** unit's identity, because that is the unit
/// the bodies were preprocessed and parsed from.
fn check_memory_close_target(
    target: &TargetProfile,
    import_revision: &str,
    identity: &SourceInputIdentity,
    translation_unit: &str,
    contracts: &MemoryCloseContracts<'_>,
) -> Result<(), String> {
    if translation_unit.is_empty()
        || contracts.contract.is_empty()
        || contracts.probe_recipe.is_empty()
        || contracts.probe_function.is_empty()
        || contracts.frame_contract.is_empty()
        || contracts.backend_contract.is_empty()
        || contracts.unlink_contract.is_empty()
        || contracts.counter_contract.is_empty()
        || contracts.counter_field.is_empty()
        || target.library.is_empty()
        || target.profile != MEMORY_CLOSE_SEMANTIC_PROFILE
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible memory close target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native composition platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the composition route imports no helper bodies".into());
    }
    Ok(())
}

impl WitnessChecker for MemoryCloseCompositionSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("composition task differs from its checked import context".into());
        }
        let SemanticWitness::Composition(proposed) = witness else {
            return Err("expected composition witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("composition witness is bound to a different task".into());
        }
        // Every prerequisite is re-established and the composition re-run by the
        // core before the proposal is looked at at all.
        let reading = self.reading()?;
        if proposed.summary != self.record(&reading)? {
            return Err("composition witness differs from the checked source composition".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}

// ---------------------------------------------------------------------------
// The filesystem-chunk-close composition: two composition prerequisites, over
// two derived units, in two different translation units.
// ---------------------------------------------------------------------------

/// Digest keys the filesystem route binds beyond the shared input key.
pub const FS_CLOSE_BASE_IMPORT_KEY: &str = "qcore:composition-fs-base-import:v1";
pub const FS_CLOSE_DERIVED_IMPORT_KEY: &str = "qcore:composition-fs-derived-import:v1";
/// **This family's own** derivation — recipe, recipe function, both measured
/// text digests and both input identity classes.
pub const FS_CLOSE_DERIVATION_KEY: &str = "qcore:composition-fs-derivation:v1";
/// The **nested memory-close question's** derivation, bound separately so a
/// moved memory recipe retires this task even where its own recipe did not
/// move. Two derivations, two keys.
pub const FS_CLOSE_MEMORY_DERIVATION_KEY: &str = "qcore:composition-fs-memory-derivation:v1";
pub const FS_CLOSE_CONTRACT_KEY: &str = "qcore:composition-fs-close-contract:v1";
/// The two nested questions, each bound separately from its prerequisite
/// identity so a moved nested *question* and a moved nested *task* are two
/// different retirements.
pub const FS_CLOSE_MEMORY_QUESTION_KEY: &str = "qcore:composition-fs-memory-question:v1";
pub const FS_CLOSE_BACKEND_QUESTION_KEY: &str = "qcore:composition-fs-backend-question:v1";
/// The backend prerequisite's whole pinned contract selection, which this route
/// re-runs its recheck under and which is not in either nested question.
pub const FS_CLOSE_BACKEND_CONTRACTS_KEY: &str = "qcore:composition-fs-backend-contracts:v1";

/// The filesystem route's prerequisite contexts, each the core's **own**. Both
/// are compositions, which makes this the first route whose every prerequisite
/// re-establishes further prerequisites of its own.
pub struct FilesystemChunkClosePrerequisiteTasks {
    pub memory_close: MemoryCloseCompositionSourceTask,
    /// The **deleting** `cio_file_close` composition. The non-deleting arm is
    /// refused at re-establishment: this question crosses both case vectors.
    pub file_close_delete: FileCloseCompositionSourceTask,
}

impl FilesystemChunkClosePrerequisiteTasks {
    fn premise_sets(&self) -> Vec<&BTreeMap<String, bool>> {
        vec![
            &self.memory_close.task().assumptions,
            &self.file_close_delete.task().assumptions,
        ]
    }

    fn identities(&self) -> FilesystemChunkClosePrerequisites {
        FilesystemChunkClosePrerequisites {
            memory_close: self.memory_close.prerequisite_identity(),
            file_close_delete: self.file_close_delete.prerequisite_identity(),
        }
    }
}

/// Sealed filesystem-chunk-close composition context. A raw wire projection
/// cannot construct one.
pub struct FilesystemChunkCloseCompositionSourceTask {
    import: FilesystemChunkCloseSourceImport,
    contracts: OwnedFilesystemChunkCloseContracts,
    prerequisites: FilesystemChunkClosePrerequisiteTasks,
    task: VerificationTask,
}

impl FilesystemChunkCloseCompositionSourceTask {
    /// Build the task. `prerequisites` are the core's **own** composition
    /// contexts; the projection's claimed identities are required to be the ones
    /// they derive, both transported nested questions are required to be the
    /// ones those contexts imported, and this family's own transported
    /// derivation is required to be the one the core's own importer measured
    /// under the core's own recipe.
    pub fn derive(
        import: &FilesystemChunkCloseSourceImport,
        prerequisites: FilesystemChunkClosePrerequisiteTasks,
        translation_unit: &str,
        contracts: &FilesystemChunkCloseContracts<'_>,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_filesystem_close_target(
            &target,
            import.source_revision(),
            import.derived_identity(),
            translation_unit,
            contracts,
        )?;
        let input = import.source_input();
        if !input.version_matches() {
            return Err("unsupported composition projection version".into());
        }
        if input.family.semantic_profile() != FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE {
            return Err("the transported question answers another semantic profile".into());
        }
        if !input.family.matches(&input.prerequisites) {
            return Err("composition family and prerequisite set disagree".into());
        }
        let CompositionFamilyInput::FilesystemChunkClose {
            memory,
            backend,
            derivation,
            ..
        } = &input.family
        else {
            return Err("this context checks the filesystem chunk close composition only".into());
        };
        // 1. This family's own derivation is the core's own. Nothing about the
        //    derived text is taken on a producer's word: the text does not
        //    travel, and its account of the derivation is compared, not
        //    believed.
        if derivation != import.derivation() {
            return Err(
                "the transported derivation is not the one the core's own importer measured".into(),
            );
        }
        if !derivation.is_complete() || !derivation.is_a_derivation() {
            return Err("the transported derivation is incomplete or derives no new text".into());
        }
        if derivation.recipe != contracts.probe_recipe
            || derivation.recipe_function != contracts.probe_function
        {
            return Err("the transported recipe is not the core's own derivation recipe".into());
        }
        if derivation.base_source_digest != import.base_identity().raw_source_digest()
            || derivation.derived_source_digest != import.derived_identity().raw_source_digest()
        {
            return Err("the transported derivation names text the core did not derive".into());
        }
        // 2. The *second* derivation — the nested memory question's — must
        //    derive from the same base text under a different recipe, and must
        //    be the one that question's own context measured. Two derivations
        //    over one base, and the core owns both.
        let memory_derivation = memory
            .derivation()
            .ok_or("the nested memory-close question carries no derivation")?;
        if memory_derivation != prerequisites.memory_close.derivation() {
            return Err(
                "the nested memory derivation is not the one the memory context measured".into(),
            );
        }
        if memory_derivation.base_source_digest != derivation.base_source_digest {
            return Err("the two derived units do not derive from one base text".into());
        }
        if memory_derivation.recipe == derivation.recipe
            || memory_derivation.recipe_function == derivation.recipe_function
            || memory_derivation.derived_source_digest == derivation.derived_source_digest
        {
            return Err(
                "the two derived units must be derived by two different recipes into two \
                 different texts"
                    .into(),
            );
        }
        // 3. Each claimed identity is a lookup key, checked against the identity
        //    the core's own prerequisite context derives.
        let claimed = input
            .prerequisites
            .filesystem_chunk_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        if !claimed.is_complete() || !claimed.are_distinct() {
            return Err(
                "filesystem chunk close prerequisite identities are incomplete or repeated".into(),
            );
        }
        if claimed != prerequisites.identities() {
            return Err(
                "filesystem chunk close prerequisite identities are not the ones the core derives"
                    .into(),
            );
        }
        // 4. Both transported nested *questions* must be, verbatim, the
        //    questions the core's own nested contexts imported. A nested
        //    question that is merely well-formed, or that belongs to another
        //    unit, is refused here.
        if **memory != prerequisites.memory_close.family_input() {
            return Err(
                "the transported memory question is not the one the memory composition imported"
                    .into(),
            );
        }
        if **backend != prerequisites.file_close_delete.family_input() {
            return Err(
                "the transported backend question is not the one the file_close composition \
                 imported"
                    .into(),
            );
        }
        if !prerequisites.file_close_delete.is_deleting() {
            return Err("the backend prerequisite is not the deleting composition".into());
        }
        // 5. The memory prerequisite is over *this* base text. The backend
        //    prerequisite deliberately is not, and cannot be: it lives in
        //    another translation unit (`P04-COMPOSE-FS-CROSS-UNIT`).
        if prerequisites.memory_close.task().unit.source_digest
            != import.base_identity().raw_source_digest()
        {
            return Err(
                "the memory prerequisite was checked over a different unit than the one this \
                 derivation derives from"
                    .into(),
            );
        }
        if prerequisites.file_close_delete.task().unit.source_digest
            == import.base_identity().raw_source_digest()
        {
            return Err(
                "the backend prerequisite is expected in another translation unit than the \
                 wrapper's"
                    .into(),
            );
        }
        let paths = u64::try_from(import.wrapper_record().paths.len())
            .map_err(|_| "composition path count out of range")?;
        if paths == 0 {
            return Err("composition import has no complete wrapper path".into());
        }
        let serialized_base =
            serde_json::to_vec(import.base_identity()).map_err(|e| e.to_string())?;
        let serialized_derived =
            serde_json::to_vec(import.derived_identity()).map_err(|e| e.to_string())?;
        let serialized_derivation = serde_json::to_vec(derivation).map_err(|e| e.to_string())?;
        let serialized_memory_derivation =
            serde_json::to_vec(memory_derivation).map_err(|e| e.to_string())?;
        let serialized_memory = serde_json::to_vec(memory).map_err(|e| e.to_string())?;
        let serialized_backend = serde_json::to_vec(backend).map_err(|e| e.to_string())?;
        let serialized_backend_contracts =
            serde_json::to_vec(&backend_contract_identity(contracts.backend))
                .map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (COMPOSITION_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            // Two import keys, for the same measured reason the memory route
            // records: `for_derived_text` erases a declared file, so a derived
            // identity cannot stand in for the base import.
            (
                FS_CLOSE_BASE_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_base),
            ),
            (
                FS_CLOSE_DERIVED_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_derived),
            ),
            // Two derivations, two keys: one recipe can move without the other.
            (
                FS_CLOSE_DERIVATION_KEY.to_owned(),
                sha256_hex(&serialized_derivation),
            ),
            (
                FS_CLOSE_MEMORY_DERIVATION_KEY.to_owned(),
                sha256_hex(&serialized_memory_derivation),
            ),
            (
                FS_CLOSE_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.contract.as_bytes()),
            ),
            (
                FS_CLOSE_MEMORY_QUESTION_KEY.to_owned(),
                sha256_hex(&serialized_memory),
            ),
            (
                FS_CLOSE_BACKEND_QUESTION_KEY.to_owned(),
                sha256_hex(&serialized_backend),
            ),
            // A composition binds every contract it composes under. The memory
            // prerequisite's contracts are already bound inside its own task
            // digest, which this task binds through its identity; the backend
            // selection this route rechecks under is bound here explicitly.
            (
                FS_CLOSE_BACKEND_CONTRACTS_KEY.to_owned(),
                sha256_hex(&serialized_backend_contracts),
            ),
            // Two distinct keys, so a dependent whose *either* prerequisite
            // moved is a different task and one key cannot absorb the other.
            (
                FilesystemChunkClosePrerequisites::MEMORY_CLOSE_KEY.to_owned(),
                claimed.memory_close.digest(),
            ),
            (
                FilesystemChunkClosePrerequisites::FILE_CLOSE_DELETE_KEY.to_owned(),
                claimed.file_close_delete.digest(),
            ),
        ]);
        // This route's own premises, over and above what its components already
        // assume. The last is the card's own: the backend reading is taken from
        // a composition checked over a **different** translation unit, tied by
        // identity, revision and prototype rather than by text.
        let own = BTreeMap::from([
            ("libc_free_bindings_are_declared_premises".into(), true),
            (
                "derived_probe_text_preprocesses_as_the_base_unit_does".into(),
                true,
            ),
            (
                "filesystem_discriminant_is_read_from_a_source_probe".into(),
                true,
            ),
            (
                "backend_prerequisite_is_linked_from_another_translation_unit".into(),
                true,
            ),
        ]);
        let mut sets = prerequisites.premise_sets();
        sets.push(&own);
        let assumptions = composition_premise_union(&sets)?;
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
                // The wrapper's own unit, not the text either recipe made.
                source_digest: import.base_identity().raw_source_digest().into(),
                // Both of this family's identities, because both are
                // load-bearing here.
                build_digest: sha256_hex(
                    &serde_json::to_vec(&(&serialized_base, &serialized_derived))
                        .map_err(|e| e.to_string())?,
                ),
            },
            target,
            semantic_profile: FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::CompositionContract;
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
            contracts: OwnedFilesystemChunkCloseContracts::of(contracts),
            prerequisites,
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    /// This family's own derivation, as the core's importer measured it.
    pub fn derivation(&self) -> &qcore_schemas::composition_summary::DerivedUnitRecord {
        self.import.derivation()
    }

    /// The probe body this task imported, from its own derived unit.
    pub fn probe_summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.probe_summary()
    }

    /// How many composition levels this task re-establishes, counting itself.
    /// Three: this family, its deleting `file_close` prerequisite, and that
    /// composition's own nested unmap composition.
    #[must_use]
    pub fn nesting_depth(&self) -> u32 {
        self.import.source_input().family.nesting_depth()
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("composition:{}", self.task.task_digest()),
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
            cleanup_replay_source: None,
            composition_source: Some(self.import.source_input()),
        }
    }

    /// Re-establish both composition prerequisites, at every level. Each
    /// re-establishes its own prerequisites in turn — four for the memory half,
    /// three plus a nested composition for the backend half — and the core does
    /// every level itself. Nothing a provider sends reaches any level.
    pub fn revalidate_prerequisites(
        &self,
    ) -> Result<(MemoryCloseCertificate, Box<FileCloseDeletionEvidence>), String> {
        let input = self.import.source_input();
        let claimed = input
            .prerequisites
            .filesystem_chunk_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        let memory = self
            .prerequisites
            .memory_close
            .revalidate_as_prerequisite(&claimed.memory_close)?;
        let backend = self
            .prerequisites
            .file_close_delete
            .revalidate_as_prerequisite(&claimed.file_close_delete)?;
        Ok((memory, backend))
    }

    /// Re-establish every prerequisite at every level and re-run the unedited
    /// composition over the core's own imports and its own contracts. This is
    /// the core's own evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<FilesystemChunkCloseEvidence, String> {
        let (memory, backend) = self.revalidate_prerequisites()?;
        check_filesystem_chunk_close_bodies(
            &memory,
            self.import.probe_summary(),
            &backend,
            &self.contracts.borrowed(),
        )
    }

    /// This task's own identity, for a *dependent* capability to name it by.
    /// `unsupported_close` is the one level above this route.
    pub fn prerequisite_identity(&self) -> PrerequisiteIdentity {
        PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// The question this context imported, for a *dependent* to carry verbatim.
    pub fn family_input(&self) -> CompositionFamilyInput {
        self.import.source_input().family
    }

    /// Re-run this whole three-level composition for a dependent and confirm the
    /// claimed identity is the one this context derives.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<FilesystemChunkCloseEvidence, String> {
        if !claimed.is_complete() {
            return Err("filesystem chunk close prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err(
                "filesystem chunk close prerequisite identity is not the one the core derives"
                    .into(),
            );
        }
        self.evaluate()
    }

    /// The composed reading the core reads out of its own checked certificate.
    fn record(
        &self,
        evidence: &FilesystemChunkCloseEvidence,
    ) -> Result<CompositionProposalRecord, String> {
        let index =
            |value: usize| u32::try_from(value).map_err(|_| "index out of range".to_owned());
        let residual = |value: ResidualCustody| match value {
            ResidualCustody::Absent => ResidualCustodyRecord::Absent,
            ResidualCustody::Unresolved => ResidualCustodyRecord::Unresolved,
        };
        let outcome = |value: DeletionOutcome| match value {
            DeletionOutcome::NoBackend => DeletionOutcomeRecord::NoBackend,
            DeletionOutcome::BlockedByFields => DeletionOutcomeRecord::BlockedByFields,
            DeletionOutcome::SyscallFailed => DeletionOutcomeRecord::SyscallFailed,
            DeletionOutcome::SyscallSucceeded => DeletionOutcomeRecord::SyscallSucceeded,
        };
        let mut cases = Vec::new();
        for case in evidence.cases() {
            cases.push(FilesystemChunkCloseCaseRecord {
                source_path: index(case.source_path)?,
                chunk_present: case.chunk_present,
                delete: case.delete,
                backend_case: case.backend_case.map(index).transpose()?,
                backend_released: case.backend_released,
                path_released: case.path_released,
                mapping: residual(case.mapping),
                descriptor: residual(case.descriptor),
                deletion: case.deletion.map(outcome),
                primary_unlinked: case.primary_unlinked,
                state_unlinked: case.state_unlinked,
                name_released: case.name_released,
                chunk_released: case.chunk_released,
                total_counter_decrements: index(case.total_counter_decrements)?,
                up_counter_decrements: index(case.up_counter_decrements)?,
            });
        }
        let source = evidence.memory_scope().source();
        Ok(CompositionProposalRecord::FilesystemChunkClose(
            FilesystemChunkCloseCompositionRecord {
                function: source.function().into(),
                source_revision: source.source_revision().into(),
                signature: source.signature().into(),
                filesystem_kind: evidence.filesystem_kind()?,
                memory_kind: evidence.memory_scope().memory_kind()?,
                cases,
                checked_paths: evidence
                    .checked_paths()
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<Vec<_>, _>>()?,
                uncovered_paths: evidence
                    .uncovered_paths()
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<Vec<_>, _>>()?,
                combined_uncovered_paths: evidence
                    .combined_uncovered_paths()
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<Vec<_>, _>>()?,
                profile_assumptions: source.profile_assumptions().clone(),
            },
        ))
    }
}

/// A stable identity for the backend prerequisite's pinned contract selection,
/// so a moved backend contract, policy-derived profile or eligibility field
/// retires this task even though none of them is in either nested question.
fn backend_contract_identity(
    contracts: &crate::filesystem_chunk_close_composition::OwnedBackendContracts,
) -> Vec<String> {
    let profile = |profile: &CleanupProfile| {
        serde_json::to_string(profile).unwrap_or_else(|_| "unserializable".into())
    };
    vec![
        contracts.pinned_revision.clone(),
        contracts.contract.clone(),
        contracts.delete_contract.clone(),
        contracts.close_function.clone(),
        profile(&contracts.close_profile),
        contracts.close_contract.clone(),
        contracts.delete_function.clone(),
        profile(&contracts.delete_profile),
        contracts.delete_mapping_field.clone(),
        contracts.delete_prerequisite_contract.clone(),
        contracts.unmap_native_function.clone(),
        profile(&contracts.unmap_native_profile),
        contracts.unmap_native_contract.clone(),
        contracts.unmap_counter_contract.clone(),
        contracts.unmap_counter_root_c_type.clone(),
        contracts.unmap_counter_root_qcore_type.clone(),
        contracts.unmap_contract.clone(),
    ]
}

/// Identity and target checks for the filesystem route. The dialect is compared
/// against **this family's** derived unit's identity, because that is the unit
/// this family's own body was preprocessed and parsed from.
fn check_filesystem_close_target(
    target: &TargetProfile,
    import_revision: &str,
    identity: &SourceInputIdentity,
    translation_unit: &str,
    contracts: &FilesystemChunkCloseContracts<'_>,
) -> Result<(), String> {
    if translation_unit.is_empty()
        || contracts.contract.is_empty()
        || contracts.probe_recipe.is_empty()
        || contracts.probe_function.is_empty()
        || contracts.pinned_revision.is_empty()
        || contracts.backend.contract.is_empty()
        || contracts.backend.delete_contract.is_empty()
        || contracts.backend.close_contract.is_empty()
        || contracts.backend.delete_prerequisite_contract.is_empty()
        || contracts.backend.unmap_contract.is_empty()
        || contracts.backend.unmap_native_contract.is_empty()
        || contracts.backend.unmap_counter_contract.is_empty()
        || contracts.backend.delete_mapping_field.is_empty()
        || target.library.is_empty()
        || target.profile != FILESYSTEM_CHUNK_CLOSE_SEMANTIC_PROFILE
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible filesystem close target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native composition platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the composition route imports no helper bodies".into());
    }
    Ok(())
}

impl WitnessChecker for FilesystemChunkCloseCompositionSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("composition task differs from its checked import context".into());
        }
        let SemanticWitness::Composition(proposed) = witness else {
            return Err("expected composition witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("composition witness is bound to a different task".into());
        }
        // Both prerequisite compositions are re-established, at every level, and
        // this composition re-run by the core before the proposal is looked at.
        let evidence = self.evaluate()?;
        if proposed.summary != self.record(&evidence)? {
            return Err("composition witness differs from the checked source composition".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}

// ---------------------------------------------------------------------------
// The symbolic other-backend composition: one prerequisite, no body of its own,
// and a reading over a class rather than a value.
// ---------------------------------------------------------------------------

/// Digest keys the symbolic route binds beyond the shared input key.
pub const UNSUPPORTED_CLOSE_BASE_IMPORT_KEY: &str = "qcore:composition-unsupported-base-import:v1";
/// This composition's own pinned contract.
pub const UNSUPPORTED_CLOSE_CONTRACT_KEY: &str = "qcore:composition-unsupported-contract:v1";
/// The rest of the core's own selection — the pinned revision and the
/// filesystem probe's function name — which the contract's bytes do not carry
/// and which decide which body the classification will read at all.
pub const UNSUPPORTED_CLOSE_SELECTION_KEY: &str = "qcore:composition-unsupported-selection:v1";
/// The nested filesystem question, verbatim. It carries both derivation records
/// and all three lower questions, so a moved recipe at any level moves this
/// digest; a test pins that in both directions rather than assuming it.
pub const UNSUPPORTED_CLOSE_QUESTION_KEY: &str = "qcore:composition-unsupported-fs-question:v1";

/// The symbolic route's one prerequisite context, which is itself a three-level
/// composition. Re-establishing it re-establishes every level below.
pub struct UnsupportedClosePrerequisiteTasks {
    pub filesystem_chunk_close: FilesystemChunkCloseCompositionSourceTask,
}

impl UnsupportedClosePrerequisiteTasks {
    fn identities(&self) -> UnsupportedClosePrerequisites {
        UnsupportedClosePrerequisites {
            filesystem_chunk_close: self.filesystem_chunk_close.prerequisite_identity(),
        }
    }
}

/// Sealed symbolic other-backend composition context. A raw wire projection
/// cannot construct one.
pub struct UnsupportedCloseCompositionSourceTask {
    import: UnsupportedCloseSourceImport,
    contracts: OwnedUnsupportedCloseContracts,
    prerequisites: UnsupportedClosePrerequisiteTasks,
    task: VerificationTask,
}

impl UnsupportedCloseCompositionSourceTask {
    /// Build the task. `prerequisites` is the core's **own** filesystem
    /// composition context; the projection's claimed identity is required to be
    /// the one it derives, and the transported nested question is required to be
    /// the one that context imported.
    pub fn derive(
        import: &UnsupportedCloseSourceImport,
        prerequisites: UnsupportedClosePrerequisiteTasks,
        translation_unit: &str,
        contracts: &UnsupportedCloseContracts<'_>,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_unsupported_close_target(
            &target,
            import.source_revision(),
            import.base_identity(),
            translation_unit,
            contracts,
        )?;
        let input = import.source_input();
        if !input.version_matches() {
            return Err("unsupported composition projection version".into());
        }
        if input.family.semantic_profile() != UNSUPPORTED_CLOSE_SEMANTIC_PROFILE {
            return Err("the transported question answers another semantic profile".into());
        }
        if !input.family.matches(&input.prerequisites) {
            return Err("composition family and prerequisite set disagree".into());
        }
        let CompositionFamilyInput::UnsupportedClose { filesystem } = &input.family else {
            return Err("this context checks the symbolic other-backend composition only".into());
        };
        // 1. The transported nested *question* must be, verbatim, the question
        //    the core's own filesystem context imported. That one comparison
        //    carries both derivation records and all three lower questions,
        //    because they are inside it.
        if **filesystem != prerequisites.filesystem_chunk_close.family_input() {
            return Err(
                "the transported filesystem question is not the one the filesystem composition \
                 imported"
                    .into(),
            );
        }
        // 2. The claimed identity is a lookup key, checked against the identity
        //    the core's own prerequisite context derives.
        let claimed = input
            .prerequisites
            .unsupported_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        if !claimed.is_complete() {
            return Err("unsupported close prerequisite identity is incomplete".into());
        }
        if claimed != prerequisites.identities() {
            return Err(
                "unsupported close prerequisite identity is not the one the core derives".into(),
            );
        }
        // 3. The single-unit tie. Unlike the filesystem family, this route's own
        //    prerequisite *is* over the wrapper's own text, so the tie is an
        //    equality and is pinned in both directions. The cross-unit premise
        //    the filesystem half carries about its backend arrives here through
        //    the premise union rather than being restated.
        if prerequisites
            .filesystem_chunk_close
            .task()
            .unit
            .source_digest
            != import.base_identity().raw_source_digest()
        {
            return Err(
                "the filesystem prerequisite was checked over a different unit than this one"
                    .into(),
            );
        }
        let paths = u64::try_from(import.wrapper_record().paths.len())
            .map_err(|_| "composition path count out of range")?;
        if paths == 0 {
            return Err("composition import has no complete wrapper path".into());
        }
        let serialized_base =
            serde_json::to_vec(import.base_identity()).map_err(|e| e.to_string())?;
        let serialized_question =
            serde_json::to_vec(import.nested_question()).map_err(|e| e.to_string())?;
        let serialized_selection =
            serde_json::to_vec(&(contracts.pinned_revision, contracts.probe_function))
                .map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (COMPOSITION_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                UNSUPPORTED_CLOSE_BASE_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_base),
            ),
            (
                UNSUPPORTED_CLOSE_CONTRACT_KEY.to_owned(),
                sha256_hex(contracts.contract.as_bytes()),
            ),
            (
                UNSUPPORTED_CLOSE_SELECTION_KEY.to_owned(),
                sha256_hex(&serialized_selection),
            ),
            (
                UNSUPPORTED_CLOSE_QUESTION_KEY.to_owned(),
                sha256_hex(&serialized_question),
            ),
            (
                UnsupportedClosePrerequisites::FILESYSTEM_CHUNK_CLOSE_KEY.to_owned(),
                claimed.filesystem_chunk_close.digest(),
            ),
        ]);
        // This route's own premises, over and above what its one component
        // already assumes. Both are about the symbolic class, and both are
        // assumptions rather than facts this route establishes.
        let own = BTreeMap::from([
            (
                "symbolic_backend_class_excludes_exactly_the_two_source_probed_constants".into(),
                true,
            ),
            (
                "int_backend_domain_is_exhausted_by_the_two_constants_and_their_complement".into(),
                true,
            ),
        ]);
        let sets = vec![
            &prerequisites.filesystem_chunk_close.task().assumptions,
            &own,
        ];
        let assumptions = composition_premise_union(&sets)?;
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
                source_digest: import.base_identity().raw_source_digest().into(),
                // This family derives no text, so its build identity is the base
                // import's alone. The two derived identities its reading depends
                // on are bound through the nested question's digest.
                build_digest: sha256_hex(&serialized_base),
            },
            target,
            semantic_profile: UNSUPPORTED_CLOSE_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::CompositionContract;
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
            contracts: OwnedUnsupportedCloseContracts::of(contracts),
            prerequisites,
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    /// How many composition levels this task re-establishes, counting itself.
    /// Four: this family, its filesystem prerequisite, that composition's own
    /// deleting `file_close` prerequisite, and that composition's nested unmap
    /// composition. It is the deepest question this vocabulary carries.
    #[must_use]
    pub fn nesting_depth(&self) -> u32 {
        self.import.source_input().family.nesting_depth()
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("composition:{}", self.task.task_digest()),
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
            cleanup_replay_source: None,
            composition_source: Some(self.import.source_input()),
        }
    }

    /// Re-establish the one composition prerequisite, which re-establishes the
    /// three levels below it. Nothing a provider sends reaches any level.
    pub fn revalidate_prerequisites(&self) -> Result<FilesystemChunkCloseEvidence, String> {
        let input = self.import.source_input();
        let claimed = input
            .prerequisites
            .unsupported_close()
            .ok_or("composition projection names another family's prerequisite set")?
            .clone();
        self.prerequisites
            .filesystem_chunk_close
            .revalidate_as_prerequisite(&claimed.filesystem_chunk_close)
    }

    /// Re-establish every prerequisite at every level and re-run the unedited
    /// symbolic classification over the core's own certificates and its own
    /// contract. This is the core's own evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<UnsupportedCloseEvidence, String> {
        let filesystem = self.revalidate_prerequisites()?;
        check_unsupported_close_bodies(
            filesystem.memory_scope(),
            filesystem.filesystem_kind_probe(),
            &self.contracts.borrowed(),
        )
    }

    /// This task's own identity, so a further dependent could name it. There is
    /// none today; the accessor exists so the ladder's top rung is uniform.
    pub fn prerequisite_identity(&self) -> PrerequisiteIdentity {
        PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// The composed reading the core reads out of its own checked certificate.
    ///
    /// Two figures and the wrapper's identity, because that is what the
    /// certificate decides. The six release facts are constants of the
    /// admission, not decided figures, and the `Unknown` status is a constant of
    /// the arm, so none of them is published as agreement.
    fn record(
        &self,
        evidence: &UnsupportedCloseEvidence,
    ) -> Result<CompositionProposalRecord, String> {
        let source = evidence.memory_scope().source();
        let [memory_kind, filesystem_kind] = evidence.excluded_kinds();
        Ok(CompositionProposalRecord::UnsupportedClose(
            UnsupportedCloseCompositionRecord {
                function: source.function().into(),
                source_revision: source.source_revision().into(),
                signature: source.signature().into(),
                memory_kind,
                filesystem_kind,
                source_path: u32::try_from(evidence.source_path())
                    .map_err(|_| "index out of range".to_owned())?,
                profile_assumptions: source.profile_assumptions().clone(),
            },
        ))
    }
}

/// Identity and target checks for the symbolic route. The dialect is compared
/// against the **base** unit's identity, because this family imports no derived
/// text of its own; the two derived identities its reading depends on are the
/// nested question's, and that question is bound verbatim.
fn check_unsupported_close_target(
    target: &TargetProfile,
    import_revision: &str,
    identity: &SourceInputIdentity,
    translation_unit: &str,
    contracts: &UnsupportedCloseContracts<'_>,
) -> Result<(), String> {
    if translation_unit.is_empty()
        || contracts.contract.is_empty()
        || contracts.probe_function.is_empty()
        || contracts.pinned_revision.is_empty()
        || target.library.is_empty()
        || target.profile != UNSUPPORTED_CLOSE_SEMANTIC_PROFILE
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.revision != contracts.pinned_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible unsupported close target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native composition platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the composition route imports no helper bodies".into());
    }
    Ok(())
}

impl WitnessChecker for UnsupportedCloseCompositionSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("composition task differs from its checked import context".into());
        }
        let SemanticWitness::Composition(proposed) = witness else {
            return Err("expected composition witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("composition witness is bound to a different task".into());
        }
        // The whole four-level stack is re-established and this classification
        // re-run by the core before the proposal is looked at. An admitted
        // proposal still publishes `Unknown`: agreement cannot promote an arm
        // whose conclusion is a withheld one.
        let evidence = self.evaluate()?;
        if proposed.summary != self.record(&evidence)? {
            return Err("composition witness differs from the checked source composition".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}

/// The witness envelope a provider's proposal is read out of, kept beside the
/// checker so both sides name one type.
pub type ProposedCompositionWitness = CompositionWitness;

/// The transported projection type, re-exported for callers that build one.
pub type CompositionProjection = CompositionSourceInput;

/// The prerequisite identity type, re-exported for the same reason.
pub type CompositionPrerequisiteIdentity = PrerequisiteIdentity;
