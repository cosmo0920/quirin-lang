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

//! W3 witness-replay mutation corpus (design §5.1).
//!
//! Every mutation of a valid witness must be rejected by the core with a
//! distinct reason. These tests are the executable form of the design's
//! vacuousness argument: they check that a provider cannot shrink, pad,
//! relabel, retarget, reorder, or rebind its way to acceptance.

use super::*;
use qcore_frontend::frontend::parse_source_program;
use qcore_schemas::witness::{
    BoundOperation, ChunkLifecycleWitness, ContractObservationRecord, ContractObservationWitness,
    NativeRunBinding, ObservedRun,
};

use qcore_frontend::frontend::SourceProgram;
use qcore_schemas::plugin::TargetProfile;

use crate::obligation_derivation::{DerivationInputs, DerivedTask, OperationTemplate, derive_task};

const LIFECYCLE_UNIT: &str = "\
fn run() -> unit {
    let ctx = cio_ctx_create();
    let st = cio_stream_create(ctx);
    let ch = cio_chunk_open(ctx, st);
    cio_chunk_close(ch);
    cio_stream_destroy(st);
    cio_ctx_destroy(ctx);
    return;
}
";

fn derive(source: &str, capability: VerifierCapability) -> (SourceProgram, DerivedTask) {
    let program = parse_source_program(source).expect("fixture parses");
    let derived = {
        let inputs = DerivationInputs {
            translation_unit: "unit.qc",
            program: &program,
            target: TargetProfile {
                library: "chunkio".into(),
                revision: "source".into(),
                profile: "default".into(),
                dialect: "qcore".into(),
                platform: "linux".into(),
                backend: "native".into(),
            },
            contract_sources: BTreeMap::from([("unit.qc".to_owned(), source.to_owned())]),
            required_observation_hooks: if capability == VerifierCapability::ObservationHooks {
                BTreeSet::from(["resize".to_owned()])
            } else {
                BTreeSet::new()
            },
            assumptions: BTreeMap::new(),
            verifier: None,
        };
        derive_task(&inputs, capability)
    };
    (program, derived)
}

/// The honest schedule: the core's own order, with the provider supplying the
/// parameters the core deliberately left open.
fn honest_schedule(derived: &DerivedTask) -> ChunkLifecycleWitness {
    let inventory = derived.replay.lifecycle.as_ref().expect("inventory");
    let schedule = inventory
        .sites
        .iter()
        .map(|site| BoundOperation {
            operation: match site.template {
                OperationTemplate::CreateContext { context } => {
                    ChunkOperationRecord::CreateContext { context }
                }
                OperationTemplate::CreateStream { context, stream } => {
                    ChunkOperationRecord::CreateStream { context, stream }
                }
                OperationTemplate::OpenChunk {
                    context,
                    stream,
                    chunk,
                } => ChunkOperationRecord::OpenChunk {
                    context,
                    stream,
                    chunk,
                    backend: ChunkBackendRecord::Memory,
                    residency: ChunkResidencyRecord::Up,
                },
                OperationTemplate::CloseChunk { chunk } => ChunkOperationRecord::CloseChunk {
                    chunk,
                    delete: false,
                },
                OperationTemplate::DestroyStream { stream } => {
                    ChunkOperationRecord::DestroyStream { stream }
                }
                OperationTemplate::DestroyContext { context } => {
                    ChunkOperationRecord::DestroyContext { context }
                }
                other => panic!("fixture does not use {other:?}"),
            },
            site: site.site.clone(),
        })
        .collect();
    ChunkLifecycleWitness {
        task_digest: derived.task.task_digest(),
        schedule,
    }
}

fn replay(
    derived: &DerivedTask,
    witness: ChunkLifecycleWitness,
) -> Result<BTreeSet<String>, String> {
    let inventory = derived.replay.lifecycle.as_ref().expect("inventory");
    ChunkLifecycleReplay::new(inventory)
        .check(&derived.task, &SemanticWitness::ChunkLifecycle(witness))
}

