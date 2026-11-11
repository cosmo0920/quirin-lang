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

//! Trusted-core replay of evidence-v2 semantic witnesses.
//!
//! Implements `docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` §2.3 and §2.4. The
//! firewall trait these types implement lives in the verifier host
//! (`verifier_plugin::WitnessChecker`); the checking itself is trusted-core
//! code and is never reachable from a provider (design open question 5).
//!
//! The anti-vacuousness rule (design §2.2) is structural here: every
//! acceptance path ends in a recomputation over core-derived inputs — the
//! core's own lifecycle call inventory, the core's own chunk model, the
//! core's digest-pinned contract text, the core's own required-hook set — and
//! every quantifier's empty case is decided against a core-computed
//! denominator, never against a witness-supplied list.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::obligation::{
    ChunkEntityRef, ObligationProperty, ObligationScope, ObligationSubject, TaskBounds,
    VerificationTask,
};
use qcore_schemas::plugin::VerifierCapability;
use qcore_schemas::witness::{
    ChunkBackendRecord, ChunkOperationRecord, ChunkResidencyRecord, SemanticWitness, SourceSite,
};

use crate::chunk_lifecycle::{
    ChunkBackend, ChunkOperation, ChunkProgram, ChunkResidency, ChunkState, check_chunk_program,
};
use crate::obligation_derivation::{LifecycleInventory, ObservationInventory};
use crate::verifier_plugin::WitnessChecker;

// ---------------------------------------------------------------------------
// Chunk lifecycle (design §2.3)
// ---------------------------------------------------------------------------

/// Replays a `ChunkLifecycleWitness` against the core's lifecycle call
/// inventory and `chunk_lifecycle::check_chunk_program`.
pub struct ChunkLifecycleReplay<'a> {
    inventory: &'a LifecycleInventory,
}

impl<'a> ChunkLifecycleReplay<'a> {
    #[must_use]
    pub const fn new(inventory: &'a LifecycleInventory) -> Self {
        Self { inventory }
    }
}

fn convert(operation: ChunkOperationRecord) -> ChunkOperation {
    let backend = |backend: ChunkBackendRecord| match backend {
        ChunkBackendRecord::Memory => ChunkBackend::Memory,
        ChunkBackendRecord::File => ChunkBackend::File,
    };
    let residency = |residency: ChunkResidencyRecord| match residency {
        ChunkResidencyRecord::Up => ChunkResidency::Up,
        ChunkResidencyRecord::Down => ChunkResidency::Down,
    };
    match operation {
        ChunkOperationRecord::CreateContext { context } => {
            ChunkOperation::CreateContext { context }
        }
        ChunkOperationRecord::CreateStream { context, stream } => {
            ChunkOperation::CreateStream { context, stream }
        }
        ChunkOperationRecord::OpenChunk {
            context,
            stream,
            chunk,
            backend: kind,
            residency: place,
        } => ChunkOperation::OpenChunk {
            context,
            stream,
            chunk,
            backend: backend(kind),
            residency: residency(place),
        },
        ChunkOperationRecord::SetResidency {
            chunk,
            residency: place,
        } => ChunkOperation::SetResidency {
            chunk,
            residency: residency(place),
        },
        ChunkOperationRecord::Lock { chunk } => ChunkOperation::Lock { chunk },
        ChunkOperationRecord::Unlock { chunk } => ChunkOperation::Unlock { chunk },
        ChunkOperationRecord::BeginTransaction { chunk } => {
            ChunkOperation::BeginTransaction { chunk }
        }
        ChunkOperationRecord::CommitTransaction { chunk } => {
            ChunkOperation::CommitTransaction { chunk }
        }
        ChunkOperationRecord::RollbackTransaction { chunk } => {
            ChunkOperation::RollbackTransaction { chunk }
        }
        ChunkOperationRecord::CloseChunk { chunk, delete } => {
            ChunkOperation::CloseChunk { chunk, delete }
        }
        ChunkOperationRecord::DestroyStream { stream } => ChunkOperation::DestroyStream { stream },
        ChunkOperationRecord::DestroyContext { context } => {
            ChunkOperation::DestroyContext { context }
        }
    }
}

