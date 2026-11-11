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

//! Card MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: a reading aid. Runs rule
//! 30's body check on the member-path declarations of a contract file over
//! one unit's functions, and prints each declaration's standing with the
//! reading's reason.
//!
//! ```text
//! cargo run -p qcore-c-gateway --example member_path_probe -- \
//!     CONTRACTS.qv FUNCTIONS.json TAG NAME...
//! ```
//!
//! `FUNCTIONS.json` is the unit's top-level function definitions (the card's
//! `scripts/unit_functions.py`); `TAG` is the record every stated formal's
//! pointee is read as (the probe does not read typedefs, unlike the
//! gateway's `MemberFacts::record_of_node`). Not the arbiter: the corpus
//! measurement is the published standing.

use std::collections::BTreeMap;

use qcore_c_gateway::member_path_check::{Standing, validates};
use qcore_contract::contract_loader::load_resource_contract_sources;
use serde::Deserialize;
use serde_json::Value;

fn run(arguments: Vec<String>) -> Result<(), String> {
    let [contracts, functions, tag, names @ ..] = arguments.as_slice() else {
        return Err("usage: CONTRACTS.qv FUNCTIONS.json TAG NAME...".into());
    };
    let text = std::fs::read_to_string(contracts).map_err(|error| error.to_string())?;
    let environment = load_resource_contract_sources(&BTreeMap::from([(contracts.clone(), text)]))?;
    let bytes = std::fs::read(functions).map_err(|error| error.to_string())?;
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    deserializer.disable_recursion_limit();
    let functions =
        Vec::<Value>::deserialize(&mut deserializer).map_err(|error| error.to_string())?;
    let mut by_name: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for function in &functions {
        if let Some(name) = function["name"].as_str() {
            by_name.entry(name).or_default().push(function);
        }
    }
    let unique = |name: &str| match by_name.get(name).map(Vec::as_slice) {
        Some([function]) => Some(*function),
        _ => None,
    };
    for name in names {
        let Some(declaration) = environment.retaining_contract(name) else {
            println!("{name}: no retaining declaration");
            continue;
        };
        let Some(function) = unique(name) else {
            println!("{name}: no unique body in the unit");
            continue;
        };
        let standing = validates(
            function,
            &declaration.member_paths,
            &|_| Some(tag.clone()),
            &|callee| unique(callee),
        );
        match standing {
            Ok(Standing::Validated) => println!("{name}: validated"),
            Ok(Standing::Assumption(why)) => println!("{name}: declared assumption: {why}"),
            Err(why) => println!("{name}: contradiction: {why}"),
        }
    }
    Ok(())
}

fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example member_path_probe");
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    // Deep Clang ASTs: read and walk them on a large stack.
    let worker = std::thread::Builder::new()
        .stack_size(1 << 29)
        .spawn(move || run(arguments))
        .expect("the probe thread starts");
    match worker.join().expect("the probe thread finishes") {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
