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

#[path = "fixtures/boolean_source.rs"]
mod fixture;
use fixture::*;
use qcore_schemas::SemanticWitness;
use qcore_schemas::chunk_summary::BooleanFieldContractWitness;
use qcore_verification::source_effect::BooleanFieldSourceTask;
use qcore_verifier_host::WitnessChecker;

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent source-effect test: {reason}");
            false
        }
    }
}

fn witness(
    task: &BooleanFieldSourceTask,
    summary: &qcore_c_gateway::boolean_source::ChunkSourceSummary,
) -> SemanticWitness {
    SemanticWitness::BooleanFieldContract(Box::new(BooleanFieldContractWitness {
        task_digest: task.task().task_digest(),
        summary: (**summary).clone(),
    }))
}

#[test]
fn real_c_prestates_and_contract_are_recomputed() {
    if !clang_available() {
        return;
    }
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    assert_eq!(replay.evaluate().unwrap().checked_states(), 4);
    assert_eq!(
        replay
            .check(replay.task(), &witness(&replay, &summary))
            .unwrap(),
        replay.task().coverage_for(replay.capability())
    );
    let false_contract = CONTRACT.replace("#result == #old_lock", "#result == 7");
    let rejected = task(&summary, &false_contract);
    assert!(
        rejected
            .check(rejected.task(), &witness(&rejected, &summary))
            .is_err(),
        "exact paths alone cannot discharge an invalid postcondition"
    );
}

