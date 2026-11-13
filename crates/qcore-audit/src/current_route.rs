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

//! The configured current-revision route (card **W44-QUIRIN-AUDIT-PROVIDERS**),
//! moved here from `compiler/examples/support/cycle19_compatibility.rs`
//! unchanged in behaviour: `quirin-audit current` runs it, and the compiler
//! example's configured route is a thin wrapper over it.
//!
//! Every selected compilation configuration of the profile's `audit.json`
//! library groups, then of an optional Monkey (forced-Libevent) database, is
//! one [`CurrentAudit`] task, analysed on up to `jobs` threads and folded in
//! selection order (card W44-AUDIT-PARALLEL-UNITS). Card
//! CURRENT-MONKEY-LINKED-CONFIGURATION: with `monkey_linked`, Monkey's units
//! come from the main compile database instead (the configuration linked into
//! `fluent-bit`, label `monkey`), and a Monkey database is only accounted
//! for: each of its units is replaced by the linked unit of the same file or
//! stays as a not-linked `Unknown` unit. Every database record must
//! name the configured Clang C driver (`/usr/bin/cc` and other substitutes
//! refuse before any unit is analysed). Historical source/native/provider
//! evidence is not admitted: its sidecar paths are overwritten with explicit
//! Unknown records so no older Checked evidence survives beside a new
//! inventory.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use qcore_contract::profile_loader::LoadedProfile;
use serde_json::Value;

use crate::compile_db::{
    CompileEntry, compile_entries, compiler_options_compatibility as compiler_options,
};
use crate::cross_unit_links::{DerivedResolution, LinkMap};
use crate::current::{CurrentAudit, EntryOutcome, distinct_configurations, select_configurations};
use crate::evidence::EvidenceMode;
use crate::parallel::run_ordered;
use crate::providers::ContractInput;
use crate::unit_selection::UnitSelection;

/// Everything the configured current-revision route reads and writes.
pub struct CurrentRoute<'a> {
    /// The checkout under audit; its `HEAD` is the audited revision.
    pub checkout: &'a Path,
    /// The source profile: library groups (`audit.json`), source roots, pins.
    pub profile: &'a LoadedProfile,
    /// The resource contracts: a manifest or a lowered provider selection.
    pub contracts: &'a ContractInput,
    /// Which evidence the run admits.
    pub mode: EvidenceMode,
    /// The main compilation database; `<checkout>/build/compile_commands.json`
    /// when absent.
    pub compile_database: Option<&'a Path>,
    /// The Monkey compilation database, whose units join as `monkey-libevent`.
    /// Under [`Self::monkey_linked`] it is only accounted for, never analysed.
    pub monkey_database: Option<&'a Path>,
    /// Card CURRENT-MONKEY-LINKED-CONFIGURATION: analyse Monkey's units as the
    /// main compile database configures them (the configuration linked into
    /// `fluent-bit`), under [`LINKED_MONKEY_LABEL`]. With
    /// [`Self::monkey_database`], each standalone unit is replaced by its
    /// linked counterpart (the same file) or, without one, stays in the
    /// denominator as a not-linked `Unknown` unit under its own key.
    pub monkey_linked: bool,
    /// Report paths. The diagnostic report also names its `.contracts.json`,
    /// `.chunk-contracts.json` and `.source-effects.json` siblings.
    pub monkey_source_report: Option<&'a Path>,
    pub diagnostic_report: Option<&'a Path>,
    pub binding_directory: Option<&'a Path>,
    /// The first console line's label (`<label> <revision> (current checkout)`).
    pub label: &'a str,
    /// The second console line, naming where historical evidence lives.
    pub historical_note: &'a str,
    /// The historical reference revision the Unknown sidecars name.
    pub historical_reference_revision: &'a str,
    /// Translation units analysed concurrently (already resolved).
    pub jobs: usize,
    /// Card W45-FB-SYSTEM-SELECTION: an explicit unit selection replacing the
    /// profile's `audit.json` groups, or `None` for the profile's own groups.
    pub selection: Option<&'a UnitSelection>,
    /// Optional per-library provider scoping table: each library slice is
    /// audited sequentially under its own provider selection plus its
    /// transitive `dependent_libraries` closure, dropping the slice's
    /// contract environment and unit outcomes before the next slice starts.
    pub library_providers: Option<&'a crate::providers::ResolvedLibraryProvidersTable>,
}

/// One configured-route compilation configuration, in selection order.
#[derive(Clone)]
pub(crate) struct ConfiguredTask<'a> {
    pub(crate) group: usize,
    pub(crate) library: &'a str,
    pub(crate) expected_source: Result<PathBuf, String>,
    pub(crate) input: TaskInput<'a>,
    pub(crate) unresolved: Option<&'a str>,
}

