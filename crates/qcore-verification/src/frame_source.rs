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

//! Function-scoped verification task for a source-derived Boolean **frame
//! helper** (card `P04-COMPOSE-FRAME`).
//!
//! Before this module the frame helper was the one ChunkIO capability with an
//! importer (`qcore_c_gateway::boolean_source::import_chunk_frame_helper*`) and
//! a contract evaluator (`qcore_contract::chunk_contract`) and **no
//! verification task at all**. A `PrerequisiteIdentity` is a lookup key whose
//! three fields — semantic profile, task digest, obligation identifier — are
//! re-derived by a dependent's core from its own import. A capability with no
//! task has none of the three, so nothing could name the frame helper and the
//! `memory_close` chain was blocked behind it.
//!
//! This module supplies the missing identity and nothing else. **The evaluator
//! is unedited**: `evaluate` calls the same
//! `evaluate_chunk_frame_contract` / `evaluate_boolean_frame_contract` the
//! in-process routes call, on the same sealed import, and every existing
//! refusal keeps its exact message. What is new is that the reading is now
//! wrapped in a task that binds its complete inputs, under the G01
//! compile-task-identity discipline: the summary record, the captured source /
//! build / preprocessing identity, the pinned contract bytes, the declared
//! record and protected-field vector, the declared profile selection, and the
//! contract flavour. Moving any one of them retires the task.
//!
//! Live storage, exclusive access and cross-unit record ABI equality remain
//! explicit declared premises of the pinned contract, published in the task's
//! assumptions. This is not a custody, lifecycle, layout or whole-unit claim:
//! an admitted frame helper writes constants into ordinary declared metadata
//! fields and leaves the modeled Boolean fields alone, and that is all.
//!
//! **This capability has no transported witness form.** No projection travels,
//! no `SemanticWitness` variant can carry a frame reading, and the
//! `WitnessChecker` implementation refuses every witness by construction. The
//! obligation is discharged only by the core's own evaluation, reached through
//! `evaluate` or through `revalidate_as_prerequisite` on behalf of a dependent.

use qcore_c_gateway::boolean_source::ChunkFrameSummary;
use qcore_schemas::call_use_summary::PrerequisiteIdentity;
use qcore_schemas::obligation::*;
use qcore_schemas::plugin::*;
use qcore_schemas::{SemanticWitness, sha256_hex};
use qcore_verifier_host::WitnessChecker;
use std::collections::{BTreeMap, BTreeSet};

pub const FRAME_HELPER_SEMANTIC_PROFILE: &str = "qcore.c_frame_helper_contract.v1";

/// The serialized schema record: function, retained source, ordered field/type
/// layout and ordered constant writes.
pub const FRAME_HELPER_SUMMARY_KEY: &str = "qcore:frame-helper-summary:v1";
/// The importer's captured source/preprocessing/build identity.
pub const FRAME_HELPER_IMPORT_KEY: &str = "qcore:frame-helper-import:v1";
/// The pinned contract this helper is checked against.
pub const FRAME_HELPER_CONTRACT_KEY: &str = "qcore:frame-helper-contract:v1";
/// The declared record, its protected-field vector, and which ghost-name
/// vocabulary the contract must use. None of the three is in the summary
/// record or in the captured identity.
pub const FRAME_HELPER_BINDING_KEY: &str = "qcore:frame-helper-binding:v1";
/// The declared profile selection: model library plus loaded-manifest digest,
/// or the explicit absence a legacy import records.
pub const FRAME_HELPER_PROFILE_KEY: &str = "qcore:frame-helper-profile:v1";

/// Which pinned-contract ghost-name vocabulary this task evaluates against.
///
/// The two unedited evaluators state the same property with different names.
/// The selection is part of the checked question, not a convenience: it is
/// bound into the task identity, and a contract written in the other
/// vocabulary is refused by the evaluator rather than reinterpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameContractFlavour {
    /// `evaluate_chunk_frame_contract`: the ChunkIO `Chunk` declaration, whose
    /// premises are `chunk_is_live` / `exclusive_chunk_access` and whose
    /// preserved-frame ghost name is `lock_is_preserved`. The evaluator pins
    /// its protected set to `{"lock"}`.
    Chunk,
    /// `evaluate_boolean_frame_contract`: the generic declared-record
    /// declaration, whose premises are `object_is_live` /
    /// `exclusive_object_access` and whose preserved-frame ghost name is
    /// `modeled_fields_are_preserved`. The evaluator uses the import's own
    /// protected-field vector.
    BooleanRecord,
}

