//! Card W47-LOOP-OWNER-CONTRACTS: run the thread-entry census over one
//! program's sources and print every loop-owner site as JSON.
//!
//! ```text
//! cargo run -p qcore-c-gateway --example loop_owner_census -- \
//!     CONTRACT_MODULE SOURCE_ROOT OVERRIDE_TEST|- PATH...
//! ```
//!
//! Each `PATH` is a file or directory under `SOURCE_ROOT`; directories are read
//! recursively for `.c` and `.h` files, in sorted order, and `!PATH` excludes
//! a file or directory. `OVERRIDE_TEST`, when
//! not `-`, replaces the contents of `tests/runtime/in_tail.c` (the previous
//! test at `90978ac6~1`, which is blob-identical to `046accf6`'s). Read-only.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::{CensusFile, FLUENT_BIT_HOPS, census};
use qcore_contract::contract_loader::loop_owner_obligations;
use qcore_core::loop_owner::OwnerVerdict;

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
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [module, root, test, paths @ ..] = arguments.as_slice() else {
        eprintln!("usage: loop_owner_census MODULE SOURCE_ROOT OVERRIDE_TEST|- PATH...");
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
    let result = census(&files, &obligations, &FLUENT_BIT_HOPS);
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
    let focus = ["flb_input_collector_fd", "input_collector_fd", "flb_engine_start"]
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
    let report = serde_json::json!({
        "files": files.len(),
        "functions": result.program.functions.len(),
        "roots": result.program.roots.iter().map(|r| result.program.functions[r.function].clone()).collect::<Vec<_>>(),
        "spawns": result.program.spawns.iter().map(|(s, e)| format!("{} -> {}", result.program.functions[*s], result.program.functions[*e])).collect::<Vec<_>>(),
        "edges": result.program.edges.len(),
        "unresolved_seeds": unresolved,
        "owners": {
            "engine_loop": result.owners.non_threaded,
            "worker_loop": result.owners.threaded,
        },
        "dispatchers": focus,
        "sites": sites,
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
}