fn discharged(
    state: &ChunkState,
    subject: &ObligationSubject,
    property: ObligationProperty,
) -> bool {
    let entity = match subject {
        ObligationSubject::ChunkEntity { entity } => *entity,
        ObligationSubject::Unit => {
            // `LifecycleProgramComplete` is discharged by the completeness
            // stage, which has already run against the core's inventory.
            return property == ObligationProperty::LifecycleProgramComplete;
        }
        _ => return false,
    };
    match (property, entity) {
        (ObligationProperty::ContextDischarged, ChunkEntityRef::Context { id }) => state
            .contexts
            .get(&id)
            .is_some_and(|record| !record.live && record.chunk_total == 0),
        (ObligationProperty::StreamDischarged, ChunkEntityRef::Stream { id }) => {
            state.streams.get(&id).is_some_and(|record| !record.live)
        }
        (ObligationProperty::ChunkDischarged, ChunkEntityRef::Chunk { id }) => {
            state.chunks.get(&id).is_some_and(|record| {
                !record.live
                    && !record.name_live
                    && !record.primary_linked
                    && !record.residency_linked
                    && !record.backend_object_live
                    && !record.memory_buffer_live
                    && !record.metadata_buffer_live
                    && !record.file_mapping_live
                    && !record.file_descriptor_live
                    && !record.file_path_live
            })
        }
        (ObligationProperty::TransactionSettled, ChunkEntityRef::Chunk { id }) => state
            .chunks
            .get(&id)
            .is_some_and(|record| !record.transaction_active),
        _ => false,
    }
}

impl WitnessChecker for ChunkLifecycleReplay<'_> {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ChunkLifecycle
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        let SemanticWitness::ChunkLifecycle(witness) = witness else {
            return Err(format!(
                "witness kind `{}` does not serve the chunk-lifecycle capability",
                witness.kind()
            ));
        };
        // The inventory is core-derived; a duplicate site in it would make the
        // completeness denominator ambiguous, so refuse rather than guess.
        let mut sites = BTreeMap::<&SourceSite, usize>::new();
        for site in &self.inventory.sites {
            if sites.insert(&site.site, 0).is_some() {
                return Err("the core lifecycle inventory is ambiguous for this unit".into());
            }
        }

        // 1. Site validity: only core-discovered sites may appear.
        for bound in &witness.schedule {
            let Some(count) = sites.get_mut(&bound.site) else {
                return Err(format!(
                    "the schedule names call site `{}`@{}..{}, which is not in the core's \
                     lifecycle call inventory",
                    bound.site.callee, bound.site.span_start, bound.site.span_end
                ));
            };
            *count += 1;
        }

        // 2. Completeness against the core's denominator, never the witness's.
        if let Some((site, _)) = sites.iter().find(|(_, count)| **count == 0) {
            return Err(format!(
                "the schedule omits core-discovered call site `{}`@{}..{}",
                site.callee, site.span_start, site.span_end
            ));
        }
        if let Some((site, count)) = sites.iter().find(|(_, count)| **count > 1) {
            return Err(format!(
                "the schedule uses core-discovered call site `{}`@{}..{} {count} times; \
                 every site appears exactly once",
                site.callee, site.span_start, site.span_end
            ));
        }

        // 3. Operation faithfulness: the kind and every entity identity are
        //    the ones the core's API table assigned to that site.
        let templates = self
            .inventory
            .sites
            .iter()
            .map(|site| (&site.site, site.template))
            .collect::<BTreeMap<_, _>>();
        for bound in &witness.schedule {
            let template = templates[&bound.site];
            if !template.accepts(bound.operation) {
                return Err(format!(
                    "the schedule claims operation `{}` at call site `{}`, which the core's \
                     API table maps to a different operation or entity",
                    bound.operation.kind(),
                    bound.site.callee
                ));
            }
        }

        // 4. Order admissibility: the evidence-v2.0 bound is a single path per
        //    analyzed function, so the admissible order is the core's own
        //    source order. Anything else is `Unknown`, never approximated.
        for (bound, site) in witness.schedule.iter().zip(&self.inventory.sites) {
            if bound.site != site.site {
                return Err(format!(
                    "the schedule orders call site `{}`@{}..{} where the core's single-path \
                     order has `{}`@{}..{}",
                    bound.site.callee,
                    bound.site.span_start,
                    bound.site.span_end,
                    site.site.callee,
                    site.site.span_start,
                    site.site.span_end
                ));
            }
        }

        // 5. Semantic replay through the trusted core checker.
        let program = ChunkProgram {
            operations: witness
                .schedule
                .iter()
                .map(|bound| convert(bound.operation))
                .collect(),
        };
        let certificate = check_chunk_program(&program).map_err(|diagnostics| {
            format!("the chunk lifecycle model rejected the replayed schedule: {diagnostics:?}")
        })?;

        // 6. Obligation discharge over the replayed final state.
        let obligations = task.obligations_for(VerifierCapability::ChunkLifecycle);
        if obligations.is_empty() {
            return Err("the core derived no chunk-lifecycle obligations for this unit".into());
        }
        let mut covered = BTreeSet::new();
        for obligation in obligations {
            if !discharged(
                &certificate.final_state,
                &obligation.subject,
                obligation.property,
            ) {
                return Err(format!(
                    "obligation `{}` ({} of {}) is not discharged by the replayed schedule",
                    obligation.obligation_id,
                    obligation.property,
                    obligation.subject.canonical()
                ));
            }
            covered.insert(obligation.obligation_id.clone());
        }
        Ok(covered)
    }
}

