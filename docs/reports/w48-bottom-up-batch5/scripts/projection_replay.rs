//! Exact production projection replay from held completed rows and runtime inputs.
//! This does not rerun custody/null analysis or discharge source-binding premises.
use qcore_schemas::contract::ResourceContracts;
use serde_json::{Value, json};
use std::{fs, path::Path};
fn read(path: &Value) -> Value {
    serde_json::from_slice(&fs::read(path.as_str().unwrap()).unwrap()).unwrap()
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "SPEC OUTPUT");
    let spec: Value = serde_json::from_slice(&fs::read(&args[1]).unwrap()).unwrap();
    let diagnostic = read(&spec["diagnostics"]);
    let sidecar = read(&spec["contracts"]);
    let units = diagnostic["units"].as_array().unwrap();
    let environment_value = read(&spec["environment"]);
    let environment: ResourceContracts = serde_json::from_value(environment_value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&environment).unwrap(), environment_value);
    assert_eq!(serde_json::to_value(environment.source_assumptions()).unwrap(),
        sidecar.get("sources").or_else(|| sidecar.get("contracts")).unwrap().clone());
    let null_text = fs::read_to_string(spec["null"].as_str().unwrap()).unwrap();
    let null_rows = null_text.lines().filter(|s| !s.is_empty())
        .map(|s| serde_json::from_str::<Value>(s).unwrap()).collect::<Vec<_>>();
    let actual_summary = read(&spec["null_summary"]);
    let derived_summary = qcore_audit::null_tally::summary(&null_rows);
    for (key, value) in derived_summary.as_object().unwrap() {
        assert_eq!(&actual_summary[key], value, "production null tally field {key}");
    }
    let supplemental_summary_keys = actual_summary.as_object().unwrap().keys()
        .filter(|key| derived_summary.get(key.as_str()).is_none()).cloned().collect::<Vec<_>>();
    let manifest = Path::new(spec["manifest"].as_str().unwrap());
    let database = Path::new(spec["database"].as_str().unwrap());
    let database_bytes = fs::read(database).unwrap();
    let entries = qcore_audit::compile_db::compile_entries(std::str::from_utf8(&database_bytes).unwrap()).unwrap();
    // The checker is single threaded; this variable selects projection only.
    unsafe { std::env::set_var(qcore_audit::outer_selection::PROJECTION, "private-replay-no-write"); }
    qcore_audit::outer_selection::select_for_route(
        spec["route"].as_str().unwrap(), manifest, &fs::read_to_string(manifest).unwrap(),
        Path::new(spec["source_root"].as_str().unwrap()),
    ).unwrap();
    qcore_audit::outer_selection::compute_reach(&entries).unwrap();
    let premises = qcore_audit::contract_dependencies::premises(&environment).unwrap();
    assert_eq!(premises, sidecar["contract_premises"]);
    let actual_outer = read(&spec["outer"]);
    let derived_outer = qcore_audit::outer_selection::project(units, Some(&null_rows), &premises).unwrap();
    assert_eq!(derived_outer, actual_outer, "complete production runtime projection");
    let mut wrong_units = units.clone();
    let mut wrong_status_checked = false;
    for unit in &mut wrong_units {
        for row in unit["functions"].as_array_mut().unwrap() {
            if row["status"] == "Safe" {
                row["status"] = json!("Unknown");
                wrong_status_checked = true;
                break;
            }
        }
        if wrong_status_checked { break; }
    }
    assert!(wrong_status_checked);
    assert_ne!(qcore_audit::outer_selection::project(&wrong_units, Some(&null_rows), &premises).unwrap(), actual_outer);
    let mut wrong_null = null_rows.clone();
    let mut wrong_premise_checked = false;
    for row in &mut wrong_null {
        if let Some(premise) = row.get_mut("custody_null_premises")
            .and_then(Value::as_array_mut).and_then(|v| v.first_mut()) {
            premise["null_property"] = json!("unprovided-standing");
            wrong_premise_checked = true;
            break;
        }
    }
    assert!(wrong_premise_checked);
    assert_ne!(qcore_audit::null_tally::summary(&wrong_null), derived_summary);
    let receipt = json!({
        "status": "production projection and null-tally replay exact",
        "complete_outer_projection_reproduced": true,
        "all_production_null_tally_fields_reproduced": true,
        "supplemental_null_summary_keys_for_separate_complete_difference_review": supplemental_summary_keys,
        "fresh_outer_pin_and_loader_reach_read": true,
        "compile_database_sha256": qcore_schemas::sha256_hex(&database_bytes),
        "wrong_custody_status_negative_rejected": true,
        "unprovided_null_premise_standing_negative_rejected": true,
        "source_binding": "Open",
        "truth_transport": false,
        "scope": "Exact replay of published rows, declared outer contracts and actual source/pin/reach inputs; no new custody/null source proof or outer-premise authority.",
    });
    fs::write(&args[2], serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}
