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

//! Card **QUANT-INDEX-CHECKS** measurement tool: the bounds property (gateway
//! rules 11s and 12l) of selected functions of one C file, with a bounds
//! contract module the caller names.
//!
//! ```sh
//! cargo run -p qcore-c-gateway --example quant_index_probe -- \
//!     probe.c contracts.qv INCLUDE_DIR FUNCTION...
//! ```
//!
//! The audit routes run the bounds consumer with an empty bounds environment
//! (`CG16-BOUNDS-CONTRACT-TRANSPORT`), so this API call is the only route that
//! selects a bounds module today. It prints the bounds report as JSON and
//! publishes no custody verdict.

use std::collections::BTreeMap;

use qcore_c_gateway::index_extent::analyze_bounds_c11;
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::bounds_contract::load_bounds_contract_sources;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example quant_index_probe");
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [source, contract, include, functions @ ..] = arguments.as_slice() else {
        return Err("usage: SOURCE.c CONTRACT.qv INCLUDE_DIR FUNCTION...".into());
    };
    if functions.is_empty() {
        return Err("name at least one function".into());
    }
    let text = std::fs::read_to_string(source)?;
    let contracts = load_bounds_contract_sources(&BTreeMap::from([(
        contract.clone(),
        std::fs::read_to_string(contract)?,
    )]))?;
    let report = analyze_bounds_c11(
        &C11TranslationUnit {
            source: &text,
            include_paths: vec![include.clone()],
            ..Default::default()
        },
        functions,
        &contracts,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
