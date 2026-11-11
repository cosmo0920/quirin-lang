// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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

use qcore_c_gateway::condvar_census::ReadTraceJoin;
use qcore_c_gateway::thread_entry_census::CensusFile;
use qcore_contract::contract_loader::lifecycle_obligations;
use qcore_core::condvar_join::{DescendantEvidence, JoinEvidence, SyncVerdict, join_succeeds};
use qcore_core::paired_lifecycle::ClauseStatus;
use quirin_source_analysis::condvar_join_census::{FLUENT_BIT_SYNC, census, census_with_lifecycle};
use quirin_source_analysis::lifecycle_census::{
    FLUENT_BIT_LIFECYCLE_CREATORS, LifecycleCensus, governing_at,
};
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
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example condvar_join_census");
    let mut arguments = std::env::args().skip(1).collect::<Vec<_>>();
    // Card W47-FLB-LIB-LIFECYCLE: `--lifecycle-module=PATH` reads the declared
    // lifecycle obligations from that contract module.
    let mut lifecycle_module: Option<String> = None;
    arguments.retain(
        |argument| match argument.strip_prefix("--lifecycle-module=") {
            Some(path) => {
                lifecycle_module = Some(path.to_owned());
                false
            }
            None => true,
        },
    );
    let [root, test, paths @ ..] = arguments.as_slice() else {
        eprintln!(
            "usage: condvar_join_census [--lifecycle-module=MODULE] SOURCE_ROOT OVERRIDE_TEST|- \
             PATH..."
        );
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
    let result = match &lifecycle_module {
        Some(module) => {
            let text = fs::read_to_string(module).expect("lifecycle contract module");
            let declared = lifecycle_obligations(&BTreeMap::from([(module.clone(), text)]))
                .expect("the module declares its lifecycle obligations");
            census_with_lifecycle(
                &files,
                &FLUENT_BIT_HOPS,
                &FLUENT_BIT_SYNC,
                &declared,
                FLUENT_BIT_LIFECYCLE_CREATORS,
            )
        }
        None => census(&files, &FLUENT_BIT_HOPS, &FLUENT_BIT_SYNC),
    };
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
            let mut entry = serde_json::json!({
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
            });
            if lifecycle_module.is_some()
                && let Some(object) = entry.as_object_mut()
            {
                object.insert(
                    "lifecycle_violations".to_owned(),
                    serde_json::json!(release.lifecycle_violations),
                );
                if let Some(lifecycle) = &result.lifecycle {
                    object.insert(
                        "lifecycle_stops".to_owned(),
                        serde_json::json!(governing_at(&release.reading, lifecycle)),
                    );
                }
                // Card W47-FLB-LIB-STRICT-PAIRING: the release decided on the
                // paths that conform to the strict pair.
                if let Some(split) = &release.lifecycle_split {
                    object.insert(
                        "lifecycle_paths".to_owned(),
                        serde_json::json!({
                            "conforming": split.conforming_paths,
                            "undetermined": split.undetermined_paths,
                            "refused": split.refused_paths,
                            "conforming_result": split.conforming_verdict.as_ref().map(verdict),
                            "conforming_live": split.conforming_site.as_ref().map(|site| site.live.iter().map(|live| (
                                live.thread.clone(),
                                live.paths.iter().map(|(evidence, witness)| format!("{} @ {witness}", evidence_label(evidence))).collect::<std::collections::BTreeSet<_>>(),
                            )).collect::<BTreeMap<_, _>>()),
                            "conforming_descendants": split.conforming_site.as_ref().map(|site| site.descendants.iter().map(|d| format!("{} -> {}: {}", d.parent, d.child, match &d.evidence {
                                DescendantEvidence::Joined => "joined".to_owned(),
                                DescendantEvidence::NeverJoined(why) => format!("never joined: {why}"),
                                DescendantEvidence::Unresolved(why) => format!("unresolved: {why}"),
                            })).collect::<Vec<_>>()),
                            "conforming_unresolved_count": split.conforming_site.as_ref().map(|site| site.unresolved.len()),
                            "conforming_premises": split.conforming_premises,
                        }),
                    );
                }
            }
            entry
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
            "unresolved": exit.unresolved.iter().collect::<Vec<_>>(),
        }))).collect::<BTreeMap<_, _>>(),
        "tally": tally,
        "sites": sites,
        "releases": releases,
    });
    let mut report = report;
    if let (Some(module), Some(lifecycle)) = (&lifecycle_module, &result.lifecycle)
        && let Some(object) = report.as_object_mut()
    {
        object.insert("lifecycle".to_owned(), lifecycle_json(module, lifecycle));
    }
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
}

/// A short spelling of one join's evidence (card W47-FLB-LIB-STRICT-PAIRING).
fn evidence_label(evidence: &JoinEvidence) -> String {
    match evidence {
        JoinEvidence::UncheckedFirst(exclusions) => format!(
            "unchecked first join ({})",
            if join_succeeds(exclusions).is_ok() {
                "counts as success".to_owned()
            } else {
                format!(
                    "not counted: joinable {:?}, not busy {:?}, not self {:?}, no cycle {:?}",
                    exclusions.joinable,
                    exclusions.not_busy,
                    exclusions.not_self,
                    exclusions.no_cycle
                )
            }
        ),
        other => format!("{other:?}"),
    }
}

