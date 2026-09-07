//! Policy evaluations consume declared data; they do not attest source import.
use qcore_contract::chunk_contract::evaluate_chunk_source_contract;
use qcore_schemas::chunk_summary::{ChunkSourceField, ChunkSourcePath, ChunkSourceSummaryRecord};
use std::collections::BTreeMap;

const POLICY: &str = r#"module contract;
extern fn clear_busy(chunk: borrow<'call, mut, InputChunk> use one) -> i32
contract {
    requires chunk_is_live;
    requires exclusive_chunk_access;
    requires busy_is_boolean;
    requires lock_is_boolean;
    ensures normal busy == 0;
    ensures normal result == old_busy - 1;
    ensures normal lock == old_lock;
};"#;

fn declared_record() -> ChunkSourceSummaryRecord {
    ChunkSourceSummaryRecord {
        result_kind: qcore_schemas::chunk_summary::BooleanFieldResult::I32,
        function: "clear_busy".into(),
        source_revision: "declared-test-premise".into(),
        field: ChunkSourceField::Busy,
        paths: [false, true].map(|initial| ChunkSourcePath {
            initial_value: initial,
            final_value: false,
            return_value: i32::from(initial) - 1,
            branches: vec![initial],
            writes: if initial { vec![false] } else { vec![] },
            frame_writes: vec![],
        }),
        dependencies: BTreeMap::new(),
        profile_assumptions: BTreeMap::from([("source_binding".into(), "not_verified".into())]),
    }
}

#[test]
fn declared_paths_can_be_evaluated_without_claiming_source_provenance() {
    let record = declared_record();
    let serialized = serde_json::to_value(&record).unwrap();
    let decoded: ChunkSourceSummaryRecord = serde_json::from_value(serialized).unwrap();
    assert_eq!(record, decoded);
    let result = evaluate_chunk_source_contract(&decoded, POLICY).unwrap();
    assert_eq!(result.checked_states(), 4);
    assert_eq!(result.function(), "clear_busy");
    assert!(
        evaluate_chunk_source_contract(&decoded, &POLICY.replace("busy == 0", "busy == 1"))
            .is_err()
    );
}

#[test]
fn incomplete_or_inconsistent_paths_cannot_hide_boolean_cases() {
    let mut record = declared_record();
    record.paths.swap(0, 1);
    assert!(evaluate_chunk_source_contract(&record, POLICY).is_err());
    record = declared_record();
    record.paths[1].initial_value = false;
    assert!(evaluate_chunk_source_contract(&record, POLICY).is_err());
    record = declared_record();
    record.paths[1].writes.clear();
    assert!(evaluate_chunk_source_contract(&record, POLICY).is_err());
    record = declared_record();
    record.paths[1].final_value = true;
    assert!(evaluate_chunk_source_contract(&record, POLICY).is_err());
    record = declared_record();
    record.paths[1].frame_writes.push(("lock".into(), 0));
    let contradictory = POLICY.replace(
        "ensures normal lock == old_lock;",
        "ensures normal lock == old_lock; ensures normal field_lock == 0;",
    );
    assert!(evaluate_chunk_source_contract(&record, &contradictory).is_err());
}

#[test]
fn new_transport_records_reject_unsupported_fields() {
    let mut json = serde_json::to_value(declared_record()).unwrap();
    json["proved"] = true.into();
    assert!(serde_json::from_value::<ChunkSourceSummaryRecord>(json).is_err());
    let mut json = serde_json::to_value(declared_record()).unwrap();
    json["paths"][0]["ignored_effect"] = true.into();
    assert!(serde_json::from_value::<ChunkSourceSummaryRecord>(json).is_err());
}
