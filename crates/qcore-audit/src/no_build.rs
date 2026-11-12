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

//! `quirin-audit contracts` (card **W44-CONTRACTS-ONLY-NO-BUILD**): a
//! contracts-only audit that needs no build settings.
//!
//! The user's definition: "Contract only should be meant as using all of
//! loaded contracts without build settings." Card W44-PROVIDER-RESOURCE-VIEW
//! delivered the first half (a contracts-only run with no selection loads every
//! crate's resource view). This route delivers the second: it audits C sources
//! with the loaded contracts alone, without a compilation database, a prepared
//! build or a source profile.
//!
//! **Selection.** Explicit files, or a walk of `*.c` under directories (the
//! source root when nothing else is named). The walk is sorted, does not enter
//! directories whose name starts with `.`, and does not follow symbolic links;
//! a named directory with no unit, and an empty selection, refuse.
//!
//! **Assumed configuration.** Each unit is preprocessed with only what the user
//! supplied (`-I`, `-D`, `--std`) and three stated defaults: the `gnu17`
//! dialect, the unit's own directory for quoted includes (Clang's includer
//! rule for the declared input), and the host Clang's own system headers.
//! Nothing is searched for or guessed: no include directory discovery, no
//! configuration header, no build-system reading.
//!
//! **Publication rule.** Every verdict is relative to the recorded assumptions
//! and to nothing else. A unit that does not preprocess or parse under them is
//! `Unknown` with a reason code (`missing-include`, `unresolved-configuration`,
//! `preprocessing-error`, `syntax-error`, …) and its first diagnostic; it is
//! never dropped and nothing is inferred. A no-build verdict is never presented
//! as equivalent to a build-configured one: the assumption record is each
//! compile task's `source_profile` dependency, so a no-build Safe certificate
//! is bound to it and cannot be admitted for any configured compile task, and
//! every report row carries its assumptions. Lean model:
//! `proof/QCore/NoBuildScope.lean`.
//!
//! Everything else is shared, unchanged: the gateway, the provider selection
//! (with the all-loaded default), the ordered parallel fold and the
//! fail-closed projection.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, CCompilerDialect, LegacyCError, LegacyFunctionScope,
    PreparationStageOutcome, PreparedTranslationUnit, legacy_c_source_functions,
    prepare_translation_unit,
};
use qcore_contract::profile_loader::sha256_hex;
use qcore_schemas::contract::ResourceContracts;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::compile_db::{CompileEntry, EffectiveInput, compile_task_identity, compiler_options};
use crate::current_cli::{ContractDeclarations, anchor, choose_contracts, one};
use crate::evidence::EvidenceMode;
use crate::parallel::run_ordered;
use crate::projection::{AuditCounts, PreparedAuditCertificateContext, project_prepared_unit_into};
use crate::providers::ContractInput;
use crate::report::{AuditReport, UnitRefusal};
use crate::user_plugins::PluginCheckers;

/// Usage text of `quirin-audit contracts`.
pub const USAGE: &str = "usage: quirin-audit contracts [--config FILE] [--source-root DIR] \
[PATH]... [-I DIR]... [-D NAME[=VALUE]]... [--std c11|gnu99|gnu11|gnu17] [--load-path PATH] \
[--provider NAME[:MODULE]]... [--contract-manifest PATH] [--quirin-providers-only] \
[--report PATH] [--jobs N]";

/// The evidence kind every record of this route carries.
pub const EVIDENCE: &str = "contracts-only-no-build";

/// The contract load path used when neither a load path nor a manifest is
/// given, relative to the working directory (the repository root).
pub const DEFAULT_LOAD_PATH: &str = "providers/contract-load-path.json";

/// The scope every verdict of this route is published under.
pub const VERDICT_SCOPE: &str = "relative to the recorded assumptions only: the unit as preprocessed under this dialect, these include directories and definitions, its own directory for quoted includes and the host Clang's system headers; not equivalent to, comparable with or a substitute for a build-configured verdict of the same file, whose configuration can select different code";

/// The stated default dialect.
pub const DEFAULT_STD: &str = "gnu17";

/// The JSON configuration of `quirin-audit contracts`. Relative paths resolve
/// against the configuration file's directory, except `sources`, which resolve
/// against the source root.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoBuildConfig {
    schema_version: u32,
    /// Required: `contracts-only`.
    mode: EvidenceMode,
    /// Required: `none`.
    build: String,
    #[serde(default)]
    source_root: Option<PathBuf>,
    #[serde(default)]
    sources: Option<Vec<PathBuf>>,
    #[serde(default)]
    include_dirs: Option<Vec<PathBuf>>,
    #[serde(default)]
    defines: Option<Vec<String>>,
    #[serde(default)]
    std: Option<String>,
    #[serde(default)]
    load_path: Option<PathBuf>,
    #[serde(default)]
    providers: Option<Vec<String>>,
    #[serde(default)]
    contract_manifest: Option<PathBuf>,
    #[serde(default)]
    quirin_providers_only: Option<bool>,
    #[serde(default)]
    report: Option<PathBuf>,
    #[serde(default)]
    jobs: Option<usize>,
    /// Named only to be refused with a reason.
    #[serde(default)]
    compile_database: Option<Value>,
    #[serde(default)]
    checkout: Option<Value>,
    #[serde(default)]
    profile_directory: Option<Value>,
    #[serde(default)]
    source_contracts: Option<Value>,
    #[serde(default)]
    packages: Option<Value>,
}

/// One form's settings. Paths are anchored, except `sources`, which stay as
/// spelled until the source root is known.
#[derive(Debug, Default)]
struct Settings {
    source_root: Option<PathBuf>,
    sources: Option<Vec<PathBuf>>,
    include_dirs: Option<Vec<PathBuf>>,
    defines: Option<Vec<String>>,
    std: Option<String>,
    load_path: Option<PathBuf>,
    providers: Option<Vec<String>>,
    contract_manifest: Option<PathBuf>,
    quirin_providers_only: Option<bool>,
    report: Option<PathBuf>,
    jobs: Option<usize>,
}

