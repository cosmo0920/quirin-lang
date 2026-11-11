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

//! Card **LOADER-ABI-MODES** measurement tool: decide declared-mode /
//! derived-summary agreement over already-preprocessed translation units.
//!
//! ```sh
//! cargo run -p qcore-c-gateway --example declared_mode_census -- \
//!     cycle19-resource-contracts.json UNIT.i [UNIT.i ...]
//! ```
//!
//! It runs the production decision (`legacy_c::declared_mode_census`) and prints
//! one row per (unit, contracted name). It publishes no verdict; it exists so
//! the card's census is measured with the same code the analyzer applies rather
//! than a second implementation.
//!
//! The row vocabulary sizes the whole family, which is the point: `undefined`
//! and `no-summary` are the two ways a declared mode is *structurally
//! uncheckable* here, and a corpus of library contracts is expected to be
//! almost entirely `undefined` — the bodies live in other translation units.
//! That is a measurement about where the sources are, not about whether the
//! declarations are faithful.

use std::collections::BTreeMap;
use std::path::PathBuf;

use qcore_c_gateway::legacy_c::{ModeOutcome, declared_mode_census};
use qcore_contract::contract_loader::{
    load_resource_contract_files, load_resource_contract_manifest,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example declared_mode_census");
    let mut arguments = std::env::args().skip(1).peekable();
    let first = arguments
        .next()
        .ok_or("usage: MANIFEST|MODULE.qcore... -- UNIT.i ...")?;
    // Either the production manifest, or an explicit module list ended by `--`,
    // which is how the test fixtures' own contract modules are swept.
    let contracts = if first.ends_with(".json") {
        load_resource_contract_manifest(&PathBuf::from(first))?
    } else {
        let mut modules = vec![PathBuf::from(first)];
        for argument in arguments.by_ref() {
            if argument == "--" {
                break;
            }
            modules.push(PathBuf::from(argument));
        }
        load_resource_contract_files(&modules)?
    };
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    for unit in arguments {
        let preprocessed = std::fs::read_to_string(&unit)?;
        for row in declared_mode_census(&preprocessed, &contracts) {
            let outcome = match row.outcome {
                ModeOutcome::Undefined => "undefined".to_owned(),
                ModeOutcome::NoSummary => "no-summary".to_owned(),
                ModeOutcome::Agreed => "agreed".to_owned(),
                ModeOutcome::Disagreed => "disagreed".to_owned(),
                ModeOutcome::Contradicted(refusal) => {
                    format!("contradicted:{}", refusal.as_str())
                }
            };
            // An `undefined` row for a name this unit never mentions is noise:
            // it says only that the contract corpus is larger than this unit.
            // Every other outcome is printed unconditionally, so a withdrawal
            // can never be filtered away.
            if row.outcome == ModeOutcome::Undefined && !preprocessed.contains(&row.function) {
                *totals
                    .entry("undefined-unmentioned".to_owned())
                    .or_default() += 1;
                continue;
            }
            println!("{unit}\t{}\t{outcome}", row.function);
            *totals.entry(outcome).or_default() += 1;
        }
    }
    println!("--- totals");
    for (outcome, count) in totals {
        println!("{outcome}\t{count}");
    }
    Ok(())
}