fn status_json(status: &ClauseStatus) -> serde_json::Value {
    match status {
        ClauseStatus::InForce => serde_json::json!({"status": "in force"}),
        ClauseStatus::Refuted { witness } => {
            serde_json::json!({"status": "refuted by body", "witness": witness})
        }
        ClauseStatus::Held { reason } => serde_json::json!({"status": "held", "reason": reason}),
    }
}

/// Card W47-FLB-LIB-LIFECYCLE: the clauses per body, the calls per root and
/// the per-root conformance.
fn lifecycle_json(module: &str, lifecycle: &LifecycleCensus) -> serde_json::Value {
    let clauses = lifecycle
        .clauses
        .iter()
        .map(|check| {
            serde_json::json!({
                "clause": check.clause.name(),
                "function": check.function,
                "after_start": check.start,
                "result": status_json(&check.status),
                "paths": check.paths.iter().map(|path| serde_json::json!({
                    "joins": path.joins.iter().map(|(join, site)| format!("{} at {site}", join.label())).collect::<Vec<_>>(),
                    "unresolved_count": path.unresolved.len(),
                    "unresolved": path.unresolved.iter().take(4).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "start_paths": check.start_paths.iter().map(|path| serde_json::json!({
                    "left_unjoined": path.left_unjoined,
                    "unresolved_count": path.unresolved.len(),
                    "unresolved": path.unresolved.iter().take(4).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "start_unresolved": check.start_unresolved,
            })
        })
        .collect::<Vec<_>>();
    // Card W47-FLB-LIB-STRICT-PAIRING: what the stop's body does from the
    // states the strict pair forbids (outside the contract; never a clause).
    let observations = lifecycle
        .observations
        .iter()
        .map(|observation| {
            serde_json::json!({
                "function": observation.function,
                "forbidden_by": observation.forbidden_by.name(),
                "after_start": observation.start,
                "every_path_joins": observation.every_path_joins(),
                "paths": observation.paths.iter().map(|path| serde_json::json!({
                    "joins": path.joins.iter().map(|(join, site)| format!("{} at {site}", join.label())).collect::<Vec<_>>(),
                    "unresolved_count": path.unresolved.len(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let calls = lifecycle
        .calls
        .iter()
        .map(|call| {
            serde_json::json!({
                "root": call.reading.root,
                "file": call.reading.file,
                "line": call.reading.line,
                "caller": call.reading.caller,
                "callee": call.reading.callee,
                "operation": call.reading.operation.spelling(),
                "key": call.reading.key,
                "states": call.states,
                "undetermined": call.undetermined,
                "undetermined_reasons": call.undetermined_reasons,
                "violations": call.violations,
                "not_established": call.not_established,
                "governed_by": call.governed.iter().map(|(clause, status)| format!("{} ({})", clause.name(), status.label())).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let mut roots: BTreeMap<&str, (usize, usize, usize, usize)> = BTreeMap::new();
    for call in &lifecycle.calls {
        let entry = roots.entry(call.reading.root.as_str()).or_default();
        entry.0 += 1;
        entry.1 += call.violations.len();
        entry.2 += usize::from(call.undetermined);
        entry.3 += call.not_established.len();
    }
    let conformance = roots
        .iter()
        .map(
            |(root, (calls, violations, undetermined, not_established))| {
                (
                    (*root).to_owned(),
                    serde_json::json!({
                        "calls": calls,
                        "violations": violations,
                        "not_established": not_established,
                        "undetermined_calls": undetermined,
                        "result": if *violations > 0 {
                            "violates"
                        } else if *not_established > 0 {
                            "not established"
                        } else if *undetermined > 0 {
                            "conforms where derived"
                        } else {
                            "conforms"
                        },
                    }),
                )
            },
        )
        .collect::<BTreeMap<_, _>>();
    let destroys = lifecycle
        .destroy_bodies
        .iter()
        .map(|body| {
            serde_json::json!({
                "function": body.function,
                "paths": body.paths.len(),
                "joins": body.paths.iter().map(|p| p.joins.len()).sum::<usize>(),
                "paths_joining_the_started_thread": body.paths.iter().filter(|p| p.joins.iter().any(|(j, _)| matches!(j, ReadTraceJoin::First | ReadTraceJoin::Repeated))).count(),
                "join_sites": body.paths.iter().flat_map(|p| p.joins.iter().map(|(j, s)| format!("{j:?} at {s}"))).collect::<std::collections::BTreeSet<_>>(),
                "unresolved": body.paths.iter().flat_map(|p| p.unresolved.iter().cloned()).collect::<std::collections::BTreeSet<_>>().len(),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "module": module,
        "roles": lifecycle.roles.functions.iter().map(|(name, (parameter, op))| (name.clone(), format!("{} (parameter {parameter})", op.spelling()))).collect::<BTreeMap<_, _>>(),
        "creators": lifecycle.roles.creators,
        "clauses": clauses,
        "status": lifecycle.status.iter().map(|((clause, function), status)| serde_json::json!({
            "clause": clause.name(),
            "function": function,
            "result": status_json(status),
        })).collect::<Vec<_>>(),
        "outside_contract": observations,
        "destroy_bodies": destroys,
        "calls": calls,
        "conformance": conformance,
    })
}
