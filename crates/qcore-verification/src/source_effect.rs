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

//! Function-scoped source-derived Boolean-field contract evidence.
//!
//! Only an opaque typed-C import can construct this replay context. Task identity
//! binds the actual input/preprocessing/build/profile/policy/helper snapshots.
//! Live/exclusive/Boolean-domain caller conditions remain explicit assumptions;
//! this is not a general custody, lifecycle, synchronization or whole-unit claim.

use qcore_c_gateway::boolean_source::ChunkSourceSummary;
use qcore_contract::chunk_contract::{ChunkContractEvaluation, evaluate_chunk_source_contract};
use qcore_schemas::chunk_summary::BooleanFieldResult;
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;
use std::collections::{BTreeMap, BTreeSet};

pub const BOOLEAN_FIELD_SEMANTIC_PROFILE: &str = "qcore.c_boolean_field_contract.v2";

pub const BOOLEAN_FIELD_UNIT_SEMANTIC_PROFILE: &str = "qcore.c_boolean_field_contract.v3";

pub fn semantic_profile_for(summary: &ChunkSourceSummary) -> &'static str {
    match summary.result_kind {
        BooleanFieldResult::I32 => BOOLEAN_FIELD_SEMANTIC_PROFILE,
        BooleanFieldResult::Unit => BOOLEAN_FIELD_UNIT_SEMANTIC_PROFILE,
    }
}

/// Sealed replay context. A raw wire summary cannot construct one.
pub struct BooleanFieldSourceTask {
    summary: ChunkSourceSummary,
    contract_source: String,
    task: VerificationTask,
}

impl BooleanFieldSourceTask {
    pub fn derive(
        summary: &ChunkSourceSummary,
        translation_unit: &str,
        contract_source: &str,
        target: TargetProfile,
    ) -> Result<Self, String> {
        if translation_unit.is_empty()
            || contract_source.is_empty()
            || target.profile != semantic_profile_for(summary)
            || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
            || target.revision != summary.source_revision()
            || summary.input_identity().model_library() != Some(target.library.as_str())
            || target.dialect != summary.input_identity().dialect()
            || target.platform != std::env::consts::OS
            || target.backend != "native"
        {
            return Err("incomplete or incompatible source-effect target identity".into());
        }
        let target_os = match std::env::consts::OS {
            "macos" => "darwin",
            "windows" => "windows",
            other => other,
        };
        if !summary
            .input_identity()
            .compiler_target()
            .contains(target_os)
        {
            return Err("Clang target does not match the native source-effect platform".into());
        }
        fn compatible_helpers(
            identity: &qcore_c_gateway::boolean_source::BooleanSourceIdentity,
            target: &str,
        ) -> bool {
            identity.helpers().values().all(|helper| {
                helper.compiler_target() == target && compatible_helpers(helper, target)
            })
        }
        if !compatible_helpers(
            summary.input_identity(),
            summary.input_identity().compiler_target(),
        ) {
            return Err(
                "source-effect helper compiler target differs from the importing unit".into(),
            );
        }
        let input = summary.source_input();
        let identity = summary.input_identity();
        let serialized_identity = serde_json::to_vec(identity).map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (input.digest_key().into(), input.digest()),
            (
                "qcore:boolean-field-import:v1".into(),
                sha256_hex(&serialized_identity),
            ),
            (
                "qcore:boolean-field-contract:v1".into(),
                sha256_hex(contract_source.as_bytes()),
            ),
        ]);
        let mut assumptions = BTreeMap::from([
            ("borrowed_parameter_is_live".into(), true),
            ("borrowed_parameter_is_exclusive".into(), true),
            ("busy_is_boolean".into(), true),
            ("lock_is_boolean".into(), true),
            (
                "declared_source_revision_is_outer_audit_assumption".into(),
                true,
            ),
            ("clang_typed_import_is_trusted".into(), true),
            ("source_projection_policy_is_declared_trusted".into(), true),
            ("field_mapping_profile_is_declared_trusted".into(), true),
        ]);
        if summary.result_kind == BooleanFieldResult::Unit {
            assumptions.remove("busy_is_boolean");
            assumptions.remove("lock_is_boolean");
            assumptions.insert("field_is_boolean".into(), true);
        }
        assumptions.extend(
            qcore_contract::boolean_condition_profile::entry_assumptions(
                contract_source,
                summary.function.as_str(),
                summary.result_kind == BooleanFieldResult::Unit,
            )?,
        );
        let bounds = TaskBounds {
            path_exploration: PathBounds::BoundedPaths {
                paths: if summary.result_kind == BooleanFieldResult::Unit {
                    2
                } else {
                    4
                },
            },
            schedules_explored: None,
            observed_runs_only: false,
            max_operations: None,
        };
        let mut task = VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                translation_unit: translation_unit.into(),
                source_digest: identity.raw_source_digest().into(),
                build_digest: sha256_hex(&serialized_identity),
            },
            target,
            semantic_profile: semantic_profile_for(summary).into(),
            contract_sources,
            bounds: bounds.clone(),
            assumptions: assumptions.clone(),
            evidence_version: 2,
            obligations: Vec::new(),
            derivation_incomplete: Vec::new(),
        };
        let subject = ObligationSubject::Function {
            symbol: summary.function().into(),
            translation_unit: translation_unit.into(),
        };
        let property = ObligationProperty::BooleanFieldContract;
        let capability = VerifierCapability::ResourceContracts;
        task.obligations.push(Obligation {
            obligation_id: task.obligation_id(capability, property, &subject),
            capability,
            property,
            subject,
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([summary.function().into()]),
            },
            bounds,
            assumptions,
            evidence_version: 2,
        });
        task.validate().map_err(|e| e.to_string())?;
        Ok(Self {
            summary: summary.clone(),
            contract_source: contract_source.into(),
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    pub fn request(&self, plugin_id: PluginId) -> VerificationRequestV2 {
        VerificationRequestV2 {
            protocol_version: 1,
            evidence_version: 2,
            request_id: format!("boolean-field:{}", self.task.task_digest()),
            plugin_id,
            capability: VerifierCapability::ResourceContracts,
            target: self.task.target.clone(),
            unit: self.task.unit.clone(),
            facts: Vec::new(),
            assumptions: self.task.assumptions.clone(),
            task_digest: self.task.task_digest(),
            obligations: self.task.obligations.clone(),
            source_effect: Some(self.summary.source_input()),
            counter_effect: None,
            reactor_program: None,
            teardown_source: None,
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: None,
        }
    }

    /// Re-evaluate both source prestates with both unchanged companion-field values.
    pub fn evaluate(&self) -> Result<ChunkContractEvaluation, String> {
        evaluate_chunk_source_contract(&self.summary, &self.contract_source)
    }
}

impl WitnessChecker for BooleanFieldSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("source-effect task differs from its checked import context".into());
        }
        let SemanticWitness::BooleanFieldContract(proposed) = witness else {
            return Err("expected Boolean-field source contract witness".into());
        };
        if proposed.task_digest != task.task_digest() || proposed.summary != *self.summary {
            return Err(
                "source-effect witness differs from the checked source paths or identity".into(),
            );
        }
        // Equality only binds the proposal to the import. Acceptance terminates
        // in independently evaluating the actual contract over all source states.
        let evaluation = self.evaluate()?;
        if evaluation.checked_states()
            != if self.summary.result_kind == BooleanFieldResult::Unit {
                2
            } else {
                4
            }
            || evaluation.function() != self.summary.function()
        {
            return Err("source-effect replay lacks complete Boolean prestate coverage".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}
