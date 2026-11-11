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

//! Export a report-owned candidate graph without granting it semantic authority.
use qcore_contract::hypothesis::{digest, replay};
use std::{collections::BTreeMap, env, fs};
fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example premise_graph");
    let args: Vec<_> = env::args().collect();
    assert_eq!(args.len(), 3, "CJSON_VARIANT OUTPUT");
    let sources = BTreeMap::from([("cjson.qv".into(), fs::read_to_string(&args[1]).unwrap())]);
    let graph = replay(&sources)
        .unwrap()
        .remove("cJSON_DetachItemViaPointer")
        .unwrap();
    let bytes = serde_json::to_vec(&graph).unwrap();
    fs::write(&args[2], bytes).unwrap();
    println!(
        "{}",
        serde_json::json!({"graph_sha256":digest(&graph).unwrap(),"atoms":graph.atoms.len(),"premises":graph.premises.len(),"standing":"report-owned, all opaque; no evidence or typed consumer"})
    );
}