impl FrameContractFlavour {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chunk => "chunk",
            Self::BooleanRecord => "boolean_record",
        }
    }

    /// The protected fields the selected evaluator actually checks for
    /// preservation. For the generic flavour this is the import's own vector;
    /// for the Chunk flavour the unedited evaluator pins `{"lock"}`.
    fn evaluated_protected_fields(self, imported: &BTreeSet<String>) -> BTreeSet<String> {
        match self {
            Self::Chunk => BTreeSet::from(["lock".to_owned()]),
            Self::BooleanRecord => imported.clone(),
        }
    }

    /// The contract's own declared premises, in the vocabulary this flavour
    /// requires, so a dependent that unions premise sets records which
    /// spelling was assumed.
    fn declared_premises(self) -> [(&'static str, bool); 2] {
        match self {
            Self::Chunk => [("chunk_is_live", true), ("exclusive_chunk_access", true)],
            Self::BooleanRecord => [("object_is_live", true), ("exclusive_object_access", true)],
        }
    }
}

/// The reading the core computes from its own import and its own contract.
///
/// Every field is read back off the sealed import or off the core's own
/// evaluation; nothing here is supplied by a caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameHelperCertificate {
    function: String,
    source_revision: String,
    record: String,
    imported_protected_fields: BTreeSet<String>,
    evaluated_protected_fields: BTreeSet<String>,
    flavour: FrameContractFlavour,
    layout: Vec<(String, String)>,
    writes: Vec<(String, i32)>,
    checked_postconditions: usize,
}

impl FrameHelperCertificate {
    pub fn function(&self) -> &str {
        &self.function
    }
    pub fn source_revision(&self) -> &str {
        &self.source_revision
    }
    pub fn record(&self) -> &str {
        &self.record
    }
    /// Fields the *importer* forbade the helper to write.
    pub fn imported_protected_fields(&self) -> &BTreeSet<String> {
        &self.imported_protected_fields
    }
    /// Fields the *evaluator* checked for preservation. A subset of the
    /// imported vector, by a condition `derive` enforces.
    pub fn evaluated_protected_fields(&self) -> &BTreeSet<String> {
        &self.evaluated_protected_fields
    }
    pub fn flavour(&self) -> FrameContractFlavour {
        self.flavour
    }
    /// Ordered declared field/type vector of the record. Type names do not
    /// establish recursive layout or alignment; that stays an outer premise.
    pub fn layout(&self) -> &[(String, String)] {
        &self.layout
    }
    /// Ordered constant writes, exactly as the helper body performs them.
    pub fn writes(&self) -> &[(String, i32)] {
        &self.writes
    }
    pub fn checked_postconditions(&self) -> usize {
        self.checked_postconditions
    }
}

/// Sealed frame-helper replay context. Only the opaque gateway importer can
/// produce the `ChunkFrameSummary` this is derived from; a raw schema record
/// cannot construct one.
pub struct FrameHelperSourceTask {
    summary: ChunkFrameSummary,
    contract_source: String,
    flavour: FrameContractFlavour,
    task: VerificationTask,
}

