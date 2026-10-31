//! Explicit opt-in native fixture/replay, not part of the production audit route.
use quirin_source_analysis::plugin_source_witnesses::{self as witness, Observation};
use std::{fs, path::PathBuf};
fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example plugin_source_witnesses");
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: plugin_source_witnesses FRESH_OUTPUT_DIRECTORY")?,
    );
    fs::create_dir(&root).map_err(|e| e.to_string())?;
    let observed = witness::observe(&root.join("observe"))?;
    let context = witness::context(&observed)?;
    fs::write(
        root.join("presented.json"),
        serde_json::to_vec_pretty(&observed).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // JSON does not create a checked receipt; the actual source/native replay
    // below is mandatory even after this successful wire deserialization.
    let presented: Observation =
        serde_json::from_slice(&fs::read(root.join("presented.json")).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let checked = witness::replay(&presented, &context, &root.join("independent-replay"))?;
    let flow = checked.evaluate_duplicate_flow()?;
    let controls = witness::refusal_controls(&presented, &root.join("refused"))?;
    fs::write(
        root.join("refusal-controls.json"),
        serde_json::to_vec_pretty(&controls).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        root.join("checked.json"),
        serde_json::to_vec_pretty(&checked).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        root.join("bounded-flow.json"),
        serde_json::to_vec_pretty(&flow).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "checked {} selected cases; source-bound terminal flow serialized",
        checked.cases().len()
    );
    Ok(())
}
