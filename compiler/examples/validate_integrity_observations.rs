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

//! Card O01: validate a recorded chunk-file integrity family and project it
//! onto the declared observation policy.
//!
//! The decision is `qcore_core::chunk_integrity`; this program only reads the
//! canonical record text and writes the observations the policy consumes. A
//! refusal exits nonzero and writes nothing, so a forged record can never reach
//! the policy evaluation.
use qcore_core::chunk_integrity::{parse_records, policy_values, validate_case};
use serde_json::json;
use std::{collections::BTreeMap, env, fs, process::ExitCode};

fn run() -> Result<String, String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [records_path, observations_path] = args.as_slice() else {
        return Err("usage: validate_integrity_observations RECORDS.txt OBSERVATIONS.jsonl".into());
    };
    let text = fs::read_to_string(records_path).map_err(|error| error.to_string())?;
    let records = parse_records(&text).map_err(|refusal| refusal.to_string())?;
    let mut lines = String::new();
    let mut classes = BTreeMap::<String, usize>::new();
    let mut injections = BTreeMap::<String, usize>::new();
    for record in &records {
        let verdict =
            validate_case(record).map_err(|refusal| format!("case {}: {refusal}", record.case))?;
        *classes
            .entry(verdict.derived_class.label().to_string())
            .or_default() += 1;
        *injections
            .entry(verdict.injection.label().to_string())
            .or_default() += 1;
        let values: BTreeMap<&str, i128> = policy_values(record, &verdict).into_iter().collect();
        lines.push_str(
            &json!({
                "function": "native_file_integrity_observation",
                "case": record.case,
                "values": values,
            })
            .to_string(),
        );
        lines.push('\n');
    }
    fs::write(observations_path, lines).map_err(|error| error.to_string())?;
    Ok(json!({
        "kind": "chunk_integrity_validation",
        "validated_executions": records.len(),
        "content_classes": classes,
        "injected_operations": injections,
        "static_certificate": false,
        "durability_proved": false,
    })
    .to_string())
}

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example validate_integrity_observations");
    match run() {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
