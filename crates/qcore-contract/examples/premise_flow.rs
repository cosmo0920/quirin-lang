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

//! Executable accepted contract syntax. Supply explicit model/audit artifact bytes.
#[path = "support/premise_flow_fixture.rs"]
mod fixture;
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3, "MODEL_LEAN AUDIT_LEAN");
    let f = fixture::Fixture::new(
        &std::fs::read(&args[1]).unwrap(),
        &std::fs::read(&args[2]).unwrap(),
    );
    let results = ["and_flow", "or_flow", "not_flow", "end_flow"]
        .into_iter()
        .map(|entry| {
            qcore_contract::premise_flow::evaluate_proposed(&f.packet, f.held(), entry).unwrap()
        })
        .collect::<Vec<_>>();
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"mode":"proposed logic only; assigned evidence; source binding Open","document":f.document,"explicit_artifact_paths":&args[1..],"results":results})).unwrap());
}
