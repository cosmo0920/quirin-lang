//! Card W44-PROVIDER-RESOURCE-VIEW: print, as JSON, the resource view the
//! resource-contract loader admits for every crate on a contract load path
//! (`qcore_audit::providers::probe_resource_views`). `providers/contracts.py`
//! runs this to generate (`sync`) and check (`check`) each crate's
//! `resource_view` in `providers/contract-load-path.json`.

use std::path::PathBuf;

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example provider_resource_views");
    let usage = "usage: provider_resource_views LOAD_PATH.json";
    let mut arguments = std::env::args_os().skip(1);
    let load_path = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let probe = qcore_audit::providers::probe_resource_views(&PathBuf::from(load_path))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&probe).map_err(|error| error.to_string())?
    );
    Ok(())
}
