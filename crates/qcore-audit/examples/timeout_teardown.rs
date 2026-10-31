use std::path::PathBuf;

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example timeout_teardown");
    let mut arguments = std::env::args_os().skip(1);
    let config = arguments
        .next()
        .ok_or("usage: timeout_teardown CONFIG.json")?;
    if arguments.next().is_some() {
        return Err("usage: timeout_teardown CONFIG.json".into());
    }
    qcore_audit::timeout_teardown::run_timeout_teardown_config(&PathBuf::from(config))
}
