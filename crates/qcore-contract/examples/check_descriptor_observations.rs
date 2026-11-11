// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
