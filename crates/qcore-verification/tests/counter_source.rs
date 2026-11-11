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

//! Trusted replay of the process-transported unsigned counter capability.
//!
//! Every accepted case still terminates in the core evaluating the pinned
//! contract over its own import; the proposal only has to agree with it.
#[path = "fixtures/counter_source.rs"]
mod fixture;
use fixture::*;
use qcore_c_gateway::counter_source::{CounterSourceResult, import_counter_sources};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_schemas::SemanticWitness;
use qcore_schemas::counter_summary::{CounterContractWitness, CounterUpdateRecord};
use qcore_verification::counter_source::{COUNTER_SEMANTIC_PROFILE, CounterSourceTask};
use qcore_verifier_host::WitnessChecker;

type RecordMutation = Box<dyn Fn(&mut CounterUpdateRecord)>;
type TargetMutation = Box<dyn Fn(&mut qcore_schemas::TargetProfile)>;

/// The reading a correct provider must produce, written out independently of
/// the core's own derivation.
fn reading(task: &CounterSourceTask, delta: i8) -> CounterUpdateRecord {
    CounterUpdateRecord {
        function: FUNCTION.into(),
        source_revision: task.task().target.revision.clone(),
        root_c_type: ROOT_C_TYPE.into(),
        counter_c_type: "unsigned long".into(),
        field: "total_chunks".into(),
        delta,
        result_is_updated_counter: true,
        only_selected_field_written: true,
        profile_assumptions: Default::default(),
    }
}

fn witness(task: &CounterSourceTask, summary: CounterUpdateRecord) -> SemanticWitness {
    SemanticWitness::CounterContract(Box::new(CounterContractWitness {
        task_digest: task.task().task_digest(),
        summary,
    }))
}

#[test]
fn real_c_update_and_contract_are_recomputed() {
    if !clang_available() {
        return;
    }
    let import = imported(SOURCE, vec![]);
    let replay = task(&import, CONTRACT);
    assert_eq!(replay.task().semantic_profile, COUNTER_SEMANTIC_PROFILE);
    assert_eq!(replay.task().obligations.len(), 1);
    assert_eq!(
        replay.task().obligations[0].property.as_str(),
        "counter_update_contract"
    );
    let certificate = replay.evaluate().unwrap();
    assert_eq!(certificate.field(), "total_chunks");
    assert_eq!(certificate.delta(), -1);
    assert_eq!(certificate.counter_c_type(), "unsigned long");
    assert_eq!(
        replay
            .check(replay.task(), &witness(&replay, reading(&replay, -1)))
            .unwrap(),
        replay.task().coverage_for(replay.capability())
    );
    // The transported projection carries the path, never a derived result.
    let input = serde_json::to_value(import.source_input()).unwrap();
    assert!(input.get("delta").is_none() && input.get("field").is_none());
    assert!(input.get("operations").is_some());
}

#[test]
fn a_false_contract_refuses_a_correct_reading() {
    if !clang_available() {
        return;
    }
    let import = imported(SOURCE, vec![]);
    // Claim the decrement increments; the path is unchanged and still exact.
    let baseline = CONTRACT.replace("\r\n", "\n");
    let inverted = baseline.replace(
        "requires counter_field__total_chunks;\n    requires counter_positive;\n    ensures normal delta == -1;",
        "requires counter_field__total_chunks;\n    requires counter_below_maximum;\n    ensures normal delta == 1;",
    );
    assert_ne!(inverted, baseline);
    let rejected = task(&import, &inverted);
    assert!(rejected.evaluate().is_err());
    for delta in [-1, 1] {
        assert!(
            rejected
                .check(
                    rejected.task(),
                    &witness(&rejected, reading(&rejected, delta))
                )
                .is_err(),
            "agreement alone cannot discharge an invalid contract"
        );
    }
}

#[test]
fn forged_readings_wrong_kinds_and_foreign_tasks_are_rejected() {
    if !clang_available() {
        return;
    }
    let import = imported(SOURCE, vec![]);
    let replay = task(&import, CONTRACT);
    let mutations: Vec<RecordMutation> = vec![
        Box::new(|record| record.delta = 1),
        Box::new(|record| record.field = "other".into()),
        Box::new(|record| record.counter_c_type = "unsigned int".into()),
        Box::new(|record| record.root_c_type = "struct other *".into()),
        Box::new(|record| record.function = "cio_chunk_counter_total_add".into()),
        Box::new(|record| record.source_revision = "forged".into()),
        Box::new(|record| record.result_is_updated_counter = false),
        Box::new(|record| record.only_selected_field_written = false),
        Box::new(|record| {
            record
                .profile_assumptions
                .insert("forged".into(), "source".into());
        }),
    ];
    for (index, mutation) in mutations.iter().enumerate() {
        let mut record = reading(&replay, -1);
        mutation(&mut record);
        assert!(
            replay
                .check(replay.task(), &witness(&replay, record))
                .is_err(),
            "mutation {index}"
        );
    }
    let mut rebound = witness(&replay, reading(&replay, -1));
    let SemanticWitness::CounterContract(value) = &mut rebound else {
        unreachable!()
    };
    value.task_digest = "a".repeat(64);
    assert!(replay.check(replay.task(), &rebound).is_err());
    // A witness of another kind never satisfies this capability's checker.
    let foreign =
        SemanticWitness::ContractObservation(qcore_schemas::witness::ContractObservationWitness {
            task_digest: replay.task().task_digest(),
            runs: Vec::new(),
        });
    assert!(replay.check(replay.task(), &foreign).is_err());
}

