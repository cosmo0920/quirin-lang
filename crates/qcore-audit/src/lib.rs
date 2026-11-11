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

//! Provider-independent, configured existing-C resource audits (L3).
//!
//! Pins, selected paths and contracts are declared inputs; no provider module
//! is linked. Safe/Unsafe/Unknown remain the gateway's bounded projection
//! verdicts. Configured process source-contract checks accept only sealed
//! gateway imports and trusted witness replay. The separate `observed-wait`
//! mode joins contract-backed source obligations with one bounded native run;
//! the existing compatibility adapters retain their original scopes.

// Card CG19-ATOMIC-SOURCE-ROUTE (gateway rule 11i): the production caller of
// the bounded atomic-publication judgment.
pub mod atomic_publication;
pub mod callback_task_handoff;
pub mod certificate_context;
pub mod checkpoint;
pub mod checkpoint_cli;
pub mod checkpoint_run;
pub mod compile_db;
pub mod compose;
pub mod current;
pub mod current_cli;
pub mod current_route;
pub mod cycle18_composition;
mod cycle18_inputs;
pub mod event_payload;
pub mod evidence;
mod ir_capture;
pub mod native_platform_matrix;
pub mod no_build;
pub mod observed_wait;
mod observed_wait_inputs;
pub mod parallel;
pub mod pinned_cli;
pub mod pinned_route;
mod retained_products;
// Card W45-CONTRACT-DEFERRED-REVIEW: the pending-review report section.
pub mod pending_review;
// Card W47-REVIEW-HOLD-CONCRETE-REASON: each held row's concrete review reasons.
pub mod review_hold_reasons;
// Card W45-CONTRACT-REVISION-MODE: source identities, contract locks, the pin
// evaluation and its report section.
// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rules 14f-14h).
pub mod bounds_evidence;
pub mod bounds_selection;
// Card NULL-SAFETY-PROPERTY (rules 14k-14p): the null selection's tally.
pub mod c_identity;
pub mod contract_pin;
pub mod contract_revision;
pub mod null_fold;
pub mod null_hypotheses;
// Card OUTER-CONTRACT (rule 31): the runtime projection.
pub mod outer_candidates;
pub mod outer_selection;
// Card NULL-MEMBER-INVARIANT-SCOPE (gateway rules 14x-14z).
pub mod null_member_scope_fold;
pub mod null_tally;
// Card VERDICT-CONTRACT-DEPENDENCIES (rules 13f-13h): the row fields and the
// run's dependency section.
pub mod callback_member_fold;
pub mod cross_unit_links;
// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, R1, F1): the member-facts
// pre-pass of the pinned route.
pub mod contract_dependencies;
pub mod contract_tip;
pub mod link_resolution;
pub mod member_prepass;
pub mod pin_window;
pub mod prepare;
mod prepare_json;
pub mod projection;
pub mod providers;
pub mod refuted_contracts;
pub mod report;
// Card AUDIT-COMPOSITION: per-row components and the derived report rows.
pub mod report_components;
pub mod revision_compare;
// Card CURRENT-RUN-WORKER-SPIN: per-phase and per-unit progress lines.
pub mod route_progress;
mod source_audit;
pub mod source_contract;
pub mod timeout_teardown;
pub mod unit_selection;
// Card DEEP-AST-TRANSPORTS-AND-WALL-TIME: measurement only, compiled out
// unless the `unit-timing` feature is on.
#[cfg(all(feature = "unit-timing", target_os = "linux"))]
mod unit_timing;
// Card W45-USER-CONTRACT-PLUGINS: user contract crates on an extending load
// path, their collisions with shipped contracts, checker plugins, and the
// resource views of user crates (`quirin-audit plugin views`).
pub mod plugin_cli;
pub mod user_plugins;
// Card W45-AUDIT-SUBCOMMAND-PLUGINS: external `quirin-audit NAME` subcommands
// (`quirin-audit-NAME` on a declared search path), through the plugin firewall.
pub mod subcommand_plugins;
// Card RULE-SWITCH-ENTRY-POINTS: which command lines record the rule set.
pub mod rule_switch_entry;
pub mod wait_failures;
// Card AUDIT-CANONICAL-PATHS: unit selection compares located (canonical)
// paths, never path strings, so a moved or symlinked checkout keeps working.
pub mod input_paths;
pub use source_audit::{SourceContractAuditConfig, SourceContractLibrary};

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use qcore_c_gateway::legacy_c::{C11TranslationUnit, LegacyFunctionScope};
use qcore_contract::profile_loader::{
    LoadedProfile, load_audited_bundle, load_profile_directory, verify_pin,
};
use qcore_schemas::profile::{LibraryProfile, UnitSelection, WitnessCheck};
use serde::{Deserialize, Serialize};

