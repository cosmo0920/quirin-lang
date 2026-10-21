//! Executable accepted contract syntax. Supply explicit model/audit artifact bytes.
#[path = "support/premise_flow_fixture.rs"]
mod fixture;
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "MODEL_LEAN AUDIT_LEAN");
    let f = fixture::Fixture::new(
        &std::fs::read(&args[1]).unwrap(),
        &std::fs::read(&args[2]).unwrap(),
    );
    let results = ["and_flow", "or_flow", "not_flow", "end_flow"]
        .into_iter()
        .map(|entry| {
            qcore_contract::premise_flow::evaluate_proposed(&f.packet, f.held(), entry).unwrap()
        })
        .collect::<Vec<_>>();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"mode":"proposed logic only; assigned evidence; source binding Open","document":f.document,"explicit_artifact_paths":&args[1..],"results":results})).unwrap());
}
