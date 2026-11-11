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

//! Function-scoped source-derived teardown contract evidence for two bounded
//! ChunkIO families: the intrusive-list unlink and the nullable-field backend
//! destructor.
//!
//! Only the opaque typed-C teardown import can construct these replay
//! contexts. Task identity binds the actual input/preprocessing/build/contract
//! snapshots and the declared root or node QCore type. Ring membership,
//! exclusive link access, allocation provenance, field disjointness and the
//! `free`/libc binding remain explicit declared premises of the pinned
//! contracts; this is not a lifecycle, aggregate-teardown or whole-unit claim.
//!
//! A provider's proposal is compared with the reading the core computes from
//! its own import, and acceptance still terminates in the core evaluating the
//! pinned contract itself over its own summaries. Agreement alone never
//! discharges an obligation.

use qcore_c_gateway::boolean_source::SourceInputIdentity;
use qcore_c_gateway::teardown_source::{FieldDestructorSourceImport, ListUnlinkSourceImport};
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::teardown_summary::{
    FieldDestructorRecord, FieldReleaseCaseRecord, LinkWriteRecord, ListUnlinkRecord,
    TEARDOWN_SOURCE_INPUT_KEY, TeardownProposalRecord, UnlinkCaseRecord,
};
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;
use std::collections::{BTreeMap, BTreeSet};

use crate::field_destructor::{
    FieldDestructorCertificate, check_field_destructor, declared_owned_fields,
};
use crate::list_unlink_contract::{ListUnlinkCertificate, check_list_unlink};

pub const LIST_UNLINK_SEMANTIC_PROFILE: &str = "qcore.c_list_unlink_contract.v1";
pub const FIELD_DESTRUCTOR_SEMANTIC_PROFILE: &str = "qcore.c_field_destructor_contract.v1";

/// Identity and target checks both families share. The declared revision is an
/// outer-audit assumption in every route; what is checked here is that the
/// task, the target and the importer's own captured identity agree.
fn check_target(
    target: &TargetProfile,
    profile: &str,
    import_revision: &str,
    identity: &SourceInputIdentity,
    translation_unit: &str,
    contract_source: &str,
    declared_qcore_type: &str,
) -> Result<(), String> {
    if translation_unit.is_empty()
        || contract_source.is_empty()
        || declared_qcore_type.is_empty()
        || target.library.is_empty()
        || target.profile != profile
        || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
        || target.revision != import_revision
        || target.dialect != identity.dialect()
        || target.platform != std::env::consts::OS
        || target.backend != "native"
    {
        return Err("incomplete or incompatible teardown target identity".into());
    }
    let target_os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "windows",
        other => other,
    };
    if !identity.compiler_target().contains(target_os) {
        return Err("Clang target does not match the native teardown platform".into());
    }
    if !identity.helpers().is_empty() {
        return Err("the teardown routes import no helper bodies".into());
    }
    Ok(())
}

fn trust_assumptions() -> [(String, bool); 3] {
    [
        ("clang_typed_import_is_trusted".into(), true),
        ("source_projection_policy_is_declared_trusted".into(), true),
        (
            "declared_source_revision_is_outer_audit_assumption".into(),
            true,
        ),
    ]
}

/// Sealed replay context for the unlink family. A raw wire projection cannot
/// construct one.
pub struct ListUnlinkSourceTask {
    import: ListUnlinkSourceImport,
    contract_source: String,
    node_qcore_type: String,
    task: VerificationTask,
}

