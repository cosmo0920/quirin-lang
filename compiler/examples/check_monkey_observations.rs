//! Check the descriptor/event identity projection of a native Monkey trace.
use qcore_compiler::verifier_plugins::libevent::monkey_observation::check_monkey_observations;
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let [path] = args.as_slice() else {
        return Err("usage: check_monkey_observations OBSERVATIONS.jsonl".into());
    };
    let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let report = check_monkey_observations(&source)?;
    println!(
        "{}",
        serde_json::json!({"kind": "runtime_identity_projection", "checked": report, "static_certificate": false})
    );
    Ok(())
}

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example check_monkey_observations");
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