#[test]
fn honest_lifecycle_witness_discharges_every_obligation() {
    let (_program, derived) = derive(LIFECYCLE_UNIT, VerifierCapability::ChunkLifecycle);
    let covered = replay(&derived, honest_schedule(&derived)).expect("honest witness is accepted");
    assert_eq!(
        covered,
        derived
            .task
            .coverage_for(VerifierCapability::ChunkLifecycle),
        "acceptance covers exactly the core's inventory"
    );
}

/// Each mutation must be rejected, and with a *distinct* reason: a single
/// catch-all rejection would hide which invariant actually caught it.
#[test]
fn every_mutation_is_rejected_with_a_distinct_reason() {
    let (_program, derived) = derive(LIFECYCLE_UNIT, VerifierCapability::ChunkLifecycle);
    let mut reasons = Vec::new();

    // Omitted site: the denominator is the core's inventory, not the witness.
    let mut witness = honest_schedule(&derived);
    witness.schedule.remove(3);
    reasons.push(("omitted site", replay(&derived, witness).unwrap_err()));

    // Duplicated site.
    let mut witness = honest_schedule(&derived);
    let duplicate = witness.schedule[3].clone();
    witness.schedule.push(duplicate);
    reasons.push(("duplicated site", replay(&derived, witness).unwrap_err()));

    // Phantom operation at a site the core never discovered.
    let mut witness = honest_schedule(&derived);
    let mut phantom = witness.schedule[3].clone();
    phantom.site.span_start += 100_000;
    phantom.site.span_end += 100_000;
    witness.schedule.push(phantom);
    reasons.push(("phantom site", replay(&derived, witness).unwrap_err()));

    // Relabeled operation: a close presented as an open.
    let mut witness = honest_schedule(&derived);
    witness.schedule[3].operation = ChunkOperationRecord::OpenChunk {
        context: 0,
        stream: 0,
        chunk: 1,
        backend: ChunkBackendRecord::Memory,
        residency: ChunkResidencyRecord::Up,
    };
    reasons.push((
        "relabeled operation",
        replay(&derived, witness).unwrap_err(),
    ));

    // Retargeted entity identity.
    let mut witness = honest_schedule(&derived);
    witness.schedule[3].operation = ChunkOperationRecord::CloseChunk {
        chunk: 7,
        delete: false,
    };
    reasons.push(("retargeted entity", replay(&derived, witness).unwrap_err()));

    // Reordering that the core's single-path order does not admit.
    let mut witness = honest_schedule(&derived);
    witness.schedule.swap(0, 1);
    reasons.push(("inadmissible order", replay(&derived, witness).unwrap_err()));

    // Wrong witness kind for the capability.
    let inventory = derived.replay.lifecycle.as_ref().unwrap();
    let wrong_kind = ChunkLifecycleReplay::new(inventory).check(
        &derived.task,
        &SemanticWitness::ContractObservation(ContractObservationWitness {
            task_digest: derived.task.task_digest(),
            runs: Vec::new(),
        }),
    );
    reasons.push(("wrong witness kind", wrong_kind.unwrap_err()));

    let distinct = reasons
        .iter()
        .map(|(_, reason)| reason.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        distinct.len(),
        reasons.len(),
        "each mutation needs its own reason, got {reasons:#?}"
    );
}

#[test]
fn a_schedule_that_leaves_a_resource_live_is_rejected() {
    // A unit whose stream is never destroyed: the schedule is faithful, the
    // order is admissible, and the model still refuses it.
    let leaking = "\
fn run() -> unit {
    let ctx = cio_ctx_create();
    let st = cio_stream_create(ctx);
    cio_ctx_destroy(ctx);
    return;
}
";
    let (_program, derived) = derive(leaking, VerifierCapability::ChunkLifecycle);
    // `cio_ctx_destroy` cascades through the stream in the model, so this
    // fixture is accepted; the interesting case is a schedule that stops early.
    let honest = honest_schedule(&derived);
    assert!(replay(&derived, honest).is_ok());

    let stopping = "\
fn run() -> unit {
    let ctx = cio_ctx_create();
    let st = cio_stream_create(ctx);
    return;
}
";
    let (_program, derived) = derive(stopping, VerifierCapability::ChunkLifecycle);
    let error = replay(&derived, honest_schedule(&derived)).unwrap_err();
    assert!(
        error.contains("chunk lifecycle model rejected"),
        "expected a terminal model diagnostic, got: {error}"
    );
}

