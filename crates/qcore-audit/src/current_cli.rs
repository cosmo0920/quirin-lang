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

//! `quirin-audit current` (card **W44-QUIRIN-AUDIT-PROVIDERS**): the configured
//! current-revision route, driven by flags, by a JSON configuration
//! (`--config FILE`), or both.
//!
//! **One rule for both forms.** Every setting may be given on the command
//! line, in the configuration, or in both; when both give it, the two must
//! declare the same value — for providers, the same resolved module selection
//! — or the command refuses. Nothing is merged or overridden, so a run is
//! described completely by either form, and the record names which forms
//! declared the provider selection. The one exception is the job count, which
//! changes wall time only: `--jobs` over the configuration's `jobs` over
//! `QCORE_AUDIT_JOBS` over the capped host default
//! ([`crate::parallel::jobs_with_precedence`]).
//!
//! **Contracts.** Exactly one source: a provider selection (`--provider` /
//! `providers`, resolved on `--load-path` / `load_path`), an explicit manifest
//! (`--contract-manifest` / `contract_manifest`), or, when neither is given,
//! the manifest the source profile declares. A provider selection supersedes
//! that profile default (recorded as not read); a provider selection together
//! with an explicit manifest is two declarations of one input and refuses.
//! In the full (configured) mode there is no default provider set: no
//! selection, no manifest and a profile that declares none refuses.
//!
//! **Contracts only loads all contracts** (card W44-PROVIDER-RESOURCE-VIEW,
//! the user's definition). A contracts-only run with no provider selection
//! and no explicit manifest selects every crate on the load path whole — each
//! crate's resource view — and records `"selection": "all-loaded"`; it needs
//! the load path, supersedes the profile's default manifest like any
//! selection, and an explicit selection or manifest narrows it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_contract::profile_loader::{load_profile_directory, sha256_hex};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::current_route::{
    CurrentRoute, historical_reference_revision, historical_reference_revision_of,
    run_current_route_with_checkpoints,
};
use crate::evidence::EvidenceMode;
use crate::providers::{
    ContractInput, LibraryProviderScope, LibraryProvidersFile, SelectionSource, agree,
    agree_library_providers, resolve_all_loaded, resolve_library_providers_table,
    resolve_providers,
};

/// Usage text of `quirin-audit current`.
pub const USAGE: &str = "usage: quirin-audit current [--config FILE] [--checkout DIR] \
[--profile-directory DIR] [--compile-database PATH] [--monkey-database PATH] [--monkey-linked] [--report PATH] \
[--monkey-source-report PATH] [--binding-directory DIR] [--provider NAME[:MODULE]]... \
[--load-path PATH] [--contract-manifest PATH] [--contracts-only] [--quirin-providers-only] \
[--selection FILE] [--jobs N] [--memory-budget-mib N] [--checkpoint-dir DIR [--resume]]";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
enum LibraryProvidersSetting {
    File(PathBuf),
    Inline(BTreeMap<String, LibraryProviderScope>),
}

/// The JSON configuration of `quirin-audit current`. Relative paths resolve
/// against the configuration file's directory.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentConfig {
    schema_version: u32,
    #[serde(default)]
    checkout: Option<PathBuf>,
    #[serde(default)]
    profile_directory: Option<PathBuf>,
    #[serde(default)]
    compile_database: Option<PathBuf>,
    #[serde(default)]
    monkey_database: Option<PathBuf>,
    /// Card CURRENT-MONKEY-LINKED-CONFIGURATION: analyse Monkey as the
    /// compile database configures it (the configuration linked into
    /// fluent-bit); a `monkey_database` is then only accounted for.
    #[serde(default)]
    monkey_linked: Option<bool>,
    #[serde(default)]
    report: Option<PathBuf>,
    #[serde(default)]
    monkey_source_report: Option<PathBuf>,
    #[serde(default)]
    binding_directory: Option<PathBuf>,
    #[serde(default)]
    providers: Option<Vec<String>>,
    #[serde(default)]
    library_providers: Option<LibraryProvidersSetting>,
    #[serde(default)]
    load_path: Option<PathBuf>,
    #[serde(default)]
    contract_manifest: Option<PathBuf>,
    #[serde(default)]
    mode: Option<EvidenceMode>,
    #[serde(default)]
    quirin_providers_only: Option<bool>,
    #[serde(default)]
    jobs: Option<usize>,
    memory_budget_mib: Option<usize>,
    /// Card W45-FB-SYSTEM-SELECTION: an explicit unit selection file that
    /// replaces the profile's `audit.json` groups.
    #[serde(default)]
    selection: Option<PathBuf>,
    /// Card W45-AUDIT-CHECKPOINTS: seal complete compile-configuration
    /// checkpoints, then compose them; `resume` reuses matching ones.
    #[serde(default)]
    checkpoint_dir: Option<PathBuf>,
    #[serde(default)]
    resume: Option<bool>,
    /// Named only to be refused with a reason: this route admits no process
    /// package or source-contract evidence.
    #[serde(default)]
    source_contracts: Option<Value>,
    #[serde(default)]
    packages: Option<Value>,
}

