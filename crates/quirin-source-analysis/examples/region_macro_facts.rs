//! Card W47-PREMISES-HOLDER-REGION: read, from the recorded compiler's own
//! token dump, the macro invocations of the selected units (the files the
//! census's list-head region reads in place).
//!
//! ```text
//! cargo run -p quirin-source-analysis --example region_macro_facts -- \
//!     --compile-commands=DB --root=SOURCE_ROOT --provenance=BUILD_PROVENANCE.json \
//!     --output=FACTS.json [--jobs=N] --only=REL... \
//!     [--like=REL --include=DIR... --extra=ABS_FILE::KEY...] PATH...
//! ```
//!
//! Each `--only` database file is derived with every command that compiles
//! it, each with its own compiler; an `--extra` unit borrows the `--like`
//! unit's command. `PATH`s (under `SOURCE_ROOT`, `!PATH` excluded) are the
//! census's files: a unit's definition there is the census's to read. The
//! recorded compiler is the build record's `compiler_version`. Read-only on
//! every tree.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::predefined_macros::ExtraUnit;
use qcore_c_gateway::thread_entry_census::region_macros::{FactsRequest, derive_facts};

fn collect(root: &Path, path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .map(|entry| entry.expect("directory entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            if !entry.is_symlink() {
                collect(root, &entry, out);
            }
        }
    } else if path.extension().is_some_and(|ext| ext == "c" || ext == "h") {
        out.push(path.strip_prefix(root).expect("under root").to_owned());
    }
}

fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example region_macro_facts");
    let mut database = None;
    let mut root = None;
    let mut provenance = None;
    let mut output = None;
    let mut jobs = 1usize;
    let mut only = BTreeSet::new();
    let mut like = None;
    let mut includes = Vec::new();
    let mut extras = Vec::new();
    let mut paths = Vec::new();
    for argument in std::env::args().skip(1) {
        if let Some(value) = argument.strip_prefix("--compile-commands=") {
            database = Some(value.to_owned());
        } else if let Some(value) = argument.strip_prefix("--root=") {
            root = Some(PathBuf::from(value));
        } else if let Some(value) = argument.strip_prefix("--provenance=") {
            provenance = Some(value.to_owned());
        } else if let Some(value) = argument.strip_prefix("--output=") {
            output = Some(value.to_owned());
        } else if let Some(value) = argument.strip_prefix("--jobs=") {
            jobs = value.parse().expect("--jobs takes a number");
        } else if let Some(value) = argument.strip_prefix("--only=") {
            only.insert(value.to_owned());
        } else if let Some(value) = argument.strip_prefix("--like=") {
            like = Some(value.to_owned());
        } else if let Some(value) = argument.strip_prefix("--include=") {
            includes.push(value.to_owned());
        } else if let Some(value) = argument.strip_prefix("--extra=") {
            let (file, key) = value.split_once("::").expect("--extra=ABS_FILE::KEY");
            extras.push((PathBuf::from(file), key.to_owned()));
        } else {
            paths.push(argument);
        }
    }
    let (Some(database), Some(root), Some(provenance), Some(output)) =
        (database, root, provenance, output)
    else {
        eprintln!(
            "usage: region_macro_facts --compile-commands=DB --root=ROOT --provenance=FILE \
             --output=FILE [--jobs=N] --only=REL... [--like=REL --include=DIR... \
             --extra=ABS_FILE::KEY...] PATH..."
        );
        std::process::exit(2);
    };
    let record: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&provenance).expect("build provenance"))
            .expect("build provenance JSON");
    let recorded = record["compiler_version"]
        .as_str()
        .expect("the build record names its compiler version")
        .to_owned();
    let mut relative = Vec::new();
    let mut excluded = Vec::new();
    for path in &paths {
        match path.strip_prefix('!') {
            Some(path) => excluded.push(PathBuf::from(path)),
            None => collect(&root, &root.join(path), &mut relative),
        }
    }
    relative.retain(|path| !excluded.iter().any(|excluded| path.starts_with(excluded)));
    let census_files = relative
        .iter()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .collect::<BTreeSet<_>>();
    let request = FactsRequest {
        database_text: fs::read_to_string(&database).expect("compile database"),
        root,
        recorded_compiler_version: recorded,
        only,
        extras: extras
            .into_iter()
            .map(|(file, key)| ExtraUnit {
                file,
                key,
                like: like.clone().expect("--extra needs --like"),
                includes: includes.clone(),
            })
            .collect(),
        census_files,
        jobs,
    };
    let facts = derive_facts(&request).expect("region-macro facts");
    let failures = facts.units.iter().filter(|u| u.failure.is_some()).count();
    eprintln!(
        "{} units, {} failed, {} invocations",
        facts.units.len(),
        failures,
        facts
            .units
            .iter()
            .map(|u| u.invocations.len())
            .sum::<usize>()
    );
    fs::write(
        &output,
        serde_json::to_string_pretty(&facts).expect("facts JSON"),
    )
    .expect("write facts");
}