#[test]
fn forged_paths_frames_and_source_assumptions_are_rejected() {
    if !clang_available() {
        return;
    }
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    for mutation in 0..11 {
        let mut proposal = witness(&replay, &summary);
        let SemanticWitness::BooleanFieldContract(value) = &mut proposal else {
            unreachable!()
        };
        match mutation {
            0 => value.summary.paths.swap(0, 1),
            1 => value.summary.paths[0].return_value = 7,
            2 => value.summary.paths[0].writes.push(true),
            3 => value.summary.paths[0].final_value = true,
            4 => value.summary.paths[1].branches.push(false),
            5 => value.summary.paths[0]
                .frame_writes
                .push(("error_n".into(), 0)),
            6 => {
                value
                    .summary
                    .dependencies
                    .insert("forged".into(), "source".into());
            }
            7 => {
                value.summary.profile_assumptions.clear();
            }
            8 => value.summary.source_revision = "forged".into(),
            9 => value.summary.function = "other".into(),
            10 => value.task_digest = "a".repeat(64),
            _ => unreachable!(),
        }
        assert!(
            replay.check(replay.task(), &proposal).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn actual_source_build_and_contract_snapshots_separate_tasks() {
    if !clang_available() {
        return;
    }
    let original = checked(SOURCE, vec![]);
    let source_changed = checked(&format!("/* actual bytes changed */{SOURCE}"), vec![]);
    let build_changed = checked(SOURCE, vec!["QCORE_UNUSED=1".into()]);
    let replay = task(&original, CONTRACT);
    for changed in [source_changed, build_changed] {
        let other = task(&changed, CONTRACT);
        assert_ne!(replay.task().task_digest(), other.task().task_digest());
        assert!(
            replay
                .check(other.task(), &witness(&other, &changed))
                .is_err()
        );
    }
    let other = task(
        &original,
        &format!("// contract identity changed\n{CONTRACT}"),
    );
    assert_ne!(replay.task().task_digest(), other.task().task_digest());
    let mut changed = replay.task().clone();
    changed
        .assumptions
        .insert("exclusive_chunk_access".into(), false);
    assert!(
        replay
            .check(&changed, &witness(&replay, &original))
            .is_err()
    );
}

/// A used header or generated configuration header can change the effective
/// preprocessing input while the selected source bytes and the checked paths
/// stay identical. The task and its witness must retire anyway, in both
/// directions: evidence filed under the old input is never reusable.
#[test]
fn a_changed_header_expansion_retires_the_prior_task_and_its_witness() {
    if !clang_available() {
        return;
    }
    const CONDITIONAL: &str = "struct cio_chunk { int lock; };\n#ifdef QCORE_GENERATED_CONFIG\ntypedef int qcore_generated_spare;\n#endif\nint cio_chunk_is_locked(struct cio_chunk *ch) { return ch->lock; }";
    let before = checked(CONDITIONAL, vec![]);
    let after = checked(CONDITIONAL, vec!["QCORE_GENERATED_CONFIG=1".into()]);
    assert_eq!(before.paths, after.paths, "the checked paths are unchanged");
    assert_eq!(
        before.input_identity().raw_source_digest(),
        after.input_identity().raw_source_digest(),
        "the selected source bytes are unchanged"
    );
    assert_ne!(
        before.input_identity().preprocessed_source_digest(),
        after.input_identity().preprocessed_source_digest(),
        "the effective preprocessing input did change"
    );
    let before_task = task(&before, CONTRACT);
    let after_task = task(&after, CONTRACT);
    assert_eq!(
        before_task.task().unit.source_digest,
        after_task.task().unit.source_digest
    );
    assert_ne!(
        before_task.task().unit.build_digest,
        after_task.task().unit.build_digest
    );
    assert_ne!(
        before_task.task().task_digest(),
        after_task.task().task_digest()
    );
    assert!(
        before_task
            .check(before_task.task(), &witness(&after_task, &after))
            .is_err(),
        "a witness for the newer input was admitted against the retired task"
    );
    assert!(
        after_task
            .check(after_task.task(), &witness(&before_task, &before))
            .is_err(),
        "a witness for the retired input was admitted against the newer task"
    );
    // Each task still discharges its own witness, so this is separation, not
    // a blanket refusal.
    for (replay, summary) in [(&before_task, &before), (&after_task, &after)] {
        assert_eq!(
            replay
                .check(replay.task(), &witness(replay, summary))
                .unwrap(),
            replay.task().coverage_for(replay.capability())
        );
    }
}

#[test]
fn target_substitution_and_unsupported_selected_functions_reject() {
    if !clang_available() {
        return;
    }
    let summary = checked(SOURCE, vec![]);
    for mutation in 0..5 {
        let mut profile = target(&summary);
        match mutation {
            0 => profile.dialect = "gnu17".into(),
            1 => profile.revision = "other".into(),
            2 => profile.platform = "other".into(),
            3 => profile.backend = "other".into(),
            4 => profile.library = "fluent_bit".into(),
            _ => unreachable!(),
        }
        assert!(BooleanFieldSourceTask::derive(&summary, "fixture.c", CONTRACT, profile).is_err());
    }
    for source in [
        SOURCE.replace("return ch->lock;", "while (ch->lock) {} return 0;"),
        SOURCE.replace("int cio_chunk_is_locked", "long cio_chunk_is_locked"),
        SOURCE.replace("int lock;", "volatile int lock;"),
    ] {
        assert!(matches!(
            imports(&source, vec![]).as_slice(),
            [qcore_c_gateway::boolean_source::ChunkSourceResult::Unknown { .. }]
        ));
    }
    let source =
        format!("{SOURCE} int extra(struct cio_chunk *ch) {{ while(ch->lock) {{}} return 0; }}");
    let results = imports(&source, vec![]);
    assert_eq!(results.len(), 2, "unsupported functions stay in inventory");
    assert!(results.iter().any(|r| matches!(r, qcore_c_gateway::boolean_source::ChunkSourceResult::Unknown { function, .. } if function == "extra")));
}

const UNIT_CONTRACT: &str = "module contract; extern fn reset(ch: borrow<'call, mut, cio_chunk> use one)->unit contract { requires object_is_live; requires exclusive_object_access; requires field_is_boolean; ensures normal field == 1; };";

#[test]
fn void_field_source_has_explicit_unit_abi_and_two_checked_states() {
    if !clang_available() {
        return;
    }
    use qcore_schemas::chunk_summary::BooleanFieldResult;
    for body in [
        "ch->lock=1;",
        "ch->lock=1; return;",
        "if (ch->lock) { return; } ch->lock=1;",
    ] {
        let summary = checked(
            &format!("struct cio_chunk {{int lock;}}; void reset(struct cio_chunk *ch) {{{body}}}"),
            vec![],
        );
        assert_eq!(summary.result_kind, BooleanFieldResult::Unit);
        assert_eq!(summary.source_input().schema_version, 3);
        assert!(
            summary
                .paths
                .iter()
                .all(|path| path.return_value == 0 && path.final_value)
        );
        let replay = task(&summary, UNIT_CONTRACT);
        assert_eq!(
            replay.task().semantic_profile,
            "qcore.c_boolean_field_contract.v3"
        );
        assert_eq!(replay.evaluate().unwrap().checked_states(), 2);
        assert!(
            replay
                .request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap())
                .validate()
                .is_ok()
        );
        assert_eq!(
            replay
                .check(replay.task(), &witness(&replay, &summary))
                .unwrap()
                .len(),
            1
        );
    }
}

#[test]
fn void_contracts_and_proposals_cannot_forge_a_scalar_result_or_record_identity() {
    if !clang_available() {
        return;
    }
    use qcore_schemas::chunk_summary::BooleanFieldResult;
    let summary = checked(
        "struct cio_chunk {int lock;}; void reset(struct cio_chunk *ch) { ch->lock=1; }",
        vec![],
    );
    for bad in [
        UNIT_CONTRACT.replace("->unit", "->i32"),
        UNIT_CONTRACT.replace("field == 1", "result == 0"),
        UNIT_CONTRACT.replace("cio_chunk>", "Unrelated>"),
        UNIT_CONTRACT.replace("field == 1", "field == 0"),
    ] {
        assert!(task(&summary, &bad).evaluate().is_err());
    }
    let replay = task(&summary, UNIT_CONTRACT);
    for mutation in 0..4 {
        let mut proposal = witness(&replay, &summary);
        let SemanticWitness::BooleanFieldContract(value) = &mut proposal else {
            unreachable!()
        };
        match mutation {
            0 => value.summary.result_kind = BooleanFieldResult::I32,
            1 => value.summary.paths[0].return_value = 1,
            2 => {
                value
                    .summary
                    .profile_assumptions
                    .insert("boolean_field_record".into(), "Other".into());
            }
            3 => {
                value
                    .summary
                    .profile_assumptions
                    .insert("boolean_field_name".into(), "other".into());
            }
            _ => unreachable!(),
        }
        assert!(replay.check(replay.task(), &proposal).is_err());
    }
    let mut request = replay.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
    request.source_effect.as_mut().unwrap().schema_version = 2;
    assert!(request.validate().is_err());
    let mut mismatched = target(&summary);
    mismatched.profile = "qcore.c_boolean_field_contract.v2".into();
    assert!(
        BooleanFieldSourceTask::derive(&summary, "fixture.c", UNIT_CONTRACT, mismatched).is_err()
    );
}

#[test]
fn integer_profile_bytes_and_missing_return_rejection_are_preserved() {
    if !clang_available() {
        return;
    }
    let summary = checked(SOURCE, vec![]);
    let input = summary.source_input();
    assert_eq!(input.schema_version, 2);
    assert!(
        !serde_json::to_string(&input)
            .unwrap()
            .contains("result_kind")
    );
    assert!(
        !serde_json::to_string(&*summary)
            .unwrap()
            .contains("result_kind")
    );
    assert_eq!(
        task(&summary, CONTRACT)
            .evaluate()
            .unwrap()
            .checked_states(),
        4
    );
    for source in [
        "struct cio_chunk {int lock;}; int reset(struct cio_chunk *ch) {ch->lock=1;}",
        "struct cio_chunk {int lock;}; void reset(struct cio_chunk *ch) {ch->lock=2;}",
    ] {
        assert!(matches!(
            imports(source, vec![]).as_slice(),
            [qcore_c_gateway::boolean_source::ChunkSourceResult::Unknown { .. }]
        ));
    }
}

#[test]
fn ordered_preprocessing_options_bind_source_tasks_even_when_output_is_identical() {
    if !clang_available() {
        return;
    }
    use qcore_c_gateway::boolean_source::{ChunkSourceResult, import_chunk_source_with_profile};
    use qcore_c_gateway::legacy_c::{C11TranslationUnit, CPreprocessorOption};
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let profile = qcore_contract::profile_loader::load_profile_directory(
        &root.join("contract/profiles/fluent-bit"),
    )
    .unwrap();
    let derive = |preprocessor_options| {
        let results = import_chunk_source_with_profile(
            &C11TranslationUnit {
                source: SOURCE,
                preprocessor_options,
                ..Default::default()
            },
            &profile.pin_for("chunkio").unwrap().git_commit,
            &profile,
            &["chunkio"],
            &[],
            include_str!(
                "../../../providers/quirin-provider-shared/contracts/library/source_projection.qv"
            ),
        )
        .unwrap();
        let ChunkSourceResult::Summary(summary) = results.into_iter().next().unwrap() else {
            panic!("expected imported source");
        };
        *summary
    };
    let original = derive(vec![]);
    let replay = task(&original, CONTRACT);
    let mut digests = std::collections::BTreeSet::from([replay.task().task_digest()]);
    for options in [
        vec![CPreprocessorOption::FrontendFlag(
            qcore_c_gateway::legacy_c::CFrontendFlag::AllWarnings,
        )],
        vec![CPreprocessorOption::FrontendFlag(
            qcore_c_gateway::legacy_c::CFrontendFlag::DebugInformation,
        )],
        vec![CPreprocessorOption::FrontendFlag(
            qcore_c_gateway::legacy_c::CFrontendFlag::AssemblerNoExecStack,
        )],
        vec![CPreprocessorOption::Define("UNUSED=1".into())],
        vec![CPreprocessorOption::Define("UNUSED=2".into())],
        vec![
            CPreprocessorOption::Define("UNUSED=1".into()),
            CPreprocessorOption::Undefine("UNUSED".into()),
        ],
        vec![
            CPreprocessorOption::Undefine("UNUSED".into()),
            CPreprocessorOption::Define("UNUSED=1".into()),
        ],
        vec![CPreprocessorOption::Include(
            root.to_string_lossy().into_owned(),
        )],
        vec![CPreprocessorOption::SystemInclude(
            root.to_string_lossy().into_owned(),
        )],
        vec![CPreprocessorOption::QuoteInclude(
            root.to_string_lossy().into_owned(),
        )],
    ] {
        let changed = derive(options);
        assert_eq!(original.paths, changed.paths);
        let other = task(&changed, CONTRACT);
        assert!(digests.insert(other.task().task_digest()));
        assert!(
            replay
                .check(other.task(), &witness(&other, &changed))
                .is_err()
        );
    }
}
