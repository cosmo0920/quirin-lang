//! Print the multi-module declaration census and every registered relation's
//! evidence as JSON (card CONTRACT-VIEW-PROJECTION-CHECK).
//!
//! Usage: `contract_view_census [CORPUS_INDEX]` (default `contract/corpus.json`
//! relative to the repository root). Exits 1 and prints every refusal when a
//! pair is unregistered, a registration is stale, or a relation fails.

use std::path::PathBuf;

use qcore_contract::contract_views::{REGISTERED_RELATIONS, check_registry, read_corpus};

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
    match check_registry(&modules, REGISTERED_RELATIONS) {
        Ok(report) => {
            let document = serde_json::json!({
                "schema_version": 1,
                "modules": modules.len(),
                "census": report.census,
                "registry": REGISTERED_RELATIONS,
                "relations": report.relations,
            });
            println!("{}", serde_json::to_string_pretty(&document).unwrap());
        }
        Err(errors) => {
            for error in errors {
                eprintln!("refused: {error}");
            }
            std::process::exit(1);
        }
    }
}
