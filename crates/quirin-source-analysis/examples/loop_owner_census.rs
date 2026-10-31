//! Card W47-LOOP-OWNER-CONTRACTS: run the thread-entry census over one
//! program's sources and print every loop-owner site as JSON.
//!
//! ```text
//! cargo run -p quirin-source-analysis --example loop_owner_census -- \
//!     CONTRACT_MODULE SOURCE_ROOT OVERRIDE_TEST|- PATH...
//! ```
//!
//! Each `PATH` is a file or directory under `SOURCE_ROOT`; directories are read
//! recursively for `.c` and `.h` files, in sorted order, and `!PATH` excludes
//! a file or directory. `OVERRIDE_TEST`, when
//! not `-`, replaces the contents of `tests/runtime/in_tail.c` (the previous
//! test at `90978ac6~1`, which is blob-identical to `046accf6`'s). Read-only.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::predefined_macros::{PredefinedFacts, PredefinedMacros};
use qcore_c_gateway::thread_entry_census::region_macros::{RegionMacroFacts, RegionMacros};
use qcore_c_gateway::thread_entry_census::{
    BuildConfiguration, CensusFile, GeneratedFile, PremiseInputs, SitePremise,
};
use qcore_c_gateway::typed_dispatch_census::TypedFacts;
use qcore_contract::contract_loader::loop_owner_obligations;
use qcore_core::loop_owner::OwnerVerdict;
use quirin_source_analysis::loop_owner_census::{FLUENT_BIT_HOPS, census_premised};

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

/// The macros each translation unit's compile command defines (`-DNAME`,
/// `-DNAME=VALUE`, `-D NAME`; a later `-UNAME` removes it), keyed by the
/// file's path under `root`.
fn compile_defines(database: &str, root: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let text = fs::read_to_string(database).expect("compile database");
    let entries: Vec<serde_json::Value> = serde_json::from_str(&text).expect("compile database");
    let mut units = BTreeMap::new();
    for entry in entries {
        let directory = PathBuf::from(entry["directory"].as_str().unwrap_or(""));
        let file = PathBuf::from(entry["file"].as_str().expect("file"));
        let file = if file.is_absolute() {
            file
        } else {
            directory.join(file)
        };
        let Ok(relative) = file.strip_prefix(root) else {
            continue;
        };
        let words = match entry.get("arguments").and_then(|a| a.as_array()) {
            Some(arguments) => arguments
                .iter()
                .filter_map(|a| a.as_str().map(str::to_owned))
                .collect::<Vec<_>>(),
            None => entry["command"]
                .as_str()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
        };
        let mut defined = BTreeSet::new();
        let mut words = words.iter();
        while let Some(word) = words.next() {
            let (flag, value) = match word.as_str() {
                "-D" | "-U" => (word.as_str(), words.next().map_or("", String::as_str)),
                _ if word.starts_with("-D") => ("-D", &word[2..]),
                _ if word.starts_with("-U") => ("-U", &word[2..]),
                _ => continue,
            };
            let name = value.split('=').next().unwrap_or("").to_owned();
            if flag == "-D" {
                defined.insert(name);
            } else {
                defined.remove(&name);
            }
        }
        let key = relative.to_string_lossy().replace('\\', "/");
        // One file compiled into several targets: keep what every command
        // defines.
        units
            .entry(key)
            .and_modify(|seen: &mut BTreeSet<String>| {
                *seen = seen.intersection(&defined).cloned().collect();
            })
            .or_insert(defined);
    }
    units
}

