//! Re-render retained dependency metadata with the unchanged production code.
//! This checks report derivation, not C source correctness or new authority.
use qcore_audit::contract_dependencies::{Facts, premises, render};
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};
use std::{fs, path::Path};
fn read(path: &str) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let diagnostics = read(&args[1]);
    let contracts = read(&args[2]);
    let bytes = fs::read(&args[3]).unwrap();
    let environment: ResourceContracts = serde_json::from_slice(&bytes).unwrap();
    let source_field = if contracts.get("sources").is_some() { "sources" } else { "contracts" };
    assert_eq!(serde_json::to_value(environment.source_assumptions()).unwrap(), contracts[source_field]);
    let declared = premises(&environment).unwrap();
    let units = diagnostics["units"].as_array().unwrap();
    let actual = render(&Facts::of_rows(units), &contracts["contract_revision"], &declared);
    assert_eq!(actual, contracts["contract_dependencies"], "complete dependency metadata differs from production replay");
    let (unit_index, row_index) = units.iter().enumerate().find_map(|(i, unit)| {
        unit["functions"].as_array().unwrap().iter().enumerate().find_map(|(j, row)| {
            (row["status"] == "Safe" && row["contract_dependencies"]["declarations"].as_array().is_some_and(|v| !v.is_empty())).then_some((i, j))
        })
    }).unwrap();
    let mut wrong_status = units.clone();
    wrong_status[unit_index]["functions"][row_index]["status"] = json!("Unknown");
    assert_ne!(render(&Facts::of_rows(&wrong_status), &contracts["contract_revision"], &declared), actual);
    let mut wrong_context = units.clone();
    wrong_context[unit_index]["functions"][row_index]["hypothesis_context"]["build_identity"] = json!("0".repeat(64));
    assert_ne!(render(&Facts::of_rows(&wrong_context), &contracts["contract_revision"], &declared), actual);
    let (body_unit, name) = units.iter().enumerate().find_map(|(i, unit)| {
        unit["contract_body_evidence"]["validated"].as_object().and_then(|values| values.keys().next().map(|name| (i, name.clone())))
    }).unwrap();
    let mut wrong_body = units.clone();
    wrong_body[body_unit]["contract_body_evidence"]["validated"][&name].as_array_mut().unwrap().push(json!("unprovided-evidence"));
    assert_ne!(render(&Facts::of_rows(&wrong_body), &contracts["contract_revision"], &declared), actual);
    let receipt = json!({"complete_dependency_metadata_reproduced": true,
        "source_assumptions_match_retained_modules": true,
        "serialized_environment_sha256": qcore_schemas::sha256_hex(&bytes),
        "wrong_status_negative_rejected": true,
        "wrong_build_context_negative_rejected": true,
        "unprovided_body_method_negative_rejected": true,
        "diagnostics": args[1], "contracts": args[2], "environment": args[3],
        "negative_row": {"unit":unit_index,"row":row_index},
        "negative_body": {"unit":body_unit,"function":name},
        "meaning":"Fresh production Facts::of_rows + render replay preserves every count, declaration, clause, standing and caller-context field. It adds no source proof or graph authority."});
    fs::write(Path::new(&args[4]), serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}