/// The settings of one form, paths already anchored.
#[derive(Debug, Default)]
struct Settings {
    checkout: Option<PathBuf>,
    profile_directory: Option<PathBuf>,
    compile_database: Option<PathBuf>,
    monkey_database: Option<PathBuf>,
    monkey_linked: Option<bool>,
    report: Option<PathBuf>,
    monkey_source_report: Option<PathBuf>,
    binding_directory: Option<PathBuf>,
    providers: Option<Vec<String>>,
    library_providers: Option<BTreeMap<String, LibraryProviderScope>>,
    load_path: Option<PathBuf>,
    contract_manifest: Option<PathBuf>,
    mode: Option<EvidenceMode>,
    quirin_providers_only: Option<bool>,
    jobs: Option<usize>,
    memory_budget_mib: Option<usize>,
    selection: Option<PathBuf>,
    checkpoint_dir: Option<PathBuf>,
    resume: Option<bool>,
}

pub(crate) fn anchor(base: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn parse_command_line(
    arguments: &[String],
    cwd: &Path,
) -> Result<(Settings, Option<PathBuf>), String> {
    let mut settings = Settings::default();
    let mut config = None;
    let mut providers = Vec::new();
    let mut index = 0;
    let set_path = |slot: &mut Option<PathBuf>, flag: &str, value: Option<&String>| {
        let value = value.ok_or_else(|| format!("{flag} requires a value"))?;
        if slot.is_some() {
            return Err(format!("{flag} is given twice"));
        }
        *slot = Some(anchor(cwd, PathBuf::from(value)));
        Ok::<(), String>(())
    };
    while index < arguments.len() {
        let flag = arguments[index].as_str();
        let value = arguments.get(index + 1);
        let mut takes_value = true;
        match flag {
            "--config" => set_path(&mut config, flag, value)?,
            "--checkout" => set_path(&mut settings.checkout, flag, value)?,
            "--profile-directory" => set_path(&mut settings.profile_directory, flag, value)?,
            "--compile-database" => set_path(&mut settings.compile_database, flag, value)?,
            "--monkey-database" => set_path(&mut settings.monkey_database, flag, value)?,
            "--monkey-linked" => {
                if settings.monkey_linked.is_some() {
                    return Err("--monkey-linked is given twice".into());
                }
                settings.monkey_linked = Some(true);
                takes_value = false;
            }
            "--report" => set_path(&mut settings.report, flag, value)?,
            "--monkey-source-report" => {
                set_path(&mut settings.monkey_source_report, flag, value)?;
            }
            "--binding-directory" => set_path(&mut settings.binding_directory, flag, value)?,
            "--load-path" => set_path(&mut settings.load_path, flag, value)?,
            "--contract-manifest" => set_path(&mut settings.contract_manifest, flag, value)?,
            "--selection" => set_path(&mut settings.selection, flag, value)?,
            "--checkpoint-dir" => set_path(&mut settings.checkpoint_dir, flag, value)?,
            "--resume" => {
                settings.resume = Some(true);
                takes_value = false;
            }
            "--provider" => {
                providers.push(
                    value
                        .ok_or("--provider requires a provider crate name")?
                        .clone(),
                );
            }
            "--library-providers" => {
                let raw = value.ok_or("--library-providers requires a file path")?;
                if settings.library_providers.is_some() {
                    return Err("--library-providers is given twice".into());
                }
                let path = anchor(cwd, PathBuf::from(raw));
                let parsed = LibraryProvidersFile::load(&path)?;
                if settings.load_path.is_none()
                    && let Some(lp) = parsed.contract_load_path
                {
                    let file_dir = path.parent().unwrap_or(cwd);
                    settings.load_path = Some(anchor(file_dir, lp));
                }
                if parsed.quirin_providers_only {
                    settings.quirin_providers_only = Some(true);
                }
                settings.library_providers = Some(parsed.libraries);
            }
            "--memory-budget-mib" => {
                if settings.memory_budget_mib.is_some() {
                    return Err("--memory-budget-mib is given twice".into());
                }
                let budget = value
                    .ok_or("--memory-budget-mib requires an integer")?
                    .parse::<usize>()
                    .map_err(|_| "--memory-budget-mib requires an integer")?;
                crate::parallel::budgeted_jobs(1, Some(budget))?;
                settings.memory_budget_mib = Some(budget);
            }
            "--jobs" => {
                let text = value.ok_or("--jobs requires a positive integer")?;
                if settings.jobs.is_some() {
                    return Err("--jobs is given twice".into());
                }
                settings.jobs = Some(crate::parallel::resolve_jobs(Some(text))?);
            }
            "--contracts-only" => {
                settings.mode = Some(EvidenceMode::ContractsOnly);
                takes_value = false;
            }
            "--quirin-providers-only" => {
                settings.quirin_providers_only = Some(true);
                takes_value = false;
            }
            other => return Err(format!("unknown argument `{other}`\n{USAGE}")),
        }
        index += if takes_value { 2 } else { 1 };
    }
    if !providers.is_empty() {
        settings.providers = Some(providers);
    }
    Ok((settings, config))
}

fn read_config(path: &Path) -> Result<(Settings, Vec<u8>), String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let config: CurrentConfig =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    if config.schema_version != 1 {
        return Err(format!(
            "{}: unsupported current-audit configuration version {}",
            path.display(),
            config.schema_version
        ));
    }
    if config.source_contracts.is_some() || config.packages.is_some() {
        return Err(format!(
            "{}: the current route admits no process packages or source-contract checks, and a \
             contracts-only run never does; `source_contracts` belongs to `quirin-audit \
             CONFIG.json`",
            path.display()
        ));
    }
    let base = path
        .parent()
        .ok_or("configuration has no parent directory")?;
    let anchor_all = |path: Option<PathBuf>| path.map(|path| anchor(base, path));
    let mut cfg_load_path = anchor_all(config.load_path);
    let mut cfg_quirin_only = config.quirin_providers_only;
    let library_providers = match config.library_providers {
        Some(LibraryProvidersSetting::File(file_path)) => {
            let resolved_file = anchor(base, file_path);
            let parsed = LibraryProvidersFile::load(&resolved_file)?;
            if cfg_load_path.is_none()
                && let Some(lp) = parsed.contract_load_path
            {
                let file_dir = resolved_file.parent().unwrap_or(base);
                cfg_load_path = Some(anchor(file_dir, lp));
            }
            if parsed.quirin_providers_only && cfg_quirin_only.is_none() {
                cfg_quirin_only = Some(true);
            }
            Some(parsed.libraries)
        }
        Some(LibraryProvidersSetting::Inline(map)) => Some(map),
        None => None,
    };
    Ok((
        Settings {
            checkout: anchor_all(config.checkout),
            profile_directory: anchor_all(config.profile_directory),
            compile_database: anchor_all(config.compile_database),
            monkey_database: anchor_all(config.monkey_database),
            monkey_linked: config.monkey_linked,
            report: anchor_all(config.report),
            monkey_source_report: anchor_all(config.monkey_source_report),
            binding_directory: anchor_all(config.binding_directory),
            providers: config.providers,
            library_providers,
            load_path: cfg_load_path,
            contract_manifest: anchor_all(config.contract_manifest),
            mode: config.mode,
            quirin_providers_only: cfg_quirin_only,
            jobs: config.jobs,
            memory_budget_mib: config.memory_budget_mib,
            selection: anchor_all(config.selection),
            checkpoint_dir: anchor_all(config.checkpoint_dir),
            resume: config.resume,
        },
        bytes,
    ))
}

