//! Provider-independent, configured existing-C resource audits (L3).
//!
//! Pins, selected paths and contracts are declared inputs; no provider module
//! is linked. Safe/Unsafe/Unknown remain the gateway's bounded projection
//! verdicts. Configured process source-contract checks accept only sealed
//! gateway imports and trusted witness replay. Specialized native observation
//! composition remains with the existing compatibility adapter.

pub mod compile_db;
pub mod current;
pub mod projection;
pub mod report;
mod source_audit;
pub mod source_contract;
pub use source_audit::{SourceContractAuditConfig, SourceContractLibrary};

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
use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub source_revision_policy: SourceRevisionPolicy,
    #[serde(default)]
    pub source_contracts: Option<SourceContractAuditConfig>,
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
    let mut selected = BTreeMap::<PathBuf, CompileEntry>::new();
    for root in &library.source_roots {
        let source_root = checkout
            .join(&root.root)
            .canonicalize()
            .unwrap_or_else(|_| checkout.join(&root.root));
        for entry in entries {
            let entry_file = entry
                .file
                .canonicalize()
                .unwrap_or_else(|_| entry.file.clone());
            let matches = match &root.selection {
                UnitSelection::TopLevel => entry_file.parent() == Some(source_root.as_path()),
                UnitSelection::Recursive => entry_file.starts_with(&source_root),
                UnitSelection::Exact { file } => entry_file == checkout.join(file),
            } && entry_file
                .extension()
                .is_some_and(|extension| extension == "c");
            if !matches {
                continue;
            }
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

fn verify_pinned_bytes(
    checkout: &Path,
    revision: &str,
    path: &Path,
    source: &[u8],
) -> Result<(), String> {
    let canonical = path.canonicalize().map_err(|error| error.to_string())?;
    if !canonical.starts_with(checkout) {
        return Err("source outside selected checkout".into());
    }
    // Keep the selected path's Git blob, rather than letting an unpinned
    // symlink borrow the bytes of a different pinned file.
    let file = canonical
        .strip_prefix(checkout)
        .map_err(|_| "source outside selected checkout")?;
    let git_file = repository_path(file);
    let pinned = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["show", &format!("{revision}:{git_file}")])
        .output()
        .map_err(|error| error.to_string())?;
    if !pinned.status.success() || pinned.stdout != source {
        return Err(format!(
            "selected source `{}` does not match its pinned bytes",
            file.display()
        ));
    }
    Ok(())
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
    let contract_manifest = profile
        .resource_contract_manifest_path()
        .ok_or("source-set profile supplies no resource contracts")?;
    let contracts = load_resource_contract_manifest(&contract_manifest)?;
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
    let contract_manifest_text =
        fs::read_to_string(&contract_manifest).map_err(|error| error.to_string())?;
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
    let mut counts = AuditCounts::default();
    let mut source_identities = BTreeMap::new();
    let mut compile_tasks = Vec::new();
    for (library, entries) in work {
        for entry in entries {
            let source = fs::read_to_string(&entry.file).map_err(|error| error.to_string())?;
            let canonical_file = entry
                .file
                .canonicalize()
                .map_err(|error| error.to_string())?;
            let file = canonical_file
                .strip_prefix(&checkout)
                .map_err(|_| "source outside selected checkout")?;
            let file = repository_path(file);
            let pin = profile
                .pin_for(library)
                .ok_or("selected source has no pin")?;
            verify_pinned_bytes(
                &checkout,
                selected_head.as_deref().unwrap_or(&pin.git_commit),
                &entry.file,
                source.as_bytes(),
            )?;
            source_identities.insert(
                format!("{library}/{file}"),
                qcore_contract::profile_loader::sha256_hex(source.as_bytes()),
            );
            let options = compiler_options(&entry)?;
            let unit = C11TranslationUnit {
                source: &source,
                include_paths: options.include_paths,
                definitions: options.definitions,
                preprocessor_options: options.preprocessor_options,
                dialect: options.dialect,
                function_scope: LegacyFunctionScope::MainSourceDefinitions,
            };
            // Bind the effective preprocessing input, not just the main source
            // bytes: a used header, a generated configuration header or an
            // admitted macro option changes the analyzed code with the selected
            // `.c` file untouched.
            let effective = qcore_c_gateway::legacy_c::validate_unrestricted_c11(&unit)
                .map_err(|error| error.to_string());
            let identity = compile_db::compile_task_identity(
                &entry,
                &unit,
                match &effective {
                    Ok(text) => compile_db::EffectiveInput::Accepted(text),
                    Err(error) => compile_db::EffectiveInput::Rejected(error),
                },
                &compiler,
                task_dependencies.clone(),
            )?;
            compile_tasks.push(serde_json::json!({
                "library": library, "file": file,
                "identity_sha256": identity.digest()?, "identity": identity,
            }));
            let source_functions = legacy_c_source_functions(&source);
            project_unit(
                library,
                &file,
                &unit,
                &source_functions,
                &contracts,
                &mut report,
                &mut counts,
            )?;
            if let Some(audit) = &mut source_contracts {
                audit.analyze_unit(library, &file, &unit, &source_functions, &profile);
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
    report.write(&report_path)?;
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
