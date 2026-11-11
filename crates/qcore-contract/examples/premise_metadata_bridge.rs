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

//! Explicit actual-file bridge capture. No default evidence/source consumer.
use qcore_contract::premise_metadata_bridge;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 6 {
        return Err("PLAN EXPECTED_PLAN_SHA256 BASE_ROOT TIP_ROOT FRESH_OUTPUT".into());
    }
    let bytes = fs::read(&args[1]).map_err(|e| e.to_string())?;
    if qcore_schemas::sha256_hex(&bytes) != args[2] {
        return Err("independently held plan SHA mismatch".into());
    }
    let plan: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let mut old = BTreeMap::new();
    let mut new = BTreeMap::new();
    for module in plan["files"].as_array().ok_or("file inventory absent")? {
        let path = module["path"].as_str().ok_or("module path absent")?;
        if Path::new(path).is_absolute()
            || Path::new(path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err("module path must be strictly relative".into());
        }
        old.insert(
            path.to_owned(),
            fs::read_to_string(Path::new(&args[3]).join(path)).map_err(|e| e.to_string())?,
        );
        new.insert(
            path.to_owned(),
            fs::read_to_string(Path::new(&args[4]).join(path)).map_err(|e| e.to_string())?,
        );
    }
    let checked = premise_metadata_bridge::check(&bytes, &args[2], &old, &new)?;
    let pairs=old.iter().map(|(module,before)|json!({"module":module,"before":before,"after":new[module],"before_sha256":qcore_schemas::sha256_hex(before.as_bytes()),"after_sha256":qcore_schemas::sha256_hex(new[module].as_bytes())})).collect::<Vec<_>>();
    let out = Path::new(&args[5]);
    if out.exists() {
        return Err("fresh output required".into());
    }
    fs::write(out,serde_json::to_vec_pretty(&json!({"schema":"premise-symbolic-actual-source-bridge.v1","checked":checked,"actual_source_pairs":pairs,"plan_sha256":args[2],"source_truth":"open","verdict_mutations":0})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(())
}