/// A setting given by both forms must be equal.
pub(crate) fn one<T: PartialEq + std::fmt::Debug>(
    name: &str,
    command_line: Option<T>,
    configuration: Option<T>,
) -> Result<Option<T>, String> {
    match (command_line, configuration) {
        (Some(left), Some(right)) if left != right => Err(format!(
            "`{name}` conflicts: the command line gives {left:?} and the configuration gives \
             {right:?}"
        )),
        (Some(value), _) | (None, Some(value)) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

/// Every declaration of a run's resource contracts, from both forms.
pub(crate) struct ContractDeclarations<'a> {
    pub command_line: Option<&'a Vec<String>>,
    pub configuration: Option<&'a Vec<String>>,
    pub load_path: Option<&'a Path>,
    pub contract_manifest: Option<&'a Path>,
    pub mode: EvidenceMode,
    pub quirin_providers_only: bool,
    /// The source profile's declared manifest, when the route has a profile.
    pub profile_manifest: Option<&'a Path>,
}

/// Choose the one contract source of a run, with the refusals this module's
/// header states. `quirin-audit current` and the no-build `quirin-audit
/// contracts` route (card W44-CONTRACTS-ONLY-NO-BUILD) share it, so both
/// resolve a selection, a manifest and the all-loaded default identically.
pub(crate) fn choose_contracts(
    declared: &ContractDeclarations<'_>,
) -> Result<(ContractInput, &'static str), String> {
    let has_providers = declared.command_line.is_some() || declared.configuration.is_some();
    let all_loaded =
        !has_providers && declared.mode.is_contracts_only() && declared.contract_manifest.is_none();
    if !has_providers && !all_loaded && declared.load_path.is_some() {
        return Err("a contract load path is given but no provider is selected".into());
    }
    if !has_providers && !all_loaded && declared.quirin_providers_only {
        return Err(
            "--quirin-providers-only guards a provider selection; select providers with \
             --provider"
                .into(),
        );
    }
    Ok(if has_providers {
        if declared.contract_manifest.is_some() {
            return Err(
                "a provider selection and an explicit contract manifest are two declarations of \
                 one input; give one"
                    .into(),
            );
        }
        let load_path = declared.load_path.ok_or(
            "a provider selection needs the contract load path (--load-path \
             providers/contract-load-path.json)",
        )?;
        let resolve = |entries: Option<&Vec<String>>, source| {
            entries
                .map(|entries| {
                    resolve_providers(load_path, entries, declared.quirin_providers_only, source)
                })
                .transpose()
        };
        let resolved = match (
            resolve(declared.command_line, SelectionSource::CommandLine)?,
            resolve(declared.configuration, SelectionSource::Configuration)?,
        ) {
            (Some(left), Some(right)) => agree(left, right)?,
            (Some(only), None) | (None, Some(only)) => only,
            (None, None) => unreachable!("a provider selection was given"),
        };
        (
            ContractInput::providers(&resolved, declared.profile_manifest),
            "providers",
        )
    } else if all_loaded {
        let load_path = declared.load_path.ok_or(
            "a contracts-only run with no provider selection loads every crate's resource view on \
             the contract load path: give it (--load-path providers/contract-load-path.json), or \
             narrow the run with --provider or --contract-manifest",
        )?;
        (
            ContractInput::providers(
                &resolve_all_loaded(load_path, declared.quirin_providers_only)?,
                declared.profile_manifest,
            ),
            "providers",
        )
    } else if let Some(manifest) = declared.contract_manifest {
        (ContractInput::manifest(manifest)?, "manifest")
    } else if let Some(manifest) = declared.profile_manifest {
        (ContractInput::manifest(manifest)?, "profile_manifest")
    } else {
        return Err(
            "no resource contracts selected: name providers (--provider NAME / \"providers\") or \
             a manifest (--contract-manifest PATH); there is no default provider set, and the \
             source profile declares no manifest"
                .into(),
        );
    })
}

/// Card W45-AUDIT-CHECKPOINTS: the checkpoint settings of both forms, which
/// must agree like every other setting. `--resume` needs a directory.
fn checkpoint_options(
    directories: (Option<PathBuf>, Option<PathBuf>),
    resumes: (Option<bool>, Option<bool>),
) -> Result<Option<crate::checkpoint_run::CheckpointOptions>, String> {
    let directory = one("checkpoint_dir", directories.0, directories.1)?;
    let resume = one("resume", resumes.0, resumes.1)?.unwrap_or(false);
    match directory {
        Some(directory) => Ok(Some(crate::checkpoint_run::CheckpointOptions {
            directory,
            resume,
        })),
        None if resume => Err("--resume needs --checkpoint-dir DIR".into()),
        None => Ok(None),
    }
}

/// Parse, resolve and run. Returns the process exit code.
pub fn main(arguments: Vec<String>) -> ExitCode {
    match run(&arguments) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("current audit rejected (Unknown): {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<bool, String> {
    crate::parallel::reset_admission_trace();
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let (command_line, config_path) = parse_command_line(arguments, &cwd)?;
    let (configuration, config_bytes) = match &config_path {
        Some(path) => {
            let (settings, bytes) = read_config(path)?;
            (settings, Some(bytes))
        }
        None => (Settings::default(), None),
    };
    let checkout = one("checkout", command_line.checkout, configuration.checkout)?
        .ok_or_else(|| format!("a checkout is required (--checkout DIR)\n{USAGE}"))?;
    let profile_directory = one(
        "profile_directory",
        command_line.profile_directory,
        configuration.profile_directory,
    )?
    .ok_or_else(|| format!("a source profile is required (--profile-directory DIR)\n{USAGE}"))?;
    let compile_database = one(
        "compile_database",
        command_line.compile_database,
        configuration.compile_database,
    )?;
    let monkey_database = one(
        "monkey_database",
        command_line.monkey_database,
        configuration.monkey_database,
    )?;
    let monkey_linked = one(
        "monkey_linked",
        command_line.monkey_linked,
        configuration.monkey_linked,
    )?
    .unwrap_or(false);
    let report = one("report", command_line.report, configuration.report)?;
    // Card RULE-SWITCH-ENTRY-POINTS: without a report no component records
    // `rule_set.off`, so the counts alone must not run under a switch list.
    if report.is_none() {
        qcore_c_gateway::gateway_rules::refuse_unrecorded("quirin-audit current without a report")?;
    }
    let monkey_source_report = one(
        "monkey_source_report",
        command_line.monkey_source_report,
        configuration.monkey_source_report,
    )?;
    let binding_directory = one(
        "binding_directory",
        command_line.binding_directory,
        configuration.binding_directory,
    )?;
    let load_path = one("load_path", command_line.load_path, configuration.load_path)?;
    let contract_manifest = one(
        "contract_manifest",
        command_line.contract_manifest,
        configuration.contract_manifest,
    )?;
    let mode = one("mode", command_line.mode, configuration.mode)?.unwrap_or_default();
    let quirin_providers_only = one(
        "quirin_providers_only",
        command_line.quirin_providers_only,
        configuration.quirin_providers_only,
    )?
    .unwrap_or(false);
    let selection_path = one("selection", command_line.selection, configuration.selection)?;
    let jobs_source = if command_line.jobs.is_some() {
        "command_line"
    } else if configuration.jobs.is_some() {
        "configuration"
    } else if std::env::var_os(crate::parallel::JOBS_VARIABLE).is_some() {
        "environment"
    } else {
        "default"
    };
    let jobs = crate::parallel::jobs_with_precedence(command_line.jobs, configuration.jobs)?;
    let memory_budget_mib = one(
        "memory_budget_mib",
        command_line.memory_budget_mib,
        configuration.memory_budget_mib,
    )?;
    let effective_jobs =
        crate::parallel::effective_jobs(crate::parallel::budgeted_jobs(jobs, memory_budget_mib)?);
    let profile = load_profile_directory(&profile_directory).map_err(|error| error.to_string())?;
    let profile_manifest = profile.resource_contract_manifest_path();
    let selection = selection_path
        .as_deref()
        .map(crate::unit_selection::UnitSelection::load)
        .transpose()?;
    if let Some(selection) = &selection {
        for group in &selection.groups {
            if profile.library(&group.library).is_none() {
                return Err(format!(
                    "unit selection `{}` names library `{}`, which the source profile does not \
                     declare",
                    selection.name, group.library
                ));
            }
        }
    }
    let selection_scopes = match &selection {
        Some(selection) => selection.library_provider_scopes(),
        None => crate::load_audit_library_scopes(&profile_directory.join("audit.json"))?,
    };
    let has_library_providers = command_line.library_providers.is_some()
        || configuration.library_providers.is_some()
        || selection_scopes.is_some();
    let mut audited_libraries = match &selection {
        Some(selection) => selection
            .groups
            .iter()
            .map(|group| group.library.clone())
            .collect::<Vec<_>>(),
        None => crate::load_library_selections(&profile_directory.join("audit.json"))?
            .into_iter()
            .map(|item| item.library)
            .collect::<Vec<_>>(),
    };
    // Card CURRENT-MONKEY-LINKED-CONFIGURATION: a linked run's Monkey units
    // carry the `monkey` label; the profile's per-library provider table
    // scopes only `monkey-libevent` (the standalone configuration), so a
    // linked run under per-library scoping refuses instead of reusing it.
    if monkey_linked {
        audited_libraries.push(crate::current_route::LINKED_MONKEY_LABEL.into());
    } else if monkey_database.is_some() {
        audited_libraries.push("monkey-libevent".into());
    }
    let resolved_library_table = if has_library_providers {
        if command_line.providers.is_some()
            || configuration.providers.is_some()
            || contract_manifest.is_some()
        {
            return Err(
                "per-library provider scoping and an explicit provider list or contract manifest are two declarations of one input; give one"
                    .into(),
            );
        }
        let lp = load_path.as_deref().ok_or(
            "per-library provider scoping needs the contract load path (--load-path providers/contract-load-path.json)",
        )?;
        let resolve_table = |scopes: &BTreeMap<String, LibraryProviderScope>,
                             source: SelectionSource| {
            resolve_library_providers_table(
                lp,
                scopes,
                &audited_libraries,
                quirin_providers_only,
                source,
                profile_manifest.as_deref(),
            )
        };
        let mut resolved = None;
        for (scopes_opt, source) in [
            (
                command_line.library_providers.as_ref(),
                SelectionSource::CommandLine,
            ),
            (
                configuration.library_providers.as_ref(),
                SelectionSource::Configuration,
            ),
            (selection_scopes.as_ref(), SelectionSource::Configuration),
        ] {
            if let Some(scopes) = scopes_opt {
                let next = resolve_table(scopes, source)?;
                resolved = Some(match resolved {
                    Some(prev) => agree_library_providers(prev, next)?,
                    None => next,
                });
            }
        }
        resolved
    } else {
        None
    };
    let (contracts, contract_source) = match &resolved_library_table {
        Some(table) => (table.union_input.clone(), "library_providers"),
        None => choose_contracts(&ContractDeclarations {
            command_line: command_line.providers.as_ref(),
            configuration: configuration.providers.as_ref(),
            load_path: load_path.as_deref(),
            contract_manifest: contract_manifest.as_deref(),
            mode,
            quirin_providers_only,
            profile_manifest: profile_manifest.as_deref(),
        })?,
    };
    let historical = match &selection {
        Some(selection) => historical_reference_revision_of(
            &profile,
            &selection
                .groups
                .iter()
                .map(|group| group.library.clone())
                .collect::<Vec<_>>(),
        )?,
        None => historical_reference_revision(&profile)?,
    };
    let source_set = match &selection {
        Some(selection) => selection.name.clone(),
        None => profile_directory
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "source set".into()),
    };
    let checkpoints = checkpoint_options(
        (command_line.checkpoint_dir, configuration.checkpoint_dir),
        (command_line.resume, configuration.resume),
    )?;
    let failures = run_current_route_with_checkpoints(
        &CurrentRoute {
            checkout: &checkout,
            profile: &profile,
            contracts: &contracts,
            mode,
            compile_database: compile_database.as_deref(),
            monkey_database: monkey_database.as_deref(),
            monkey_linked,
            monkey_source_report: monkey_source_report.as_deref(),
            diagnostic_report: report.as_deref(),
            binding_directory: binding_directory.as_deref(),
            label: &source_set,
            historical_note: "Historical source/native/provider evidence: Unknown; use quirin-audit pinned for historical replay",
            historical_reference_revision: &historical,
            jobs: effective_jobs,
            selection: selection.as_ref(),
            library_providers: resolved_library_table.as_ref(),
        },
        checkpoints.as_ref(),
    )?;
    if let Some(report) = &report {
        let digest = |path: &Option<PathBuf>| -> Result<Value, String> {
            path.as_ref()
                .map(|path| {
                    std::fs::read(path)
                        .map(|bytes| json!(sha256_hex(&bytes)))
                        .map_err(|error| format!("{}: {error}", path.display()))
                })
                .transpose()
                .map(|value| value.unwrap_or(Value::Null))
        };
        let default_database = checkout.join("build/compile_commands.json");
        let database = compile_database.clone().unwrap_or(default_database);
        let mut invocation = json!({
            "schema_version": 1,
            "tool": "quirin-audit current",
            "tool_version": env!("CARGO_PKG_VERSION"),
            "build_profile": if cfg!(debug_assertions) { "dev" } else { "release" },
            "argv": std::iter::once("quirin-audit".to_owned()).chain(std::iter::once("current".to_owned())).chain(arguments.iter().cloned()).collect::<Vec<_>>(),
            "cwd": cwd,
            "configuration": config_path,
            "configuration_sha256": config_bytes.as_deref().map(sha256_hex),
            "source_revision": crate::checkout_head(&checkout.canonicalize().map_err(|error| error.to_string())?)?,
            "checkout": checkout,
            "profile_directory": profile_directory,
            "compile_database": database,
            "compile_database_sha256": digest(&Some(database.clone()))?,
            "monkey_database": monkey_database,
            "monkey_database_sha256": digest(&monkey_database)?,
            "reports": {
                "report": report, "monkey_source_report": monkey_source_report,
                "binding_directory": binding_directory,
            },
            "contracts": {
                "source": contract_source,
                "selection": contracts.provider_record().map(|record| record["selection"].clone()),
                "manifest": contracts.manifest_path(),
                "manifest_sha256": sha256_hex(contracts.manifest_text().as_bytes()),
            },
            "mode": mode,
            "memory_admission": crate::parallel::take_admission_trace(),
            "jobs": crate::parallel::jobs_record(jobs, effective_jobs, jobs_source, memory_budget_mib),
            "environment": {
                "QCORE_CLANG": std::env::var("QCORE_CLANG").ok(),
                "QCORE_AUDIT_JOBS": std::env::var(crate::parallel::JOBS_VARIABLE).ok(),
                "QCORE_GATEWAY_RULES_OFF": std::env::var(qcore_c_gateway::gateway_rules::RULES_OFF_VARIABLE).ok(),
            },
            "gateway_rules_off": qcore_c_gateway::gateway_rules::effective_off(),
            "clang": qcore_c_gateway::ClangIdentity::capture()?,
            "gateway_failures": failures,
            "scope": "invocation provenance of this run; not an input to any verdict. Preparation (the Clang CMake build and the Monkey inputs, including the C++ compiler) is recorded by quirin-audit prepare beside its outputs (build-provenance.json, qcore-monkey-inputs.json and prepare.json); resource analysis does not establish GCC equivalence or whole-library safety",
        });
        // Card W45-FB-SYSTEM-SELECTION: named only when given, so a run over
        // the profile's own groups keeps its record byte for byte.
        if let Some(selection) = &selection {
            invocation["selection"] = selection.record();
        }
        // Card CURRENT-MONKEY-LINKED-CONFIGURATION: likewise.
        if monkey_linked {
            invocation["monkey_linked"] = json!(true);
        }
        // Card W45-AUDIT-CHECKPOINTS: likewise named only when given.
        if let Some(options) = &checkpoints {
            invocation["checkpoints"] = json!({
                "directory": options.directory, "resume": options.resume,
                "run_record": options.directory.join(crate::checkpoint_run::RUN_RECORD),
            });
        }
        crate::current_route::write_invocation(report, &invocation)?;
    }
    Ok(failures)
}

#[cfg(test)]
mod tests {
    #[test]
    fn memory_budget_flags_and_configuration_agree_and_refuse_bad_values() {
        let cwd = std::path::Path::new("/tmp");
        let args = vec![
            "--memory-budget-mib".into(),
            "7168".into(),
            "--jobs".into(),
            "8".into(),
        ];
        let (settings, _) = super::parse_command_line(&args, cwd).unwrap();
        assert_eq!(settings.memory_budget_mib, Some(7168));
        assert_eq!(settings.jobs, Some(8));
        let config: super::CurrentConfig =
            serde_json::from_str(r#"{"schema_version":1,"memory_budget_mib":7168}"#).unwrap();
        assert_eq!(
            super::one(
                "memory_budget_mib",
                settings.memory_budget_mib,
                config.memory_budget_mib
            )
            .unwrap(),
            Some(7168)
        );
        assert!(super::one("memory_budget_mib", Some(7168), Some(6144)).is_err());
        for value in ["0", "3071", "x", "-1"] {
            assert!(
                super::parse_command_line(&["--memory-budget-mib".into(), value.into()], cwd)
                    .is_err()
            );
        }
        assert!(super::parse_command_line(&["--memory-budget-mib".into()], cwd).is_err());
        assert!(
            super::parse_command_line(
                &[
                    "--memory-budget-mib".into(),
                    "7168".into(),
                    "--memory-budget-mib".into(),
                    "7168".into()
                ],
                cwd
            )
            .is_err()
        );
    }

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn command_line_flags_anchor_to_the_working_directory_and_repeat_providers() {
        let (settings, config) = parse_command_line(
            &args(&[
                "--checkout",
                "src",
                "--provider",
                "quirin-provider-libc",
                "--provider",
                "quirin-provider-cfl:cfl/time.qv",
                "--contracts-only",
                "--jobs",
                "3",
                "--config",
                "/abs/config.json",
            ]),
            Path::new("/work"),
        )
        .unwrap();
        assert_eq!(settings.checkout, Some(PathBuf::from("/work/src")));
        assert_eq!(
            settings.providers,
            Some(args(&[
                "quirin-provider-libc",
                "quirin-provider-cfl:cfl/time.qv"
            ]))
        );
        assert_eq!(settings.mode, Some(EvidenceMode::ContractsOnly));
        assert_eq!(settings.jobs, Some(3));
        assert_eq!(config, Some(PathBuf::from("/abs/config.json")));
    }

    #[test]
    fn a_repeated_single_value_flag_or_an_unknown_flag_refuses() {
        let cwd = Path::new("/work");
        assert!(parse_command_line(&args(&["--checkout", "a", "--checkout", "b"]), cwd).is_err());
        assert!(parse_command_line(&args(&["--checkout"]), cwd).is_err());
        assert!(parse_command_line(&args(&["--frobnicate"]), cwd).is_err());
        assert!(parse_command_line(&args(&["--jobs", "0"]), cwd).is_err());
    }

    #[test]
    fn a_setting_given_by_both_forms_must_agree() {
        assert_eq!(one("x", Some(1), None).unwrap(), Some(1));
        assert_eq!(one("x", None, Some(2)).unwrap(), Some(2));
        assert_eq!(one("x", Some(3), Some(3)).unwrap(), Some(3));
        assert!(one("x", Some(3), Some(4)).is_err());
        assert_eq!(one::<u8>("x", None, None).unwrap(), None);
    }
}
