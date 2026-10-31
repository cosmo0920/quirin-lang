//! Card W47-LOOP-OWNER-TYPED-DISPATCH: read the typed field-dispatch facts of
//! one build (Clang's JSON AST of every selected translation unit) for the
//! thread-entry census.
//!
//! ```text
//! cargo run -p quirin-source-analysis --example typed_dispatch_facts -- \
//!     --compile-commands=DB --root=SOURCE_ROOT --output=FACTS.json [--jobs=N] \
//!     [--like=REL --include=DIR... --extra=ABS_FILE::KEY...] PATH...
//! ```
//!
//! Each compile command whose file lies under `SOURCE_ROOT` and under one of
//! the census `PATH`s (`!PATH` excludes) is read with its own arguments. An
//! `--extra` unit (a test the build does not compile) is read with the
//! arguments of the `--like` unit, its file replaced and the `--include`
//! directories added, and its facts are keyed as `KEY`. Clang is the one
//! `QCORE_CLANG` names or the gateway discovers. Read-only on every tree.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::{
    FLUENT_BIT_HOPS, FLUENT_BIT_INSTANCE_FLOW, FLUENT_BIT_INSTANCE_PREMISES,
    FLUENT_BIT_PROPERTY_KEYS,
};
use qcore_c_gateway::typed_dispatch_census::{TypedUnit, read_units};

/// A compile command's words as a POSIX shell splits them: single quotes,
/// double quotes (with `\` escaping `"`, `\`, `$` and `` ` ``) and a bare `\`.
fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = command.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                started = true;
                for q in chars.by_ref() {
                    if q == '\'' {
                        break;
                    }
                    word.push(q);
                }
            }
            '"' => {
                started = true;
                while let Some(q) = chars.next() {
                    match q {
                        '"' => break,
                        '\\' => match chars.next() {
                            Some(e @ ('"' | '\\' | '$' | '`')) => word.push(e),
                            Some(e) => {
                                word.push('\\');
                                word.push(e);
                            }
                            None => word.push('\\'),
                        },
                        _ => word.push(q),
                    }
                }
            }
            '\\' => {
                started = true;
                if let Some(e) = chars.next() {
                    word.push(e);
                }
            }
            c if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                started = true;
                word.push(c);
            }
        }
    }
    if started {
        words.push(word);
    }
    words
}

fn selected(relative: &str, paths: &[String]) -> bool {
    let within = |path: &str| relative == path || relative.starts_with(&format!("{path}/"));
    paths
        .iter()
        .any(|path| !path.starts_with('!') && within(path))
        && !paths
            .iter()
            .filter_map(|path| path.strip_prefix('!'))
            .any(within)
}

fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example typed_dispatch_facts");
    let mut database = None;
    let mut root = None;
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
    let (Some(database), Some(root), Some(output)) = (database, root, output) else {
        eprintln!(
            "usage: typed_dispatch_facts --compile-commands=DB --root=ROOT --output=FILE \
             [--jobs=N] [--like=REL --include=DIR... --extra=ABS_FILE::KEY...] PATH..."
        );
        std::process::exit(2);
    };
    let text = fs::read_to_string(&database).expect("compile database");
    let entries: Vec<serde_json::Value> = serde_json::from_str(&text).expect("compile database");
    let mut units = Vec::new();
    let mut by_relative: BTreeMap<String, TypedUnit> = BTreeMap::new();
    for entry in entries {
        let directory = PathBuf::from(entry["directory"].as_str().unwrap_or(""));
        let file = PathBuf::from(entry["file"].as_str().expect("file"));
        let file = if file.is_absolute() {
            file
        } else {
            directory.join(file)
        };
        let Ok(relative) = file.strip_prefix(&root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        let words = match entry.get("arguments").and_then(|a| a.as_array()) {
            Some(arguments) => arguments
                .iter()
                .filter_map(|a| a.as_str().map(str::to_owned))
                .collect::<Vec<_>>(),
            None => shell_words(entry["command"].as_str().unwrap_or("")),
        };
        let unit = TypedUnit {
            file: file.clone(),
            directory,
            arguments: words.into_iter().skip(1).collect(),
        };
        by_relative
            .entry(relative.clone())
            .or_insert_with(|| unit.clone());
        // One file compiled into several targets is read once.
        if selected(&relative, &paths)
            && !units.iter().any(|seen: &TypedUnit| seen.file == unit.file)
        {
            units.push(unit);
        }
    }
    // Card W47-LOOP-OWNER-INSTANCE-FLOW: with the instance-flow facts; card
    // W47-SERVICE-PROPERTY-KEYS: and the property-key facts.
    // Card W47-PREMISES-INSTANCE: the premise facts (an input instance's
    // list link, the records' non-member uses) besides the earlier kinds.
    let mut options = FLUENT_BIT_HOPS.premise_options(
        &FLUENT_BIT_INSTANCE_FLOW,
        &FLUENT_BIT_PROPERTY_KEYS,
        &FLUENT_BIT_INSTANCE_PREMISES,
    );
    if !extras.is_empty() {
        let like = like.expect("--extra needs --like");
        let template = by_relative
            .get(&like)
            .expect("the --like unit is in the database");
        for (file, key) in extras {
            let mut arguments = Vec::new();
            for include in &includes {
                arguments.push(format!("-I{include}"));
            }
            for argument in &template.arguments {
                if Path::new(argument) == template.file {
                    arguments.push(file.to_string_lossy().into_owned());
                } else {
                    arguments.push(argument.clone());
                }
            }
            options.aliases.insert(file.clone(), key);
            units.push(TypedUnit {
                file,
                directory: template.directory.clone(),
                arguments,
            });
        }
    }
    let facts = read_units(&units, &root, &options, jobs);
    eprintln!(
        "{} units read, {} failed, {} dispatch sites, {} store sites",
        facts.units.len(),
        facts.failures.len(),
        facts.dispatches.len(),
        facts.stores.len()
    );
    fs::write(
        &output,
        serde_json::to_string_pretty(&facts).expect("facts JSON"),
    )
    .expect("write facts");
}
