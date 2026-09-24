//! Replay native descriptor observations under the lifecycle policy.
//!
//! Usage: check_descriptor_observations POLICY.qcore RECORDS.jsonl
//!
//! Prints one JSON report. Exit 0 means the records were accepted as evidence
//! (property violations are part of the report, not an error); exit 1 means
//! the records or policy were rejected. Never a static certificate.
use qcore_contract::descriptor_observation::check_descriptor_observations;
use std::{env, fs, process::ExitCode};

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [policy_path, records_path] = args.as_slice() else {
        return Err("usage: check_descriptor_observations POLICY.qcore RECORDS.jsonl".into());
    };
    let policy = fs::read_to_string(policy_path).map_err(|e| format!("{policy_path}: {e}"))?;
    let records = fs::read_to_string(records_path).map_err(|e| format!("{records_path}: {e}"))?;
    let report = check_descriptor_observations(&records, &policy)?;
    println!(
        "{}",
        serde_json::json!({"kind": "native_descriptor_observation", "report": report})
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