impl FrameHelperSourceTask {
    /// Derive the task. Refuses before any task exists when the declared
    /// target, the import's own identity, the contract flavour and the
    /// imported protected-field vector do not agree.
    pub fn derive(
        summary: &ChunkFrameSummary,
        translation_unit: &str,
        contract_source: &str,
        flavour: FrameContractFlavour,
        target: TargetProfile,
    ) -> Result<Self, String> {
        let identity = summary.input_identity();
        if translation_unit.is_empty()
            || contract_source.is_empty()
            || target.library.is_empty()
            || target.profile != FRAME_HELPER_SEMANTIC_PROFILE
            || !matches!(target.dialect.as_str(), "c11" | "gnu99" | "gnu11" | "gnu17")
            || target.revision != identity.source_revision()
            || target.dialect != identity.dialect()
            || target.platform != std::env::consts::OS
            || target.backend != "native"
        {
            return Err("incomplete or incompatible frame helper target identity".into());
        }
        let target_os = match std::env::consts::OS {
            "macos" => "darwin",
            "windows" => "windows",
            other => other,
        };
        if !identity.compiler_target().contains(target_os) {
            return Err("Clang target does not match the native frame helper platform".into());
        }
        // A helper imports no helper bodies of its own; a nonempty map would
        // mean this summary came from somewhere the importer cannot produce.
        if !identity.helpers().is_empty() {
            return Err("the frame helper route imports no helper bodies".into());
        }
        if summary.function().is_empty() || summary.record().is_empty() {
            return Err("frame helper import is missing its function or record identity".into());
        }
        // The importer already refuses an empty write set; requiring it here
        // keeps the task from ever existing over a vacuous reading, whatever a
        // future importer change does.
        if summary.writes().is_empty() {
            return Err("a frame helper task requires at least one checked field write".into());
        }
        let imported = summary.protected_fields().clone();
        if imported.is_empty() {
            return Err("frame helper import declares no protected fields".into());
        }
        let evaluated = flavour.evaluated_protected_fields(&imported);
        // Soundness of composing an unedited evaluator with this import: the
        // importer refuses every write to the *imported* protected vector, and
        // the evaluator checks preservation of the *evaluated* one. Requiring
        // the second to be contained in the first means the evaluator never
        // certifies preservation of a field the importer did not already
        // forbid. The Chunk flavour's pinned `{"lock"}` is a strict subset of
        // the ChunkIO import's `{"busy", "lock"}`; that asymmetry is recorded,
        // not hidden.
        if !evaluated.is_subset(&imported) {
            return Err(
                "the selected frame contract flavour checks a field the import did not protect"
                    .into(),
            );
        }
        if flavour == FrameContractFlavour::Chunk && summary.record() != "cio_chunk" {
            return Err("the Chunk frame contract flavour binds the cio_chunk record only".into());
        }

        let serialized_summary = serde_json::to_vec(summary).map_err(|e| e.to_string())?;
        let serialized_identity = serde_json::to_vec(identity).map_err(|e| e.to_string())?;
        // Three facts that live in neither the summary record nor the captured
        // identity: without this key two different declared protections, or
        // two different contract vocabularies, over one body would record one
        // task identity.
        let serialized_binding =
            serde_json::to_vec(&(flavour.as_str(), summary.record(), &imported))
                .map_err(|e| e.to_string())?;
        // `None` and `Some` serialize differently, so a legacy import and a
        // profile-selected import over the same body are distinct tasks.
        let serialized_profile =
            serde_json::to_vec(&summary.profile_binding()).map_err(|e| e.to_string())?;

        let contract_sources = BTreeMap::from([
            (
                FRAME_HELPER_SUMMARY_KEY.to_owned(),
                sha256_hex(&serialized_summary),
            ),
            (
                FRAME_HELPER_IMPORT_KEY.to_owned(),
                sha256_hex(&serialized_identity),
            ),
            (
                FRAME_HELPER_CONTRACT_KEY.to_owned(),
                sha256_hex(contract_source.as_bytes()),
            ),
            (
                FRAME_HELPER_BINDING_KEY.to_owned(),
                sha256_hex(&serialized_binding),
            ),
            (
                FRAME_HELPER_PROFILE_KEY.to_owned(),
                sha256_hex(&serialized_profile),
            ),
        ]);

        let mut assumptions = BTreeMap::from([
            ("clang_typed_import_is_trusted".to_owned(), true),
            (
                "source_projection_policy_is_declared_trusted".to_owned(),
                true,
            ),
            (
                "declared_source_revision_is_outer_audit_assumption".to_owned(),
                true,
            ),
            // Ordered field/type names do not prove recursive record layout or
            // alignment equality across translation units.
            (
                "cross_unit_record_abi_equality_is_an_outer_audit_premise".to_owned(),
                true,
            ),
        ]);
        for (name, value) in flavour.declared_premises() {
            assumptions.insert(name.to_owned(), value);
        }

        // The admitted helper grammar is a nonempty straight-line sequence of
        // direct constant field assignments: no call, no branch, no loop. One
        // path is the whole body, not a sampled one.
        let bounds = TaskBounds::single_path();
        let mut task = VerificationTask {
            task_version: VERIFICATION_TASK_VERSION,
            unit: UnitIdentity {
                translation_unit: translation_unit.into(),
                source_digest: identity.raw_source_digest().into(),
                build_digest: sha256_hex(&serialized_identity),
            },
            target,
            semantic_profile: FRAME_HELPER_SEMANTIC_PROFILE.into(),
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
        let property = ObligationProperty::FrameHelperContract;
        let capability = VerifierCapability::ResourceContracts;
        task.obligations.push(Obligation {
            obligation_id: task.obligation_id(capability, property, &subject),
            capability,
            property,
            subject,
            scope: ObligationScope::Functions {
                symbols: BTreeSet::from([summary.function().to_owned()]),
            },
            bounds,
            assumptions,
            evidence_version: 2,
        });
        task.validate().map_err(|e| e.to_string())?;
        Ok(Self {
            summary: summary.clone(),
            contract_source: contract_source.into(),
            flavour,
            task,
        })
    }

    pub fn task(&self) -> &VerificationTask {
        &self.task
    }

    pub fn flavour(&self) -> FrameContractFlavour {
        self.flavour
    }

    /// The sealed helper summary this task imported, so a dependent can
    /// require that the helper body it composes over is the one this
    /// prerequisite checked.
    pub fn summary(&self) -> &ChunkFrameSummary {
        &self.summary
    }

    /// Re-check the imported helper against the pinned contract. This is the
    /// core's own evaluation, through the unedited evaluator; no external
    /// input reaches it.
    pub fn evaluate(&self) -> Result<FrameHelperCertificate, String> {
        let checked_postconditions = match self.flavour {
            FrameContractFlavour::Chunk => {
                qcore_contract::chunk_contract::evaluate_chunk_frame_contract(
                    &self.summary,
                    &self.contract_source,
                )?
            }
            FrameContractFlavour::BooleanRecord => {
                qcore_contract::chunk_contract::evaluate_boolean_frame_contract(
                    &self.summary,
                    &self.contract_source,
                    self.summary.record(),
                    self.summary.protected_fields(),
                )?
            }
        };
        let imported = self.summary.protected_fields().clone();
        Ok(FrameHelperCertificate {
            function: self.summary.function().into(),
            source_revision: self.summary.input_identity().source_revision().into(),
            record: self.summary.record().into(),
            evaluated_protected_fields: self.flavour.evaluated_protected_fields(&imported),
            imported_protected_fields: imported,
            flavour: self.flavour,
            layout: self.summary.layout.clone(),
            writes: self.summary.writes().to_vec(),
            checked_postconditions,
        })
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
    /// claimed identity is the one this context derives. The dependent never
    /// receives a certificate it did not cause the core to recompute.
    pub fn revalidate_as_prerequisite(
        &self,
        claimed: &PrerequisiteIdentity,
    ) -> Result<FrameHelperCertificate, String> {
        if !claimed.is_complete() {
            return Err("frame helper prerequisite identity is incomplete".into());
        }
        if *claimed != self.prerequisite_identity() {
            return Err(
                "frame helper prerequisite identity is not the one the core derives".into(),
            );
        }
        self.evaluate()
    }
}

impl WitnessChecker for FrameHelperSourceTask {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ResourceContracts
    }

    /// The frame-helper capability has no transported vocabulary: no
    /// projection travels in a request and no `SemanticWitness` variant can
    /// carry a frame reading. Every witness is therefore refused, and the
    /// obligation is discharged only by the core's own `evaluate`. This is an
    /// explicit decision, recorded as a refusal rather than left as an
    /// omission, so a future transported form has to be added deliberately.
    fn check(
        &self,
        task: &VerificationTask,
        _witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        if task != &self.task {
            return Err("the frame helper checker did not derive this task".into());
        }
        Err("the frame helper capability admits no transported witness".into())
    }
}
