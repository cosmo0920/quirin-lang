//! Current-revision inventory driver. Each distinct compilation configuration
//! is analyzed or explicitly Unknown; unsupported inputs never disappear.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, legacy_c_source_functions, validate_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_manifest;
use qcore_contract::profile_loader::{LoadedProfile, sha256_hex};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::profile::{LibraryProfile, UnitSelection};
use serde_json::{Value, json};

use crate::compile_db::{CompileEntry, EffectiveInput, compile_task_identity, compiler_options};
use crate::projection::{AuditCounts, project_unit};
use crate::report::AuditReport;

/// Keep distinct commands, deduplicating only identical file/directory/command
/// triples. Output and macro variants are separate audit obligations.
pub fn distinct_configurations(
    entries: impl IntoIterator<Item = CompileEntry>,
) -> Vec<CompileEntry> {
    entries
        .into_iter()
        .map(|entry| {
            (
                (
                    entry.file.clone(),
                    entry.directory.clone(),
                    entry.command.clone(),
                ),
                entry,
            )
        })
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}

pub fn select_configurations(
    entries: &[CompileEntry],
    checkout: &Path,
    library: &LibraryProfile,
) -> Vec<CompileEntry> {
    distinct_configurations(
        entries
            .iter()
            .filter(|entry| {
                let entry_file = entry
                    .file
                    .canonicalize()
                    .unwrap_or_else(|_| entry.file.clone());
                entry_file.extension().is_some_and(|ext| ext == "c")
                    && library.source_roots.iter().any(|root| {
                        let source_root = checkout
                            .join(&root.root)
                            .canonicalize()
                            .unwrap_or_else(|_| checkout.join(&root.root));
                        match &root.selection {
                            UnitSelection::TopLevel => {
                                entry_file.parent() == Some(source_root.as_path())
                            }
                            UnitSelection::Recursive => entry_file.starts_with(source_root),
                            UnitSelection::Exact { file } => entry_file == checkout.join(file),
                        }
                    })
            })
            .cloned(),
    )
}

pub struct CurrentAudit {
    checkout: PathBuf,
    revision: String,
    compiler: ClangIdentity,
    contracts: ResourceContracts,
    dependencies: BTreeMap<String, String>,
    assumptions: Value,
    report: AuditReport,
    sources: BTreeSet<PathBuf>,
    counts: AuditCounts,
}

