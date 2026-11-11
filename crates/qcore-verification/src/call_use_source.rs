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

//! Function-scoped source-derived call-result evidence for the bounded ChunkIO
//! filesystem cleanup-attempt family.
//!
//! Only the opaque typed-C call-use import can construct this replay context.
//! Task identity binds the actual input/preprocessing/build snapshots, the
//! pinned contract, the selected policy and the closed callee table the
//! projection was taken under. OS cleanup success, descriptor and mapping
//! custody, and any discharge of a resource remain outside every claim here:
//! a recorded `close` call is a recorded call.
//!
//! A provider's proposal is compared with the def/use reading the core
//! computes from its own import, and acceptance still terminates in the core
//! evaluating the pinned contract itself over its own summary. Agreement alone
//! never discharges an obligation.

use qcore_c_gateway::boolean_source::SourceInputIdentity;
use qcore_c_gateway::call_use_source::FileAttemptSourceImport;
use qcore_schemas::call_use_summary::{
    CALL_USE_SOURCE_INPUT_KEY, CallResultUseRecord, CallUseProposalRecord, CallUseRecord,
    PathCallUseRecord, PrerequisiteIdentity,
};
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;
use std::collections::{BTreeMap, BTreeSet};

use crate::call_use_contract::{
    CallResultUseCertificate, CallResultUsePolicy, check_call_result_use,
};

pub const CALL_RESULT_USE_SEMANTIC_PROFILE: &str = "qcore.c_call_result_use.v1";

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
        || target.profile != CALL_RESULT_USE_SEMANTIC_PROFILE
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible call-use target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native call-use platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the call-use route imports no helper bodies".into());
    }
    Ok(())
}

/// Sealed replay context. A raw wire projection cannot construct one.
pub struct FileAttemptSourceTask {
    import: FileAttemptSourceImport,
    contract_source: String,
    policy: CallResultUsePolicy,
    task: VerificationTask,
}

impl FileAttemptSourceTask {
    pub fn derive(
        import: &FileAttemptSourceImport,
        translation_unit: &str,
        contract_source: &str,
        policy: &CallResultUsePolicy,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_target(
            &target,
            import.source_revision(),
            import.input_identity(),
            translation_unit,
            contract_source,
        )?;
        if policy.policy.is_empty() || policy.first.is_empty() || policy.signature.is_empty() {
            return Err("call-use policy is incomplete".into());
        }
        // The projection's own signature is part of the question: a body whose
        // declared C signature is not the one the policy names is refused
        // before any task exists, exactly as the in-process route refuses it.
        if import.signature() != policy.signature {
            return Err("call-use import signature is not the policy's signature".into());
        }
        // Every callee the policy counts must be admissible in the projection.
        // Otherwise the policy would count zero calls to a callee the closed
        // table could never have projected — a silently smaller question.
        let admitted: BTreeSet<&str> = import
            .admitted_callees()
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        let mut counted = vec![policy.first.as_str()];
        if let Some(second) = &policy.second {
            counted.push(second.as_str());
            counted.push("free");
        }
        if counted.iter().any(|name| !admitted.contains(name)) {
            return Err("the closed callee table omits a callee this policy counts".into());
        }
        let paths = u64::try_from(import.summary().paths().len())
            .map_err(|_| "call-use path count out of range")?;
        if paths == 0 {
            return Err("call-use import has no complete path".into());
        }
        let input = import.source_input();
        let serialized_identity =
            serde_json::to_vec(import.input_identity()).map_err(|e| e.to_string())?;
        let serialized_policy = serde_json::to_vec(policy).map_err(|e| e.to_string())?;
        let serialized_callees =
            serde_json::to_vec(import.admitted_callees()).map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (CALL_USE_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                "qcore:call-use-import:v1".into(),
                sha256_hex(&serialized_identity),
            ),
            (
                "qcore:call-use-contract:v1".into(),
                sha256_hex(contract_source.as_bytes()),
            ),
            // The policy selects which callees the facts count, so it is part
            // of the checked question, not a private implementation detail.
            (
                "qcore:call-use-policy:v1".into(),
                sha256_hex(&serialized_policy),
            ),
            // The closed callee table the projection was taken under. A body
            // projected under a wider or narrower table is a different task.
            (
                "qcore:call-use-admitted-callees:v1".into(),
                sha256_hex(&serialized_callees),
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
            semantic_profile: CALL_RESULT_USE_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::CallResultUseContract;
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
            policy: policy.clone(),
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    /// The identity a dependent capability must bind. It is *derived here*,
    /// from this task, so a dependent that re-derives the prerequisite the
    /// same way gets the same triple and one that is handed a forged triple
    /// gets a different one. Nothing a provider sends can produce it.
    /// The typed summary this task imported, so a dependent can require that
    /// the body it composes over is the one this prerequisite checked.
    pub fn summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.summary()
    }

    pub fn prerequisite_identity(&self) -> PrerequisiteIdentity {
        PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// Re-run the prerequisite's own evaluation and confirm the claimed
    /// identity is the one this context derives. A dependent capability calls
    /// this before it looks at any dependent proposal: the prerequisite is
    /// re-established by the core, never read out of a request.
    pub fn revalidate_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<CallResultUseCertificate, String> {
        if !claimed.is_complete() {
            return Err("prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err("prerequisite identity is not the one the core derives".into());
        }
        self.evaluate()
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("call-result-use:{}", self.task.task_digest()),
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
            call_use_source: Some(self.import.source_input()),
            cleanup_replay_source: None,
            composition_source: None,
        }
    }

    /// Re-check the imported body against the pinned contract. This is the
    /// core's own evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<CallResultUseCertificate, String> {
        check_call_result_use(self.import.summary(), &self.contract_source, &self.policy)
    }

    /// The def/use reading the core computes from its own checked certificate.
    fn record(
        &self,
        certificate: &CallResultUseCertificate,
    ) -> Result<CallResultUseRecord, String> {
        let index =
            |value: usize| u32::try_from(value).map_err(|_| "index out of range".to_owned());
        let mut paths = Vec::new();
        for path in certificate.calls() {
            let mut calls = Vec::new();
            for call in &path.calls {
                calls.push(CallUseRecord {
                    operation: index(call.operation)?,
                    function: call.function.clone(),
                    result_type: call.result_type.clone(),
                    uses: call
                        .uses
                        .iter()
                        .copied()
                        .map(index)
                        .collect::<Result<_, _>>()?,
                    guard_uses: call
                        .guard_uses
                        .iter()
                        .copied()
                        .map(index)
                        .collect::<Result<_, _>>()?,
                });
            }
            paths.push(PathCallUseRecord {
                path: index(path.path)?,
                calls,
            });
        }
        Ok(CallResultUseRecord {
            function: certificate.source().function().into(),
            source_revision: certificate.source().source_revision().into(),
            signature: certificate.source().signature().into(),
            paths,
            profile_assumptions: certificate.source().profile_assumptions().clone(),
        })
    }
}

impl WitnessChecker for FileAttemptSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("call-use task differs from its checked import context".into());
        }
        let SemanticWitness::CallResultUse(proposed) = witness else {
            return Err("expected call-result use witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("call-use witness is bound to a different task".into());
        }
        let CallUseProposalRecord::FilesystemAttempt(proposal) = &proposed.summary;
        // Acceptance terminates in the core's own contract evaluation over its
        // own import; the proposal is then required to be that same reading.
        let certificate = self.evaluate()?;
        if *proposal != self.record(&certificate)? {
            return Err("call-use witness differs from the checked source def/use".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}
