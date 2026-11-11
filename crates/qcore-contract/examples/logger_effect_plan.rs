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

//! Replay reviewed raw-source inputs. Emits a model receipt, never a verdict.
use std::{collections::BTreeMap, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("usage: logger_effect_plan DOCUMENT EXPECTED_SHA256 SOURCE_ROOT".into());
    }
    let document = std::fs::read(&args[1])?;
    if document.len() > 131_072 || qcore_schemas::sha256_hex(&document) != args[2] {
        return Err("document digest or byte bound".into());
    }
    let plan: qcore_contract::logger_effects::Plan = serde_json::from_slice(&document)?;
    let root = PathBuf::from(&args[3]);
    let mut sources = BTreeMap::new();
    for path in plan.sources.keys() {
        // Refuse traversal before any IO; the checker repeats this boundary.
        if path.starts_with('/')
            || path
                .split('/')
                .any(|s| s.is_empty() || s == "." || s == "..")
        {
            return Err("invalid source path".into());
        }
        sources.insert(path.clone(), std::fs::read(root.join(path))?);
    }
    let checked = qcore_contract::logger_effects::check(&document, &args[2], &sources)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "kind": "proposed-model-replay", "version": "logger-effects-v1",
            "model_sha256": checked.digest(), "reachable_nodes": checked.reachable(),
            "open_obligations": checked.open_obligations(), "terminals": checked.terminals(),
            "source_truth": "Open", "production_verdict_consumer": false,
            "recursion_termination": "Open; finite effect reachability is not C termination"
        }))?
    );
    Ok(())
}