impl ListUnlinkSourceTask {
    pub fn derive(
        import: &ListUnlinkSourceImport,
        translation_unit: &str,
        contract_source: &str,
        node_qcore_type: &str,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_target(
            &target,
            LIST_UNLINK_SEMANTIC_PROFILE,
            import.source_revision(),
            import.input_identity(),
            translation_unit,
            contract_source,
            node_qcore_type,
        )?;
        // Both bodies must come from the same declared checkout; a helper from
        // another revision is refused before any task exists.
        if import.entry().source_revision() != import.helper().source_revision()
            || import.entry().function() == import.helper().function()
        {
            return Err("unlink entry and helper disagree on identity or revision".into());
        }
        let input = import.source_input();
        let serialized_identity =
            serde_json::to_vec(import.input_identity()).map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (TEARDOWN_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                "qcore:teardown-import:v1".into(),
                sha256_hex(&serialized_identity),
            ),
            (
                "qcore:list-unlink-contract:v1".into(),
                sha256_hex(contract_source.as_bytes()),
            ),
            // The declared QCore node type selects which contract declaration
            // family may bind this import; it is part of the checked question.
            (
                "qcore:list-unlink-node-type:v1".into(),
                sha256_hex(node_qcore_type.as_bytes()),
            ),
            // The helper's identity is part of the question: the entry's
            // contract is only meaningful against this exact helper body.
            (
                "qcore:list-unlink-helper:v1".into(),
                sha256_hex(import.helper().function().as_bytes()),
            ),
        ]);
        let assumptions = BTreeMap::from([
            ("entry_is_live".into(), true),
            ("exclusive_link_access".into(), true),
            ("entry_has_live_reciprocal_neighbors".into(), true),
            ("entry_is_distinct_from_neighbors".into(), true),
            ("neighbor_link_cells_are_live".into(), true),
            ("no_containing_allocation_is_released".into(), true),
        ])
        .into_iter()
        .chain(trust_assumptions())
        .collect::<BTreeMap<_, _>>();
        let bounds = TaskBounds::single_path();
        let mut task = VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                translation_unit: translation_unit.into(),
                source_digest: import.input_identity().raw_source_digest().into(),
                build_digest: sha256_hex(&serialized_identity),
            },
            target,
            semantic_profile: LIST_UNLINK_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::ListUnlinkContract;
        let capability = VerifierCapability::ResourceContracts;
        task.obligations.push(Obligation {
            obligation_id: task.obligation_id(capability, property, &subject),
            capability,
            property,
            subject,
            // Both bodies are checked, so both are named in the scope.
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([
                    import.function().to_owned(),
                    import.helper().function().to_owned(),
                ]),
            },
            bounds,
            assumptions,
            evidence_version: 2,
        });
        task.validate().map_err(|e| e.to_string())?;
        Ok(Self {
            import: import.clone(),
            contract_source: contract_source.into(),
            node_qcore_type: node_qcore_type.into(),
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
            request_id: format!("list-unlink-contract:{}", self.task.task_digest()),
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
            teardown_source: Some(self.import.source_input()),
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: None,
        }
    }

    /// This task's own identity, for a *dependent* capability to name it by.
    /// It carries no verdict, certificate or reading — only the semantic
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
    /// claimed identity is the one this context derives.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<ListUnlinkCertificate, String> {
        if !claimed.is_complete() {
            return Err("list unlink prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err("list unlink prerequisite identity is not the one the core derives".into());
        }
        self.evaluate()
    }

    /// The entry body this task imported, so a dependent composition can
    /// require that the frame it resolves is the one this prerequisite checked.
    pub fn summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.entry()
    }

    /// Re-check both imported bodies against the pinned contract. This is the
    /// core's own evaluation; no provider input reaches it.
    pub fn evaluate(&self) -> Result<ListUnlinkCertificate, String> {
        check_list_unlink(
            self.import.entry(),
            self.import.helper(),
            &self.contract_source,
            self.import.node_c_type(),
            &self.node_qcore_type,
        )
    }

    /// The reading the core computes from its own checked certificate.
    fn record(&self, certificate: &ListUnlinkCertificate) -> Result<ListUnlinkRecord, String> {
        let mut cases = Vec::new();
        for case in certificate.cases() {
            cases.push(UnlinkCaseRecord {
                neighbors_alias: case.neighbors_alias,
                helper_calls: u32::try_from(case.helper_calls)
                    .map_err(|_| "helper call count out of range")?,
                writes: case
                    .writes
                    .iter()
                    .map(|(node, field, value)| LinkWriteRecord {
                        node: *node,
                        field: field.clone(),
                        value: *value,
                    })
                    .collect(),
            });
        }
        Ok(ListUnlinkRecord {
            function: certificate.source().function().into(),
            helper_function: certificate.helper_source().function().into(),
            source_revision: certificate.source().source_revision().into(),
            node_c_type: self.import.node_c_type().into(),
            cases,
            profile_assumptions: certificate.source().profile_assumptions().clone(),
        })
    }
}