fn parse_command_line(
    arguments: &[String],
    cwd: &Path,
) -> Result<(Settings, Option<PathBuf>), String> {
    let mut settings = Settings::default();
    let mut config = None;
    let mut providers = Vec::new();
    let mut sources = Vec::new();
    let mut include_dirs = Vec::new();
    let mut defines = Vec::new();
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
            "--source-root" => set_path(&mut settings.source_root, flag, value)?,
            "--load-path" => set_path(&mut settings.load_path, flag, value)?,
            "--contract-manifest" => set_path(&mut settings.contract_manifest, flag, value)?,
            "--report" => set_path(&mut settings.report, flag, value)?,
            "--provider" => providers.push(
                value
                    .ok_or("--provider requires a provider crate name")?
                    .clone(),
            ),
            "-I" => include_dirs.push(anchor(
                cwd,
                PathBuf::from(value.ok_or("-I requires a directory")?),
            )),
            "-D" => defines.push(value.ok_or("-D requires NAME[=VALUE]")?.clone()),
            "--std" => {
                let value = value.ok_or("--std requires a dialect")?;
                if settings.std.replace(value.clone()).is_some() {
                    return Err("--std is given twice".into());
                }
            }
            "--jobs" => {
                let text = value.ok_or("--jobs requires a positive integer")?;
                if settings.jobs.is_some() {
                    return Err("--jobs is given twice".into());
                }
                settings.jobs = Some(crate::parallel::resolve_jobs(Some(text))?);
            }
            "--quirin-providers-only" => {
                settings.quirin_providers_only = Some(true);
                takes_value = false;
            }
            other => {
                takes_value = false;
                if let Some(directory) = other.strip_prefix("-I").filter(|rest| !rest.is_empty()) {
                    include_dirs.push(anchor(cwd, PathBuf::from(directory)));
                } else if let Some(define) =
                    other.strip_prefix("-D").filter(|rest| !rest.is_empty())
                {
                    defines.push(define.to_owned());
                } else if let Some(dialect) = other.strip_prefix("--std=") {
                    if settings.std.replace(dialect.to_owned()).is_some() {
                        return Err("--std is given twice".into());
                    }
                } else if other.starts_with('-') {
                    return Err(format!("unknown argument `{other}`\n{USAGE}"));
                } else {
                    sources.push(PathBuf::from(other));
                }
            }
        }
        index += if takes_value { 2 } else { 1 };
    }
    if !providers.is_empty() {
        settings.providers = Some(providers);
    }
    if !sources.is_empty() {
        settings.sources = Some(sources);
    }
    if !include_dirs.is_empty() {
        settings.include_dirs = Some(include_dirs);
    }
    if !defines.is_empty() {
        settings.defines = Some(defines);
    }
    Ok((settings, config))
}

fn read_config(path: &Path) -> Result<(Settings, Vec<u8>), String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let config: NoBuildConfig =
        serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let at = path.display();
    if config.schema_version != 1 {
        return Err(format!(
            "{at}: unsupported contracts configuration version {}",
            config.schema_version
        ));
    }
    if !config.mode.is_contracts_only() {
        return Err(format!(
            "{at}: `quirin-audit contracts` is contracts-only; `mode` must be `contracts-only`"
        ));
    }
    if config.build != "none" {
        return Err(format!(
            "{at}: `quirin-audit contracts` reads no build; `build` must be `none` (a configured \
             build is `quirin-audit current --compile-database`)"
        ));
    }
    if config.compile_database.is_some()
        || config.checkout.is_some()
        || config.profile_directory.is_some()
    {
        return Err(format!(
            "{at}: a no-build audit reads no compilation database, checkout pin or source \
             profile; `compile_database`, `checkout` and `profile_directory` belong to \
             `quirin-audit current`"
        ));
    }
    if config.source_contracts.is_some() || config.packages.is_some() {
        return Err(format!(
            "{at}: a contracts-only run admits no process packages or source-contract checks"
        ));
    }
    let base = path
        .parent()
        .ok_or("configuration has no parent directory")?;
    let anchor_all = |path: Option<PathBuf>| path.map(|path| anchor(base, path));
    Ok((
        Settings {
            source_root: anchor_all(config.source_root),
            sources: config.sources,
            include_dirs: config
                .include_dirs
                .map(|dirs| dirs.into_iter().map(|dir| anchor(base, dir)).collect()),
            defines: config.defines,
            std: config.std,
            load_path: anchor_all(config.load_path),
            providers: config.providers,
            contract_manifest: anchor_all(config.contract_manifest),
            quirin_providers_only: config.quirin_providers_only,
            report: anchor_all(config.report),
            jobs: config.jobs,
        },
        bytes,
    ))
}

/// The assumed configuration: what the user supplied plus the stated defaults.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssumedConfiguration {
    dialect: CCompilerDialect,
    std: String,
    std_source: &'static str,
    include_dirs: Vec<PathBuf>,
    defines: Vec<String>,
}

fn parse_dialect(name: &str) -> Result<CCompilerDialect, String> {
    // Card CG30-DIALECTS-FLAGS-TYPES (rule 11y): the seven dialect identities.
    CCompilerDialect::from_std_name(name).ok_or_else(|| {
        format!(
            "unsupported dialect `{name}`: the gateway accepts c11, gnu99, gnu11, gnu17, c17, c23 and c2y"
        )
    })
}

impl AssumedConfiguration {
    /// Validate the supplied settings. Include directories must exist (a
    /// misspelt one would otherwise be ignored by Clang without a trace), and
    /// no value may carry an unexpanded `$` or a sysroot-relative `=`.
    pub fn new(
        std: Option<(String, &'static str)>,
        include_dirs: Vec<PathBuf>,
        defines: Vec<String>,
    ) -> Result<Self, String> {
        let (std, std_source) = std.map_or_else(
            || (DEFAULT_STD.to_owned(), "default"),
            |(std, _)| (std, "declared"),
        );
        let dialect = parse_dialect(&std)?;
        let mut canonical = Vec::new();
        for dir in include_dirs {
            let text = dir.to_string_lossy();
            if text.contains('$') || text.starts_with('=') {
                return Err(format!(
                    "include directory `{text}` is unexpanded or sysroot-relative"
                ));
            }
            let resolved = dir
                .canonicalize()
                .map_err(|error| format!("include directory `{text}`: {error}"))?;
            if !resolved.is_dir() {
                return Err(format!("include directory `{text}` is not a directory"));
            }
            canonical.push(resolved);
        }
        for define in &defines {
            let name = define.split('=').next().unwrap_or_default();
            if name.is_empty()
                || define.contains('$')
                || !name
                    .bytes()
                    .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
                || name.as_bytes()[0].is_ascii_digit()
            {
                return Err(format!(
                    "definition `{define}` is not NAME[=VALUE] with a C identifier and no `$`"
                ));
            }
        }
        Ok(Self {
            dialect,
            std,
            std_source,
            include_dirs: canonical,
            defines,
        })
    }

    /// The run-wide record.
    #[must_use]
    pub fn record(&self) -> Value {
        json!({
            "evidence": EVIDENCE,
            "build": "none",
            "std": self.std,
            "std_source": self.std_source,
            "include_dirs": self.include_dirs,
            "defines": self.defines,
            "quote_include": "each unit's own directory (Clang's includer-directory rule for the unit's declared input)",
            "system_headers": "the host Clang's default system include directories (the recorded Clang identity)",
            "discovery": "none: no include directory is searched for, no configuration header is guessed or generated, and no build system is read",
            "header_files": "read as found on disk in the include directories and each unit's own directory, whether tracked by version control or not (a header an earlier in-tree configure left behind is read like any other); header bytes are bound only through each unit's preprocessed-text digest",
        })
    }

    /// One unit's effective settings.
    fn unit_record(&self, directory: &Path) -> Value {
        json!({
            "evidence": EVIDENCE,
            "build": "none",
            "std": self.std,
            "include_dirs": self.include_dirs,
            "defines": self.defines,
            "quote_include_directory": directory,
            "system_headers": "host Clang default",
            "verdict_scope": VERDICT_SCOPE,
        })
    }
}

/// One selected translation unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    /// The selection argument that reached it (its report `library`).
    pub library: String,
    /// Its path relative to the source root, `/`-separated.
    pub file: String,
    /// Its canonical absolute path.
    pub path: PathBuf,
}

