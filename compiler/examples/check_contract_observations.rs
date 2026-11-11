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

//! Check native QCore predicates on JSONL observations; no static proof claim.
use qcore_compiler::contract_observation::{ObservationVerdict, check_contract_observation};
use qcore_compiler::{CompileOptions, SourceFile, compile_files};
use serde::Deserialize;
use std::{collections::BTreeMap, env, fs, process::ExitCode};
#[derive(Deserialize)]
struct Observation {
    function: String,
    case: u64,
    values: BTreeMap<String, i128>,
}
fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [contract_path, observations_path] = args.as_slice() else {
        return Err("usage: check_contract_observations CONTRACT.qcore OBSERVATIONS.jsonl".into());
    };
    let source = fs::read_to_string(contract_path).map_err(|e| e.to_string())?;
    compile_files(
        &[SourceFile::new(contract_path, source.clone())],
        &CompileOptions::default(),
    )
    .map_err(|e| format!("QCore compilation failed: {e:?}"))?;
    let observations = fs::read_to_string(observations_path).map_err(|e| e.to_string())?;
    let mut counts = BTreeMap::<String, usize>::new();
    let mut cases = std::collections::BTreeSet::new();
    for line in observations.lines() {
        let observation: Observation = serde_json::from_str(line).map_err(|e| e.to_string())?;
        if !cases.insert(observation.case) {
            return Err("duplicate observation case".into());
        }
        match check_contract_observation(&source, &observation.function, &observation.values)
            .map_err(|e| format!("case {}: {e}", observation.case))?
        {
            ObservationVerdict::Checked { .. } => {
                *counts.entry(observation.function).or_default() += 1
            }
            ObservationVerdict::OutsidePreconditions { predicate } => {
                return Err(format!(
                    "case {} is outside precondition: {predicate}",
                    observation.case
                ));
            }
        }
    }
    if cases.is_empty() {
        return Err("no observations".into());
    }
    println!(
        "{}",
        serde_json::json!({"kind":"runtime_contract_evaluation", "checked_executions": cases.len(), "functions":counts, "static_certificate":false})
    );
    Ok(())
}
fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example check_contract_observations");
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