/// What a task analyses: a compilation record, or a selected file the
/// database has no record for (card W45-FB-SYSTEM-SELECTION), which stays in
/// the denominator as an `Unknown` unit.
#[derive(Clone, Copy)]
pub(crate) enum TaskInput<'a> {
    Configured(&'a CompileEntry),
    Absent(&'a str),
    /// Card CURRENT-MONKEY-LINKED-CONFIGURATION: a standalone Monkey unit the
    /// linked configuration does not compile (checkout-relative file, reason).
    NotLinked(&'a str, &'a str),
}

/// One selected group: its report label, its configured units in order (a
/// file without a record is `Absent` at its own position), and its refusal.
type Group<'a> = (String, Vec<Selected<'a>>, Option<&'static str>);

enum Selected<'a> {
    Configured(CompileEntry),
    Absent(&'a str),
    NotLinked { file: String, reason: String },
}

/// Card CURRENT-MONKEY-LINKED-CONFIGURATION: the report label (and unit key
/// prefix) of Monkey's units in the configuration linked into `fluent-bit`.
pub const LINKED_MONKEY_LABEL: &str = "monkey";
/// The label of the standalone, forced-Libevent Monkey configuration.
pub const STANDALONE_MONKEY_LABEL: &str = "monkey-libevent";

/// Card CURRENT-MONKEY-LINKED-CONFIGURATION: Monkey's units in the main
/// compile database, by the route's Monkey unit rule ([`is_monkey_unit`])
/// under `<checkout>/lib/monkey`: every distinct configuration, records
/// matched by located path and two spellings of one file refused. Empty when
/// the database compiles none; [`run_current_route`] refuses that, and the
/// link step reads it as no Monkey unit.
///
/// # Errors
/// An aliased spelling (`input-path-alias`).
pub fn linked_monkey_configurations(
    entries: &[CompileEntry],
    checkout: &Path,
) -> Result<Vec<CompileEntry>, String> {
    let root = checkout.join("lib/monkey");
    let selected = entries
        .iter()
        .filter(|entry| is_monkey_unit(&root, &entry.file))
        .collect::<Vec<_>>();
    crate::input_paths::refuse_aliases(selected.iter().copied())?;
    Ok(distinct_configurations(selected.into_iter().cloned()))
}

/// The checkout-relative file of a linked Monkey configuration.
fn checkout_relative(file: &Path, checkout: &Path) -> Result<String, String> {
    crate::input_paths::relative_within(file, checkout)
        .map(|relative| crate::repository_path(&relative))
        .ok_or_else(|| format!("Monkey source outside the checkout: {}", file.display()))
}

/// Card CURRENT-MONKEY-LINKED-CONFIGURATION: how a linked run accounts for
/// the standalone database's units: each is replaced by the linked unit of
/// the same file, or kept as a not-linked unit. The record joins the
/// report's assumptions.
struct LinkedMonkey {
    entries: Vec<CompileEntry>,
    not_linked: Vec<(String, String)>,
    record: Value,
}

fn linked_monkey(
    entries: &[CompileEntry],
    root: &Path,
    standalone: Option<&Path>,
) -> Result<LinkedMonkey, String> {
    let linked = linked_monkey_configurations(entries, root)?;
    if linked.is_empty() {
        return Err(
            "--monkey-linked: the compile database contains no Monkey sources under lib/monkey (mk_core/, mk_server/, tls/)"
                .into(),
        );
    }
    for entry in &linked {
        crate::compile_db::require_clang_driver(entry)?;
    }
    let files = linked
        .iter()
        .map(|entry| checkout_relative(&entry.file, root))
        .collect::<Result<Vec<_>, _>>()?;
    let mut replaced = Vec::new();
    let mut not_linked = Vec::new();
    let mut pairs = Vec::new();
    if let Some(path) = standalone {
        let standalone_entries =
            compile_entries(&fs::read_to_string(path).map_err(|error| error.to_string())?)?;
        let source_root = monkey_source_root(&standalone_entries)?;
        crate::input_paths::refuse_aliases(
            standalone_entries
                .iter()
                .filter(|entry| is_monkey_unit(&source_root, &entry.file)),
        )?;
        let selected = distinct_configurations(
            standalone_entries
                .into_iter()
                .filter(|entry| is_monkey_unit(&source_root, &entry.file)),
        );
        let mut seen = std::collections::BTreeSet::new();
        for entry in &selected {
            let relative = crate::input_paths::relative_within(&entry.file, &source_root)
                .ok_or_else(|| "Monkey source outside compiled root".to_owned())?;
            let file = format!("lib/monkey/{}", crate::repository_path(&relative));
            // Two standalone configurations of one file are one unit key.
            if !seen.insert(file.clone()) {
                continue;
            }
            let standalone_key = format!("{STANDALONE_MONKEY_LABEL}/{file}");
            if files.contains(&file) {
                replaced.push(serde_json::json!({
                    "standalone": standalone_key,
                    "linked": format!("{LINKED_MONKEY_LABEL}/{file}"),
                }));
            } else {
                let reason = format!(
                    "not linked: the configuration linked into fluent-bit (the run's compile database) does not compile {file}; the standalone Monkey configuration (--monkey-database) is accounted for, not analysed, under --monkey-linked"
                );
                not_linked.push(serde_json::json!({"unit": standalone_key, "reason": reason}));
                pairs.push((file, reason));
            }
        }
    }
    let record = serde_json::json!({
        "analysed": "linked",
        "label": LINKED_MONKEY_LABEL,
        "meaning": "Monkey's units (lib/monkey mk_core/ without mk_core/deps/, mk_server/, tls/) as the run's compile database configures them, which is the configuration linked into fluent-bit. A unit of the standalone Monkey database (--monkey-database) is replaced by the linked unit of the same file; one the linked configuration does not compile stays in the denominator as a not-linked Unknown unit under its own key. Nothing is dropped.",
        "units": files,
        "standalone_database": standalone,
        "replaced": replaced,
        "not_linked": not_linked,
    });
    Ok(LinkedMonkey {
        entries: linked,
        not_linked: pairs,
        record,
    })
}

/// Run the route; `Ok(true)` when any unit hit a gateway failure.
pub fn run_current_route(route: &CurrentRoute<'_>) -> Result<bool, String> {
    run_current_route_with_checkpoints(route, None)
}

/// Card **W45-AUDIT-CHECKPOINTS**: run the route, or, with `checkpoints`,
/// analyse it one group at a time, sealing each group into a `.qo` checkpoint
/// and composing the report from the checkpoints
/// ([`crate::checkpoint_run`]). Without `checkpoints` this is exactly
/// [`run_current_route`].
pub fn run_current_route_with_checkpoints(
    route: &CurrentRoute<'_>,
    checkpoints: Option<&crate::checkpoint_run::CheckpointOptions>,
) -> Result<bool, String> {
    let stated_links = LinkMap::load_selected()?;
    // Card LINK-STANDING (rule 40): a supplied link-resolution artifact is
    // the run's resolution, derived instead of stated.
    let derived_resolution = DerivedResolution::load_selected()?;
    if stated_links.is_some() && derived_resolution.is_some() {
        return Err(format!(
            "{} and {} are exclusive: a run either states its link resolution or derives it",
            crate::cross_unit_links::SELECTION,
            crate::cross_unit_links::RESOLUTION_SELECTION
        ));
    }
    let opted = stated_links.is_some() || derived_resolution.is_some();
    // Card CURRENT-RUN-WORKER-SPIN: per-phase and per-unit progress on
    // standard error for the cross-unit route (or when asked).
    crate::route_progress::enable_if(opted);
    if opted && route.mode.is_contracts_only() {
        return Err("cross-unit imports require configured home and caller units".into());
    }
    if opted && checkpoints.is_some() {
        return Err("cross-unit imports are not supported by checkpoint composition".into());
    }
    let root = route
        .checkout
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let profile = route.profile;
    let mut audit = CurrentAudit::with_contracts(&root, profile, route.contracts, route.mode)?;
    if let Some(selection) = route.selection {
        audit.record_selection(selection.record());
    }
    let revision = audit.revision().to_owned();
    let mut preamble = vec![
        format!("{} {revision} (current checkout)", route.label),
        route.historical_note.to_owned(),
    ];
    if route.mode.is_contracts_only() {
        preamble.push(
            "evidence: contracts-only (resource projection from the selected contract modules; no package, verifier-host, plugin, native or pinned evidence)".to_owned()
        );
    }
    for line in &preamble {
        println!("{line}");
    }
    let default_database = root.join("build/compile_commands.json");
    let database_text = fs::read_to_string(route.compile_database.unwrap_or(&default_database))
        .map_err(|error| error.to_string())?;
    let entries = compile_entries(&database_text)?;
    // Card LINK-STANDING (rule 40): the artifact must describe this database.
    let database_sha256 = qcore_schemas::sha256_hex(database_text.as_bytes());
    drop(database_text);
    // Select all groups before auditing; missing groups cannot disappear from
    // the denominator. Different commands for one file remain separate tasks.
    let mut groups: Vec<Group<'_>> = Vec::new();
    let mut group_keys: Vec<(String, String)> = Vec::new();
    let unresolved_for = |library: &qcore_schemas::profile::LibraryProfile| {
        (!library.required_evidence.is_empty()).then_some("selected library requires evidence unavailable in current-revision resource projection")
    };
    if let Some(selection) = route.selection {
        for group in &selection.groups {
            let library = profile.library(&group.library).ok_or_else(|| {
                format!(
                    "unit selection `{}` names library `{}`, which the source profile does not declare",
                    selection.name, group.library
                )
            })?;
            let mut selected = Vec::new();
            // Card AUDIT-CANONICAL-PATHS: two selected files that locate to
            // one source would analyse its records twice; refused, named.
            let mut claimed = BTreeMap::<PathBuf, &str>::new();
            for file in &group.files {
                let configurations = select_file_configurations(&entries, &root, file)?;
                if let Some(first) = configurations.first() {
                    let located = crate::input_paths::locate(&first.file).path().to_path_buf();
                    if let Some(prior) = claimed.insert(located.clone(), file.as_str()) {
                        return Err(format!(
                            "{}: selected files `{prior}` and `{file}` of group `{}` locate to one source file {}; the audit neither merges nor double-counts them",
                            crate::input_paths::ALIAS_DIAGNOSTIC,
                            group.label(),
                            located.display()
                        ));
                    }
                }
                if configurations.is_empty() {
                    selected.push(Selected::Absent(file.as_str()));
                }
                for entry in configurations {
                    crate::compile_db::require_clang_driver(&entry)?;
                    selected.push(Selected::Configured(entry));
                }
            }
            group_keys.push((group.library.clone(), group.label().to_owned()));
            groups.push((group.label().to_owned(), selected, unresolved_for(library)));
        }
    } else {
        for selection in crate::load_library_selections(&profile.directory().join("audit.json"))? {
            let library = profile
                .library(&selection.library)
                .ok_or("selected library is absent from profile")?;
            let selected = select_configurations(&entries, &root, library)?;
            if selected.is_empty() {
                return Err(format!(
                    "compile database contains no selected {} sources",
                    selection.library
                ));
            }
            for entry in &selected {
                crate::compile_db::require_clang_driver(entry)?;
            }
            let label = selection.label.unwrap_or_else(|| selection.library.clone());
            group_keys.push((selection.library, label.clone()));
            groups.push((
                label,
                selected.into_iter().map(Selected::Configured).collect(),
                unresolved_for(library),
            ));
        }
    }
    // Card CURRENT-MONKEY-LINKED-CONFIGURATION: Monkey from the main compile
    // database (the configuration linked into fluent-bit), after the
    // profile's or the selection's groups; the standalone database, when
    // given, is accounted for and not analysed.
    if route.monkey_linked {
        let linked = linked_monkey(&entries, &root, route.monkey_database)?;
        let monkey_root = root.join("lib/monkey");
        for (label, selected, _) in &groups {
            if label == LINKED_MONKEY_LABEL || label == STANDALONE_MONKEY_LABEL {
                return Err(format!(
                    "--monkey-linked: group `{label}` already uses the label of a Monkey configuration"
                ));
            }
            for unit in selected {
                let file = match unit {
                    Selected::Configured(entry) => entry.file.clone(),
                    Selected::Absent(file) => root.join(file),
                    Selected::NotLinked { file, .. } => root.join(file),
                };
                if is_monkey_unit(&monkey_root, &file) {
                    return Err(format!(
                        "--monkey-linked: group `{label}` already selects the Monkey unit {}; a unit is analysed under one configuration",
                        file.display()
                    ));
                }
            }
        }
        let line = format!(
            "{LINKED_MONKEY_LABEL}: Monkey as linked into fluent-bit ({} unit(s) of the compile database); {} standalone unit(s) replaced, {} not linked",
            linked.entries.len(),
            linked.record["replaced"].as_array().map_or(0, Vec::len),
            linked.not_linked.len()
        );
        println!("{line}");
        preamble.push(line);
        audit.record_monkey_configuration(linked.record);
        group_keys.push((LINKED_MONKEY_LABEL.into(), LINKED_MONKEY_LABEL.into()));
        groups.push((
            LINKED_MONKEY_LABEL.into(),
            linked
                .entries
                .into_iter()
                .map(Selected::Configured)
                .collect(),
            None,
        ));
        if !linked.not_linked.is_empty() {
            group_keys.push((
                STANDALONE_MONKEY_LABEL.into(),
                STANDALONE_MONKEY_LABEL.into(),
            ));
            groups.push((
                STANDALONE_MONKEY_LABEL.into(),
                linked
                    .not_linked
                    .into_iter()
                    .map(|(file, reason)| Selected::NotLinked { file, reason })
                    .collect(),
                None,
            ));
        }
    }
    let monkey = route
        .monkey_database
        .filter(|_| !route.monkey_linked)
        .map(|path| {
            let entries =
                compile_entries(&fs::read_to_string(path).map_err(|error| error.to_string())?)?;
            let source_root = monkey_source_root(&entries)?;
            // Card AUDIT-CANONICAL-PATHS: two spellings of one Monkey unit
            // are refused, never two units.
            crate::input_paths::refuse_aliases(
                entries
                    .iter()
                    .filter(|entry| is_monkey_unit(&source_root, &entry.file)),
            )?;
            let selected = distinct_configurations(
                entries
                    .into_iter()
                    .filter(|entry| is_monkey_unit(&source_root, &entry.file)),
            );
            if selected.is_empty() {
                return Err("no compiled Monkey sources selected".into());
            }
            for entry in &selected {
                crate::compile_db::require_clang_driver(entry)?;
            }
            // The copy may have been prepared at an older revision. It is usable
            // only where its actual source and adapter/header bytes still match.
            let unresolved =
                verify_monkey_source_copy(&root.join("lib/monkey"), &source_root, &selected).err();
            Ok::<_, String>((source_root, selected, unresolved))
        })
        .transpose()?;
    // Card MEMBER-PREPASS-OTHER-ROUTES (rule 12n; user decisions R1, F1):
    // before any unit is analysed, read the member facts of every unit of the
    // route's compile database and of the Monkey units it analyses, so each
    // unit decides a member invocation over the run's set, as the pinned
    // route does. A selection chooses the units analysed, not the program:
    // the pre-pass reads the whole database.
    let mut prepass_entries = entries.clone();
    if let Some((_, monkey_entries, _)) = &monkey {
        prepass_entries.extend(monkey_entries.iter().cloned());
    }
    let scope = crate::member_prepass::load_scope(profile.directory())?;
    // Card OUTER-CONTRACT (rule 31): what each loader's binders reach.
    crate::outer_selection::compute_reach(&prepass_entries)?;
    let prepass_phase = crate::route_progress::Phase::begin(
        "member facts pre-pass",
        prepass_entries.len(),
        route.jobs,
    );
    let prepass = crate::member_prepass::run(
        &prepass_entries,
        audit.contracts(),
        &scope,
        Some(&root),
        route.jobs,
    );
    prepass_phase.end(if prepass.is_some() { "read" } else { "not run" });
    if let Some(prepass) = prepass {
        // The line joins the preamble a checkpoint composer prints.
        preamble.push(crate::member_prepass::announce(
            &prepass,
            audit.contracts(),
        )?);
        audit.set_member_prepass(prepass);
    }
    drop(prepass_entries);
    // Card W44-AUDIT-PARALLEL-UNITS: every selected configuration is one task,
    // in exactly the order the sequential loops visited them. Tasks may be
    // analysed concurrently; their console lines, report rows and tallies are
    // folded strictly in this order, and a group's header is printed just
    // before its first configuration's lines, where the loops printed it.
    let mut headers = Vec::new();
    let mut tasks = Vec::new();
    for (library, selected, unresolved) in &groups {
        headers.push(format!(
            "{library}: {} selected compilation configuration(s)",
            selected.len()
        ));
        for selected in selected {
            let (expected_source, input) = match selected {
                Selected::Configured(entry) => {
                    (Ok(entry.file.clone()), TaskInput::Configured(entry))
                }
                Selected::Absent(file) => (Ok(root.join(file)), TaskInput::Absent(file)),
                Selected::NotLinked { file, reason } => {
                    (Ok(root.join(file)), TaskInput::NotLinked(file, reason))
                }
            };
            tasks.push(ConfiguredTask {
                group: headers.len() - 1,
                library: library.as_str(),
                expected_source,
                input,
                unresolved: *unresolved,
            });
        }
    }
    if let Some((source_root, entries, unresolved)) = &monkey {
        group_keys.push(("monkey-libevent".into(), "monkey-libevent".into()));
        headers.push(format!(
            "monkey-libevent: {} selected compilation configuration(s)",
            entries.len()
        ));
        for entry in entries {
            let expected_source = crate::input_paths::relative_within(&entry.file, source_root)
                .map(|relative| root.join("lib/monkey").join(relative))
                .ok_or_else(|| "Monkey source outside compiled root".to_owned());
            tasks.push(ConfiguredTask {
                group: headers.len() - 1,
                library: "monkey-libevent",
                expected_source,
                input: TaskInput::Configured(entry),
                unresolved: unresolved.as_deref(),
            });
        }
    }
    // Card LINK-STANDING (rule 40): the run-level conditions (database and
    // bound digests), then the home-level conditions against the route's own
    // configurations. A refused artifact leaves every call uncontracted.
    let mut derived = None;
    let mut link_refused = None;
    let cross_unit_links = match derived_resolution {
        None => stated_links,
        Some(resolution) => match resolution.verify(&database_sha256) {
            Err((refusal, detail)) => {
                println!(
                    "cross-unit import phase refused: {}: {detail}",
                    refusal.phrase()
                );
                link_refused = Some(serde_json::json!({
                    "evidence": "derived",
                    "refused": refusal.phrase(),
                    "detail": detail,
                    "artifact": resolution.path,
                    "artifact_sha256": resolution.artifact_sha256,
                }));
                None
            }
            Ok(()) => {
                let route_units = tasks
                    .iter()
                    .filter_map(|task| match (&task.expected_source, task.input) {
                        (Ok(expected_source), TaskInput::Configured(entry)) => {
                            crate::input_paths::relative_within(expected_source, &root).map(
                                |file| {
                                    (
                                        format!(
                                            "{}/{}",
                                            task.library,
                                            crate::repository_path(&file)
                                        ),
                                        crate::cross_unit_links::ConfigurationIdentity::of(entry),
                                    )
                                },
                            )
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let found = resolution.derive(&route_units);
                println!(
                    "cross-unit link resolution derived (rule 40) from {} (sha256 {}): {} home(s); refused {}",
                    resolution.path.display(),
                    resolution.artifact_sha256,
                    found.map.requested().len(),
                    found.refused_by_reason()
                );
                let map = found.map.clone();
                derived = Some(found);
                (!map.requested().is_empty()).then_some(map)
            }
        },
    };
    if let Some(options) = checkpoints {
        let unresolved = groups
            .iter()
            .map(|(_, _, unresolved)| *unresolved)
            .chain(
                monkey
                    .iter()
                    .map(|(_, _, unresolved)| unresolved.as_deref()),
            )
            .collect::<Vec<_>>();
        let plan = crate::checkpoint_run::RoutePlan {
            preamble,
            headers,
            unresolved,
            tasks,
        };
        // Card W45-REFUTED-CONTRACT-WITHDRAWAL: phase one under the loaded
        // contracts; when the composed rows refute a declaration, every group
        // again under that table, whose digest joins each group's identity.
        audit.set_refuted_table(&qcore_schemas::contract_refutation::RefutationTable::new());
        let (failed, found) = crate::checkpoint_run::run(route, &audit, plan.clone(), options)?;
        if found.is_empty() {
            return Ok(failed);
        }
        audit.set_refuted_table(&found);
        return crate::checkpoint_run::run(route, &audit, plan, options).map(|(failed, _)| failed);
    }
    if let Some(library_table) = route.library_providers {
        if opted {
            return Err(
                "cross-unit imports are not supported with per-library provider scoping".into(),
            );
        }
        let mut accumulated_refutations =
            qcore_schemas::contract_refutation::RefutationTable::new();
        for (group_idx, header) in headers.iter().enumerate() {
            let (library_name, label) = &group_keys[group_idx];
            let slice_providers = library_table
                .slice_for(label)
                .or_else(|_| library_table.slice_for(library_name))?;
            audit.activate_slice_contracts(&slice_providers.input)?;
            let slice_tasks: Vec<ConfiguredTask<'_>> = tasks
                .iter()
                .filter(|task| task.group == group_idx)
                .cloned()
                .collect();
            let slice_dispatch = task_dispatch(&slice_tasks);
            let base_refutations: qcore_schemas::contract_refutation::RefutationTable =
                accumulated_refutations
                    .iter()
                    .filter(|(name, _)| {
                        qcore_c_gateway::contract_dependencies::declared(audit.contracts(), name)
                    })
                    .map(|(name, entry)| (name.clone(), entry.clone()))
                    .collect();
            let phases = crate::refuted_contracts::run_two_phases_with_bounds(
                |table| {
                    let mut effective = base_refutations.clone();
                    effective.extend(table.iter().map(|(k, v)| (k.clone(), v.clone())));
                    audit.set_refuted_table(&effective);
                    let mut outcomes = Vec::new();
                    let stopped = run_ordered(
                        slice_tasks.clone(),
                        route.jobs,
                        Some(slice_dispatch.clone()),
                        |task| (task.group, evaluate_task(&audit, &task)),
                        |(group, outcome): (usize, EntryOutcome)| {
                            let outcome = outcome.park();
                            let failed = outcome.result.is_err();
                            outcomes.push((group, outcome));
                            if failed { Err(()) } else { Ok(()) }
                        },
                    )
                    .err();
                    (outcomes, stopped)
                },
                |outcomes| {
                    let mut found = base_refutations.clone();
                    found.extend(retained_refutations(
                        outcomes
                            .iter()
                            .filter_map(|(_, outcome)| outcome.result.as_ref().ok())
                            .map(|products| &products.report.refutations),
                    ));
                    found
                },
                |outcomes| {
                    retained_bounds(
                        outcomes
                            .iter()
                            .filter_map(|(_, outcome)| outcome.result.as_ref().ok())
                            .map(|products| &products.report.bounds_refutations),
                    )
                },
            );
            accumulated_refutations.extend(base_refutations);
            accumulated_refutations.extend(phases.applied);
            println!("{header}");
            for (_, outcome) in phases.results {
                print!("{}", outcome.output);
                audit.absorb_entry(outcome.result?.restore()?)?;
            }
            crate::contract_dependencies::clear_premises_cache();
        }
        audit.restore_run_contracts(
            &library_table.union_input,
            &accumulated_refutations,
            library_table.record(),
        )?;
        let counts = audit.finish(route.diagnostic_report)?;
        write_unknown_sidecars(
            &unknown_sidecar(&revision, route.historical_reference_revision),
            route.diagnostic_report,
            route.monkey_source_report,
            route.binding_directory,
        )?;
        return Ok(counts.gateway_failures != 0);
    }
    let mut printed_headers = 0;
    let dispatch = task_dispatch(&tasks);
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: every task is analysed under the
    // loaded contracts and, when a body refutes its own declaration, once more
    // under the refutation table; only the published phase is printed and
    // folded, in task order, stopping at its first error as before.
    let phases = crate::refuted_contracts::run_two_phases_with_bounds(
        |table| {
            audit.set_refuted_table(table);
            let progress = crate::route_progress::Phase::begin(
                if table.is_empty() {
                    "units under the loaded contracts"
                } else {
                    "units under the refutation table"
                },
                tasks.len(),
                route.jobs,
            );
            let mut outcomes = Vec::new();
            let stopped = run_ordered(
                tasks.clone(),
                route.jobs,
                Some(dispatch.clone()),
                |task| {
                    let outcome =
                        progress.unit(&task_label(&root, &task), || evaluate_task(&audit, &task));
                    (task.group, outcome)
                },
                |(group, outcome): (usize, EntryOutcome)| {
                    let outcome = outcome.park();
                    let failed = outcome.result.is_err();
                    outcomes.push((group, outcome));
                    if failed { Err(()) } else { Ok(()) }
                },
            )
            .err();
            progress.end(if stopped.is_some() {
                "stopped at a failed unit"
            } else {
                ""
            });
            (outcomes, stopped)
        },
        |outcomes| {
            retained_refutations(
                outcomes
                    .iter()
                    .filter_map(|(_, outcome)| outcome.result.as_ref().ok())
                    .map(|products| &products.report.refutations),
            )
        },
        // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14h).
        |outcomes| {
            retained_bounds(
                outcomes
                    .iter()
                    .filter_map(|(_, outcome)| outcome.result.as_ref().ok())
                    .map(|products| &products.report.bounds_refutations),
            )
        },
    );
    audit.set_refuted_table(&phases.applied);
    let mut published = phases.results;
    if let Some(links) = &cross_unit_links {
        let mut exports = Vec::new();
        let mut definitions = Vec::new();
        let mut census_complete = true;
        // Card CURRENT-RUN-WORKER-SPIN: the units are scanned concurrently
        // and folded strictly in task order, stopping at the first error in
        // that order, exactly as the sequential loop did.
        let progress =
            crate::route_progress::Phase::begin("cross-unit home export", tasks.len(), route.jobs);
        let derived_here = derived.as_ref();
        run_ordered(
            tasks.clone(),
            route.jobs,
            Some(dispatch.clone()),
            |task| match (&task.expected_source, task.input) {
                // Card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS (rule
                // 40): a configuration the derived resolution shows outside
                // the link (recorded, no link input attributed to it) exports
                // and defines nothing the link could choose, as a not-linked
                // standalone unit below.
                (Ok(_), TaskInput::Configured(entry))
                    if derived_here.is_some_and(|derived| {
                        derived.outside_the_link(
                            &crate::cross_unit_links::ConfigurationIdentity::of(entry),
                        )
                    }) =>
                {
                    Some(Ok((Vec::new(), Vec::new(), true)))
                }
                (Ok(expected_source), TaskInput::Configured(entry)) => {
                    Some(progress.unit(&task_label(&root, &task), || {
                        audit.export_cross_unit_home(task.library, expected_source, entry, links)
                    }))
                }
                // Card CURRENT-MONKEY-LINKED-CONFIGURATION: a not-linked
                // standalone unit is outside the linked program, so it
                // exports and defines nothing the link could choose.
                (_, TaskInput::NotLinked(..)) => Some(Ok((Vec::new(), Vec::new(), true))),
                _ => None,
            },
            |scan| {
                match scan {
                    Some(scan) => {
                        let (found_exports, found_definitions, complete) = scan?;
                        exports.extend(found_exports);
                        definitions.extend(found_definitions);
                        census_complete &= complete;
                    }
                    None => census_complete = false,
                }
                Ok::<(), String>(())
            },
        )?;
        progress.end(&format!("{} home export(s)", exports.len()));
        let (environment, homes) = if census_complete {
            links.select(exports, definitions, audit.contracts())
        } else {
            println!("cross-unit import phase refused: incomplete definition census");
            (audit.contracts().clone(), Vec::new())
        };
        if !homes.is_empty() {
            match derived.take() {
                Some(derived) => audit.set_cross_unit_derived(environment, homes, derived),
                None => audit.set_cross_unit(environment, homes, links),
            }
            let progress =
                crate::route_progress::Phase::begin("cross-unit import", tasks.len(), route.jobs);
            let mut promoted = Vec::new();
            let stopped = run_ordered(
                tasks.clone(),
                route.jobs,
                Some(dispatch.clone()),
                |task| {
                    let outcome =
                        progress.unit(&task_label(&root, &task), || evaluate_task(&audit, &task));
                    (task.group, outcome)
                },
                |(group, outcome): (usize, EntryOutcome)| {
                    let outcome = outcome.park();
                    let failed = outcome.result.is_err();
                    promoted.push((group, outcome));
                    if failed { Err(()) } else { Ok(()) }
                },
            )
            .is_err();
            progress.end(if stopped {
                "stopped at a failed unit"
            } else {
                ""
            });
            let new_refutations = retained_refutations(
                promoted
                    .iter()
                    .filter_map(|(_, outcome)| outcome.result.as_ref().ok())
                    .map(|products| &products.report.refutations),
            );
            if !stopped && promoted.len() == tasks.len() && new_refutations == phases.applied {
                published = promoted;
            } else {
                // A newly refuted declaration may have supported a home
                // summary. Keep the already settled baseline phase instead.
                audit.set_refuted_table(&phases.applied);
                println!(
                    "cross-unit import phase refused: incomplete run or changed refutation table"
                );
            }
        }
    }
    if let Some(refused) = link_refused {
        audit.record_link_resolution_refused(refused);
    }
    for (group, outcome) in published {
        for header in &headers[printed_headers..=group] {
            println!("{header}");
        }
        printed_headers = printed_headers.max(group + 1);
        print!("{}", outcome.output);
        audit.absorb_entry(outcome.result?.restore()?)?;
    }
    for header in &headers[printed_headers..] {
        println!("{header}");
    }
    let counts = audit.finish(route.diagnostic_report)?;
    write_unknown_sidecars(
        &unknown_sidecar(&revision, route.historical_reference_revision),
        route.diagnostic_report,
        route.monkey_source_report,
        route.binding_directory,
    )?;
    Ok(counts.gateway_failures != 0)
}

/// One task's analysis: exactly what the whole-run fold computes for it.
/// Card CURRENT-RUN-WORKER-SPIN: a task's progress label, its file named
/// below the checkout when it lies there.
fn task_label(root: &Path, task: &ConfiguredTask<'_>) -> String {
    let file = match (&task.expected_source, task.input) {
        (_, TaskInput::Absent(file) | TaskInput::NotLinked(file, _)) => {
            return format!("{}/{file}", task.library);
        }
        (Ok(source), _) => source.as_path(),
        (Err(_), TaskInput::Configured(entry)) => entry.file.as_path(),
    };
    let file = file.strip_prefix(root).unwrap_or(file);
    format!("{}/{}", task.library, file.display())
}

pub(crate) fn evaluate_task(audit: &CurrentAudit, task: &ConfiguredTask<'_>) -> EntryOutcome {
    match (&task.expected_source, task.input) {
        (_, TaskInput::Absent(file)) => audit.evaluate_absent(task.library, file),
        (_, TaskInput::NotLinked(file, reason)) => {
            audit.evaluate_not_linked(task.library, file, reason)
        }
        (Ok(expected_source), TaskInput::Configured(entry)) => {
            audit.evaluate_entry(task.library, expected_source, entry, task.unresolved)
        }
        (Err(error), TaskInput::Configured(_)) => EntryOutcome {
            output: String::new(),
            result: Err(error.clone()),
        },
    }
}

/// Start the largest units first; dispatch moves wall time only.
pub(crate) fn task_dispatch(tasks: &[ConfiguredTask<'_>]) -> Vec<usize> {
    crate::parallel::heaviest_first(
        &tasks
            .iter()
            .map(|task| match task.input {
                TaskInput::Configured(entry) => source_weight(entry),
                TaskInput::Absent(_) | TaskInput::NotLinked(..) => 0,
            })
            .collect::<Vec<_>>(),
    )
}

/// The explicit Unknown record that replaces historical sidecars.
pub(crate) fn unknown_sidecar(revision: &str, historical_reference_revision: &str) -> Value {
    serde_json::json!({
        "schema_version": 1, "source_revision": revision, "status": "Unknown",
        "reason": "historical source/native/provider evidence is not admitted by the current-revision resource audit",
        "historical_reference_revision": historical_reference_revision,
        "results": [], "callbacks": [], "wrappers": [], "backend_teardown": [],
    })
}

/// Replace old sidecars explicitly: success must not leave old Checked
/// evidence adjacent to a new-revision resource inventory at the same paths.
pub(crate) fn write_unknown_sidecars(
    unknown: &Value,
    diagnostic_report: Option<&Path>,
    monkey_source_report: Option<&Path>,
    binding_directory: Option<&Path>,
) -> Result<(), String> {
    let write = |path: &Path| -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(
            path,
            serde_json::to_vec_pretty(unknown).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    };
    if let Some(path) = diagnostic_report {
        write(&path.with_extension("chunk-contracts.json"))?;
        write(&path.with_extension("source-effects.json"))?;
    }
    if let Some(path) = monkey_source_report {
        write(path)?;
    }
    if let Some(path) = binding_directory {
        write(&path.join("monkey_binding_result.json"))?;
    }
    Ok(())
}

/// The historical reference revision of a profile's audited groups: the pin
/// commit their libraries share, or every distinct pin commit joined by `,`.
pub fn historical_reference_revision(profile: &LoadedProfile) -> Result<String, String> {
    let libraries = crate::load_library_selections(&profile.directory().join("audit.json"))?
        .into_iter()
        .map(|selection| selection.library)
        .collect::<Vec<_>>();
    historical_reference_revision_of(profile, &libraries)
}

/// [`historical_reference_revision`] over named libraries: the groups of an
/// explicit unit selection (card W45-FB-SYSTEM-SELECTION).
pub fn historical_reference_revision_of(
    profile: &LoadedProfile,
    libraries: &[String],
) -> Result<String, String> {
    let mut commits = std::collections::BTreeSet::new();
    for library in libraries {
        let pin = profile
            .pin_for(library)
            .ok_or_else(|| format!("selected library `{library}` has no pin"))?;
        commits.insert(pin.git_commit.clone());
    }
    Ok(commits.into_iter().collect::<Vec<_>>().join(","))
}

/// Every distinct configuration of one exact selected file (card
/// W45-FB-SYSTEM-SELECTION): `select_configurations`' exact-file arm, with
/// the static and shared variants kept as separate units. Card
/// AUDIT-CANONICAL-PATHS: records match by located path, two spellings of the
/// file are refused (`input-path-alias`), and a selected file that resolves
/// outside the checkout is refused with the route's outside diagnostic.
fn select_file_configurations(
    entries: &[CompileEntry],
    checkout: &Path,
    file: &str,
) -> Result<Vec<CompileEntry>, String> {
    let target = checkout.join(file);
    if crate::input_paths::place(&target, checkout) == crate::input_paths::Placement::Escapes {
        return Err(format!("source outside current checkout: {file}"));
    }
    Ok(crate::current::distinct_configurations(
        crate::input_paths::select_exact(entries, &target)?
            .into_iter()
            .cloned(),
    ))
}

/// A unit's scheduling weight: its source size in bytes, which ranks analysis
/// time closely on the pinned corpus. An unreadable file weighs nothing here;
/// reading it fails in its own unit, at its own position.
#[must_use]
pub fn source_weight(entry: &CompileEntry) -> u64 {
    fs::metadata(&entry.file).map_or(0, |metadata| metadata.len())
}

/// The Monkey source root of a forced-Libevent Monkey database: the parent of
/// `mk_core/`, found through `mk_core/mk_event.c`, which must be compiled with
/// `-DMK_EVENT_LOOP_LIBEVENT`.
pub fn monkey_source_root(entries: &[CompileEntry]) -> Result<PathBuf, String> {
    let event_entry = entries
        .iter()
        .find(|entry| entry.file.ends_with("mk_core/mk_event.c"))
        .ok_or("Monkey compile database has no mk_core/mk_event.c entry")?;
    let options = compiler_options(event_entry)?;
    if !options
        .definitions
        .iter()
        .any(|definition| definition == "MK_EVENT_LOOP_LIBEVENT")
    {
        return Err("Monkey mk_event.c was not compiled with -DMK_EVENT_LOOP_LIBEVENT".into());
    }
    event_entry
        .file
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "could not derive Monkey source root from {}",
                event_entry.file.display()
            )
        })
}

/// Whether `file` is an audited Monkey unit under `source_root`.
#[must_use]
pub fn is_monkey_unit(source_root: &Path, file: &Path) -> bool {
    // Card AUDIT-CANONICAL-PATHS: membership by located path; the root is
    // derived from the same database, so a consistent spelling places as
    // before.
    let crate::input_paths::Placement::Inside(placed) =
        crate::input_paths::place(file, source_root)
    else {
        return false;
    };
    let relative = placed.canonical;
    if file.extension().is_none_or(|extension| extension != "c") {
        return false;
    }
    // By path components, not text: the located path is spelled with the
    // host's separator (`mk_core\mk_event.c` on Windows).
    (relative.starts_with("mk_core") && !relative.starts_with(Path::new("mk_core").join("deps")))
        || relative.starts_with("mk_server")
        || relative.starts_with("tls")
}

/// The compiled Monkey copy must equal the checkout's `lib/monkey` bytes for
/// every compiled source and for the textually included adapter and headers.
pub fn verify_monkey_source_copy(
    canonical_root: &Path,
    compiled_root: &Path,
    entries: &[CompileEntry],
) -> Result<(), String> {
    for entry in entries {
        let relative = crate::input_paths::relative_within(&entry.file, compiled_root)
            .ok_or_else(|| format!("source is outside Monkey root: {}", entry.file.display()))?;
        let canonical = canonical_root.join(&relative);
        let compiled = fs::read(&entry.file)
            .map_err(|error| format!("could not read {}: {error}", entry.file.display()))?;
        let expected = fs::read(&canonical)
            .map_err(|error| format!("could not read {}: {error}", canonical.display()))?;
        if compiled != expected {
            return Err(format!(
                "compiled Monkey source differs from pinned Fluent Bit source: {}",
                relative.display()
            ));
        }
    }
    // The adapter is textually included, so it is absent from the standalone
    // command list. Bind its body, layout and core macro headers explicitly.
    for relative in [
        "mk_core/mk_event_libevent.c",
        "include/monkey/mk_core/mk_event.h",
        "include/monkey/mk_core/mk_event_libevent.h",
        "include/monkey/mk_core/mk_macros.h",
        "include/monkey/mk_core/mk_memory.h",
        "include/monkey/mk_core/mk_limits.h",
        "include/monkey/mk_core/mk_list.h",
    ] {
        let compiled = fs::read(compiled_root.join(relative)).map_err(|error| error.to_string())?;
        let expected =
            fs::read(canonical_root.join(relative)).map_err(|error| error.to_string())?;
        if compiled != expected {
            return Err(format!(
                "compiled Monkey callback source/layout differs from pinned source: {relative}"
            ));
        }
    }
    Ok(())
}

/// Write a run's invocation provenance beside its report (card
/// W44-QUIRIN-AUDIT-PROVIDERS): what the retired Cycle 19 preparation
/// wrapper used to record in `audit-command.json`, now written by the command
/// itself.
pub fn write_invocation(report: &Path, invocation: &Value) -> Result<(), String> {
    let path = report.with_extension("invocation.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut bytes = serde_json::to_vec_pretty(invocation).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn retained_refutations<'a>(
    tables: impl Iterator<Item = &'a qcore_schemas::contract_refutation::RefutationTable>,
) -> qcore_schemas::contract_refutation::RefutationTable {
    tables.fold(
        Default::default(),
        qcore_schemas::contract_refutation::union,
    )
}

fn retained_bounds<'a>(
    tables: impl Iterator<Item = &'a std::collections::BTreeSet<String>>,
) -> std::collections::BTreeSet<String> {
    tables.flat_map(|table| table.iter().cloned()).collect()
}
