//! Card **LOADER-ABI** measurement tool: decide declared-contract / C-declaration
//! ABI agreement over already-preprocessed translation units.
//!
//! ```sh
//! cargo run -p qcore-c-gateway --example declared_abi_census -- \
//!     cycle19-resource-contracts.json UNIT.i [UNIT.i ...]
//! ```
//!
//! It runs the production decision (`legacy_c::declared_abi_census`) and prints
//! one row per (unit, contracted name). It analyses nothing and publishes no
//! verdict; it exists so the card's sweep is measured with the same code the
//! analyzer applies rather than a second implementation.

use std::collections::BTreeMap;
use std::path::PathBuf;

use qcore_c_gateway::legacy_c::{AbiOutcome, declared_abi_census};
use qcore_contract::contract_loader::{
    load_resource_contract_files, load_resource_contract_manifest,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example declared_abi_census");
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
        for row in declared_abi_census(&preprocessed, &contracts) {
            let outcome = match row.outcome {
                AbiOutcome::Agreed => "agreed".to_owned(),
                AbiOutcome::Undeclared => "undeclared".to_owned(),
                AbiOutcome::Withdrawn(refusal) => format!("withdrawn:{}", refusal.as_str()),
            };
            // Only names this unit actually mentions are interesting: a
            // contract for a library this unit never calls is noise.
            if !preprocessed.contains(&row.function) {
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