impl WitnessChecker for ListUnlinkSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("unlink task differs from its checked import context".into());
        }
        let SemanticWitness::TeardownContract(proposed) = witness else {
            return Err("expected teardown contract witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("teardown witness is bound to a different task".into());
        }
        let TeardownProposalRecord::ListUnlink(proposal) = &proposed.summary else {
            return Err("expected a list unlink proposal".into());
        };
        // Acceptance terminates in the core's own contract evaluation over its
        // own import; the proposal is then required to be that same reading.
        let certificate = self.evaluate()?;
        if *proposal != self.record(&certificate)? {
            return Err("unlink witness differs from the checked source unlink".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}

/// Sealed replay context for the nullable-field destructor family.
pub struct FieldDestructorSourceTask {
    import: FieldDestructorSourceImport,
    contract_source: String,
    root_qcore_type: String,
    task: VerificationTask,
}

impl FieldDestructorSourceTask {
    pub fn derive(
        import: &FieldDestructorSourceImport,
        translation_unit: &str,
        contract_source: &str,
        root_qcore_type: &str,
        target: TargetProfile,
    ) -> Result<Self, String> {
        check_target(
            &target,
            FIELD_DESTRUCTOR_SEMANTIC_PROFILE,
            import.source_revision(),
            import.input_identity(),
            translation_unit,
            contract_source,
            root_qcore_type,
        )?;
        // The projected owned-field inventory is part of the question, so it
        // must be exactly the inventory the pinned contract declares. A
        // projection that enumerates a different nullable-state domain is
        // refused before any task exists.
        let declared = declared_owned_fields(contract_source, import.function())?;
        let projected: BTreeSet<String> = import.declared_owned_fields().iter().cloned().collect();
        if projected.len() != import.declared_owned_fields().len() || projected != declared {
            return Err(
                "projected owned-field inventory is not the pinned contract's inventory".into(),
            );
        }
        let paths = u64::try_from(import.summary().paths().len())
            .map_err(|_| "destructor path count out of range")?;
        if paths == 0 {
            return Err("destructor import has no complete path".into());
        }
        let input = import.source_input();
        let serialized_identity =
            serde_json::to_vec(import.input_identity()).map_err(|e| e.to_string())?;
        let contract_sources = BTreeMap::from([
            (TEARDOWN_SOURCE_INPUT_KEY.to_owned(), input.digest()),
            (
                "qcore:teardown-import:v1".into(),
                sha256_hex(&serialized_identity),
            ),
            (
                "qcore:field-destructor-contract:v1".into(),
                sha256_hex(contract_source.as_bytes()),
            ),
            (
                "qcore:field-destructor-root-type:v1".into(),
                sha256_hex(root_qcore_type.as_bytes()),
            ),
        ]);
        let assumptions = BTreeMap::from([
            ("root_is_live".into(), true),
            ("exclusive_access".into(), true),
            ("disjoint_owned_fields".into(), true),
            ("owned_fields_are_disjoint_from_root".into(), true),
            ("free_resolves_to_libc".into(), true),
        ])
        .into_iter()
        .chain(trust_assumptions())
        .collect::<BTreeMap<_, _>>();
        // The destructor is a multi-path body: the exact path count is bound
        // into the task, so a re-import with a different path set is a
        // different task.
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
            semantic_profile: FIELD_DESTRUCTOR_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::FieldDestructorContract;
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
            request_id: format!("field-destructor-contract:{}", self.task.task_digest()),
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
            teardown_source: Some(self.import.source_input()),
            call_use_source: None,
            cleanup_replay_source: None,
            composition_source: None,
        }
    }

    /// This task's own identity, for a *dependent* capability to name it by.
    /// It carries no verdict, certificate or reading — only the semantic
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
    /// claimed identity is the one this context derives.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<FieldDestructorCertificate, String> {
        if !claimed.is_complete() {
            return Err("field destructor prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err(
                "field destructor prerequisite identity is not the one the core derives".into(),
            );
        }
        self.evaluate()
    }

    /// Re-check the imported body against the pinned contract, exhausting
    /// every realizable nullable state. This is the core's own evaluation.
    /// The destructor body this task imported, so a dependent composition can
    /// require that the frame it resolves is the one this prerequisite checked.
    pub fn summary(&self) -> &qcore_c_gateway::typed_source::SourceSummary {
        self.import.summary()
    }

    pub fn evaluate(&self) -> Result<FieldDestructorCertificate, String> {
        check_field_destructor(
            self.import.summary(),
            &self.contract_source,
            self.import.root_c_type(),
            &self.root_qcore_type,
        )
    }

    fn record(
        &self,
        certificate: &FieldDestructorCertificate,
    ) -> Result<FieldDestructorRecord, String> {
        let mut cases = Vec::new();
        for case in certificate.cases() {
            cases.push(FieldReleaseCaseRecord {
                initially_live: case.initially_live.clone(),
                released_in_order: case.released_in_order.clone(),
                source_path: u32::try_from(case.source_path)
                    .map_err(|_| "source path index out of range")?,
            });
        }
        Ok(FieldDestructorRecord {
            function: certificate.source().function().into(),
            source_revision: certificate.source().source_revision().into(),
            root_c_type: self.import.root_c_type().into(),
            declared_owned_fields: self.import.declared_owned_fields().to_vec(),
            cases,
            profile_assumptions: certificate.source().profile_assumptions().clone(),
        })
    }
}

impl WitnessChecker for FieldDestructorSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("destructor task differs from its checked import context".into());
        }
        let SemanticWitness::TeardownContract(proposed) = witness else {
            return Err("expected teardown contract witness".into());
        };
        if proposed.task_digest != task.task_digest() {
            return Err("teardown witness is bound to a different task".into());
        }
        let TeardownProposalRecord::FieldDestructor(proposal) = &proposed.summary else {
            return Err("expected a field destructor proposal".into());
        };
        let certificate = self.evaluate()?;
        if *proposal != self.record(&certificate)? {
            return Err("destructor witness differs from the checked source release".into());
        }
        Ok(task.coverage_for(VerifierCapability::ResourceContracts))
    }
}
