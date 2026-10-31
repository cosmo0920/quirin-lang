//! Card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: derive, from the recorded
//! compiler of each compile command, the reserved macros the configured
//! census reading may decide.
//!
//! ```text
//! cargo run -p quirin-source-analysis --example predefined_macro_facts -- \
//!     --compile-commands=DB --root=SOURCE_ROOT --provenance=BUILD_PROVENANCE.json \
//!     --scratch=DIR --output=FACTS.json [--jobs=N] \
//!     [--like=REL --include=DIR... --extra=ABS_FILE::KEY...] PATH...
//! ```
//!
//! The candidates are the reserved names the conditional directives of the
//! census's files name (`PATH`s under `SOURCE_ROOT`, `!PATH` excluded; an
//! `--extra` file read in place of its `KEY`). Every database command is
//! derived with its own compiler; an `--extra` unit borrows the `--like`
//! unit's command. The recorded compiler is the build record's
//! `compiler_version` (`build-provenance.json`). Read-only on every tree;
//! the probe file is written under `--scratch`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::predefined_macros::{
    ExtraUnit, FactsRequest, condition_names, derive_facts,
};

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
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example predefined_macro_facts");
    let mut database = None;
    let mut root = None;
    let mut provenance = None;
    let mut scratch = None;
    let mut output = None;
    let mut jobs = 1usize;
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
        } else if let Some(value) = argument.strip_prefix("--scratch=") {
            scratch = Some(PathBuf::from(value));
        } else if let Some(value) = argument.strip_prefix("--output=") {
            output = Some(value.to_owned());
        } else if let Some(value) = argument.strip_prefix("--jobs=") {
            jobs = value.parse().expect("--jobs takes a number");
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
    let (Some(database), Some(root), Some(provenance), Some(scratch), Some(output)) =
        (database, root, provenance, scratch, output)
    else {
        eprintln!(
            "usage: predefined_macro_facts --compile-commands=DB --root=ROOT \
             --provenance=FILE --scratch=DIR --output=FILE [--jobs=N] \
             [--like=REL --include=DIR... --extra=ABS_FILE::KEY...] PATH..."
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
    // The census's files, the extras read in place of their keys.
    let mut relative = Vec::new();
    let mut excluded = Vec::new();
    for path in &paths {
        match path.strip_prefix('!') {
            Some(path) => excluded.push(PathBuf::from(path)),
            None => collect(&root, &root.join(path), &mut relative),
        }
    }
    relative.retain(|path| !excluded.iter().any(|excluded| path.starts_with(excluded)));
    relative.sort();
    relative.dedup();
    let mut candidates = BTreeSet::new();
    for path in &relative {
        let key = path.to_string_lossy().replace('\\', "/");
        let source = extras
            .iter()
            .find(|(_, extra)| *extra == key)
            .map_or_else(|| root.join(path), |(file, _)| file.clone());
        let bytes = fs::read(&source).expect("source");
        candidates.extend(condition_names(&String::from_utf8_lossy(&bytes)));
    }
    let request = FactsRequest {
        database_text: fs::read_to_string(&database).expect("compile database"),
        root,
        recorded_compiler_version: recorded,
        candidates,
        extras: extras
            .into_iter()
            .map(|(file, key)| ExtraUnit {
                file,
                key,
                like: like.clone().expect("--extra needs --like"),
                includes: includes.clone(),
            })
            .collect(),
        scratch,
        jobs,
    };
    let facts = derive_facts(&request).expect("predefined-macro facts");
    let failures = facts
        .commands
        .iter()
        .chain(&facts.extras)
        .filter(|command| command.failure.is_some())
        .count();
    eprintln!(
        "{} commands, {} extras, {} failed, {} candidates, {} covered",
        facts.commands.len(),
        facts.extras.len(),
        failures,
        facts.candidates.len(),
        facts.covered.len()
    );
    fs::write(
        &output,
        serde_json::to_string_pretty(&facts).expect("facts JSON"),
    )
    .expect("write facts");
}