// ---------------------------------------------------------------------------
// Contract observations (design §2.4)
// ---------------------------------------------------------------------------

/// Replays a `ContractObservationWitness` against the core's digest-pinned
/// contract sources and required-hook set.
///
/// Discharge here claims *observed agreement for the bound runs*, never static
/// source safety: the obligations carry `observed_runs_only` bounds and the
/// `observations_trusted` assumption is declared on the task.
pub struct ContractObservationReplay<'a> {
    inventory: &'a ObservationInventory,
}

impl<'a> ContractObservationReplay<'a> {
    #[must_use]
    pub const fn new(inventory: &'a ObservationInventory) -> Self {
        Self { inventory }
    }
}

impl WitnessChecker for ContractObservationReplay<'_> {
    fn capability(&self) -> VerifierCapability {
        VerifierCapability::ObservationHooks
    }

    fn check(
        &self,
        task: &VerificationTask,
        witness: &SemanticWitness,
    ) -> Result<BTreeSet<String>, String> {
        let SemanticWitness::ContractObservation(witness) = witness else {
            return Err(format!(
                "witness kind `{}` does not serve the observation-hooks capability",
                witness.kind()
            ));
        };
        // This checker establishes conditional agreement for recorded runs.
        // Reject caller-built tasks that omit/refuse that assumption or relabel
        // the resulting obligations as an unbounded source-safety claim.
        if task.assumptions.get("observations_trusted") != Some(&true)
            || task.bounds != TaskBounds::observed_runs()
            || task
                .obligations_for(VerifierCapability::ObservationHooks)
                .iter()
                .any(|obligation| {
                    obligation.assumptions != task.assumptions
                        || obligation.bounds != TaskBounds::observed_runs()
                        || !matches!(&obligation.scope, ObligationScope::ObservedRuns { run_ids } if run_ids.is_empty())
                })
        {
            return Err("observation replay requires observations_trusted=true, consistent declared assumptions, exact observed-run bounds and deferred observed-run scope".into());
        }
        let identities = self
            .inventory
            .contract_sources
            .iter()
            .map(|(path, source)| {
                (
                    path.clone(),
                    qcore_schemas::digest::sha256_hex(source.as_bytes()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if task.contract_sources != identities {
            return Err(
                "observation replay sources do not match the task's pinned contracts".into(),
            );
        }
        if witness
            .runs
            .iter()
            .any(|run| run.binding.profile != task.target.profile)
        {
            return Err("observed run profile does not match the verification task".into());
        }
        // The required-hook set is the core's, taken from the manifest and the
        // trusted contract sources. A provider can neither add nor drop one.
        let required = self
            .inventory
            .required_hooks
            .iter()
            .map(|(hook, (module, _))| (hook.clone(), module.clone()))
            .collect::<BTreeMap<_, _>>();
        if required.is_empty() {
            return Err("the core required no observation hooks for this unit".into());
        }
        let checked = qcore_contract::contract_observation::replay_observation_witness(
            &self.inventory.contract_sources,
            &required,
            witness,
        )?;
        for hook in required.keys() {
            if !checked.checked_hooks.contains(hook) {
                return Err(format!(
                    "required observation hook `{hook}` has no observation that both meets \
                     its preconditions and satisfies its postconditions"
                ));
            }
        }
        let obligations = task.obligations_for(VerifierCapability::ObservationHooks);
        if obligations.is_empty() {
            return Err("the core derived no observation obligations for this unit".into());
        }
        let mut covered = BTreeSet::new();
        for obligation in obligations {
            let ok = match (&obligation.property, &obligation.subject) {
                (
                    ObligationProperty::ObservationChecked,
                    ObligationSubject::ObservedContract {
                        function,
                        contract_module,
                    },
                ) => self.inventory.required_hooks.get(function).is_some_and(
                    |(module, declared_function)| {
                        module == contract_module
                            && declared_function == function
                            && checked.checked_hooks.contains(function)
                    },
                ),
                (ObligationProperty::ObservationCoverageComplete, ObligationSubject::Unit) => {
                    required
                        .keys()
                        .all(|hook| checked.checked_hooks.contains(hook))
                }
                _ => false,
            };
            if !ok {
                return Err(format!(
                    "obligation `{}` ({} of {}) is not discharged by the recorded runs",
                    obligation.obligation_id,
                    obligation.property,
                    obligation.subject.canonical()
                ));
            }
            covered.insert(obligation.obligation_id.clone());
        }
        Ok(covered)
    }
}

#[cfg(test)]
#[path = "../tests/unit/witness_replay.rs"]
mod tests;
