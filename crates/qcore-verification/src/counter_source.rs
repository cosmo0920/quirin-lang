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

//! Function-scoped source-derived unsigned counter contract evidence.
//!
//! Only the opaque typed-C counter import can construct this replay context.
//! Task identity binds the actual input/preprocessing/build/contract snapshots
//! and the declared root QCore type. Live storage, exclusive access, the
//! unsigned-C/usize ABI binding and the entry range remain explicit declared
//! premises of the pinned contract; this is not a custody, lifecycle,
//! synchronization or whole-unit claim.
//!
//! A provider's proposal is compared with the reading the core computes from
//! its own import, and acceptance still terminates in the core evaluating the
//! pinned contract itself. Agreement alone never discharges an obligation.

use qcore_c_gateway::counter_source::CounterSourceImport;
use qcore_schemas::counter_summary::{COUNTER_SOURCE_INPUT_KEY, CounterUpdateRecord};
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;
use std::collections::{BTreeMap, BTreeSet};

use crate::counter_contract::{CounterCertificate, check_counter_contract};

pub const COUNTER_SEMANTIC_PROFILE: &str = "qcore.c_counter_contract.v1";

/// Sealed replay context. A raw wire projection cannot construct one.
pub struct CounterSourceTask {
    import: CounterSourceImport,
    contract_source: String,
    root_qcore_type: String,
    task: VerificationTask,
}

impl CounterSourceTask {
    pub fn derive(
        import: &CounterSourceImport,
        translation_unit: &str,
        contract_source: &str,
        root_qcore_type: &str,
        target: TargetProfile,
    ) -> Result<Self, String> {
        if translation_unit.is_empty()
            || contract_source.is_empty()
            || root_qcore_type.is_empty()
            || target.library.is_empty()
            || target.profile != COUNTER_SEMANTIC_PROFILE
            || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
            || target.revision != import.source_revision()
            || target.dialect != import.input_identity().dialect()
            || target.platform != std::env::consts::OS
            || target.backend != "native"
        {
            return Err("incomplete or incompatible counter target identity".into());
        }
        let target_os = match std::env::consts::OS {
            "macos" => "darwin",
            "windows" => "windows",
            other => other,
        };
        if !import
            .input_identity()
            .compiler_target()
            .contains(target_os)
        {
            return Err("Clang target does not match the native counter platform".into());
        }
        if !import.input_identity().helpers().is_empty() {
            return Err("the counter route imports no helper bodies".into());
        }
        let input = import.source_input();
        let serialized_identity =
            serde_json::to_vec(import.input_identity()).map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (COUNTER_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                "qcore:counter-import:v1".into(),
                sha256_hex(&serialized_identity),
            ),
            (
                "qcore:counter-contract:v1".into(),
                sha256_hex(contract_source.as_bytes()),
            ),
            // The declared QCore root type selects which contract declaration
            // family may bind this import; it is part of the checked question.
            (
                "qcore:counter-root-type:v1".into(),
                sha256_hex(root_qcore_type.as_bytes()),
            ),
        ]);
        let assumptions = BTreeMap::from([
            ("counter_storage_is_live".into(), true),
            ("counter_access_is_exclusive".into(), true),
            ("c_unsigned_matches_usize".into(), true),
            ("counter_entry_value_is_in_range".into(), true),
            ("clang_typed_import_is_trusted".into(), true),
            ("source_projection_policy_is_declared_trusted".into(), true),
            (
                "declared_source_revision_is_outer_audit_assumption".into(),
                true,
            ),
        ]);
        let bounds = TaskBounds::single_path();
        let mut task = VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                translation_unit: translation_unit.into(),
                source_digest: import.input_identity().raw_source_digest().into(),
                build_digest: sha256_hex(&serialized_identity),
            },
            target,
            semantic_profile: COUNTER_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::CounterUpdateContract;
        let capability = VerifierCapability::ResourceContracts;
        task.obligations.push(Obligation {
            obligation_id: task.obligation_id(capability, property, &subject),
            capability,
            property,
            subject,
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([import.function().into()]),
            },
            bounds,
            assumptions,
            evidence_version: 2,
        });
        task.validate().map_err(|e| e.to_string())?;
        Ok(Self {
            import: import.clone(),
            contract_source: contract_source.into(),
            root_qcore_type: root_qcore_type.into(),
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
            request_id: format!("counter-contract:{}", self.task.task_digest()),
            plugin_id,
            capability: VerifierCapability::ResourceContracts,
            target: self.task.target.clone(),
            unit: self.task.unit.clone(),
            facts: Vec::new(),
            assumptions: self.task.assumptions.clone(),
            task_digest: self.task.task_digest(),
            obligations: self.task.obligations.clone(),
            source_effect: None,
            counter_effect: Some(self.import.source_input()),
            reactor_program: None,
            teardown_source: None,
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: None,
        }
    }

    /// Re-check the imported path against the pinned contract. This is the
    /// core's own evaluation; no provider input reaches it.
    /// This task's own identity, for a *dependent* capability to name it by.
    /// It carries no verdict, certificate or reading — only the semantic
    /// profile, the task digest and the single obligation identifier, each of
    /// which the dependent's core re-derives for itself.
    pub fn prerequisite_identity(&self) -> qcore_schemas::call_use_summary::PrerequisiteIdentity {
        qcore_schemas::call_use_summary::PrerequisiteIdentity {
            semantic_profile: self.task.semantic_profile.clone(),
            task_digest: self.task.task_digest(),
            obligation_id: self.task.obligations[0].obligation_id.clone(),
        }
    }

    /// Re-run this task's own evaluation for a dependent and confirm the
    /// claimed identity is the one this context derives.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &qcore_schemas::call_use_summary::PrerequisiteIdentity,
    ) -> Result<CounterCertificate, String> {
        if !claimed.is_complete() {
            return Err("counter prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err("counter prerequisite identity is not the one the core derives".into());
        }
        self.evaluate()
    }

    /// The typed summary this task imported, so a dependent can require that
    /// the body it composes over is the one this prerequisite checked.
    pub fn summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.summary()
    }

    pub fn evaluate(&self) -> Result<CounterCertificate, String> {
        check_counter_contract(
            self.import.summary(),
            &self.contract_source,
            self.import.root_c_type(),
            &self.root_qcore_type,
        )
    }

    /// The reading the core computes from its own checked certificate. Both
    /// structural flags are true by construction: `check_counter_contract`
    /// refuses a path whose result is not the updated counter, or that writes
    /// anything besides the selected field.
    fn record(&self, certificate: &CounterCertificate) -> CounterUpdateRecord {
        CounterUpdateRecord {
            function: certificate.source().function().into(),
            source_revision: certificate.source().source_revision().into(),
            root_c_type: certificate.root_c_type().into(),
            counter_c_type: certificate.counter_c_type().into(),
            field: certificate.field().into(),
            delta: certificate.delta(),
            result_is_updated_counter: true,
            only_selected_field_written: true,
            profile_assumptions: certificate.source().profile_assumptions().clone(),
        }
    }
}

impl WitnessChecker for CounterSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("counter task differs from its checked import context".into());
        }
        let SemanticWitness::CounterContract(proposed) = witness else {
            return Err("expected counter contract witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("counter witness is bound to a different task".into());
        }
        // Acceptance terminates in the core's own contract evaluation over its
        // own import; the proposal is then required to be that same reading.
        let certificate = self.evaluate()?;
        if proposed.summary != self.record(&certificate) {
            return Err("counter witness differs from the checked source update".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}
