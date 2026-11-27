// SPDX-License-Identifier: Apache-2.0
//
// Card LIBEVENT-CORE-CENSUS-GUARD-CORRELATION: the raw-source census of
// files, before and after rule 49. Copied into a tip export as
// `crates/qcore-c-gateway/examples/raw_census.rs` and run as
// `cargo run --release -p qcore-c-gateway --example raw_census -- FILE...`.
// For each file: the raw scan's refusals and names, guard correlation's
// outcome, and the names correlation adds (one per line, `+ NAME`).
use qcore_c_gateway::legacy_c::guard_correlation::correlated_source_scan;
use qcore_c_gateway::legacy_c::legacy_c_source_scan;
use std::collections::BTreeSet;

fn main() {
    for path in std::env::args().skip(1) {
        let source = std::fs::read_to_string(&path).expect("readable source");
        let scan = legacy_c_source_scan(&source);
        let raw: BTreeSet<String> = scan.definitions().iter().cloned().collect();
        println!(
            "{path}: raw {} names, refusals {:?}",
            raw.len(),
            scan.refusals()
        );
        match correlated_source_scan(&source) {
            Ok(names) => {
                let all: BTreeSet<String> = names.into_iter().collect();
                let added: Vec<_> = all.difference(&raw).collect();
                let lost: Vec<_> = raw.difference(&all).collect();
                println!(
                    "  correlation decides: {} names, {} added, {} raw names not in it",
                    all.len(),
                    added.len(),
                    lost.len()
                );
                for name in added {
                    println!("  + {name}");
                }
            }
            Err(refusal) => println!("  correlation refuses: {refusal:?}"),
        }
    }
}