fn relative_spelling(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let spelled = crate::repository_path(relative);
    if spelled.is_empty() {
        ".".into()
    } else {
        spelled
    }
}

#[derive(Default)]
struct WalkTally {
    hidden_directories: usize,
    symbolic_links: usize,
}

fn walk(directory: &Path, found: &mut Vec<PathBuf>, tally: &mut WalkTally) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let kind = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", entry.path().display()))?;
        let name = entry.file_name();
        if kind.is_symlink() {
            tally.symbolic_links += 1;
        } else if kind.is_dir() {
            if name.to_string_lossy().starts_with('.') {
                tally.hidden_directories += 1;
            } else {
                walk(&entry.path(), found, tally)?;
            }
        } else if kind.is_file() && Path::new(&name).extension().is_some_and(|ext| ext == "c") {
            found.push(entry.path());
        }
    }
    Ok(())
}

/// Select the units: every named source in order (a directory contributes
/// its walk, sorted), or the whole root; the first selection of a file wins.
pub fn select_units(root: &Path, sources: &[PathBuf]) -> Result<(Vec<Unit>, Value), String> {
    let root = root
        .canonicalize()
        .map_err(|error| format!("source root {}: {error}", root.display()))?;
    if !root.is_dir() {
        return Err(format!("source root {} is not a directory", root.display()));
    }
    let named = if sources.is_empty() {
        vec![root.clone()]
    } else {
        sources
            .iter()
            .map(|source| anchor(&root, source.clone()))
            .collect()
    };
    let mut units = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut duplicates = 0;
    let mut tally = WalkTally::default();
    let mut spelled = Vec::new();
    for source in named {
        let path = source
            .canonicalize()
            .map_err(|error| format!("source {}: {error}", source.display()))?;
        if !path.starts_with(&root) {
            return Err(format!(
                "source {} is outside the source root {}",
                path.display(),
                root.display()
            ));
        }
        let (library, files) = if path.is_dir() {
            let mut files = Vec::new();
            walk(&path, &mut files, &mut tally)?;
            if files.is_empty() {
                return Err(format!(
                    "source directory {} selects no `*.c` unit",
                    path.display()
                ));
            }
            (relative_spelling(&root, &path), files)
        } else if path.is_file() {
            let parent = path.parent().unwrap_or(&root);
            (relative_spelling(&root, parent), vec![path.clone()])
        } else {
            return Err(format!(
                "source {} is not a file or directory",
                path.display()
            ));
        };
        spelled.push(relative_spelling(&root, &path));
        for file in files {
            if seen.insert(file.clone()) {
                units.push(Unit {
                    library: library.clone(),
                    file: relative_spelling(&root, &file),
                    path: file,
                });
            } else {
                duplicates += 1;
            }
        }
    }
    if units.is_empty() {
        return Err("the selection contains no translation unit".into());
    }
    let record = json!({
        "source_root": root,
        "sources": spelled,
        "rule": "explicit files as named; directories walked for regular `*.c` files in sorted order, not entering directories whose name starts with `.` and not following symbolic links; the first selection of a file wins",
        "units": units.len(),
        "duplicates_ignored": duplicates,
        "hidden_directories_skipped": tally.hidden_directories,
        "symbolic_links_skipped": tally.symbolic_links,
    });
    Ok((units, record))
}

fn shell_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\\''"))
}