#[test]
fn actual_source_build_and_contract_snapshots_separate_tasks() {
    if !clang_available() {
        return;
    }
    let original = imported(SOURCE, vec![]);
    let replay = task(&original, CONTRACT);
    let changed_source = imported(&format!("/* actual bytes changed */{SOURCE}"), vec![]);
    let changed_build = imported(SOURCE, vec!["QCORE_UNUSED=1".into()]);
    for changed in [changed_source, changed_build] {
        let other = task(&changed, CONTRACT);
        assert_ne!(replay.task().task_digest(), other.task().task_digest());
        assert!(
            replay
                .check(other.task(), &witness(&other, reading(&other, -1)))
                .is_err()
        );
    }
    let recontracted = task(
        &original,
        &format!("// contract identity changed\n{CONTRACT}"),
    );
    assert_ne!(
        replay.task().task_digest(),
        recontracted.task().task_digest()
    );
    // The declared QCore root type is part of the checked question, so it
    // retires the task as well.
    let other_root = CounterSourceTask::derive(
        &original,
        "fixture.c",
        CONTRACT,
        "OtherContext",
        target(&original),
    )
    .unwrap();
    assert_ne!(replay.task().task_digest(), other_root.task().task_digest());
}

#[test]
fn incompatible_targets_refuse_derivation() {
    if !clang_available() {
        return;
    }
    let import = imported(SOURCE, vec![]);
    let cases: Vec<TargetMutation> = vec![
        Box::new(|target| target.profile = "qcore.c_boolean_field_contract.v2".into()),
        Box::new(|target| target.dialect = "c99".into()),
        Box::new(|target| target.revision = "0".repeat(40)),
        Box::new(|target| target.platform = "other-os".into()),
        Box::new(|target| target.backend = "emulated".into()),
        Box::new(|target| target.library.clear()),
    ];
    for (index, mutation) in cases.iter().enumerate() {
        let mut profile = target(&import);
        mutation(&mut profile);
        assert!(
            CounterSourceTask::derive(&import, "fixture.c", CONTRACT, ROOT_QCORE_TYPE, profile)
                .is_err(),
            "target mutation {index}"
        );
    }
    assert!(
        CounterSourceTask::derive(&import, "", CONTRACT, ROOT_QCORE_TYPE, target(&import)).is_err()
    );
    assert!(
        CounterSourceTask::derive(&import, "fixture.c", "", ROOT_QCORE_TYPE, target(&import))
            .is_err()
    );
    assert!(
        CounterSourceTask::derive(&import, "fixture.c", CONTRACT, "", target(&import)).is_err()
    );
}

#[test]
fn unsupported_paths_stay_unknown_at_import_or_evaluation() {
    if !clang_available() {
        return;
    }
    // A call is outside the projection vocabulary: the function keeps the
    // existing Unknown result and no projection is produced at all.
    let with_call = SOURCE.replace(
        "ctx->total_chunks--;",
        "extern void sink(void); sink(); ctx->total_chunks--;",
    );
    let results = import_counter_sources(
        &C11TranslationUnit {
            source: &with_call,
            ..Default::default()
        },
        &revision(),
        &[FUNCTION],
        &["cio_ctx"],
        ROOT_C_TYPE,
    )
    .unwrap();
    assert!(matches!(
        results.as_slice(),
        [CounterSourceResult::Unknown { .. }]
    ));
    // A second field write projects, but the core's contract evaluation still
    // refuses it, so no witness can discharge the obligation.
    let two_writes = SOURCE.replace("ctx->total_chunks--;", "ctx->other--; ctx->total_chunks--;");
    let import = imported(&two_writes, vec![]);
    let replay = task(&import, CONTRACT);
    assert!(replay.evaluate().is_err());
    assert!(
        replay
            .check(replay.task(), &witness(&replay, reading(&replay, -1)))
            .is_err()
    );
    assert!(
        import_counter_sources(
            &C11TranslationUnit {
                source: SOURCE,
                ..Default::default()
            },
            &revision(),
            &[],
            &["cio_ctx"],
            ROOT_C_TYPE,
        )
        .is_err()
    );
}
