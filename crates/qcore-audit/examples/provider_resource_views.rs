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

//! Card W44-PROVIDER-RESOURCE-VIEW: print, as JSON, the resource view the
//! resource-contract loader admits for every crate on a contract load path
//! (`qcore_audit::providers::probe_resource_views`). `providers/contracts.py`
//! runs this to generate (`sync`) and check (`check`) each crate's
//! `resource_view` in `providers/contract-load-path.json`.

use std::path::PathBuf;

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example provider_resource_views");
    let usage = "usage: provider_resource_views LOAD_PATH.json";
    let mut arguments = std::env::args_os().skip(1);
    let load_path = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let probe = qcore_audit::providers::probe_resource_views(&PathBuf::from(load_path))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&probe).map_err(|error| error.to_string())?
    );
    Ok(())
}