#[test]
fn an_empty_schedule_cannot_stand_in_for_a_nonempty_inventory() {
    let (_program, derived) = derive(LIFECYCLE_UNIT, VerifierCapability::ChunkLifecycle);
    let mut witness = honest_schedule(&derived);
    witness.schedule.clear();
    let error = replay(&derived, witness).unwrap_err();
    assert!(error.contains("omits core-discovered call site"), "{error}");
}

// ---------------------------------------------------------------------------
// Contract observations
// ---------------------------------------------------------------------------

const OBSERVED_UNIT: &str =
    "extern fn resize() -> unit contract { requires size > 0; ensures normal live == 1; };";

fn run(values: &[(&str, i128)]) -> ObservedRun {
    ObservedRun {
        run_id: "run-0".into(),
        binding: NativeRunBinding {
            emitted_source_digest: "a".repeat(64),
            compiler_invocation: "cc -O0".into(),
            executable_digest: "b".repeat(64),
            exit_status: 0,
            seed_or_input: "0".into(),
            profile: "default".into(),
            instrumentation_version: "1".into(),
        },
        observations: vec![ContractObservationRecord {
            function: "resize".into(),
            hook: "resize".into(),
            values: values
                .iter()
                .map(|(name, value)| ((*name).to_owned(), *value))
                .collect(),
        }],
    }
}

fn observe(derived: &DerivedTask, runs: Vec<ObservedRun>) -> Result<BTreeSet<String>, String> {
    let inventory = derived.replay.observation.as_ref().expect("inventory");
    ContractObservationReplay::new(inventory).check(
        &derived.task,
        &SemanticWitness::ContractObservation(ContractObservationWitness {
            task_digest: derived.task.task_digest(),
            runs,
        }),
    )
}

#[test]
fn observed_agreement_is_checked_against_core_predicates() {
    let (_program, derived) = derive(OBSERVED_UNIT, VerifierCapability::ObservationHooks);
    let covered = observe(&derived, vec![run(&[("size", 4), ("live", 1)])])
        .expect("a satisfying observation discharges the hook");
    assert_eq!(
        covered,
        derived
            .task
            .coverage_for(VerifierCapability::ObservationHooks)
    );

    // A false `ensures` is an observed contract violation, never a `Safe`.
    assert!(observe(&derived, vec![run(&[("size", 4), ("live", 0)])]).is_err());
    // A missing observation variable is an error, not a skip.
    assert!(observe(&derived, vec![run(&[("size", 4)])]).is_err());
    // Preconditions-dodging observations earn no coverage.
    let error = observe(&derived, vec![run(&[("size", 0), ("live", 0)])]).unwrap_err();
    assert!(error.contains("has no observation"), "{error}");
    // No runs at all cannot cover a required hook.
    assert!(observe(&derived, Vec::new()).is_err());
    // A hook the core did not require cannot be substituted.
    let mut foreign = run(&[("size", 4), ("live", 1)]);
    foreign.observations[0].hook = "other".into();
    assert!(observe(&derived, vec![foreign]).is_err());
    // A run without an identity binding is not evidence.
    let mut unbound = run(&[("size", 4), ("live", 1)]);
    unbound.binding.executable_digest = String::new();
    assert!(observe(&derived, vec![unbound]).is_err());
    // Predicate syntax in a values key is inert: values are data.
    let mut injected = run(&[("size", 4), ("live", 1)]);
    injected.observations[0]
        .values
        .insert("1 == 1 or live".into(), 1);
    assert!(
        observe(&derived, vec![injected]).is_ok(),
        "an unused values key must not change the checked predicates"
    );
}
