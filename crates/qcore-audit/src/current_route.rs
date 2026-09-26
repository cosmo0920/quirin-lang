//! The configured current-revision route (card **W44-QUIRIN-AUDIT-PROVIDERS**),
//! moved here from `compiler/examples/support/cycle19_compatibility.rs`
//! unchanged in behaviour: `quirin-audit current` runs it, and the compiler
//! example's configured route is a thin wrapper over it.
//!
//! Every selected compilation configuration of the profile's `audit.json`
//! library groups, then of an optional Monkey (forced-Libevent) database, is
//! one [`CurrentAudit`] task, analysed on up to `jobs` threads and folded in
//! selection order (card W44-AUDIT-PARALLEL-UNITS). Every database record must
//! name the configured Clang C driver (`/usr/bin/cc` and other substitutes
//! refuse before any unit is analysed). Historical source/native/provider
//! evidence is not admitted: its sidecar paths are overwritten with explicit
//! Unknown records so no older Checked evidence survives beside a new
//! inventory.

use std::fs;
use std::path::{Path, PathBuf};

use qcore_contract::profile_loader::LoadedProfile;
use serde_json::Value;

use crate::compile_db::{
    CompileEntry, compile_entries, compiler_options_compatibility as compiler_options,
};
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
    pub monkey_database: Option<&'a Path>,
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
}

/// One configured-route compilation configuration, in selection order.
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
}

/// One selected group: its report label, its configured units in order (a
/// file without a record is `Absent` at its own position), and its refusal.
type Group<'a> = (String, Vec<Selected<'a>>, Option<&'static str>);

enum Selected<'a> {
    Configured(CompileEntry),
    Absent(&'a str),
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
    let entries = compile_entries(
        &fs::read_to_string(route.compile_database.unwrap_or(&default_database))
            .map_err(|error| error.to_string())?,
    )?;
    // Select all groups before auditing; missing groups cannot disappear from
    // the denominator. Different commands for one file remain separate tasks.
    let mut groups: Vec<Group<'_>> = Vec::new();
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
            for file in &group.files {
                let configurations = select_file_configurations(&entries, &root, file);
                if configurations.is_empty() {
                    selected.push(Selected::Absent(file.as_str()));
                }
                for entry in configurations {
                    crate::compile_db::require_clang_driver(&entry)?;
                    selected.push(Selected::Configured(entry));
                }
            }
            groups.push((group.label().to_owned(), selected, unresolved_for(library)));
        }
    } else {
        for selection in crate::load_library_selections(&profile.directory().join("audit.json"))? {
            let library = profile
                .library(&selection.library)
                .ok_or("selected library is absent from profile")?;
            let selected = select_configurations(&entries, &root, library);
            if selected.is_empty() {
                return Err(format!(
                    "compile database contains no selected {} sources",
                    selection.library
                ));
            }
            for entry in &selected {
                crate::compile_db::require_clang_driver(entry)?;
            }
            groups.push((
                selection.label.unwrap_or(selection.library),
                selected.into_iter().map(Selected::Configured).collect(),
                unresolved_for(library),
            ));
        }
    }
    let monkey = route
        .monkey_database
        .map(|path| {
            let entries =
                compile_entries(&fs::read_to_string(path).map_err(|error| error.to_string())?)?;
            let source_root = monkey_source_root(&entries)?;
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
        headers.push(format!(
            "monkey-libevent: {} selected compilation configuration(s)",
            entries.len()
        ));
        for entry in entries {
            let expected_source = entry
                .file
                .strip_prefix(source_root)
                .map(|relative| root.join("lib/monkey").join(relative))
                .map_err(|_| "Monkey source outside compiled root".to_owned());
            tasks.push(ConfiguredTask {
                group: headers.len() - 1,
                library: "monkey-libevent",
                expected_source,
                input: TaskInput::Configured(entry),
                unresolved: unresolved.as_deref(),
            });
        }
    }
    if let Some(options) = checkpoints {
        let unresolved = groups
            .iter()
            .map(|(_, _, unresolved)| *unresolved)
            .chain(monkey.iter().map(|(_, _, unresolved)| unresolved.as_deref()))
            .collect::<Vec<_>>();
        return crate::checkpoint_run::run(
            route,
            &audit,
            crate::checkpoint_run::RoutePlan {
                preamble,
                headers,
                unresolved,
                tasks,
            },
            options,
        );
    }
    let mut printed_headers = 0;
    let mut products = Vec::with_capacity(tasks.len());
    let dispatch = task_dispatch(&tasks);
    run_ordered(
        tasks,
        route.jobs,
        Some(dispatch),
        |task| (task.group, evaluate_task(&audit, &task)),
        |(group, outcome)| {
            for header in &headers[printed_headers..=group] {
                println!("{header}");
            }
            printed_headers = printed_headers.max(group + 1);
            print!("{}", outcome.output);
            products.push(outcome.result?);
            Ok::<(), String>(())
        },
    )?;
    for header in &headers[printed_headers..] {
        println!("{header}");
    }
    for entry in products {
        audit.absorb_entry(entry)?;
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
pub(crate) fn evaluate_task(audit: &CurrentAudit, task: &ConfiguredTask<'_>) -> EntryOutcome {
    match (&task.expected_source, task.input) {
        (_, TaskInput::Absent(file)) => audit.evaluate_absent(task.library, file),
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
                TaskInput::Absent(_) => 0,
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
/// the static and shared variants kept as separate units.
fn select_file_configurations(
    entries: &[CompileEntry],
    checkout: &Path,
    file: &str,
) -> Vec<CompileEntry> {
    let target = checkout.join(file);
    crate::current::distinct_configurations(
        entries
            .iter()
            .filter(|entry| {
                entry
                    .file
                    .canonicalize()
                    .unwrap_or_else(|_| entry.file.clone())
                    == target
            })
            .cloned(),
    )
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

/// The compiled Monkey copy must equal the checkout's `lib/monkey` bytes for
/// every compiled source and for the textually included adapter and headers.
pub fn verify_monkey_source_copy(
    canonical_root: &Path,
    compiled_root: &Path,
    entries: &[CompileEntry],
) -> Result<(), String> {
    for entry in entries {
        let relative = entry
            .file
            .strip_prefix(compiled_root)
            .map_err(|_| format!("source is outside Monkey root: {}", entry.file.display()))?;
        let canonical = canonical_root.join(relative);
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
