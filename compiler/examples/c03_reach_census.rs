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

//! Card **C03-REACH**: read-only census of the W4 recursive routes' reach over
//! the pinned Cycle 19 corpus.
//!
//! This example selects exactly the units the pinned compatibility route
//! selects (`quirin-audit pinned`), runs the
//! production custody derivation over each, and reports how every recursive
//! strongly connected component was disposed of — including, for a component
//! the neutral-frame route refused, which of the design's V1–V10 obligations
//! refused first and with what diagnostics.
//!
//! It publishes no verdict, writes no audit artifact and reads no cached
//! evidence. Its only output is the census JSON named on the command line.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_audit::compile_db::{
    CompileEntry, compile_entries, compiler_options_compatibility as compiler_options,
};

use qcore_c_gateway::legacy_census::{
    NeutralComponentCensus, NeutralComponentOutcome, NeutralRefusal, neutral_reach_census,
};

use qcore_compiler::contract_loader::load_resource_contract_manifest;
use qcore_compiler::profile_loader::verify_pin;
use qcore_compiler::source_profiles::{FLUENT_BIT_SOURCE_SET, load_profile};
use qcore_compiler::{C11TranslationUnit, LegacyFunctionScope, ResourceContracts};

fn main() -> ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example c03_reach_census");
    let mut arguments = std::env::args_os().skip(1);
    let Some(root) = arguments.next().map(PathBuf::from) else {
        eprintln!(
            "usage: c03_reach_census <fluent-bit-clone> <monkey-compile_commands.json> <census.json>"
        );
        return ExitCode::FAILURE;
    };
    let monkey_database = arguments.next().map(PathBuf::from);
    let output = arguments.next().map(PathBuf::from);
    match run(&root, monkey_database.as_deref(), output.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("c03_reach_census: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(root: &Path, monkey_database: Option<&Path>, output: Option<&Path>) -> Result<(), String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let profile = load_profile(FLUENT_BIT_SOURCE_SET).map_err(|error| error.to_string())?;
    let pin = profile
        .manifest()
        .pins
        .get(FLUENT_BIT_SOURCE_SET)
        .ok_or("the fluent-bit profile declares no `fluent-bit` pin")?;
    let revision = verify_pin(FLUENT_BIT_SOURCE_SET, pin, &root)
        .map_err(|error| format!("Fluent Bit pin check failed: {error}"))?;
    println!("Fluent Bit {revision} (recursive-component census; no verdict is published)");
    let manifest = profile
        .resource_contract_manifest_path()
        .ok_or("source-set profile supplies no resource contracts")?;
    let contracts = load_resource_contract_manifest(&manifest)?;

    let database = root.join("build/compile_commands.json");
    let entries = compile_entries(
        &fs::read_to_string(&database)
            .map_err(|error| format!("{}: {error}", database.display()))?,
    )?;

    let mut units = Vec::new();
    for selected_library in
        qcore_audit::load_library_selections(&profile.directory().join("audit.json"))?
    {
        let declared = profile
            .library(&selected_library.library)
            .ok_or("profile declares no selected library")?;
        let [selection] = declared.source_roots.as_slice() else {
            return Err("the census requires one source root per selected library".into());
        };
        let source_root = qcore_contract::profile_loader::source_root_path(&root, &selection.root);
        let selected = match &selection.selection {
            qcore_schemas::profile::UnitSelection::TopLevel => {
                let mut selected = entries
                    .iter()
                    .filter(|entry| {
                        entry.file.parent() == Some(source_root.as_path())
                            && entry.file.extension().is_some_and(|e| e == "c")
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                selected.sort_by(|left, right| left.file.cmp(&right.file));
                selected
            }
            qcore_schemas::profile::UnitSelection::Recursive => {
                let mut selected = entries
                    .iter()
                    .filter(|entry| {
                        entry.file.starts_with(&source_root)
                            && entry.file.extension().is_some_and(|e| e == "c")
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                selected.sort_by(|left, right| left.file.cmp(&right.file));
                selected.dedup_by(|left, right| left.file == right.file);
                selected
            }
            qcore_schemas::profile::UnitSelection::Exact { file } => entries
                .iter()
                .find(|entry| entry.file == root.join(file))
                .cloned()
                .into_iter()
                .collect(),
        };
        let library = selected_library
            .label
            .clone()
            .unwrap_or_else(|| selected_library.library.clone());
        for entry in selected {
            units.push((library.clone(), source_root.clone(), entry));
        }
    }
    if let Some(path) = monkey_database {
        let entries = compile_entries(
            &fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?,
        )?;
        let event = entries
            .iter()
            .find(|entry| entry.file.ends_with("mk_core/mk_event.c"))
            .ok_or("Monkey compile database has no mk_core/mk_event.c entry")?;
        let source_root = event
            .file
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .ok_or("could not derive the Monkey source root")?;
        let mut selected = entries
            .iter()
            .filter(|entry| is_monkey_unit(&source_root, &entry.file))
            .cloned()
            .collect::<Vec<_>>();
        selected.sort_by(|left, right| left.file.cmp(&right.file));
        selected.dedup_by(|left, right| left.file == right.file);
        for entry in selected {
            units.push(("monkey-libevent".to_owned(), source_root.clone(), entry));
        }
    }

    let mut rows = Vec::new();
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    let mut blockers: BTreeMap<String, usize> = BTreeMap::new();
    let mut admitted = 0usize;
    for (library, source_root, entry) in &units {
        let file = entry
            .file
            .strip_prefix(source_root)
            .unwrap_or(&entry.file)
            .to_string_lossy()
            .into_owned();
        let census =
            census_unit(entry, &contracts).map_err(|error| format!("{library}/{file}: {error}"))?;
        let lines = census
            .preprocessed_source
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for component in census.components {
            let (outcome, detail) = describe(&component, &lines, &mut blockers);
            *totals.entry(outcome.clone()).or_default() += 1;
            if outcome == "admitted" {
                admitted += 1;
            }
            rows.push(serde_json::json!({
                "library": library,
                "file": file,
                "members": component.members,
                "self_recursive": component.self_recursive,
                "outcome": outcome,
                "detail": detail,
            }));
        }
    }
    println!("units examined: {}", units.len());
    println!("recursive components: {}", rows.len());
    for (outcome, count) in &totals {
        println!("  {outcome}: {count}");
    }
    println!("neutral-route admissions: {admitted}");
    if !blockers.is_empty() {
        println!("blocker families at the first refusing obligation:");
        let mut ranked = blockers.iter().collect::<Vec<_>>();
        ranked.sort_by(|left, right| right.1.cmp(left.1).then(left.0.cmp(right.0)));
        for (family, count) in ranked.iter().take(20) {
            println!("  {count:5}  {family}");
        }
    }
    if let Some(path) = output {
        let document = serde_json::json!({
            "schema_version": 1,
            "card": "C03-REACH",
            "scope": "read-only reach census of the W4 recursive routes; publishes no verdict",
            "source_revision": revision,
            "units_examined": units.len(),
            "recursive_components": rows.len(),
            "outcome_totals": totals,
            "neutral_route_admissions": admitted,
            "first_refusal_blocker_families": blockers,
            "components": rows,
        });
        fs::write(
            path,
            serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("{}: {error}", path.display()))?;
        println!("census: {}", path.display());
    }
    Ok(())
}

fn census_unit(
    entry: &CompileEntry,
    contracts: &ResourceContracts,
) -> Result<qcore_c_gateway::legacy_census::UnitCensus, String> {
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    let options = compiler_options(entry)?;
    let unit = C11TranslationUnit {
        source: &source,
        include_paths: options.include_paths,
        definitions: options.definitions,
        preprocessor_options: options.preprocessor_options,
        dialect: options.dialect,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        input: options.input,
    };
    neutral_reach_census(&unit, contracts, None).map_err(|error| error.to_string())
}

/// Render one component's outcome, accumulating the diagnostic families that
/// refused a member under **V5**.
fn describe(
    component: &NeutralComponentCensus,
    lines: &[String],
    blockers: &mut BTreeMap<String, usize>,
) -> (String, serde_json::Value) {
    match &component.outcome {
        NeutralComponentOutcome::DeclaredContract => {
            ("declared-contract".to_owned(), serde_json::Value::Null)
        }
        NeutralComponentOutcome::PublishedByOrdinaryInference => (
            "published-by-ordinary-inference".to_owned(),
            serde_json::Value::Null,
        ),
        NeutralComponentOutcome::PublishedByRankedRecursion { members } => (
            "published-by-ranked-recursion".to_owned(),
            serde_json::json!({ "members": members }),
        ),
        NeutralComponentOutcome::PartiallyPublished { published } => (
            "partially-published".to_owned(),
            serde_json::json!({ "published": published }),
        ),
        NeutralComponentOutcome::Admitted { members, refused } => (
            "admitted".to_owned(),
            serde_json::json!({
                "admitted": members,
                "refused": refused.iter().map(|entry| refusal(&entry.function, &entry.refusal, lines, blockers)).collect::<Vec<_>>(),
            }),
        ),
        NeutralComponentOutcome::Refused { members } => (
            "refused".to_owned(),
            serde_json::json!({
                "refused": members.iter().map(|entry| refusal(&entry.function, &entry.refusal, lines, blockers)).collect::<Vec<_>>(),
            }),
        ),
    }
}

fn refusal(
    function: &str,
    refusal: &NeutralRefusal,
    lines: &[String],
    blockers: &mut BTreeMap<String, usize>,
) -> serde_json::Value {
    let mut record = serde_json::json!({
        "function": function,
        "obligation": refusal.obligation(),
    });
    match refusal {
        NeutralRefusal::CfgUnavailable(reason) => {
            record["reason"] = serde_json::json!(reason);
        }
        NeutralRefusal::RetainedUnknownInstruction { descriptions } => {
            let mut families: BTreeMap<String, usize> = BTreeMap::new();
            let mut sites = Vec::new();
            for (line, description) in descriptions {
                let text = normalize(description);
                *families.entry(text.clone()).or_default() += 1;
                *blockers.entry(text).or_default() += 1;
                sites.push(serde_json::json!({
                    "line": line,
                    "description": description,
                    "source": lines
                        .get(line.saturating_sub(1))
                        .map(|text| text.trim())
                        .unwrap_or(""),
                }));
            }
            record["retained"] = serde_json::json!(descriptions.len());
            record["retained_families"] = serde_json::json!(families);
            record["retained_sites"] = serde_json::json!(sites);
        }
        NeutralRefusal::BodyNotClean {
            status,
            diagnostics,
        } => {
            let mut families: BTreeMap<String, usize> = BTreeMap::new();
            for diagnostic in diagnostics {
                let text = describe_diagnostic(diagnostic);
                *families.entry(text.clone()).or_default() += 1;
                *blockers.entry(text).or_default() += 1;
            }
            record["status"] = serde_json::json!(format!("{status:?}"));
            record["diagnostics"] = serde_json::json!(diagnostics.len());
            record["diagnostic_families"] = serde_json::json!(families);
        }
        _ => {}
    }
    record
}

/// Collapse a description's backquoted identifier so families aggregate:
/// ``call to uncontracted function `foo` `` and the same for `bar` are one
/// blocker family, not two.
fn normalize(description: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for character in description.chars() {
        if character == '`' {
            if !inside {
                out.push_str("`_`");
            }
            inside = !inside;
        } else if !inside {
            out.push(character);
        }
    }
    out
}

fn describe_diagnostic(diagnostic: &qcore_compiler::LegacyDiagnostic) -> String {
    use qcore_compiler::LegacyDiagnosticKind as Kind;
    match &diagnostic.kind {
        Kind::Unsupported { description } => description.clone(),
        Kind::UnknownAlias { .. } => "unresolved resource identity".to_owned(),
        Kind::Leak { .. } => "live obligation (leak)".to_owned(),
        Kind::DoubleRelease { .. } => "repeated release".to_owned(),
        Kind::ReleaseAfterEscape { .. } => "release after transfer".to_owned(),
        Kind::CustodyConflict { .. } => "conflicting custody".to_owned(),
        Kind::InvalidRelease { .. } => "release of a derived pointer".to_owned(),
        Kind::UseAfterRelease { .. } => "use of a derived pointer after release".to_owned(),
        Kind::KindMismatchedRelease { .. } => "release of another resource kind".to_owned(),
        Kind::UseAfterClose { .. } => "use of a released handle".to_owned(),
        Kind::CountedUser { .. } => "refuted counted-user obligation".to_owned(),
        Kind::SummaryConflict { .. } => "inferred summary contradicts declaration".to_owned(),
    }
}

fn is_monkey_unit(source_root: &Path, file: &Path) -> bool {
    let Ok(relative) = file.strip_prefix(source_root) else {
        return false;
    };
    if file.extension().is_none_or(|extension| extension != "c") {
        return false;
    }
    let relative = relative.to_string_lossy();
    (relative.starts_with("mk_core/") && !relative.starts_with("mk_core/deps/"))
        || relative.starts_with("mk_server/")
        || relative.starts_with("tls/")
}
