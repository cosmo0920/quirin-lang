//! Card AUDIT-COMPOSITION: recompute every published unit row of a report
//! from its component sidecar and compare (`report_components::check`).
//!
//! Usage: `cargo run -p qcore-audit --example component_check -- REPORT [SIDECAR]`
//!
//! `SIDECAR` defaults to the report's own `<report>.components.json`. Prints
//! one JSON line with the rows, components, conflicts and per-verdict counts;
//! exits 1 naming the first disagreement.

use std::path::PathBuf;
use std::process::ExitCode;

use qcore_audit::report_components::{check, sidecar_path};
use qcore_schemas::composition::ComponentSidecar;

fn run() -> Result<String, String> {
    let mut arguments = std::env::args_os().skip(1);
    let report = PathBuf::from(
        arguments
            .next()
            .ok_or("usage: component_check REPORT [SIDECAR]")?,
    );
    let sidecar = arguments
        .next()
        .map_or_else(|| sidecar_path(&report), PathBuf::from);
    let read = |path: &PathBuf| {
        std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let report_bytes = read(&report)?;
    let mut deserializer = serde_json::Deserializer::from_slice(&report_bytes);
    deserializer.disable_recursion_limit();
    let report_value: serde_json::Value = serde::Deserialize::deserialize(&mut deserializer)
        .map_err(|error| format!("{}: {error}", report.display()))?;
    let sidecar_bytes = read(&sidecar)?;
    let mut deserializer = serde_json::Deserializer::from_slice(&sidecar_bytes);
    deserializer.disable_recursion_limit();
    let sidecar_value: ComponentSidecar = serde::Deserialize::deserialize(&mut deserializer)
        .map_err(|error| format!("{}: {error}", sidecar.display()))?;
    let summary = check(&report_value, &sidecar_value)?;
    Ok(serde_json::json!({
        "report": report.display().to_string(),
        "sidecar": sidecar.display().to_string(),
        "rows": summary.rows,
        "components": summary.components,
        "conflicts": summary.conflicts,
        "verdicts": summary
            .verdicts
            .iter()
            .map(|(verdict, count)| (verdict.as_str().to_owned(), *count))
            .collect::<std::collections::BTreeMap<_, _>>(),
        "result": "every published unit row re-derives from its components",
    })
    .to_string())
}

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example component_check");
    match run() {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("component_check: {error}");
            ExitCode::FAILURE
        }
    }
}
