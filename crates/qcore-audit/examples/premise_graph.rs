//! Export a report-owned candidate graph without granting it semantic authority.
use qcore_contract::hypothesis::{digest, replay};
use std::{collections::BTreeMap, env, fs};
fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example premise_graph");
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "CJSON_VARIANT OUTPUT");
    let sources = BTreeMap::from([("cjson.qv".into(), fs::read_to_string(&args[1]).unwrap())]);
    let graph = replay(&sources)
        .unwrap()
        .remove("cJSON_DetachItemViaPointer")
        .unwrap();
    let bytes = serde_json::to_vec(&graph).unwrap();
    fs::write(&args[2], bytes).unwrap();
    println!(
        "{}",
        serde_json::json!({"graph_sha256":digest(&graph).unwrap(),"atoms":graph.atoms.len(),"premises":graph.premises.len(),"standing":"report-owned, all opaque; no evidence or typed consumer"})
    );
}