use compile_db::{CompileEntry, compile_entries, compiler_options};
use projection::{AuditCounts, PreparedAuditCertificateContext};
use report::{AuditReport, UnitRefusal};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditConfig {
    pub schema_version: u32,
    pub profile_directory: PathBuf,
    pub checkout: PathBuf,
    pub compile_database: PathBuf,
    pub libraries: Vec<String>,
    pub report: PathBuf,
    #[serde(default)]
    pub source_revision_policy: SourceRevisionPolicy,
    #[serde(default)]
    pub source_contracts: Option<SourceContractAuditConfig>,
    /// Which build target's compilation records this audit reads, for a
    /// database that carries several commands per source file. Absent means the
    /// database is taken whole, which is the existing behaviour and keeps every
    /// installed configuration byte-identical.
    #[serde(default)]
    pub compile_target: Option<CompileTargetSelection>,
    /// Card W44-QUIRIN-AUDIT-PROVIDERS: the resource contracts by provider
    /// crate (`NAME` or `NAME:MODULE`), resolved on `load_path`. A selection
    /// supersedes the profile's declared manifest, which is recorded as not
    /// read. Absent means the profile's manifest, exactly as before.
    #[serde(default)]
    pub providers: Option<Vec<String>>,
    /// Per-library provider scoping table (`library -> { providers, dependent_libraries }`),
    /// resolved on `load_path`. Each library slice is audited sequentially under
    /// its own providers plus transitive `dependent_libraries` providers, and its
    /// contract environment and unit outcomes are dropped after the slice.
    #[serde(default)]
    pub library_providers: Option<BTreeMap<String, providers::LibraryProviderScope>>,
    /// The contract load path a provider selection resolves on.
    #[serde(default)]
    pub load_path: Option<PathBuf>,
    /// Refuse any `qcore-provider-*` crate in the resolved provider closure.
    #[serde(default)]
    pub quirin_providers_only: bool,
    /// `contracts-only` admits resource projection from contract modules only
    /// and refuses `source_contracts` (process packages, verifier host).
    #[serde(default)]
    pub mode: evidence::EvidenceMode,
    /// Translation units analysed concurrently; a `--jobs` flag takes
    /// precedence, and `QCORE_AUDIT_JOBS` applies only when both are absent.
    #[serde(default)]
    pub jobs: Option<usize>,
    /// Planning estimate: 1024 MiB coordinator + 2048 MiB per worker.
    /// Does not enforce a process/cgroup memory limit.
    #[serde(default)]
    pub memory_budget_mib: Option<usize>,
}

/// An explicit, declared narrowing of a multi-target compilation database.
///
/// This exists because refusing to choose is right and choosing silently is
/// wrong: the audit keeps refusing residual ambiguity, and the *configuration*
/// — a reviewable, digested input — says which target is under audit.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CompileTargetSelection {
    /// Keep only records whose `-o` argument begins with this fragment, as the
    /// database spells it (for CMake, `CMakeFiles/<target>.dir/`).
    pub object_prefix: String,
}

/// Historical source evidence requires its original profile pin. A moving
/// checkout can instead supply fresh, explicitly scoped resource projections.
#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceRevisionPolicy {
    #[default]
    ProfilePin,
    CheckoutHead,
}

fn checkout_head(checkout: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["rev-parse", "--verify", "HEAD^{commit}"])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err("could not resolve checkout HEAD to a commit".into());
    }
    String::from_utf8(output.stdout)
        .map(|head| head.trim().to_owned())
        .map_err(|error| error.to_string())
}

/// An explicit profile determines every selected source path. Conflicting
/// commands for a selected file reject, rather than arbitrarily choosing one.
pub fn select_profile_units(
    entries: &[CompileEntry],
    checkout: &Path,
    library: &LibraryProfile,
) -> Result<Vec<CompileEntry>, String> {
    // Card AUDIT-CANONICAL-PATHS: records match by located path and keep
    // their spelling; a record spelled under a root that resolves outside it
    // is refused, and two spellings of one file with `input-path-alias`.
    use input_paths::{Placement, locate, place, select_exact, select_under};
    let is_c = |entry: &CompileEntry| {
        locate(&entry.file)
            .path()
            .extension()
            .is_some_and(|extension| extension == "c")
    };
    let outside = |entry: &CompileEntry| {
        format!("source outside selected checkout: {}", entry.file.display())
    };
    let mut matched = Vec::new();
    for root in &library.source_roots {
        let source_root = qcore_contract::profile_loader::source_root_path(checkout, &root.root);
        matched.extend(match &root.selection {
            UnitSelection::TopLevel => select_under(
                entries,
                &source_root,
                |entry, relative| relative.components().count() == 1 && is_c(entry),
                outside,
            )?,
            UnitSelection::Recursive => {
                select_under(entries, &source_root, |entry, _| is_c(entry), outside)?
            }
            UnitSelection::Exact { file } => {
                let target = checkout.join(file);
                if place(&target, checkout) == Placement::Escapes {
                    return Err(format!("source outside selected checkout: {file}"));
                }
                select_exact(entries, &target)?
                    .into_iter()
                    .filter(|entry| is_c(entry))
                    .collect()
            }
        });
    }
    input_paths::refuse_aliases(matched.iter().copied())?;
    let mut selected = BTreeMap::<PathBuf, CompileEntry>::new();
    for entry in matched {
        if let Some(prior) = selected.get(&entry.file)
            && !compile_db::same_configuration(prior, entry)
        {
            return Err(format!(
                "conflicting compilation commands for {}",
                entry.file.display()
            ));
        }
        selected.insert(entry.file.clone(), entry.clone());
    }
    Ok(selected.into_values().collect())
}

pub fn verify_library_sources(
    profile: &LoadedProfile,
    name: &str,
    checkout: &Path,
) -> Result<String, String> {
    let library = profile
        .library(name)
        .ok_or_else(|| format!("profile contains no library `{name}`"))?;
    let pin = profile
        .pin_for(name)
        .ok_or_else(|| format!("library `{name}` has no source pin"))?;
    let revision = verify_pin(&library.pin, pin, checkout).map_err(|error| error.to_string())?;
    let bundle = load_audited_bundle(library, checkout, &revision)
        .map_err(|errors| format!("{errors:?}"))?;
    for witness in &library.source_witnesses {
        if let WitnessCheck::Substrings { needles, .. } = &witness.check {
            let contents = bundle
                .files
                .get(&witness.file)
                .ok_or("missing witness file")?;
            if needles.iter().any(|needle| !contents.contains(needle)) {
                return Err(format!("source witness `{}` does not match", witness.name));
            }
        }
    }
    Ok(revision)
}

fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn repository_path(path: &Path) -> String {
    path.iter()
        .map(|component| component.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Card REASON-CODES-B1: the two OS/tool failures here carry raw text that the
/// closed vocabulary cannot recognise, so they name their category at the site.
/// Both are environment failures, which is the one category that says nothing
/// about the analysed source; the classified refusals beside them are unchanged
/// and keep classifying from their own text.
fn verify_pinned_bytes(
    checkout: &Path,
    revision: &str,
    path: &Path,
    source: &[u8],
) -> Result<(), UnitRefusal> {
    let canonical = path
        .canonicalize()
        .map_err(|error| UnitRefusal::environment(error.to_string()))?;
    if !canonical.starts_with(checkout) {
        return Err(UnitRefusal::classified("source outside selected checkout"));
    }
    // Keep the selected path's Git blob, rather than letting an unpinned
    // symlink borrow the bytes of a different pinned file.
    let file = canonical
        .strip_prefix(checkout)
        .map_err(|_| UnitRefusal::classified("source outside selected checkout"))?;
    let git_file = repository_path(file);
    let pinned = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["show", &format!("{revision}:{git_file}")])
        .output()
        .map_err(|error| UnitRefusal::environment(error.to_string()))?;
    if !pinned.status.success() || pinned.stdout != source {
        return Err(UnitRefusal::classified(format!(
            "selected source `{}` does not match its pinned bytes",
            file.display()
        )));
    }
    Ok(())
}

pub fn run_config(path: &Path) -> Result<AuditCounts, String> {
    run_config_with_jobs(path, None)
}

/// [`run_config`] with a command-line job count (card
/// W44-QUIRIN-AUDIT-PROVIDERS). The count is the flag, else the
/// configuration's `jobs`, else `QCORE_AUDIT_JOBS`, else the host's available
/// parallelism capped at 16 ([`parallel::jobs_with_precedence`]); one job is
/// the sequential loop, and every output is the same at any count.
pub fn run_config_with_jobs(path: &Path, flag_jobs: Option<usize>) -> Result<AuditCounts, String> {
    parallel::reset_admission_trace();
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    let base = path
        .parent()
        .ok_or("configuration has no parent directory")?;
    let config: AuditConfig =
        serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if config.schema_version != 1 {
        return Err(format!(
            "unsupported audit configuration version {}",
            config.schema_version
        ));
    }
    let requested_jobs = parallel::jobs_with_precedence(flag_jobs, config.jobs)?;
    let jobs = parallel::budgeted_jobs(requested_jobs, config.memory_budget_mib)?;
    if config.mode.is_contracts_only() && config.source_contracts.is_some() {
        return Err("a contracts-only audit admits no source-contract checks: remove `source_contracts` (process packages and verifier-host execution) or drop `mode: contracts-only`".into());
    }
    if config.source_revision_policy == SourceRevisionPolicy::CheckoutHead
        && config.source_contracts.is_some()
    {
        return Err("checkout_head supports resource projection only; source-contract evidence requires a matching profile pin".into());
    }
    if config.libraries.is_empty()
        || config.libraries.iter().collect::<BTreeSet<_>>().len() != config.libraries.len()
    {
        return Err("audit must select distinct nonempty library names".into());
    }
    let profile = load_profile_directory(&resolve(base, &config.profile_directory))
        .map_err(|error| error.to_string())?;
    let checkout = resolve(base, &config.checkout)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let database_path = resolve(base, &config.compile_database);
    let database = fs::read_to_string(&database_path).map_err(|error| error.to_string())?;
    let entries = compile_entries(&database)?;
    let entries = match &config.compile_target {
        Some(selection) => compile_db::select_object_prefix(entries, &selection.object_prefix)?,
        None => entries,
    };
    if config.providers.is_some() && config.library_providers.is_some() {
        return Err(
            "a configuration cannot declare both `providers` and `library_providers`; give one"
                .into(),
        );
    }
    let resolved_library_table = match &config.library_providers {
        Some(scopes) => {
            let load_path = config.load_path.as_ref().ok_or(
                "per-library provider scoping (`library_providers`) needs `load_path` (the contract load path)",
            )?;
            Some(providers::resolve_library_providers_table(
                &resolve(base, load_path),
                scopes,
                &config.libraries,
                config.quirin_providers_only,
                providers::SelectionSource::Configuration,
                profile.resource_contract_manifest_path().as_deref(),
            )?)
        }
        None => None,
    };
    let contract_input = match (&resolved_library_table, &config.providers) {
        (Some(table), _) => table.union_input.clone(),
        (None, Some(entries)) => {
            let load_path = config
                .load_path
                .as_ref()
                .ok_or("a provider selection needs `load_path` (the contract load path)")?;
            let resolved = providers::resolve_providers(
                &resolve(base, load_path),
                entries,
                config.quirin_providers_only,
                providers::SelectionSource::Configuration,
            )?;
            providers::ContractInput::providers(
                &resolved,
                profile.resource_contract_manifest_path().as_deref(),
            )
        }
        // Card W44-PROVIDER-RESOURCE-VIEW: contracts only loads all contracts,
        // every load-path crate's resource view (`"selection": "all-loaded"`).
        (None, None) if config.mode.is_contracts_only() => {
            let load_path = config.load_path.as_ref().ok_or(
                "a contracts-only audit with no `providers` loads every crate's resource view on \
                 the contract load path: give `load_path`, or narrow it with `providers`",
            )?;
            providers::ContractInput::providers(
                &providers::resolve_all_loaded(
                    &resolve(base, load_path),
                    config.quirin_providers_only,
                )?,
                profile.resource_contract_manifest_path().as_deref(),
            )
        }
        (None, None) => {
            if config.load_path.is_some() || config.quirin_providers_only {
                return Err(
                    "`load_path` and `quirin_providers_only` apply only to a provider selection"
                        .into(),
                );
            }
            providers::ContractInput::manifest(
                &profile
                    .resource_contract_manifest_path()
                    .ok_or("source-set profile supplies no resource contracts")?,
            )?
        }
    };
    let mut contracts = contract_input.load()?;
    // Card W45-CONTRACT-REVISION-MODE: every pinned contract is compared with
    // its lock over the checkout before any unit is analysed.
    let pins = contract_revision::apply_pins(
        &mut contracts,
        &contract_revision::ModuleOwners::for_manifest(
            contract_input.manifest_path(),
            contract_input.manifest_text(),
        )?,
        &checkout,
    )?;
    let mut work = Vec::new();
    let mut revisions = BTreeSet::new();
    // Resolve a moving name once. Every unit is checked against this immutable
    // commit, and moving HEAD during the run prevents report publication.
    let selected_head = (config.source_revision_policy == SourceRevisionPolicy::CheckoutHead)
        .then(|| checkout_head(&checkout))
        .transpose()?;
    for name in &config.libraries {
        let library = profile.library(name).ok_or("selected library is absent")?;
        if let Some(head) = &selected_head {
            let mut pin = profile
                .pin_for(name)
                .ok_or("selected library has no pin")?
                .clone();
            pin.git_commit.clone_from(head);
            verify_pin(&library.pin, &pin, &checkout).map_err(|error| error.to_string())?;
            revisions.insert(head.clone());
        } else {
            revisions.insert(verify_library_sources(&profile, name, &checkout)?);
        }
        if !library.required_evidence.is_empty() {
            return Err(format!(
                "library `{name}` requires evidence that this configured driver cannot supply"
            ));
        }
        let selected = select_profile_units(&entries, &checkout, library)?;
        if selected.is_empty() {
            return Err(format!("no compiled C sources selected for `{name}`"));
        }
        // Validate every command before publishing even a partial run.
        for entry in &selected {
            compiler_options(entry)
                .map_err(|error| format!("{name}: {}: {error}", entry.file.display()))?;
        }
        work.push((name, selected));
    }
    let revision = revisions.into_iter().collect::<Vec<_>>().join(",");
    let mut source_contracts = config
        .source_contracts
        .as_ref()
        .map(|selection| {
            source_audit::SourceContractAudit::prepare(
                selection,
                base,
                &checkout,
                &entries,
                &profile,
                &config.libraries,
            )
        })
        .transpose()?;
    // One capture describes the Clang that performs every unit's analysis.
    let compiler = qcore_c_gateway::ClangIdentity::capture()?;
    let contract_manifest_text = contract_input.manifest_text().to_owned();
    // Every declared input the audit read before analyzing a unit. A change to
    // any of them changes each recorded compile-task identity.
    let mut task_dependencies = BTreeMap::from([
        (
            "compile_database".to_owned(),
            qcore_contract::profile_loader::sha256_hex(database.as_bytes()),
        ),
        (
            "resource_contract_manifest".to_owned(),
            qcore_contract::profile_loader::sha256_hex(contract_manifest_text.as_bytes()),
        ),
        (
            "source_profile".to_owned(),
            profile
                .assumptions()
                .get("profile_digest")
                .cloned()
                .ok_or("source profile reports no digest")?,
        ),
    ]);
    let source_profile_sha256 = task_dependencies
        .get("source_profile")
        .cloned()
        .expect("source profile dependency was inserted");
    if let Some(head) = &selected_head {
        task_dependencies.insert(
            "source_revision_selection".into(),
            qcore_contract::profile_loader::sha256_hex(format!("checkout_head:{head}").as_bytes()),
        );
    }
    if let Some(selection) = &config.source_contracts {
        for (name, path) in [
            ("source_contract_configuration", &selection.configuration),
            ("source_contract_rules", &selection.source_rules),
        ] {
            task_dependencies.insert(
                name.to_owned(),
                qcore_contract::profile_loader::sha256_hex(
                    &fs::read(resolve(base, path)).map_err(|error| error.to_string())?,
                ),
            );
        }
    }
    let mut report = AuditReport::new(&revision);
    let evidence_record = config
        .mode
        .is_contracts_only()
        .then(|| evidence::contracts_only_record(&contracts));
    if let Some(record) = &evidence_record {
        report.set_evidence(record.clone());
    }
    let mut counts = AuditCounts::default();
    let mut source_identities = BTreeMap::new();
    let mut compile_tasks = Vec::new();
    // Card W44-QUIRIN-AUDIT-PROVIDERS: every selected unit is one task, in the
    // order the sequential loops visited them, analysed on up to `jobs`
    // threads with W44-AUDIT-PARALLEL-UNITS's ordered fold. Console lines,
    // report rows, tallies, source identities and compile tasks are folded in
    // unit order, and the first error in unit order is the one returned. The
    // source-contract route, which may dispatch a process provider, runs in
    // the fold, one unit at a time, exactly where the loop ran it.
    let retain_units = source_contracts.is_some();
    let mut refuted_table = qcore_schemas::contract_refutation::RefutationTable::new();
    if let Some(library_table) = &resolved_library_table {
        for (library, entries) in work {
            contract_dependencies::clear_premises_cache();
            let slice_providers = library_table.slice_for(library)?;
            let mut slice_contracts = slice_providers.input.load()?;
            let _ = contract_revision::apply_pins(
                &mut slice_contracts,
                &contract_revision::ModuleOwners::for_manifest(
                    slice_providers.input.manifest_path(),
                    slice_providers.input.manifest_text(),
                )?,
                &checkout,
            )?;
            let mut slice_task_dependencies = task_dependencies.clone();
            slice_task_dependencies.insert(
                "resource_contract_manifest".to_owned(),
                qcore_contract::profile_loader::sha256_hex(
                    slice_providers.input.manifest_text().as_bytes(),
                ),
            );
            let slice_tasks: Vec<(&String, CompileEntry)> =
                entries.into_iter().map(|entry| (library, entry)).collect();
            let slice_dispatch = parallel::heaviest_first(
                &slice_tasks
                    .iter()
                    .map(|(_, entry)| current_route::source_weight(entry))
                    .collect::<Vec<_>>(),
            );
            let base_refutations: qcore_schemas::contract_refutation::RefutationTable =
                refuted_table
                    .iter()
                    .filter(|(name, _)| {
                        qcore_c_gateway::contract_dependencies::declared(&slice_contracts, name)
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
            let phases = refuted_contracts::run_two_phases(
                |table| {
                    let mut effective = base_refutations.clone();
                    effective.extend(table.iter().map(|(k, v)| (k.clone(), v.clone())));
                    let active_contracts =
                        refuted_contracts::with_table(&slice_contracts, &effective);
                    let mut outcomes = Vec::new();
                    let stopped = parallel::run_ordered(
                        slice_tasks.clone(),
                        parallel::effective_jobs(jobs),
                        Some(slice_dispatch.clone()),
                        |(library, entry): (&String, CompileEntry)| {
                            let mut out = String::new();
                            let result = configured_unit(ConfiguredUnit {
                                library,
                                entry: &entry,
                                checkout: &checkout,
                                profile: &profile,
                                selected_head: selected_head.as_deref(),
                                compiler: &compiler,
                                task_dependencies: &slice_task_dependencies,
                                contracts: &active_contracts,
                                source_profile_sha256: &source_profile_sha256,
                                fragment: AuditReport::new(&revision),
                                retain_unit: retain_units,
                                out: &mut out,
                            });
                            (out, result.map(|products| (library, entry, products)))
                        },
                        |outcome: (
                            String,
                            Result<(&String, CompileEntry, UnitProducts), String>,
                        )| {
                            let outcome = (
                                outcome.0,
                                outcome.1.and_then(|(library, entry, products)| {
                                    Ok((library, entry, products.park()?))
                                }),
                            );
                            let failed = outcome.1.is_err();
                            outcomes.push(outcome);
                            if failed { Err(()) } else { Ok(()) }
                        },
                    )
                    .err();
                    (outcomes, stopped)
                },
                |outcomes| {
                    let mut found = base_refutations.clone();
                    found.extend(current_route::retained_refutations(
                        outcomes
                            .iter()
                            .filter_map(|(_, result)| result.as_ref().ok())
                            .map(|(_, _, products)| &products.report.refutations),
                    ));
                    found
                },
            );
            refuted_table.extend(base_refutations);
            refuted_table.extend(phases.applied);
            for (out, result) in phases.results {
                print!("{out}");
                let (library, entry, products) = result?;
                let products = products.restore()?;
                let UnitProducts {
                    file,
                    source_sha256,
                    compile_task,
                    report: fragment,
                    counts: tally,
                    retained,
                } = products;
                source_identities.insert(format!("{library}/{file}"), source_sha256);
                compile_tasks.push(compile_task);
                report.absorb(fragment)?;
                counts.absorb(tally);
                if let (Some(audit), Some(prepared)) = (&mut source_contracts, retained) {
                    let options = compiler_options(&entry)?;
                    let unit = C11TranslationUnit {
                        source: prepared.raw_source(),
                        include_paths: options.include_paths,
                        definitions: options.definitions,
                        preprocessor_options: options.preprocessor_options,
                        dialect: options.dialect,
                        function_scope: LegacyFunctionScope::MainSourceDefinitions,
                        input: options.input,
                    };
                    qcore_c_gateway::with_audit_unit_lifetimes(|| {
                        audit.analyze_unit(library, &file, &unit, prepared.inventory(), &profile);
                    });
                }
            }
            contract_dependencies::clear_premises_cache();
        }
    } else {
        let tasks: Vec<(&String, CompileEntry)> = work
            .into_iter()
            .flat_map(|(library, entries)| entries.into_iter().map(move |entry| (library, entry)))
            .collect();
        let dispatch = parallel::heaviest_first(
            &tasks
                .iter()
                .map(|(_, entry)| current_route::source_weight(entry))
                .collect::<Vec<_>>(),
        );
        // Card W45-REFUTED-CONTRACT-WITHDRAWAL: phase one under the loaded
        // contracts and, when a body refutes its own declaration, phase two under
        // the refutation table; only the published phase is folded, in unit order.
        let phases = refuted_contracts::run_two_phases(
            |table| {
                let contracts = refuted_contracts::with_table(&contracts, table);
                let mut outcomes = Vec::new();
                let stopped = parallel::run_ordered(
                    tasks.clone(),
                    parallel::effective_jobs(jobs),
                    Some(dispatch.clone()),
                    |(library, entry): (&String, CompileEntry)| {
                        let mut out = String::new();
                        let result = configured_unit(ConfiguredUnit {
                            library,
                            entry: &entry,
                            checkout: &checkout,
                            profile: &profile,
                            selected_head: selected_head.as_deref(),
                            compiler: &compiler,
                            task_dependencies: &task_dependencies,
                            contracts: &contracts,
                            source_profile_sha256: &source_profile_sha256,
                            fragment: AuditReport::new(&revision),
                            retain_unit: retain_units,
                            out: &mut out,
                        });
                        (out, result.map(|products| (library, entry, products)))
                    },
                    |outcome: (
                        String,
                        Result<(&String, CompileEntry, UnitProducts), String>,
                    )| {
                        let outcome = (
                            outcome.0,
                            outcome.1.and_then(|(library, entry, products)| {
                                Ok((library, entry, products.park()?))
                            }),
                        );
                        let failed = outcome.1.is_err();
                        outcomes.push(outcome);
                        if failed { Err(()) } else { Ok(()) }
                    },
                )
                .err();
                (outcomes, stopped)
            },
            |outcomes| {
                current_route::retained_refutations(
                    outcomes
                        .iter()
                        .filter_map(|(_, result)| result.as_ref().ok())
                        .map(|(_, _, products)| &products.report.refutations),
                )
            },
        );
        refuted_table = phases.applied;
        for (out, result) in phases.results {
            print!("{out}");
            let (library, entry, products) = result?;
            let products = products.restore()?;
            let UnitProducts {
                file,
                source_sha256,
                compile_task,
                report: fragment,
                counts: tally,
                retained,
            } = products;
            source_identities.insert(format!("{library}/{file}"), source_sha256);
            compile_tasks.push(compile_task);
            report.absorb(fragment)?;
            counts.absorb(tally);
            if let (Some(audit), Some(prepared)) = (&mut source_contracts, retained) {
                let options = compiler_options(&entry)?;
                let unit = C11TranslationUnit {
                    source: prepared.raw_source(),
                    include_paths: options.include_paths,
                    definitions: options.definitions,
                    preprocessor_options: options.preprocessor_options,
                    dialect: options.dialect,
                    function_scope: LegacyFunctionScope::MainSourceDefinitions,
                    input: options.input,
                };
                qcore_c_gateway::with_audit_unit_lifetimes(|| {
                    audit.analyze_unit(library, &file, &unit, prepared.inventory(), &profile);
                });
            }
        }
    }
    if let Some(audit) = source_contracts {
        report.set_source_contracts(audit.finish());
    }
    let report_path = resolve(base, &config.report);
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut assumptions = serde_json::json!({
        "schema_version": 1, "profile": profile.assumptions(),
        "contracts": contracts.source_assumptions(),
        "source_sha256": source_identities,
        "contract_manifest": contract_manifest_text,
        "compile_database_sha256": qcore_contract::profile_loader::sha256_hex(database.as_bytes()),
        "compile_task_identity_version": compile_db::COMPILE_TASK_IDENTITY_VERSION,
        "compile_tasks": compile_tasks,
        "scope": if config.source_contracts.is_some() {
            "existing-C resource projection of selected compiled units; separate configured source-contract evidence is embedded in the report without whole-unit promotion; Unsafe is provisional"
        } else {
            "existing-C resource projection of selected compiled units; no observation or provider composition; Unsafe is provisional"
        },
    });
    // Card CONTRACT-INHERIT: an environment that inherited any contract
    // publishes each inheritance chain beside the elaborated sources.
    if let Some(provenance) = qcore_contract::contract_loader::inclusion_provenance(&contracts) {
        assumptions["contract_inclusion"] = provenance;
    }
    if let Some(record) = contract_input.provider_record() {
        assumptions["provider_selection"] = record.clone();
    }
    if let Some(library_table) = &resolved_library_table {
        assumptions["library_provider_selection"] = library_table.record();
    }
    if let Some(record) = evidence_record {
        assumptions["evidence"] = record;
    }
    if let Some(selection) = &config.compile_target {
        assumptions["compile_target_selection"] = serde_json::json!({
            "object_prefix": selection.object_prefix,
            "role": "declared narrowing of a multi-target compilation database; records whose `-o` output does not begin with this fragment were not read",
            "residual_ambiguity": "still refused: two retained records for one file that differ in any other argument reject the audit",
        });
    }
    if let Some(head) = &selected_head {
        if checkout_head(&checkout)? != *head {
            return Err("checkout HEAD changed during audit; rerun with stable inputs".into());
        }
        assumptions["source_revision_selection"] = serde_json::json!({
            "policy": config.source_revision_policy,
            "resolved_commit": head,
            "profile_pin_role": "historical reference only; source digests and witnesses from that revision are not admitted",
            "source_specific_evidence": "Unknown: historical source summaries, provider and native observations were not replayed",
            "contract_scope": "declared resource contracts are assumptions at the selected revision, not verified upstream behavior",
        });
    }
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the run's closure
    // of every invoked member, folded over the rows before anything is
    // published; it only demotes.
    let (members, from_safe, from_pending) = report.fold_callback_members(&contracts);
    counts.safe_units -= from_safe;
    counts.pending_units -= from_pending;
    counts.unknown_units += from_safe + from_pending;
    if !members.is_null() {
        println!(
            "callback members: {} demoted",
            members["demoted"].as_array().map_or(0, Vec::len)
        );
        assumptions[callback_member_fold::SECTION_KEY] = members;
    }
    // Card W45-CONTRACT-REVISION-MODE: revision modes, pin mismatches and
    // flexible assumptions, beside the contract sources they are read from.
    assumptions["contract_revision"] = contract_revision::section(&contracts, &pins, &report);
    // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13h).
    assumptions[contract_dependencies::SECTION_KEY] = contract_dependencies::section(
        &assumptions["contract_revision"],
        &contract_dependencies::premises(&contracts)?,
        &report,
    );
    // Card CONTRACT-PIN-TIP-MODE: this route prints no tally, so it prints
    // each tip entry's notice on its own (nothing when there is none).
    for line in contract_revision::notice_lines(&assumptions["contract_revision"]) {
        println!("{line}");
    }
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: the refutation table and the
    // calls it held.
    assumptions["refuted_contracts"] = refuted_contracts::section(&refuted_table, &report);
    // Card AUDIT-COMPOSITION: the contract bundle the components rest on.
    report.record_contract_bundle(&contracts);
    report.write(&report_path)?;
    fs::write(
        report_path.with_extension("assumptions.json"),
        serde_json::to_vec_pretty(&assumptions).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    // Card W44-QUIRIN-AUDIT-PROVIDERS: run provenance, kept out of the
    // assumption sidecar so that sidecar stays the same at every job count.
    current_route::write_invocation(
        &report_path,
        &serde_json::json!({
            "schema_version": 1,
            "tool": "quirin-audit",
            "tool_version": env!("CARGO_PKG_VERSION"),
            "build_profile": if cfg!(debug_assertions) { "dev" } else { "release" },
            "configuration": path,
            "configuration_sha256": qcore_contract::profile_loader::sha256_hex(
                &fs::read(&path).map_err(|error| error.to_string())?,
            ),
            "contracts": {
                "selection": contract_input
                    .provider_record()
                    .map(|record| record["selection"].clone()),
            },
            "memory_admission": parallel::take_admission_trace(),
            "jobs": parallel::jobs_record(requested_jobs, parallel::effective_jobs(jobs),
                if flag_jobs.is_some() { "command_line" }
                else if config.jobs.is_some() { "configuration" }
                else if std::env::var_os(parallel::JOBS_VARIABLE).is_some() { "environment" }
                else { "default" }, config.memory_budget_mib),
            "environment": {
                "QCORE_CLANG": std::env::var("QCORE_CLANG").ok(),
                "QCORE_AUDIT_JOBS": std::env::var(parallel::JOBS_VARIABLE).ok(),
                "QCORE_GATEWAY_RULES_OFF": std::env::var(qcore_c_gateway::gateway_rules::RULES_OFF_VARIABLE).ok(),
            },
            "gateway_rules_off": qcore_c_gateway::gateway_rules::effective_off(),
            "clang": compiler,
            "scope": "invocation provenance of this run; not an input to any verdict",
        }),
    )?;
    contract_dependencies::clear_premises_cache();
    Ok(counts)
}

/// One configured unit's inputs (card W44-QUIRIN-AUDIT-PROVIDERS).
struct ConfiguredUnit<'a> {
    library: &'a str,
    entry: &'a CompileEntry,
    checkout: &'a Path,
    profile: &'a LoadedProfile,
    selected_head: Option<&'a str>,
    compiler: &'a qcore_c_gateway::ClangIdentity,
    task_dependencies: &'a BTreeMap<String, String>,
    contracts: &'a qcore_schemas::contract::ResourceContracts,
    source_profile_sha256: &'a str,
    fragment: AuditReport,
    retain_unit: bool,
    out: &'a mut String,
}

/// What one configured unit contributes, folded in unit order.
struct UnitProducts {
    file: String,
    source_sha256: String,
    compile_task: serde_json::Value,
    report: AuditReport,
    counts: AuditCounts,
    /// The source and its preparation, kept for the source-contract route.
    retained: Option<qcore_c_gateway::legacy_c::PreparedTranslationUnit>,
}

/// The body of the configured route's unit loop, unchanged, recording into
/// its own report fragment, tally and console buffer.
fn configured_unit(unit_input: ConfiguredUnit<'_>) -> Result<UnitProducts, String> {
    qcore_c_gateway::with_audit_unit_lifetimes(|| configured_unit_unscoped(unit_input))
}

fn configured_unit_unscoped(unit_input: ConfiguredUnit<'_>) -> Result<UnitProducts, String> {
    let ConfiguredUnit {
        library,
        entry,
        checkout,
        profile,
        selected_head,
        compiler,
        task_dependencies,
        contracts,
        source_profile_sha256,
        fragment,
        retain_unit,
        out,
    } = unit_input;
    let mut report = fragment;
    let mut counts = AuditCounts::default();
    let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
    let canonical_file = entry
        .file
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let file = canonical_file
        .strip_prefix(checkout)
        .map_err(|_| "source outside selected checkout")?;
    let file = repository_path(file);
    let pin = profile
        .pin_for(library)
        .ok_or("selected source has no pin")?;
    // This route returns a whole-run `Err(String)` rather than a report
    // row, so it narrows the refusal to its text (REASON-CODES-B1
    // residual: the category has nowhere to travel on a String channel).
    verify_pinned_bytes(
        checkout,
        selected_head.unwrap_or(&pin.git_commit),
        &entry.file,
        source.as_bytes(),
    )
    .map_err(UnitRefusal::into_reason)?;
    let source_sha256 = qcore_contract::profile_loader::sha256_hex(source.as_bytes());
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
    // Bind the effective preprocessing input, not just the main source
    // bytes: a used header, a generated configuration header or an
    // admitted macro option changes the analyzed code with the selected
    // `.c` file untouched.
    let prepared = qcore_c_gateway::legacy_c::prepare_translation_unit(&unit);
    let preparation_error = prepared.validation_error().map(ToString::to_string);
    let identity = compile_db::compile_task_identity(
        entry,
        &unit,
        match prepared.preprocessed_source() {
            Ok(source) => compile_db::EffectiveInput::Accepted(source),
            Err(_) => compile_db::EffectiveInput::Rejected(
                preparation_error
                    .as_deref()
                    .expect("rejected preparation has an error"),
            ),
        },
        compiler,
        task_dependencies.clone(),
    )?;
    let compile_task = serde_json::json!({
        "library": library, "file": file,
        "identity_sha256": identity.digest()?, "identity": &identity,
    });
    // Unit E01-F1: enumerate once, here, and hand the *same* inventory
    // to the resource projection, the report and the source-contract
    // route. Two independent computations of the denominator could
    // disagree; one cannot.
    ir_capture::configured(
        &prepared,
        &identity,
        contracts,
        source_profile_sha256,
        || {
            projection::project_prepared_unit_into(
                library,
                &file,
                &prepared,
                contracts,
                PreparedAuditCertificateContext {
                    compile_task_identity: &identity,
                    source_profile_sha256,
                },
                &mut report,
                &mut counts,
                out,
            )
        },
    )?;
    Ok(UnitProducts {
        file,
        source_sha256,
        compile_task,
        report,
        counts,
        retained: retain_unit.then_some(prepared),
    })
}

/// Compatibility adapters can select named profile libraries and preserve
/// historical report labels without carrying a source-path table in Rust.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibrarySelection {
    pub library: String,
    pub label: Option<String>,
    #[serde(default)]
    pub providers: Vec<String>,
    #[serde(default)]
    pub dependent_libraries: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SelectionFile {
    schema_version: u32,
    libraries: Vec<LibrarySelection>,
    #[serde(default)]
    dependency_libraries: BTreeMap<String, providers::LibraryProviderScope>,
}

fn parse_selection_file(path: &Path) -> Result<SelectionFile, String> {
    let selection: SelectionFile =
        serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    if selection.schema_version != 1 || selection.libraries.is_empty() {
        return Err("unsupported or empty audit selection".into());
    }
    let mut seen = BTreeSet::new();
    let mut labels = BTreeSet::new();
    for library in &selection.libraries {
        if library.library.trim().is_empty()
            || !seen.insert(&library.library)
            || !labels.insert(library.label.as_deref().unwrap_or(&library.library))
            || library
                .label
                .as_ref()
                .is_some_and(|label| label.trim().is_empty())
        {
            return Err(
                "audit selection requires distinct library names and nonempty labels".into(),
            );
        }
    }
    Ok(selection)
}

pub fn load_library_selections(path: &Path) -> Result<Vec<LibrarySelection>, String> {
    Ok(parse_selection_file(path)?.libraries)
}

/// Extract per-library provider scopes from `audit.json` if any library entry
/// or `dependency_libraries` declares `providers` or `dependent_libraries`.
pub fn load_audit_library_scopes(
    path: &Path,
) -> Result<Option<BTreeMap<String, providers::LibraryProviderScope>>, String> {
    let selection = parse_selection_file(path)?;
    let any_library = selection
        .libraries
        .iter()
        .any(|lib| !lib.providers.is_empty() || !lib.dependent_libraries.is_empty());
    if !any_library && selection.dependency_libraries.is_empty() {
        return Ok(None);
    }
    let mut scopes = selection.dependency_libraries;
    for lib in selection.libraries {
        if !lib.providers.is_empty() || !lib.dependent_libraries.is_empty() {
            let scope = providers::LibraryProviderScope {
                providers: lib.providers,
                dependent_libraries: lib.dependent_libraries,
            };
            scopes.insert(lib.library.clone(), scope.clone());
            if let Some(label) = lib.label {
                scopes.entry(label).or_insert(scope);
            }
        }
    }
    Ok(Some(scopes))
}

/// Read-only binary checkpoint inspector.
pub mod dump_checkpoint_cli;

mod loop_lifetime_fold;

struct ParkedUnitProducts {
    file: String,
    source_sha256: String,
    compile_task: serde_json::Value,
    report: report::ParkedReport,
    counts: AuditCounts,
    retained: Option<qcore_c_gateway::legacy_c::PreparedTranslationUnit>,
}

impl UnitProducts {
    fn park(self) -> Result<ParkedUnitProducts, String> {
        Ok(ParkedUnitProducts {
            file: self.file,
            source_sha256: self.source_sha256,
            compile_task: self.compile_task,
            report: self.report.park()?,
            counts: self.counts,
            retained: self.retained,
        })
    }
}

impl ParkedUnitProducts {
    fn restore(self) -> Result<UnitProducts, String> {
        Ok(UnitProducts {
            file: self.file,
            source_sha256: self.source_sha256,
            compile_task: self.compile_task,
            report: self.report.restore()?,
            counts: self.counts,
            retained: self.retained,
        })
    }
}
