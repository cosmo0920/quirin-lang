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
use qcore_c_gateway::composition_source::{CompositionFamily, CompositionSourceImport};
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::composition_summary::{
    COMPOSITION_SOURCE_INPUT_KEY, CompositionFamilyInput, CompositionProposalRecord,
    CompositionRecord, CompositionSourceInput, CompositionWitness, DeletingCloseCaseRecord,
    DeletionOutcomeRecord, FILE_CLOSE_DELETE_SEMANTIC_PROFILE, FILE_CLOSE_SEMANTIC_PROFILE,
    FileCloseCaseRecord, FileCloseCompositionRecord, FileClosePrerequisites, ResidualCustodyRecord,
    UNMAP_WRAPPER_SEMANTIC_PROFILE, UnmapCaseRecord, UnmapWrapperPrerequisites,
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
use crate::unmap_composition::{
    UnmapCertificate, UnmapCompositionContracts, check_unmap_composition,
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
            CompositionFamily::UnmapWrapper => {
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
        if *input.family.native_profile() != profile_record(&close_profile) {
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
        let nested = input
            .family
            .nested()
            .ok_or("the file_close question carries no nested composition")?;
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

/// The witness envelope a provider's proposal is read out of, kept beside the
/// checker so both sides name one type.
pub type ProposedCompositionWitness = CompositionWitness;

/// The transported projection type, re-exported for callers that build one.
pub type CompositionProjection = CompositionSourceInput;

/// The prerequisite identity type, re-exported for the same reason.
pub type CompositionPrerequisiteIdentity = PrerequisiteIdentity;