/// Card W47-PREMISES-INSTANCE: each compile command's `#include <…>` search
/// directories, in order (`-I`, then `-isystem`), keyed by the file's path
/// under `root`.
fn include_dirs(database: &str, root: &Path) -> BTreeMap<String, Vec<Vec<PathBuf>>> {
    let text = fs::read_to_string(database).expect("compile database");
    let entries: Vec<serde_json::Value> = serde_json::from_str(&text).expect("compile database");
    let mut units: BTreeMap<String, Vec<Vec<PathBuf>>> = BTreeMap::new();
    for entry in entries {
        let directory = PathBuf::from(entry["directory"].as_str().unwrap_or(""));
        let file = PathBuf::from(entry["file"].as_str().expect("file"));
        let file = if file.is_absolute() {
            file
        } else {
            directory.join(file)
        };
        let Ok(relative) = file.strip_prefix(root) else {
            continue;
        };
        let words = match entry.get("arguments").and_then(|a| a.as_array()) {
            Some(arguments) => arguments
                .iter()
                .filter_map(|a| a.as_str().map(str::to_owned))
                .collect::<Vec<_>>(),
            None => entry["command"]
                .as_str()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
        };
        let (mut quoted, mut system) = (Vec::new(), Vec::new());
        let mut words = words.iter();
        while let Some(word) = words.next() {
            let (list, value) = match word.as_str() {
                "-I" => (&mut quoted, words.next().cloned().unwrap_or_default()),
                "-isystem" => (&mut system, words.next().cloned().unwrap_or_default()),
                _ if word.starts_with("-isystem") => {
                    (&mut system, word["-isystem".len()..].to_owned())
                }
                _ if word.starts_with("-I") => (&mut quoted, word[2..].to_owned()),
                _ => continue,
            };
            list.push(directory.join(value));
        }
        quoted.extend(system);
        units
            .entry(relative.to_string_lossy().replace('\\', "/"))
            .or_default()
            .push(quoted);
    }
    units
}

/// Card W47-PREMISES-INSTANCE: the generated root's files, each with the read
/// files every compile command of which resolves one of their `#include <…>`
/// lines to it (the first search directory holding the name).
fn generated_inputs(
    database: &str,
    root: &Path,
    generated: &Path,
    files: &[CensusFile],
) -> PremiseInputs {
    let mut headers = Vec::new();
    collect(generated, generated, &mut headers);
    let canonical = |path: &Path| fs::canonicalize(path).ok();
    let mut out = headers
        .iter()
        .map(|header| GeneratedFile {
            path: header.to_string_lossy().replace('\\', "/"),
            text: String::from_utf8_lossy(&fs::read(generated.join(header)).expect("generated"))
                .into_owned(),
            included_by: BTreeSet::new(),
        })
        .collect::<Vec<_>>();
    let targets = headers
        .iter()
        .map(|header| canonical(&generated.join(header)))
        .collect::<Vec<_>>();
    let commands = include_dirs(database, root);
    for file in files {
        let Some(searches) = commands.get(&file.path) else {
            continue;
        };
        for line in file.text.lines() {
            let Some(rest) = line.trim_start().strip_prefix('#') else {
                continue;
            };
            let Some(rest) = rest.trim_start().strip_prefix("include") else {
                continue;
            };
            let Some(name) = rest
                .trim()
                .strip_prefix('<')
                .and_then(|rest| rest.split_once('>'))
                .map(|(name, _)| name)
            else {
                continue;
            };
            let resolved = searches
                .iter()
                .map(|dirs| {
                    dirs.iter()
                        .map(|dir| dir.join(name))
                        .find(|candidate| candidate.is_file())
                        .and_then(|found| canonical(&found))
                })
                .collect::<BTreeSet<_>>();
            let resolved = resolved.into_iter().collect::<Vec<_>>();
            let [Some(only)] = &resolved[..] else {
                continue;
            };
            for (index, target) in targets.iter().enumerate() {
                if target.as_ref() == Some(only) {
                    out[index].included_by.insert(file.path.clone());
                }
            }
        }
    }
    out.retain(|file| !file.text.is_empty());
    PremiseInputs { generated: out }
}

/// For each coroutine body that may yield, one shortest call path (hops other
/// than the coroutine hop) to a function that yields (card
/// W47-LOOP-OWNER-TYPED-DISPATCH): what holds the body unresolved.
fn yield_paths(
    result: &quirin_source_analysis::loop_owner_census::Census,
) -> BTreeMap<String, Vec<String>> {
    use qcore_c_gateway::thread_entry_census::ReadHop;
    let reading = &result.reading;
    let mut next: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for edge in &reading.edges {
        if edge.hop != ReadHop::Coroutine {
            next.entry(edge.caller).or_default().push(edge.callee);
        }
    }
    let mut paths = BTreeMap::new();
    for (_, body) in &reading.coroutine_bodies {
        let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
        let mut queue = std::collections::VecDeque::from([*body]);
        let mut seen = BTreeSet::from([*body]);
        let mut found = None;
        while let Some(at) = queue.pop_front() {
            if reading.yields.contains_key(&at) {
                found = Some(at);
                break;
            }
            for callee in next.get(&at).into_iter().flatten() {
                if seen.insert(*callee) {
                    parent.insert(*callee, at);
                    queue.push_back(*callee);
                }
            }
        }
        if let Some(mut at) = found {
            let mut path = vec![reading.functions[at].clone()];
            while let Some(previous) = parent.get(&at) {
                at = *previous;
                path.push(reading.functions[at].clone());
            }
            path.reverse();
            paths.insert(reading.functions[*body].clone(), path);
        }
    }
    paths
}

