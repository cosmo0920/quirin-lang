#[path = "fixtures/boolean_source.rs"]
mod fixture;
use fixture::*;
use qcore_schemas::SemanticWitness;
use qcore_schemas::chunk_summary::BooleanFieldContractWitness;
use qcore_verification::source_effect::BooleanFieldSourceTask;
use qcore_verifier_host::WitnessChecker;

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
    let summary = checked(SOURCE, vec![]);
    let replay = task(&summary, CONTRACT);
    assert_eq!(replay.evaluate().unwrap().checked_states(), 4);
    assert_eq!(
        replay
            .check(replay.task(), &witness(&replay, &summary))
            .unwrap(),
        replay.task().coverage_for(replay.capability())
    );
    let false_contract = CONTRACT.replace("result == old_lock", "result == 7");
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

#[test]
fn target_substitution_and_unsupported_selected_functions_reject() {
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
