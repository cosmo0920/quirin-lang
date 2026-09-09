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
    COMPOSITION_SOURCE_INPUT_KEY, CompositionProposalRecord, CompositionRecord,
    CompositionSourceInput, CompositionWitness, UNMAP_WRAPPER_SEMANTIC_PROFILE, UnmapCaseRecord,
    UnmapWrapperPrerequisites,
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

impl CompositionPrerequisites {
    fn identities(&self) -> UnmapWrapperPrerequisites {
        UnmapWrapperPrerequisites {
            wrapper_call_use: self.wrapper_call_use.prerequisite_identity(),
            native_cleanup: self.native_cleanup.prerequisite_identity(),
            counter: self.counter.prerequisite_identity(),
        }
    }
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
        let claimed = &import.source_input().prerequisites;
        if !claimed.is_complete() || !claimed.are_distinct() {
            return Err("composition prerequisite identities are incomplete or repeated".into());
        }
        if *claimed != prerequisites.identities() {
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
        let CompositionFamily::UnmapWrapper = import.family();
        let paths = u64::try_from(import.wrapper_summary().paths().len())
            .map_err(|_| "composition path count out of range")?;
        if paths == 0 {
            return Err("composition import has no complete wrapper path".into());
        }
        let input = import.source_input();
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
        let assumptions = BTreeMap::from([
            ("record_is_live".into(), true),
            ("exclusive_access".into(), true),
            ("cleanup_callees_are_declared_external".into(), true),
            ("os_cleanup_success_is_not_inferred".into(), true),
            ("sync_and_logging_frames_are_declared_premises".into(), true),
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
        let claimed = self.import.source_input().prerequisites;
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

    /// Re-establish every prerequisite and re-run the unedited composition over
    /// the core's own imports, contracts and contract-derived profile. This is
    /// the core's own evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<UnmapCertificate, String> {
        let (native, counter) = self.revalidate_prerequisites()?;
        check_unmap_composition(
            self.import.wrapper_summary(),
            &native,
            &counter,
            &UnmapCompositionContracts {
                pinned_revision: self.import.source_revision(),
                native_function: self.native_profile.function.as_str(),
                native_profile: &self.native_profile,
                native_contract: &self.native_contract_source,
                counter_contract: &self.counter_contract_source,
                counter_root_c_type: &self.counter_root_c_type,
                counter_root_qcore_type: &self.counter_root_qcore_type,
                contract: &self.contract_source,
            },
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

/// The witness envelope a provider's proposal is read out of, kept beside the
/// checker so both sides name one type.
pub type ProposedCompositionWitness = CompositionWitness;

/// The transported projection type, re-exported for callers that build one.
pub type CompositionProjection = CompositionSourceInput;

/// The prerequisite identity type, re-exported for the same reason.
pub type CompositionPrerequisiteIdentity = PrerequisiteIdentity;