fn main() {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example loop_owner_census");
    let mut arguments = std::env::args().skip(1).collect::<Vec<_>>();
    // Card W47-LOOP-OWNER-COROUTINE-HOP: `--compile-commands=PATH` reads the
    // conditional groups under that build's configuration, with
    // `--owned-prefix=P` (repeatable) naming the macros it owns.
    let mut database = None;
    let mut generated = None;
    let mut prefixes = Vec::new();
    // Card W47-LOOP-OWNER-TYPED-DISPATCH: `--typed-facts=PATH` (written by the
    // `typed_dispatch_facts` example) keys field dispatch by record type.
    let mut typed_paths = Vec::new();
    // Card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: `--predefined-macros=PATH`
    // (written by the `predefined_macro_facts` example) decides reserved
    // macros from the recorded compiler, per command.
    let mut predefined_path = None;
    // Card W47-PREMISES-HOLDER-REGION: `--region-macros=PATH` (written by the
    // `region_macro_facts` example) reads the list-head region's macro
    // invocations from their units' own expansions.
    let mut region_path = None;
    // `--list-functions=PATH`: every census function and edge, one per line
    // (attribution of a reading's movement).
    let mut list_path = None;
    // Card W47-PREMISES-INSTANCE: `--derive-premises` runs the premised
    // census (the generated root's files read, P-INSTANCE per site).
    let mut derive_premises = false;
    arguments.retain(|argument| {
        if argument == "--derive-premises" {
            derive_premises = true;
            return false;
        }
        if let Some(path) = argument.strip_prefix("--list-functions=") {
            list_path = Some(path.to_owned());
            return false;
        }
        if let Some(path) = argument.strip_prefix("--typed-facts=") {
            typed_paths.push(path.to_owned());
            return false;
        }
        if let Some(path) = argument.strip_prefix("--predefined-macros=") {
            predefined_path = Some(path.to_owned());
            return false;
        }
        if let Some(path) = argument.strip_prefix("--region-macros=") {
            region_path = Some(path.to_owned());
            return false;
        }
        if let Some(path) = argument.strip_prefix("--compile-commands=") {
            database = Some(path.to_owned());
            false
        } else if let Some(path) = argument.strip_prefix("--generated-root=") {
            generated = Some(path.to_owned());
            false
        } else if let Some(prefix) = argument.strip_prefix("--owned-prefix=") {
            prefixes.push(prefix.to_owned());
            false
        } else {
            true
        }
    });
    let [module, root, test, paths @ ..] = arguments.as_slice() else {
        eprintln!(
            "usage: loop_owner_census [--compile-commands=DB [--owned-prefix=P]...] MODULE \
             SOURCE_ROOT OVERRIDE_TEST|- PATH..."
        );
        std::process::exit(2);
    };
    let module_text = fs::read_to_string(module).expect("contract module");
    let obligations = loop_owner_obligations(&BTreeMap::from([(module.clone(), module_text)]))
        .expect("the module declares its obligations");
    let root = PathBuf::from(root);
    let mut relative = Vec::new();
    let mut excluded = Vec::new();
    for path in paths {
        match path.strip_prefix('!') {
            Some(path) => excluded.push(PathBuf::from(path)),
            None => collect(&root, &root.join(path), &mut relative),
        }
    }
    relative.retain(|path| !excluded.iter().any(|excluded| path.starts_with(excluded)));
    relative.sort();
    relative.dedup();
    let files = relative
        .iter()
        .map(|path| {
            let name = path.to_string_lossy().replace('\\', "/");
            let text = if name == "tests/runtime/in_tail.c" && test != "-" {
                fs::read_to_string(test).expect("override test")
            } else {
                String::from_utf8_lossy(&fs::read(root.join(path)).expect("source")).into_owned()
            };
            CensusFile { path: name, text }
        })
        .collect::<Vec<_>>();
    let configuration = database.as_ref().map(|database| {
        let read = files
            .iter()
            .map(|file| file.path.clone())
            .collect::<BTreeSet<_>>();
        let mut units = compile_defines(database, &root);
        units.retain(|path, _| read.contains(path));
        let mut headers = Vec::new();
        if let Some(generated) = &generated {
            collect(Path::new(generated), Path::new(generated), &mut headers);
        }
        let defined_elsewhere = headers
            .iter()
            .filter_map(|header| {
                fs::read(Path::new(generated.as_deref().unwrap_or("")).join(header)).ok()
            })
            .flat_map(|bytes| {
                String::from_utf8_lossy(&bytes)
                    .lines()
                    .filter_map(|line| {
                        let rest = line.trim_start().strip_prefix('#')?.trim_start();
                        let name = rest.strip_prefix("define")?.trim_start();
                        let end = name
                            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                            .unwrap_or(name.len());
                        Some(name[..end].to_owned())
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        BuildConfiguration {
            units,
            owned_prefixes: prefixes.clone(),
            defined_elsewhere,
            predefined: None,
            region_macros: None,
        }
    });
    // The predefined-macro facts decide only when they match the database
    // and the recorded compiler; otherwise nothing is decided and the reason
    // is reported.
    let mut predefined_report = serde_json::Value::Null;
    let mut configuration = configuration;
    if let Some(path) = &predefined_path {
        let derived = match (&database, configuration.as_ref()) {
            (Some(database), Some(_)) => {
                let facts = fs::read_to_string(path)
                    .map_err(|e| format!("{path}: {e}"))
                    .and_then(|text| {
                        serde_json::from_str::<PredefinedFacts>(&text)
                            .map_err(|e| format!("{path}: {e}"))
                    });
                let database_text = fs::read_to_string(database).expect("compile database");
                facts.and_then(|facts| PredefinedMacros::derive(&facts, &database_text, &root))
            }
            _ => Err("no compile database: the reading is not configured".to_owned()),
        };
        predefined_report = match derived {
            Ok(view) => {
                let (units, failed) = view.unit_counts();
                let report = serde_json::json!({
                    "decided": true,
                    "units": units,
                    "failed_units": failed,
                    "files_without_a_command_decided": view.decides_uncommanded_files(),
                    "candidates": view.candidates().len(),
                    "compilers": view.compilers,
                    "failures": view.failures,
                });
                if let Some(configuration) = configuration.as_mut() {
                    configuration.predefined = Some(view);
                }
                report
            }
            Err(reason) => serde_json::json!({"decided": false, "reason": reason}),
        };
    }
    // The region-macro facts are read only when they match the database and
    // the recorded compiler; otherwise the region's invocations stay unread
    // and the reason is reported.
    let mut region_report = serde_json::Value::Null;
    if let Some(path) = &region_path {
        let derived = match (&database, configuration.as_ref()) {
            (Some(database), Some(_)) => {
                let facts = fs::read_to_string(path)
                    .map_err(|e| format!("{path}: {e}"))
                    .and_then(|text| {
                        serde_json::from_str::<RegionMacroFacts>(&text)
                            .map_err(|e| format!("{path}: {e}"))
                    });
                let database_text = fs::read_to_string(database).expect("compile database");
                facts.and_then(|facts| RegionMacros::derive(&facts, &database_text, &root))
            }
            _ => Err("no compile database: the reading is not configured".to_owned()),
        };
        region_report = match derived {
            Ok(view) => {
                let report = serde_json::json!({
                    "read": true,
                    "paths": view.paths(),
                    "compilers": view.compilers,
                });
                if let Some(configuration) = configuration.as_mut() {
                    configuration.region_macros = Some(view);
                }
                report
            }
            Err(reason) => serde_json::json!({"read": false, "reason": reason}),
        };
    }
    // Repeatable: the library's facts and the test's are read separately.
    let typed = (!typed_paths.is_empty()).then(|| {
        let mut merged = TypedFacts::default();
        for path in &typed_paths {
            let text = fs::read_to_string(path).expect("typed facts");
            merged.merge(serde_json::from_str::<TypedFacts>(&text).expect("typed facts JSON"));
        }
        merged
    });
    let premise_inputs = derive_premises.then(|| match (&database, &generated) {
        (Some(database), Some(generated)) => {
            generated_inputs(database, &root, Path::new(generated), &files)
        }
        _ => PremiseInputs::default(),
    });
    let result = census_premised(
        &files,
        &obligations,
        &FLUENT_BIT_HOPS,
        configuration.as_ref(),
        typed.as_ref(),
        premise_inputs.as_ref(),
    );
    let sites = result
        .sites
        .iter()
        .map(|site| {
            let verdict = match &site.verdict {
                OwnerVerdict::Discharged { contexts } => serde_json::json!({
                    "verdict": "discharged",
                    "contexts": contexts.iter().map(|c| format!("{} {:?}", c.thread, c.mode)).collect::<Vec<_>>(),
                }),
                OwnerVerdict::NotDischarged { context, witness } => serde_json::json!({
                    "verdict": "not-discharged",
                    "context": format!("{} {:?}", context.thread, context.mode),
                    "witness": result.names(witness),
                }),
                OwnerVerdict::Unresolved { reasons } => serde_json::json!({
                    "verdict": "unresolved",
                    "reasons": reasons.iter().take(8).collect::<Vec<_>>(),
                    "reason_count": reasons.len(),
                }),
            };
            let caller = result.derived(&site.function);
            let witnesses = caller
                .map(|derived| {
                    derived
                        .witness
                        .iter()
                        .map(|(context, path)| {
                            serde_json::json!({
                                "context": format!("{} {:?}", context.thread, context.mode),
                                "path": result.names(path),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            serde_json::json!({
                "file": site.file,
                "line": site.line,
                "function": site.function,
                "callee": site.callee,
                "operation": format!("{:?}", site.operation),
                "result": verdict,
                "caller_contexts": witnesses,
            })
        })
        .collect::<Vec<_>>();
    let unresolved = result.program.unresolved.len();
    // The derivation of the loop dispatchers and the thread entries, in full:
    // what the owners rest on.
    let focus = [
        "flb_input_collector_fd",
        "input_collector_fd",
        "flb_engine_start",
        "include/fluent-bit/flb_input.h::input_pre_cb_collect",
    ]
        .iter()
        .filter_map(|name| {
            let derived = result.derived(name)?;
            Some(serde_json::json!({
                "function": name,
                "contexts": derived.contexts.iter().map(|c| format!("{} {:?}", c.thread, c.mode)).collect::<Vec<_>>(),
                "unresolved": derived.unresolved,
            }))
        })
        .collect::<Vec<_>>();
    let mut report = serde_json::json!({
        "files": files.len(),
        "functions": result.program.functions.len(),
        "roots": result.program.roots.iter().map(|r| result.program.functions[r.function].clone()).collect::<Vec<_>>(),
        "spawns": result.program.spawns.iter().map(|(s, e)| format!("{} -> {}", result.program.functions[*s], result.program.functions[*e])).collect::<Vec<_>>(),
        "edges": result.program.edges.len(),
        "unresolved_seeds": unresolved,
        "configured_units": configuration.as_ref().map(|c| c.units.len()),
        "unstructured_files": result
            .program
            .unresolved
            .values()
            .flatten()
            .filter_map(|reason| reason.strip_suffix(": a conditional group this reader cannot structure"))
            .collect::<BTreeSet<_>>(),
        "coroutine_bodies": result.program.coroutine_bodies.len(),
        "yields": result.program.yields.len(),
        "coroutine_body_functions": result
            .program
            .coroutine_bodies
            .iter()
            .map(|(entry, body)| format!(
                "{} -> {}",
                result.program.functions[*entry], result.program.functions[*body]
            ))
            .collect::<Vec<_>>(),
        "yield_sites": result
            .program
            .yields
            .iter()
            .map(|(function, site)| (result.program.functions[*function].clone(), site.clone()))
            .collect::<BTreeMap<_, _>>(),
        "owners": {
            "engine_loop": result.owners.non_threaded,
            "worker_loop": result.owners.threaded,
        },
        "dispatchers": focus,
        "sites": sites,
    });
    // Printed only when asked for (the older readings are unchanged).
    if predefined_path.is_some() {
        if predefined_report["decided"] == serde_json::json!(true) {
            predefined_report["decisions"] = serde_json::json!(result.reading.predefined_decisions);
        }
        report["predefined_macros"] = predefined_report;
    }
    // Card W47-PREMISES-INSTANCE: printed only in the premised census.
    if let Some(inputs) = &premise_inputs {
        let premise_report = result.reading.premises.clone().unwrap_or_default();
        report["premises"] = serde_json::json!({
            "generated": inputs
                .generated
                .iter()
                .map(|file| (file.path.clone(), file.included_by.iter().cloned().collect::<Vec<_>>()))
                .collect::<BTreeMap<_, _>>(),
            "instance_sites": premise_report
                .instance_sites
                .iter()
                .map(|(site, premise)| {
                    let (status, why) = match premise {
                        SitePremise::Derived(why) => ("derived", why),
                        SitePremise::Stated(why) => ("stated", why),
                        SitePremise::Refuted(why) => ("refuted", why),
                    };
                    (site.clone(), serde_json::json!({"status": status, "reason": why}))
                })
                .collect::<BTreeMap<_, _>>(),
            "fields_by_member": {
                "read": premise_report.fields_by_member.read,
                "violations": premise_report.fields_by_member.violations,
                "escapes": premise_report.fields_by_member.escapes,
                "other_types": premise_report.fields_by_member.other_types,
                "harmless": premise_report.fields_by_member.harmless,
            },
        });
    }
    if region_path.is_some() {
        report["region_macros"] = region_report;
    }
    if let Some(path) = &list_path {
        let program = &result.program;
        let mut lines = program
            .functions
            .iter()
            .map(|name| format!("function {name}"))
            .collect::<Vec<_>>();
        lines.extend(result.reading.edges.iter().map(|edge| {
            format!(
                "edge {} -> {} {:?} {:?}",
                result.reading.functions[edge.caller],
                result.reading.functions[edge.callee],
                edge.hop,
                edge.effect
            )
        }));
        lines.sort();
        fs::write(path, lines.join("\n") + "\n").expect("write the function list");
    }
    if let Some(typed) = &typed {
        report["typed"] = serde_json::json!({
            "units": typed.units.len(),
            "failures": typed.failures,
            "dispatch_facts": typed.dispatches.len(),
            "store_facts": typed.stores.len(),
            "dispatch_sites": result
                .reading
                .dispatch_typing
                .iter()
                .map(|(kind, sites)| (kind.clone(), sites.len()))
                .collect::<BTreeMap<_, _>>(),
            "dispatch_site_list": result.reading.dispatch_typing,
            "yield_paths": yield_paths(&result),
            // The seeds this card's typed reading adds: a dispatch or a store
            // through an unmatched record type.
            "typed_unresolved_seeds": result
                .reading
                .unresolved
                .iter()
                .filter_map(|(id, reasons)| {
                    let typed = reasons
                        .iter()
                        .filter(|r| {
                            r.contains("whose record type is not matched")
                                || r.contains("through an unmatched record type")
                        })
                        .collect::<Vec<_>>();
                    (!typed.is_empty())
                        .then(|| (result.program.functions[*id].clone(), typed))
                })
                .collect::<BTreeMap<_, _>>(),
            "instance_flags": result
                .reading
                .instance_flags
                .iter()
                .map(|(id, flag)| (
                    result.program.functions[*id].clone(),
                    serde_json::json!({"derived": flag.derived, "reason": flag.reason}),
                ))
                .collect::<BTreeMap<_, _>>(),
        });
        // Card W47-SERVICE-PROPERTY-KEYS: the keyed setter calls Step B read,
        // printed only where there is one (the older readings are unchanged).
        if !result.reading.property_keys.is_empty() {
            report["typed"]["property_keys"] = serde_json::json!(result.reading.property_keys);
        }
        // Card W47-PREMISES-CORO-PLUGIN: what each premise derived, and the
        // named residuals' sites.
        if !result.reading.coro_plugin_premises.is_empty() {
            report["typed"]["premises"] = serde_json::json!(result.reading.coro_plugin_premises);
        }
    }
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
}
