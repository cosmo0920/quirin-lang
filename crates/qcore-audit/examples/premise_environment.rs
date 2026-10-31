//! Reconstruct a contract environment for exact premise-migration provenance.
use qcore_contract::contract_loader::load_resource_contract_manifest;
use std::{fs, path::Path};
fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example premise_environment");
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 4, "MANIFEST SOURCE_ROOT OUTPUT");
    let manifest = Path::new(&args[1]);
    let mut contracts = load_resource_contract_manifest(manifest).unwrap();
    let text = fs::read_to_string(manifest).unwrap();
    qcore_audit::contract_revision::apply_pins(
        &mut contracts,
        &qcore_audit::contract_revision::ModuleOwners::for_manifest(manifest, &text).unwrap(),
        Path::new(&args[2]),
    )
    .unwrap();
    let bytes = serde_json::to_vec(&contracts).unwrap();
    let sources = serde_json::to_vec(contracts.source_assumptions()).unwrap();
    let functions = serde_json::to_vec(contracts.functions()).unwrap();
    println!(
        "{}",
        serde_json::json!({"environment_sha256":qcore_schemas::sha256_hex(&bytes), "source_assumptions_sha256":qcore_schemas::sha256_hex(&sources), "function_table_sha256":qcore_schemas::sha256_hex(&functions)})
    );
    fs::write(&args[3], bytes).unwrap();
}
