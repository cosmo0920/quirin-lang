//! Read-only canonical replay of already produced tasks; no source authority.
use qcore_schemas::obligation::VerificationTask;
use serde_json::{Value, json};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input: Value = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut receipts = Vec::new();
    for row in input["records"].as_array().unwrap() {
        let before: VerificationTask = serde_json::from_value(row["before"]["task"].clone()).unwrap();
        let mut after: VerificationTask = serde_json::from_value(row["after"]["task"].clone()).unwrap();
        for (label, task) in [("before", &before), ("after", &after)] {
            task.validate().unwrap();
            assert_eq!(task.task_digest(), row[label]["witness"]["task_digest"].as_str().unwrap());
            for obligation in &task.obligations {
                assert_eq!(obligation.obligation_id, task.obligation_id(obligation.capability, obligation.property, &obligation.subject));
            }
        }
        let before_digest = before.task_digest();
        let after_digest = after.task_digest();
        let after_ids: Vec<_> = after.obligations.iter().map(|o| o.obligation_id.clone()).collect();
        after.unit.build_digest = before.unit.build_digest.clone();
        after.contract_sources.insert("qcore:boolean-field-import:v1".into(), before.contract_sources["qcore:boolean-field-import:v1"].clone());
        let restored_ids: Vec<_> = after.obligations.iter().map(|o| after.obligation_id(o.capability, o.property, &o.subject)).collect();
        for (obligation, id) in after.obligations.iter_mut().zip(&restored_ids) { obligation.obligation_id = id.clone(); }
        assert_eq!(before, after, "some non-derived task field changed");
        let mut negative = before.clone();
        negative.unit.build_digest = "0".repeat(64);
        assert_ne!(negative.task_digest(), before_digest);
        assert_ne!(negative.obligation_id(negative.obligations[0].capability, negative.obligations[0].property, &negative.obligations[0].subject), restored_ids[0]);
        receipts.push(json!({"index":row["index"], "before_task_digest":before_digest, "after_task_digest":after_digest, "before_obligation_ids":restored_ids, "after_obligation_ids":after_ids, "complete_tasks_equal_after_exact_import_identity_restoration":true, "wrong_build_digest_negative_control_rejected":true}));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
}
