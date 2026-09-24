//! Print the multi-module declaration census and every registered relation's
//! evidence as JSON (card CONTRACT-VIEW-PROJECTION-CHECK).
//!
//! Usage: `contract_view_census [CORPUS_INDEX]` (default `contract/corpus.json`
//! relative to the repository root). The census is printed whatever the
//! registry says; every refusal (an unregistered pair, a stale registration, a
//! failing relation) is listed under `refusals` and on stderr, and the exit
//! status is 1 when there is any.

use std::path::PathBuf;

use qcore_contract::contract_views::{REGISTERED_RELATIONS, census, check_registry, read_corpus};

fn main() {
    let index = std::env::args().nth(1).map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../contract/corpus.json")
                .canonicalize()
                .expect("the corpus index exists")
        },
        PathBuf::from,
    );
    let modules = match read_corpus(&index) {
        Ok(modules) => modules,
        Err(error) => {
            eprintln!("corpus refused: {error}");
            std::process::exit(1);
        }
    };
    let entries = match census(&modules) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("census refused: {error}");
            std::process::exit(1);
        }
    };
    let (relations, refusals) = match check_registry(&modules, REGISTERED_RELATIONS) {
        Ok(report) => (report.relations, Vec::new()),
        Err(errors) => (Vec::new(), errors),
    };
    let document = serde_json::json!({
        "schema_version": 1,
        "modules": modules.len(),
        "census": entries,
        "registry": REGISTERED_RELATIONS,
        "relations": relations,
        "refusals": refusals,
    });
    println!("{}", serde_json::to_string_pretty(&document).unwrap());
    for refusal in &refusals {
        eprintln!("refused: {refusal}");
    }
    if !refusals.is_empty() {
        std::process::exit(1);
    }
}