impl CurrentAudit {
    pub fn new(checkout: &Path, profile: &LoadedProfile, manifest: &Path) -> Result<Self, String> {
        let checkout = checkout.canonicalize().map_err(|error| error.to_string())?;
        let revision = crate::checkout_head(&checkout)?;
        let contracts = load_resource_contract_manifest(manifest)?;
        let manifest_text = fs::read_to_string(manifest).map_err(|error| error.to_string())?;
        let dependencies = BTreeMap::from([
            (
                "source_revision_selection".into(),
                sha256_hex(format!("checkout_head:{revision}").as_bytes()),
            ),
            ("source_profile".into(), profile.manifest_digest().into()),
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
        let assumptions = json!({
            "schema_version": 1, "source_revision": revision,
            "source_revision_policy": "checkout_head", "profile": profile.assumptions(),
            "contract_manifest": manifest_text, "contracts": contracts.source_assumptions(),
            "scope": "current-revision resource projection per distinct compilation configuration; Unsafe is provisional; declared contracts remain assumptions",
            "historical_evidence": "Unknown: historical source summaries, native observations and provider evidence are not admitted",
            "profile_pin_role": "historical reference only; historical file digests and construct witnesses are not admitted",
        });
        Ok(Self {
            report: AuditReport::new(&revision),
            checkout,
            revision,
            compiler: ClangIdentity::capture()?,
            contracts,
            dependencies,
            assumptions,
            sources: BTreeSet::new(),
            counts: AuditCounts::default(),
        })
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// expected_source is the path in the selected Git checkout corresponding
    /// to the compiled bytes; copied build inputs must also match this blob.
    pub fn audit_entry(
        &mut self,
        library: &str,
        expected_source: &Path,
        entry: &CompileEntry,
        unresolved: Option<&str>,
    ) -> Result<(), String> {
        let expected_source = expected_source
            .canonicalize()
            .map_err(|error| error.to_string())?;
        self.sources.insert(expected_source.clone());
        let relative = expected_source
            .strip_prefix(&self.checkout)
            .map_err(|_| "source outside current checkout")?;
        let file = crate::repository_path(relative);
        let bytes = fs::read(&entry.file).map_err(|error| error.to_string());
        let mut configuration = json!({
            "source_revision": self.revision, "file": entry.file, "expected_source": expected_source,
            "directory": entry.directory, "command": entry.command,
            "raw_source_sha256": bytes.as_ref().ok().map(|bytes| sha256_hex(bytes)),
            "compiler": self.compiler, "dependencies": self.dependencies,
        });
        let source = bytes
            .as_ref()
            .ok()
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        let functions = source.map(legacy_c_source_functions).unwrap_or_default();
        let prepared = (|| {
            let bytes = bytes.as_ref().map_err(Clone::clone)?;
            crate::verify_pinned_bytes(&self.checkout, &self.revision, &expected_source, bytes)?;
            let source = source.ok_or("source is not UTF-8")?;
            if let Some(reason) = unresolved {
                return Err(reason.to_owned());
            }
            let options = compiler_options(entry)?;
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
        match prepared {
            Ok(unit) => {
                let effective = validate_unrestricted_c11(&unit).map_err(|error| error.to_string());
                let identity = compile_task_identity(
                    entry,
                    &unit,
                    match &effective {
                        Ok(text) => EffectiveInput::Accepted(text),
                        Err(error) => EffectiveInput::Rejected(error),
                    },
                    &self.compiler,
                    self.dependencies.clone(),
                )?;
                configuration["compile_task_identity"] =
                    serde_json::to_value(identity).map_err(|error| error.to_string())?;
                project_unit(
                    library,
                    &file,
                    &unit,
                    &functions,
                    &self.contracts,
                    &mut self.report,
                    &mut self.counts,
                )?;
            }
            Err(reason) => {
                self.counts.units += 1;
                self.counts.unknown_units += 1;
                self.counts.source_functions += functions.len();
                self.report
                    .record_unknown(library, &file, &functions, &reason);
                configuration["unresolved"] = json!(reason);
                println!("  {library}/{file}: Unknown: {reason}");
            }
        }
        // This digest is inventory identity, not admission of a rejected build.
        let digest = sha256_hex(
            serde_json::to_vec(&configuration)
                .map_err(|error| error.to_string())?
                .as_slice(),
        );
        configuration["inventory_sha256"] = json!(digest);
        self.report.record_configuration(configuration);
        Ok(())
    }

    pub fn finish(mut self, path: Option<&Path>) -> Result<AuditCounts, String> {
        if crate::checkout_head(&self.checkout)? != self.revision {
            return Err("checkout HEAD changed during audit; rerun with stable inputs".into());
        }
        println!(
            "current revision: {} source file(s), {} configuration(s): {} Safe / {} provisional Unsafe / {} Unknown / {} data-only / {} gateway failures",
            self.sources.len(),
            self.counts.units,
            self.counts.safe_units,
            self.counts.unsafe_units,
            self.counts.unknown_units,
            self.counts.data_only_units,
            self.counts.gateway_failures
        );
        if let Some(path) = path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            self.assumptions["source_file_count"] = json!(self.sources.len());
            self.assumptions["configuration_count"] = json!(self.counts.units);
            self.report
                .write_with_revision_key(path, "fluent_bit_revision")?;
            fs::write(
                path.with_extension("contracts.json"),
                serde_json::to_vec_pretty(&self.assumptions).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(self.counts)
    }
}
