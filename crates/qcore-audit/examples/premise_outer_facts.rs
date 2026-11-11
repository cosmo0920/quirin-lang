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

//! Publish every row's actual closed-world classification, including Unknown rows.
use qcore_audit::outer_selection::custody_facts;
use serde_json::{Value, json};
use std::{env, fs};
fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example premise_outer_facts");
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 4, "DIAGNOSTICS CONTRACTS OUTPUT");
    let report: Value = serde_json::from_slice(&fs::read(&args[1]).unwrap()).unwrap();
    let contracts: Value = serde_json::from_slice(&fs::read(&args[2]).unwrap()).unwrap();
    let premises =
        qcore_audit::contract_dependencies::premise_view_from_sidecar(&contracts).unwrap();
    let mut records = Vec::new();
    for (unit_index, unit) in report["units"].as_array().unwrap().iter().enumerate() {
        for (row_index, row) in unit["functions"].as_array().unwrap().iter().enumerate() {
            let facts: Vec<_> = custody_facts(&row["contract_dependencies"], &premises)
                .into_iter()
                .map(|fact| format!("{fact:?}"))
                .collect();
            records.push(json!({"unit_index":unit_index,"row_index":row_index,
                "library":unit["library"],"file":unit["file"],"function":row["function"],
                "status":row["status"],"facts":facts}));
        }
    }
    fs::write(&args[3], serde_json::to_vec_pretty(&records).unwrap()).unwrap();
}