/// The first `error:` line of a Clang diagnostic stream, or its first line.
fn first_diagnostic(stderr: &str) -> String {
    stderr
        .lines()
        .find(|line| line.contains("error: "))
        .or_else(|| stderr.lines().next())
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// Whether the first error of a preprocessing failure is an `#error` directive:
/// the source excerpt Clang prints beneath it spells the directive.
fn first_error_is_directive(stderr: &str) -> bool {
    let mut lines = stderr.lines().skip_while(|line| !line.contains("error: "));
    lines.next();
    for line in lines {
        if line.contains("error: ") || line.contains("warning: ") || line.contains("note: ") {
            break;
        }
        let excerpt = line.split_once('|').map_or(line, |(_, text)| text).trim();
        if let Some(directive) = excerpt.strip_prefix('#')
            && directive.trim_start().starts_with("error")
        {
            return true;
        }
    }
    false
}

/// The reason code and first diagnostic of a unit that did not prepare under
/// the assumed configuration.
#[must_use]
pub fn classify_preparation(prepared: &PreparedTranslationUnit) -> Option<(&'static str, String)> {
    let error = prepared.validation_error()?;
    let preprocessing =
        prepared.metrics().plain_preprocessing.outcome == PreparationStageOutcome::Failed;
    Some(match error {
        LegacyCError::ProductionParserUnavailable(text) => ("clang-unavailable", text.clone()),
        LegacyCError::ProductionParserRejected(stderr) if preprocessing => {
            let first = first_diagnostic(stderr);
            let code = if first.contains("file not found") {
                "missing-include"
            } else if first_error_is_directive(stderr) {
                "unresolved-configuration"
            } else {
                "preprocessing-error"
            };
            (code, first)
        }
        LegacyCError::ProductionParserRejected(stderr) => {
            ("syntax-error", first_diagnostic(stderr))
        }
        LegacyCError::UnsupportedInputIdentity(text) => {
            ("unsupported-input-identity", text.clone())
        }
        other => ("preprocessing-refused", other.to_string()),
    })
}

/// A no-build audit run: contracts, assumptions and the folded report.
pub struct NoBuildAudit {
    root: PathBuf,
    compiler: ClangIdentity,
    contracts: ResourceContracts,
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the loaded contracts before any
    /// refutation table, and the table the published phase applied.
    loaded_contracts: ResourceContracts,
    refuted: qcore_schemas::contract_refutation::RefutationTable,
    pins: crate::contract_revision::PinEvaluation,
    assumed: AssumedConfiguration,
    dependencies: BTreeMap<String, String>,
    profile_sha256: String,
    sidecar: Value,
    report: AuditReport,
    counts: AuditCounts,
    reasons: BTreeMap<String, usize>,
    files: Vec<Value>,
    /// Card MEMBER-PREPASS-OTHER-ROUTES (rule 12n, R1): the run's member
    /// facts the pre-pass read over the selected units, installed on every
    /// analysing thread, with its evidence section.
    member_prepass: Option<crate::member_prepass::Prepass>,
}

/// One unit's console lines and, unless it failed, its products.
pub struct UnitOutcome {
    pub output: String,
    pub result: Result<UnitProducts, String>,
}

/// What one unit contributes to a [`NoBuildAudit`].
pub struct UnitProducts {
    report: AuditReport,
    counts: AuditCounts,
    reason: Option<&'static str>,
    file: Value,
}

impl UnitProducts {
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: this unit's rows, from which a
    /// phase's refutation table is read.
    #[must_use]
    pub fn unit_rows(&self) -> &[Value] {
        self.report.unit_rows()
    }

    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: this unit's bounds
    /// body evidence, from which a phase's bounds refutation table is read.
    #[must_use]
    pub fn bounds_evidence(&self) -> &[Value] {
        self.report.bounds_evidence()
    }
}

impl NoBuildAudit {
    /// Load the contracts and bind the assumption record as each compile
    /// task's `source_profile` dependency.
    pub fn new(
        root: &Path,
        selection: &Value,
        input: &ContractInput,
        assumed: AssumedConfiguration,
    ) -> Result<Self, String> {
        Self::with_plugin_checkers(root, selection, input, assumed, PluginCheckers::default())
    }

    /// [`Self::new`], with the trusted-core checkers a checker plugin's claim
    /// may be certified by (card W45-USER-CONTRACT-PLUGINS). The command line
    /// passes none.
    pub fn with_plugin_checkers(
        root: &Path,
        selection: &Value,
        input: &ContractInput,
        assumed: AssumedConfiguration,
        checkers: PluginCheckers<'_>,
    ) -> Result<Self, String> {
        let root = root.canonicalize().map_err(|error| error.to_string())?;
        let mut contracts = input.load()?;
        // Card W45-CONTRACT-REVISION-MODE: every pinned contract is compared
        // with its lock over the source root before any unit is analysed.
        let pins = crate::contract_revision::apply_pins(
            &mut contracts,
            &crate::contract_revision::ModuleOwners::for_manifest(
                input.manifest_path(),
                input.manifest_text(),
            )?,
            &root,
        )?;
        // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds environment.
        crate::bounds_selection::select_for_route(
            "contracts",
            input.manifest_path(),
            input.manifest_text(),
            &root,
            &contracts,
        )?;
        // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the pin-derived result
        // sets of the installed bounds environment, read by rule 38 while in force.
        let contracts = {
            let mut contracts = contracts;
            contracts.set_result_ranges(crate::bounds_selection::result_range_table());
            contracts
        };
        // Card OUTER-CONTRACT (rule 31): the outer environment.
        crate::outer_selection::select_for_route(
            "contracts",
            input.manifest_path(),
            input.manifest_text(),
            &root,
        )?;
        let manifest_text = input.manifest_text().to_owned();
        // The no-build "source profile": the selection and the assumed
        // configuration. Every compile-task identity and every certificate's
        // trust context digests it, so a certificate minted under assumed
        // settings is bound to them and to nothing else.
        let profile = json!({
            "evidence": EVIDENCE, "selection": selection,
            "assumed_configuration": assumed.record(),
        });
        let profile_sha256 = sha256_hex(
            serde_json::to_vec(&profile)
                .map_err(|error| error.to_string())?
                .as_slice(),
        );
        let dependencies = BTreeMap::from([
            ("build_settings".into(), sha256_hex(EVIDENCE.as_bytes())),
            ("source_profile".into(), profile_sha256.clone()),
            (
                "resource_contract_manifest".into(),
                sha256_hex(manifest_text.as_bytes()),
            ),
            (
                "resource_contract_sources".into(),
                sha256_hex(
                    serde_json::to_string(&contracts.source_assumptions())
                        .map_err(|error| error.to_string())?
                        .as_bytes(),
                ),
            ),
        ]);
        let mut evidence = crate::evidence::contracts_only_record(&contracts);
        evidence["evidence"] = json!(EVIDENCE);
        evidence["build"] = json!("none");
        evidence["assumed_configuration"] = assumed.record();
        evidence["verdict_scope"] = json!(VERDICT_SCOPE);
        evidence["equivalence_to_build_configured"] = json!(
            "none: no verdict of this run is equivalent to, comparable with or a substitute for a build-configured verdict; certificates bind the assumption record as their source profile"
        );
        let mut sidecar = json!({
            "schema_version": 1,
            "evidence": EVIDENCE,
            "mode": "contracts-only",
            "build": "none",
            "selection": selection,
            "assumed_configuration": assumed.record(),
            "source_profile_sha256": profile_sha256,
            "contract_manifest": manifest_text,
            "contracts": contracts.source_assumptions(),
            "scope": "contracts-only resource projection of each selected unit under the recorded assumed configuration; Unsafe is provisional; declared contracts remain assumptions; a unit that does not preprocess or parse under the assumptions is Unknown with its reason code",
            "publication_rule": VERDICT_SCOPE,
        });
        if let Some(provenance) = qcore_contract::contract_loader::inclusion_provenance(&contracts)
        {
            sidecar["contract_inclusion"] = provenance;
        }
        if let Some(record) = input.provider_record() {
            sidecar["provider_selection"] = record.clone();
        }
        sidecar["evidence_record"] = evidence.clone();
        let mut report = AuditReport::new("none");
        report.set_evidence(evidence);
        // Card W45-USER-CONTRACT-PLUGINS: every checker binding runs once,
        // before any unit, and every unit folds its links.
        if let Some(plugins) = crate::user_plugins::run(input.extension(), checkers) {
            report.set_plugin_links(plugins.links);
            sidecar["plugin_checks"] = plugins.section;
        }
        Ok(Self {
            root,
            compiler: ClangIdentity::capture()?,
            loaded_contracts: contracts.clone(),
            refuted: qcore_schemas::contract_refutation::RefutationTable::new(),
            contracts,
            pins,
            assumed,
            dependencies,
            profile_sha256,
            sidecar,
            report,
            counts: AuditCounts::default(),
            reasons: BTreeMap::new(),
            files: Vec::new(),
            member_prepass: None,
        })
    }

    /// Card MEMBER-PREPASS-OTHER-ROUTES (rule 12n; user decisions R1, F1):
    /// read the member facts of every selected unit, under the compile
    /// record it is analysed as, before any unit is analysed, so each unit
    /// decides a member invocation over the run's set. The run is the
    /// selection: no build names another unit, and the route has no profile,
    /// so no unit is excluded. Nothing runs when no declaration invokes a
    /// member.
    ///
    /// # Errors
    /// A unit whose compile record cannot be formed, or a hand-off rejection.
    pub fn run_member_prepass(&mut self, units: &[Unit], jobs: usize) -> Result<(), String> {
        let entries = units
            .iter()
            .map(|unit| self.entry(unit))
            .collect::<Result<Vec<_>, _>>()?;
        // Card OUTER-CONTRACT (rule 31): what each loader's binders reach.
        crate::outer_selection::compute_reach(&entries)?;
        if let Some(prepass) = crate::member_prepass::run(
            &entries,
            &self.contracts,
            &crate::member_prepass::Scope::default(),
            Some(&self.root),
            jobs,
        ) {
            crate::member_prepass::announce(&prepass, &self.contracts)?;
            self.member_prepass = Some(prepass);
        }
        Ok(())
    }

    /// The compile record this unit is analysed as: the host Clang, the
    /// assumed dialect, include directories and definitions, `-c` and the
    /// unit's own file name, run from its own directory.
    fn entry(&self, unit: &Unit) -> Result<CompileEntry, String> {
        let directory = unit
            .path
            .parent()
            .ok_or("unit has no parent directory")?
            .to_path_buf();
        let name = unit
            .path
            .file_name()
            .ok_or("unit has no file name")?
            .to_string_lossy()
            .into_owned();
        let mut words = vec![
            self.compiler.program().to_owned(),
            format!("-std={}", self.assumed.std),
        ];
        words.extend(
            self.assumed
                .include_dirs
                .iter()
                .map(|dir| format!("-I{}", dir.display())),
        );
        words.extend(
            self.assumed
                .defines
                .iter()
                .map(|define| format!("-D{define}")),
        );
        words.extend(["-c".to_owned(), name]);
        Ok(CompileEntry {
            directory,
            command: words
                .iter()
                .map(|word| shell_quote(word))
                .collect::<Vec<_>>()
                .join(" "),
            file: unit.path.clone(),
        })
    }

    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: analyse every later unit under
    /// the loaded contracts with `table` recorded (the empty table restores
    /// them as loaded).
    pub fn set_refuted_table(
        &mut self,
        table: &qcore_schemas::contract_refutation::RefutationTable,
    ) {
        self.contracts = crate::refuted_contracts::with_table(&self.loaded_contracts, table);
        self.refuted = table.clone();
    }

    /// Analyse one unit without touching the accumulated state.
    #[must_use]
    pub fn evaluate(&self, unit: &Unit) -> UnitOutcome {
        let mut output = String::new();
        // Card MEMBER-PREPASS-OTHER-ROUTES (R1): analysed with the run's
        // member facts installed, when the pre-pass ran.
        let facts = self
            .member_prepass
            .as_ref()
            .map(|prepass| prepass.facts.clone());
        let result = qcore_c_gateway::member_run_facts::scoped(facts, || {
            self.evaluate_into(unit, &mut output)
        });
        UnitOutcome { output, result }
    }

    fn evaluate_into(&self, unit: &Unit, output: &mut String) -> Result<UnitProducts, String> {
        use std::fmt::Write as _;
        let mut report = self.report.fragment();
        let mut counts = AuditCounts::default();
        let library = unit.library.as_str();
        let file = unit.file.as_str();
        let entry = self.entry(unit)?;
        let bytes =
            fs::read(&unit.path).map_err(|error| UnitRefusal::environment(error.to_string()));
        let raw_sha256 = bytes.as_ref().ok().map(|bytes| sha256_hex(bytes));
        let mut configuration = json!({
            "file": unit.path, "directory": entry.directory, "command": entry.command,
            "build": "none", "raw_source_sha256": raw_sha256,
            "compiler": self.compiler, "dependencies": self.dependencies,
        });
        let source = bytes
            .as_ref()
            .ok()
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        let functions = source.map(legacy_c_source_functions).unwrap_or_default();
        let translation_unit =
            (|| -> Result<C11TranslationUnit<'_>, (UnitRefusal, &'static str)> {
                bytes
                    .as_ref()
                    .map_err(|refusal| (refusal.clone(), "unreadable-source"))?;
                let source = source
                    .ok_or_else(|| (UnitRefusal::classified("source is not UTF-8"), "not-utf8"))?;
                let options = compiler_options(&entry)
                    .map_err(|error| (UnitRefusal::classified(error), "unsupported-setting"))?;
                Ok(C11TranslationUnit {
                    source,
                    include_paths: options.include_paths,
                    definitions: options.definitions,
                    preprocessor_options: options.preprocessor_options,
                    dialect: options.dialect,
                    function_scope: LegacyFunctionScope::MainSourceDefinitions,
                    input: options.input,
                })
            })();
        let refused = |refusal: UnitRefusal,
                       code: &'static str,
                       counts: &mut AuditCounts,
                       report: &mut AuditReport,
                       output: &mut String| {
            counts.units += 1;
            counts.unknown_units += 1;
            counts.source_functions += functions.len();
            report.record_unknown(library, file, &functions, &refusal);
            let _ = writeln!(output, "  {library}/{file}: Unknown: {}", refusal.reason());
            Some(code)
        };
        let mut first = None;
        let reason = match translation_unit {
            Ok(translation_unit) => {
                let prepared = prepare_translation_unit(&translation_unit);
                let refusal = classify_preparation(&prepared);
                let identity = compile_task_identity(
                    &entry,
                    &translation_unit,
                    match (prepared.preprocessed_source(), &refusal) {
                        (Ok(source), _) => EffectiveInput::Accepted(source),
                        (Err(_), Some((_, diagnostic))) => EffectiveInput::Rejected(diagnostic),
                        (Err(_), None) => unreachable!("a rejected preparation is classified"),
                    },
                    &self.compiler,
                    self.dependencies.clone(),
                )?;
                configuration["compile_task_identity"] =
                    serde_json::to_value(&identity).map_err(|error| error.to_string())?;
                if let Some((code, diagnostic)) = refusal {
                    let metrics = prepared.metrics();
                    counts.prepared_units += 1;
                    counts.preparation_clang_invocations +=
                        usize::from(metrics.clang_invocations());
                    counts.plain_preprocessing_elapsed_us += metrics.plain_preprocessing.elapsed_us;
                    counts.syntax_validation_elapsed_us += metrics.syntax_validation.elapsed_us;
                    counts.line_marked_preprocessing_elapsed_us +=
                        metrics.line_marked_preprocessing.elapsed_us;
                    let text = format!(
                        "no-build preparation refused ({code}) under the recorded assumptions: {diagnostic}"
                    );
                    first = Some(diagnostic);
                    refused(
                        UnitRefusal::environment(text),
                        code,
                        &mut counts,
                        &mut report,
                        output,
                    )
                } else {
                    project_prepared_unit_into(
                        library,
                        file,
                        &prepared,
                        &self.contracts,
                        PreparedAuditCertificateContext {
                            compile_task_identity: &identity,
                            source_profile_sha256: &self.profile_sha256,
                        },
                        &mut report,
                        &mut counts,
                        output,
                    )?;
                    None
                }
            }
            Err((refusal, code)) => {
                first = Some(refusal.reason().to_owned());
                refused(refusal, code, &mut counts, &mut report, output)
            }
        };
        let digest = sha256_hex(
            serde_json::to_vec(&configuration)
                .map_err(|error| error.to_string())?
                .as_slice(),
        );
        configuration["inventory_sha256"] = json!(digest);
        report.record_configuration(configuration);
        let directory = unit.path.parent().unwrap_or(&self.root);
        let mut assumptions = self.assumed.unit_record(directory);
        assumptions["outcome"] = json!(if reason.is_some() {
            "not-prepared"
        } else {
            "analysed"
        });
        assumptions["reason"] = json!(reason);
        assumptions["first_diagnostic"] = json!(first);
        report.record_assumptions(assumptions.clone());
        let mut file_record = json!({
            "library": library, "file": file, "raw_source_sha256": raw_sha256,
        });
        for (key, value) in assumptions.as_object().into_iter().flatten() {
            if key != "verdict_scope" {
                file_record[key] = value.clone();
            }
        }
        Ok(UnitProducts {
            report,
            counts,
            reason,
            file: file_record,
        })
    }

    /// Fold one [`Self::evaluate`] result into the audit, in unit order.
    pub fn absorb(&mut self, products: UnitProducts) -> Result<(), String> {
        let UnitProducts {
            report,
            counts,
            reason,
            file,
        } = products;
        self.report.absorb(report)?;
        self.counts.absorb(counts);
        if let Some(reason) = reason {
            *self.reasons.entry(reason.to_owned()).or_default() += 1;
        }
        self.files.push(file);
        Ok(())
    }

    /// Print the tallies and write the report and its assumption sidecar.
    pub fn finish(mut self, path: Option<&Path>) -> Result<AuditCounts, String> {
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the run's
        // closure of every invoked member, folded over the rows before
        // anything is published; it only demotes. Card
        // MEMBER-PREPASS-OTHER-ROUTES: over the pre-pass's facts, when it ran.
        if let Some(prepass) = self.member_prepass.take() {
            self.report
                .set_member_prepass(prepass.facts, prepass.section);
        }
        let (members, from_safe, from_pending) = self.report.fold_callback_members(&self.contracts);
        self.counts.safe_units -= from_safe;
        self.counts.pending_units -= from_pending;
        self.counts.unknown_units += from_safe + from_pending;
        if !members.is_null() {
            println!(
                "callback members: {} demoted",
                members["demoted"].as_array().map_or(0, Vec::len)
            );
            self.sidecar[crate::callback_member_fold::SECTION_KEY] = members;
        }
        let not_prepared: usize = self.reasons.values().sum();
        println!(
            "no-build: {} unit(s), {} prepared under the assumptions, {} not prepared: {} Safe / {} Pending / {} provisional Unsafe / {} Unknown / {} data-only / {} gateway failures (every verdict relative to the recorded assumptions)",
            self.counts.units,
            self.counts.units - not_prepared,
            not_prepared,
            self.counts.safe_units,
            self.counts.pending_units,
            self.counts.unsafe_units,
            self.counts.unknown_units,
            self.counts.data_only_units,
            self.counts.gateway_failures
        );
        if !self.reasons.is_empty() {
            println!(
                "not prepared: {}",
                self.reasons
                    .iter()
                    .map(|(reason, count)| format!("{reason}={count}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!(
            "preparation: {} unit(s), {} Clang invocation(s); -E -P {} us / syntax {} us / line markers {} us",
            self.counts.prepared_units,
            self.counts.preparation_clang_invocations,
            self.counts.plain_preprocessing_elapsed_us,
            self.counts.syntax_validation_elapsed_us,
            self.counts.line_marked_preprocessing_elapsed_us,
        );
        // Card W45-CONTRACT-DEFERRED-REVIEW: pending reviews, tallied and
        // published beside the contract sources they are read from.
        let pending = crate::pending_review::section(&self.contracts, &self.report);
        println!("{}", crate::pending_review::tally(&pending));
        self.sidecar["pending_human_review"] = pending;
        // Card W45-CONTRACT-REVISION-MODE: revision modes, pin mismatches and
        // flexible assumptions, tallied and published the same way.
        let revision = crate::contract_revision::section(&self.contracts, &self.pins, &self.report);
        println!("{}", crate::contract_revision::tally(&revision));
        // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13h).
        let premises = crate::contract_dependencies::premises(&self.contracts)?;
        let dependencies =
            crate::contract_dependencies::section(&revision, &premises, &self.report);
        println!("{}", crate::contract_dependencies::tally(&dependencies));
        self.sidecar[crate::contract_dependencies::SECTION_KEY] = dependencies;
        self.sidecar["contract_revision"] = revision;
        // Card W45-REFUTED-CONTRACT-WITHDRAWAL: the refutation table, its
        // digest and the calls it held, tallied and published the same way.
        let refuted = crate::refuted_contracts::section(&self.refuted, &self.report);
        println!("{}", crate::refuted_contracts::tally(&refuted));
        self.sidecar["refuted_contracts"] = refuted;
        // Card W45-USER-CONTRACT-PLUGINS: printed only when a checker ran.
        if self.sidecar["plugin_checks"].is_object() {
            println!(
                "{}",
                crate::user_plugins::tally(&self.sidecar["plugin_checks"])
            );
        }
        if let Some(path) = path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            self.sidecar["outcomes"] = json!({
                "units": self.counts.units,
                "prepared": self.counts.units - not_prepared,
                "not_prepared": self.reasons,
                "safe": self.counts.safe_units,
                "pending": self.counts.pending_units,
                "provisional_unsafe": self.counts.unsafe_units,
                "unknown": self.counts.unknown_units,
                "data_only": self.counts.data_only_units,
                "gateway_failures": self.counts.gateway_failures,
            });
            self.sidecar["files"] = json!(self.files);
            // Card AUDIT-COMPOSITION: the contract bundle the components rest on.
            self.report.record_contract_bundle(&self.contracts);
            self.report.write(path)?;
            // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds rows, when the
            // property is selected (nothing otherwise).
            self.report.write_bounds_selection()?;
            self.report.write_loop_measure_selection()?;
            let null_rows = self.report.write_null_selection()?;
            self.report
                .write_outer_projection(&self.contracts, null_rows.as_deref())?;
            let mut bytes =
                serde_json::to_vec_pretty(&self.sidecar).map_err(|error| error.to_string())?;
            bytes.push(b'\n');
            fs::write(path.with_extension("contracts.json"), bytes)
                .map_err(|error| error.to_string())?;
        }
        Ok(self.counts)
    }
}

/// Parse, resolve and run. Returns the process exit code.
pub fn main(arguments: Vec<String>) -> ExitCode {
    match run(&arguments) {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("contracts-only no-build audit rejected (Unknown): {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[String]) -> Result<bool, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let (command_line, config_path) = parse_command_line(arguments, &cwd)?;
    let (configuration, config_bytes) = match &config_path {
        Some(path) => {
            let (settings, bytes) = read_config(path)?;
            (settings, Some(bytes))
        }
        None => (Settings::default(), None),
    };
    let source_root = one(
        "source_root",
        command_line.source_root,
        configuration.source_root,
    )?
    .unwrap_or_else(|| cwd.clone());
    let root = source_root
        .canonicalize()
        .map_err(|error| format!("source root {}: {error}", source_root.display()))?;
    let anchor_sources = |sources: Option<Vec<PathBuf>>| {
        sources.map(|sources| {
            sources
                .into_iter()
                .map(|source| anchor(&root, source))
                .collect::<Vec<_>>()
        })
    };
    let sources = one(
        "sources",
        anchor_sources(command_line.sources),
        anchor_sources(configuration.sources),
    )?
    .unwrap_or_default();
    let include_dirs = one(
        "include_dirs",
        command_line.include_dirs,
        configuration.include_dirs,
    )?
    .unwrap_or_default();
    let defines = one("defines", command_line.defines, configuration.defines)?.unwrap_or_default();
    let std = match (command_line.std, configuration.std) {
        (Some(left), Some(right)) if left != right => {
            return Err(format!(
                "`std` conflicts: the command line gives {left:?} and the configuration gives \
                 {right:?}"
            ));
        }
        (Some(value), Some(_)) => Some((value, "command_line+configuration")),
        (Some(value), None) => Some((value, "command_line")),
        (None, Some(value)) => Some((value, "configuration")),
        (None, None) => None,
    };
    // Which form declared the dialect is invocation provenance; the report and
    // the sidecar record only whether it is the stated default, so a run is
    // described identically by either form.
    let std_form = std.as_ref().map_or("default", |(_, form)| *form);
    let assumed = AssumedConfiguration::new(std, include_dirs, defines)?;
    let report = one("report", command_line.report, configuration.report)?;
    // Card RULE-SWITCH-ENTRY-POINTS: without a report no component records
    // `rule_set.off`, so the counts alone must not run under a switch list.
    if report.is_none() {
        qcore_c_gateway::gateway_rules::refuse_unrecorded(
            "quirin-audit contracts without a report",
        )?;
    }
    let contract_manifest = one(
        "contract_manifest",
        command_line.contract_manifest,
        configuration.contract_manifest,
    )?;
    let declared_load_path = one("load_path", command_line.load_path, configuration.load_path)?;
    let (load_path, load_path_source) = match declared_load_path {
        Some(path) => (Some(path), "declared"),
        None if contract_manifest.is_none() => {
            let path = cwd.join(DEFAULT_LOAD_PATH);
            if !path.is_file() {
                return Err(format!(
                    "no contract load path: give --load-path (the default {DEFAULT_LOAD_PATH} is \
                     read from the working directory, and {} has none), or --contract-manifest",
                    cwd.display()
                ));
            }
            (Some(path), "default")
        }
        None => (None, "none"),
    };
    let quirin_providers_only = one(
        "quirin_providers_only",
        command_line.quirin_providers_only,
        configuration.quirin_providers_only,
    )?
    .unwrap_or(false);
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
    let effective_jobs = crate::parallel::effective_jobs(jobs);
    let (contracts, contract_source) = choose_contracts(&ContractDeclarations {
        command_line: command_line.providers.as_ref(),
        configuration: configuration.providers.as_ref(),
        load_path: load_path.as_deref(),
        contract_manifest: contract_manifest.as_deref(),
        mode: EvidenceMode::ContractsOnly,
        quirin_providers_only,
        profile_manifest: None,
    })?;
    let (units, selection) = select_units(&root, &sources)?;
    let failures = run_units(
        &root,
        &units,
        &selection,
        &contracts,
        assumed.clone(),
        effective_jobs,
        report.as_deref(),
    )?;
    if let Some(report) = &report {
        let argv = ["quirin-audit".to_owned(), "contracts".to_owned()]
            .into_iter()
            .chain(arguments.iter().cloned())
            .collect::<Vec<_>>();
        let invocation = json!({
            "schema_version": 1,
            "tool": "quirin-audit contracts",
            "tool_version": env!("CARGO_PKG_VERSION"),
            "build_profile": if cfg!(debug_assertions) { "dev" } else { "release" },
            "argv": argv,
            "cwd": cwd,
            "configuration": config_path,
            "configuration_sha256": config_bytes.as_deref().map(sha256_hex),
            "evidence": EVIDENCE,
            "mode": "contracts-only",
            "build": "none",
            "source_root": root,
            "selection": selection,
            "assumed_configuration": assumed.record(),
            "std_form": std_form,
            "contracts": {
                "source": contract_source,
                "selection": contracts.provider_record().map(|record| record["selection"].clone()),
                "load_path": load_path,
                "load_path_source": load_path_source,
                "manifest": contracts.manifest_path(),
                "manifest_sha256": sha256_hex(contracts.manifest_text().as_bytes()),
            },
            "jobs": {"requested": jobs, "effective": effective_jobs, "source": jobs_source},
            "environment": {
                "QCORE_CLANG": std::env::var("QCORE_CLANG").ok(),
                "QCORE_AUDIT_JOBS": std::env::var(crate::parallel::JOBS_VARIABLE).ok(),
            },
            "clang": ClangIdentity::capture()?,
            "gateway_failures": failures,
            "scope": "invocation provenance of this run; not an input to any verdict. No compilation database, build or source profile was read; every verdict is relative to the recorded assumed configuration and is not equivalent to a build-configured verdict",
        });
        crate::current_route::write_invocation(report, &invocation)?;
    }
    Ok(failures)
}

/// Run the selected units on `jobs` threads, folded in unit order; `Ok(true)`
/// when any unit hit a gateway failure.
pub fn run_units(
    root: &Path,
    units: &[Unit],
    selection: &Value,
    contracts: &ContractInput,
    assumed: AssumedConfiguration,
    jobs: usize,
    report: Option<&Path>,
) -> Result<bool, String> {
    let mut audit = NoBuildAudit::new(root, selection, contracts, assumed)?;
    println!(
        "contracts-only, no build: {} ({} unit(s); no compilation database, build or source profile)",
        audit.root.display(),
        units.len()
    );
    println!(
        "evidence: {EVIDENCE} (resource projection from the selected contract modules; every verdict relative to the recorded assumed configuration: -std={}, {} include dir(s), {} definition(s), each unit's own directory for quoted includes, the host Clang's system headers)",
        audit.assumed.std,
        audit.assumed.include_dirs.len(),
        audit.assumed.defines.len()
    );
    audit.run_member_prepass(units, jobs)?;
    let dispatch = crate::parallel::heaviest_first(
        &units
            .iter()
            .map(|unit| fs::metadata(&unit.path).map_or(0, |metadata| metadata.len()))
            .collect::<Vec<_>>(),
    );
    let mut printed: Option<String> = None;
    let mut counts_per_library = BTreeMap::<&str, usize>::new();
    for unit in units {
        *counts_per_library.entry(unit.library.as_str()).or_default() += 1;
    }
    // Card W45-REFUTED-CONTRACT-WITHDRAWAL: phase one under the loaded
    // contracts and, when a body refutes its own declaration, phase two under
    // the refutation table; only the published phase is folded, in unit order.
    let phases = crate::refuted_contracts::run_two_phases_with_bounds(
        |table| {
            audit.set_refuted_table(table);
            let mut outcomes = Vec::new();
            let stopped = run_ordered(
                units.iter().collect::<Vec<_>>(),
                jobs,
                Some(dispatch.clone()),
                |unit| (unit, audit.evaluate(unit)),
                |(unit, outcome): (&Unit, UnitOutcome)| {
                    let failed = outcome.result.is_err();
                    outcomes.push((unit, outcome));
                    if failed { Err(()) } else { Ok(()) }
                },
            )
            .err();
            (outcomes, stopped)
        },
        |outcomes| {
            crate::refuted_contracts::table_from_rows(
                outcomes
                    .iter()
                    .filter_map(|(_, outcome)| outcome.result.as_ref().ok())
                    .flat_map(|products| products.unit_rows()),
            )
        },
        // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (rule 14h).
        |outcomes| {
            crate::bounds_evidence::refutation_table(
                &outcomes
                    .iter()
                    .filter_map(|(_, outcome)| outcome.result.as_ref().ok())
                    .flat_map(|products| products.bounds_evidence().iter().cloned())
                    .collect::<Vec<_>>(),
            )
        },
    );
    audit.set_refuted_table(&phases.applied);
    for (unit, outcome) in phases.results {
        if printed.as_deref() != Some(unit.library.as_str()) {
            println!(
                "{}: {} selected unit(s)",
                unit.library,
                counts_per_library[unit.library.as_str()]
            );
            printed = Some(unit.library.clone());
        }
        print!("{}", outcome.output);
        audit.absorb(outcome.result?)?;
    }
    let counts = audit.finish(report)?;
    crate::contract_dependencies::clear_premises_cache();
    Ok(counts.gateway_failures != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn flags_joined_and_separate_and_positional_sources() {
        let (settings, config) = parse_command_line(
            &args(&[
                "--source-root",
                "tree",
                "-I",
                "inc",
                "-Iother",
                "-D",
                "A=1",
                "-DB",
                "--std=gnu11",
                "lib/a",
                "src/b.c",
                "--jobs",
                "2",
                "--config",
                "/abs/c.json",
            ]),
            Path::new("/work"),
        )
        .unwrap();
        assert_eq!(settings.source_root, Some(PathBuf::from("/work/tree")));
        assert_eq!(
            settings.include_dirs,
            Some(vec![
                PathBuf::from("/work/inc"),
                PathBuf::from("/work/other")
            ])
        );
        assert_eq!(settings.defines, Some(args(&["A=1", "B"])));
        assert_eq!(settings.std.as_deref(), Some("gnu11"));
        assert_eq!(
            settings.sources,
            Some(vec![PathBuf::from("lib/a"), PathBuf::from("src/b.c")])
        );
        assert_eq!(settings.jobs, Some(2));
        assert_eq!(config, Some(PathBuf::from("/abs/c.json")));
    }

    #[test]
    fn unknown_flags_repeats_and_bad_settings_refuse() {
        let cwd = Path::new("/work");
        assert!(parse_command_line(&args(&["--compile-database", "x"]), cwd).is_err());
        assert!(parse_command_line(&args(&["--std", "c11", "--std", "gnu17"]), cwd).is_err());
        assert!(parse_command_line(&args(&["-I"]), cwd).is_err());
        assert!(
            AssumedConfiguration::new(Some(("c99".into(), "command_line")), vec![], vec![])
                .is_err()
        );
        assert!(
            AssumedConfiguration::new(None, vec![PathBuf::from("/no/such/dir")], vec![]).is_err()
        );
        assert!(AssumedConfiguration::new(None, vec![], vec!["1X".into()]).is_err());
        assert!(AssumedConfiguration::new(None, vec![], vec!["X=$(Y)".into()]).is_err());
        let default = AssumedConfiguration::new(None, vec![], vec!["X=1".into()]).unwrap();
        assert_eq!(default.record()["std"], "gnu17");
        assert_eq!(default.record()["std_source"], "default");
    }

    #[test]
    fn preparation_failures_are_classified_from_the_first_error() {
        let missing = "In file included from a.c:1:\nb.h:2:10: fatal error: 'cfg.h' file not found\n    2 | #include \"cfg.h\"\n      |          ^~~~~~~\n1 error generated.";
        assert_eq!(
            first_diagnostic(missing),
            "b.h:2:10: fatal error: 'cfg.h' file not found"
        );
        assert!(!first_error_is_directive(missing));
        let directive = "a.c:3:2: error: \"configure first\"\n    3 | #  error \"configure first\"\n      |    ^\n1 error generated.";
        assert!(first_error_is_directive(directive));
        let other =
            "a.c:3:2: error: unterminated conditional directive\n    3 | #if X\n      |  ^\n";
        assert!(!first_error_is_directive(other));
    }

    #[test]
    fn quoting_round_trips_through_the_compile_record_reader() {
        let words = ["clang", "-DNAME=\"a b\"", "-I/x/it's", "-c", "u.c"];
        let command = words
            .iter()
            .map(|word| shell_quote(word))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(crate::compile_db::shell_words(&command).unwrap(), words);
    }
}
