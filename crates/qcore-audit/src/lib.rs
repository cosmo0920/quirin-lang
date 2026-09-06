//! Provider-independent, configured existing-C resource audits (L3).
//!
//! Pins, selected paths and contracts are declared inputs; no provider module
//! is linked. Safe/Unsafe/Unknown remain the gateway's bounded projection
//! verdicts. Configured process source-contract checks accept only sealed
//! gateway imports and trusted witness replay. Specialized native observation
//! composition remains with the existing compatibility adapter.

pub mod compile_db;
pub mod projection;
pub mod report;
pub mod source_contract;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, legacy_c_source_functions,
};
use qcore_contract::contract_loader::load_resource_contract_manifest;
use qcore_contract::profile_loader::{
    LoadedProfile, load_audited_bundle, load_profile_directory, verify_pin,
};
use qcore_schemas::profile::{LibraryProfile, UnitSelection, WitnessCheck};
use serde::Deserialize;

use compile_db::{CompileEntry, compile_entries, compiler_options};
use projection::{AuditCounts, project_unit};
use report::AuditReport;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditConfig {
    pub schema_version: u32,
    pub profile_directory: PathBuf,
    pub checkout: PathBuf,
    pub compile_database: PathBuf,
    pub libraries: Vec<String>,
    pub report: PathBuf,
}

/// An explicit profile determines every selected source path. Conflicting
/// commands for a selected file reject, rather than arbitrarily choosing one.
pub fn select_profile_units(
    entries: &[CompileEntry],
    checkout: &Path,
    library: &LibraryProfile,
) -> Result<Vec<CompileEntry>, String> {
    let mut selected = BTreeMap::<PathBuf, CompileEntry>::new();
    for root in &library.source_roots {
        let source_root = checkout.join(&root.root);
        for entry in entries {
            let matches = match &root.selection {
                UnitSelection::TopLevel => entry.file.parent() == Some(source_root.as_path()),
                UnitSelection::Recursive => entry.file.starts_with(&source_root),
                UnitSelection::Exact { file } => entry.file == checkout.join(file),
            } && entry
                .file
                .extension()
                .is_some_and(|extension| extension == "c");
            if !matches {
                continue;
            }
            if let Some(prior) = selected.get(&entry.file)
                && (prior.command != entry.command || prior.directory != entry.directory)
            {
                return Err(format!(
                    "conflicting compilation commands for {}",
                    entry.file.display()
                ));
            }
            selected.insert(entry.file.clone(), entry.clone());
        }
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

pub fn run_config(path: &Path) -> Result<AuditCounts, String> {
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
    let contract_manifest = profile
        .resource_contract_manifest_path()
        .ok_or("source-set profile supplies no resource contracts")?;
    let contracts = load_resource_contract_manifest(&contract_manifest)?;
    let mut work = Vec::new();
    let mut revisions = BTreeSet::new();
    for name in &config.libraries {
        revisions.insert(verify_library_sources(&profile, name, &checkout)?);
        let library = profile.library(name).ok_or("selected library is absent")?;
        if !library.required_evidence.is_empty() {
            return Err(format!(
                "library `{name}` requires evidence that this resource-only driver cannot supply"
            ));
        }
        let selected = select_profile_units(&entries, &checkout, library)?;
        if selected.is_empty() {
            return Err(format!("no compiled C sources selected for `{name}`"));
        }
        // Validate every command before publishing even a partial run.
        for entry in &selected {
            compiler_options(entry)?;
        }
        work.push((name, selected));
    }
    let revision = revisions.into_iter().collect::<Vec<_>>().join(",");
    let mut report = AuditReport::new(&revision);
    let mut counts = AuditCounts::default();
    let mut source_identities = BTreeMap::new();
    for (library, entries) in work {
        for entry in entries {
            let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
            let file = entry
                .file
                .strip_prefix(&checkout)
                .map_err(|_| "source outside selected checkout")?
                .to_string_lossy();
            let pin = profile
                .pin_for(library)
                .ok_or("selected source has no pin")?;
            let pinned = Command::new("git")
                .arg("-C")
                .arg(&checkout)
                .args(["show", &format!("{}:{}", pin.git_commit, file)])
                .output()
                .map_err(|error| error.to_string())?;
            if !pinned.status.success() || pinned.stdout != source.as_bytes() {
                return Err(format!(
                    "selected source `{file}` does not match its pinned bytes"
                ));
            }
            source_identities.insert(
                format!("{library}/{file}"),
                qcore_contract::profile_loader::sha256_hex(source.as_bytes()),
            );
            let options = compiler_options(&entry)?;
            let unit = C11TranslationUnit {
                source: &source,
                include_paths: options.include_paths,
                definitions: options.definitions,
                dialect: options.dialect,
                function_scope: LegacyFunctionScope::MainSourceDefinitions,
            };
            project_unit(
                library,
                &file,
                &unit,
                &legacy_c_source_functions(&source),
                &contracts,
                &mut report,
                &mut counts,
            )?;
        }
    }
    let report_path = resolve(base, &config.report);
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    report.write(&report_path)?;
    let assumptions = serde_json::json!({
        "schema_version": 1, "profile": profile.assumptions(),
        "contracts": contracts.source_assumptions(),
        "source_sha256": source_identities,
        "contract_manifest": fs::read_to_string(&contract_manifest).map_err(|error| error.to_string())?,
        "compile_database_sha256": qcore_contract::profile_loader::sha256_hex(database.as_bytes()),
        "scope": "existing-C resource projection of selected compiled units; no observation or provider composition; Unsafe is provisional",
    });
    fs::write(
        report_path.with_extension("assumptions.json"),
        serde_json::to_vec_pretty(&assumptions).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(counts)
}

/// Compatibility adapters can select named profile libraries and preserve
/// historical report labels without carrying a source-path table in Rust.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibrarySelection {
    pub library: String,
    pub label: Option<String>,
}

pub fn load_library_selections(path: &Path) -> Result<Vec<LibrarySelection>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SelectionFile {
        schema_version: u32,
        libraries: Vec<LibrarySelection>,
    }
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
    Ok(selection.libraries)
}
