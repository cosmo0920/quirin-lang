//! Card W47-CONDVAR-JOIN-PUBLICATION: run the condvar/join census over one
//! program's sources and print every C4 site and C5 release as JSON. Card
//! W47-CONDVAR-JOIN-STOP-PATHS adds each release's POSIX join exclusions,
//! descendants and named premises, every null test with its decision, and
//! every spawn wrapper's joinability.
//!
//! ```text
//! cargo run -p quirin-source-analysis --example condvar_join_census -- \
//!     SOURCE_ROOT OVERRIDE_TEST|- PATH...
//! ```
//!
//! Each `PATH` is a file or directory under `SOURCE_ROOT`; directories are read
//! recursively for `.c` and `.h` files, in sorted order, and `!PATH` excludes
//! a file or directory. `OVERRIDE_TEST`, when not `-`, replaces the contents
//! of `tests/runtime/in_tail.c` (the previous test at `90978ac6~1`, which is
//! blob-identical to `046accf6`'s). Read-only.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::thread_entry_census::CensusFile;
use qcore_core::condvar_join::{DescendantEvidence, JoinEvidence, SyncVerdict, join_succeeds};
use quirin_source_analysis::condvar_join_census::{FLUENT_BIT_SYNC, census};
use quirin_source_analysis::loop_owner_census::FLUENT_BIT_HOPS;

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

fn verdict(verdict: &SyncVerdict) -> serde_json::Value {
    match verdict {
        SyncVerdict::Discharged => serde_json::json!({"verdict": "discharged"}),
        SyncVerdict::Unresolved { reasons } => serde_json::json!({
            "verdict": "unresolved",
            "reasons": reasons.iter().take(6).collect::<Vec<_>>(),
            "reason_count": reasons.len(),
        }),
        SyncVerdict::NotDischarged { reason } => {
            serde_json::json!({"verdict": "not discharged", "reason": reason})
        }
        SyncVerdict::Refused(kind) => {
            serde_json::json!({"verdict": "refused", "refusal": kind.name()})
        }
    }
}

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let [root, test, paths @ ..] = arguments.as_slice() else {
        eprintln!("usage: condvar_join_census SOURCE_ROOT OVERRIDE_TEST|- PATH...");
        std::process::exit(2);
    };
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
    let result = census(&files, &FLUENT_BIT_HOPS, &FLUENT_BIT_SYNC);
    let sites = result
        .sites
        .iter()
        .map(|site| {
            serde_json::json!({
                "file": site.file,
                "line": site.line,
                "function": site.function,
                "callee": site.callee,
                "kind": site.kind,
                "object": site.object,
                "result": verdict(&site.verdict),
            })
        })
        .collect::<Vec<_>>();
    let releases = result
        .releases
        .iter()
        .map(|release| {
            let live = release
                .reading
                .live
                .iter()
                .map(|(thread, paths)| {
                    (
                        thread.clone(),
                        paths
                            .iter()
                            .map(|(join, witness)| format!("{join:?} @ {witness}"))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            serde_json::json!({
                "root": release.reading.root,
                "file": release.reading.file,
                "line": release.reading.line,
                "function": release.reading.function,
                "what": release.reading.what,
                "live": live,
                "unresolved": release.site.unresolved.iter().take(6).collect::<Vec<_>>(),
                "unresolved_count": release.site.unresolved.len(),
                "unchecked_first_joins": release.site.live.iter().flat_map(|live| {
                    live.paths.iter().filter_map(move |(evidence, witness)| match evidence {
                        JoinEvidence::UncheckedFirst(exclusions) => Some(serde_json::json!({
                            "thread": live.thread,
                            "at": witness,
                            "joinable": format!("{:?}", exclusions.joinable),
                            "not_busy": format!("{:?}", exclusions.not_busy),
                            "not_self": format!("{:?}", exclusions.not_self),
                            "no_cycle": format!("{:?}", exclusions.no_cycle),
                            "counts_as_success": join_succeeds(exclusions).is_ok(),
                        })),
                        _ => None,
                    })
                }).collect::<Vec<_>>(),
                "descendants": release.site.descendants.iter().map(|d| serde_json::json!({
                    "parent": d.parent,
                    "child": d.child,
                    "evidence": match &d.evidence {
                        DescendantEvidence::Joined => "joined".to_owned(),
                        DescendantEvidence::NeverJoined(why) => format!("never joined: {why}"),
                        DescendantEvidence::Unresolved(why) => format!("unresolved: {why}"),
                    },
                })).collect::<Vec<_>>(),
                "premises": release.premises,
                "result": verdict(&release.verdict),
            })
        })
        .collect::<Vec<_>>();
    let mut tally: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for site in &result.sites {
        *tally
            .entry(site.kind.to_owned())
            .or_default()
            .entry(site.verdict.label())
            .or_default() += 1;
    }
    for release in &result.releases {
        *tally
            .entry("release".to_owned())
            .or_default()
            .entry(release.verdict.label())
            .or_default() += 1;
    }
    let report = serde_json::json!({
        "files": files.len(),
        "functions_read": result.reading.functions.len(),
        "functions_unread": result.reading.functions.iter().filter(|f| f.unresolved.is_some()).count(),
        "pairing": result.pairing.mutexes,
        "predicates": result.predicates.iter().map(|(c, (keys, facts))| (c.clone(), serde_json::json!({
            "keys": keys,
            "exit_facts": facts.iter().map(|(k, t)| format!("{k} {}", if *t { "truthy" } else { "zero" })).collect::<Vec<_>>(),
        }))).collect::<BTreeMap<_, _>>(),
        "results": result.results,
        "signal_once": result.reading.signal_once.iter().map(|s| (s.condvar.clone(), match &s.derived {
            Ok(entry) => format!("derived: signalled at most once per `{entry}` thread"),
            Err(reason) => format!("not derived: {reason}"),
        })).collect::<BTreeMap<_, _>>(),
        "thread_fields": result.reading.thread_fields,
        "null_tests": result.reading.null_tests.iter().map(|t| serde_json::json!({
            "file": t.file,
            "line": t.line,
            "function": t.function,
            "test": format!("{}->{} ({})", t.base, t.field, if t.null_when_true { "true when null" } else { "true when non-null" }),
            "dominating_dereference": t.dominating_dereference,
            "local_defeat": t.local_defeat,
            "base_type": t.base_type,
            "stores": t.stores.iter().map(|(site, r)| format!("{site}: {r:?}")).collect::<Vec<_>>(),
            "unresolved": t.unresolved.iter().take(6).collect::<Vec<_>>(),
            "unresolved_count": t.unresolved.len(),
            "excluded": format!("{:?}", t.excluded),
        })).collect::<Vec<_>>(),
        "wrappers": result.facts.wrappers.iter().map(|(w, r)| (w.clone(), format!("{r:?}"))).collect::<BTreeMap<_, _>>(),
        "entry_exits": result.reading.entry_exits.iter().map(|(entry, exit)| (entry.clone(), serde_json::json!({
            "started": exit.live.keys().collect::<Vec<_>>(),
            "unresolved_count": exit.unresolved.len(),
            "unresolved": exit.unresolved.iter().take(3).collect::<Vec<_>>(),
        }))).collect::<BTreeMap<_, _>>(),
        "tally": tally,
        "sites": sites,
        "releases": releases,
    });
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
}
