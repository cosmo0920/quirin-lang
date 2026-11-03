//! Current-revision inventory driver. Each distinct compilation configuration
//! is analyzed or explicitly Unknown; unsupported inputs never disappear.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::ClangIdentity;
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, legacy_c_source_functions, prepare_translation_unit,
};
use qcore_contract::profile_loader::{LoadedProfile, sha256_hex};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::profile::{LibraryProfile, UnitSelection};
use serde_json::{Value, json};

use crate::compile_db::{CompileEntry, EffectiveInput, compile_task_identity, compiler_options};
use crate::evidence::EvidenceMode;
use crate::projection::{AuditCounts, PreparedAuditCertificateContext};
use crate::providers::ContractInput;
use crate::report::{AuditReport, UnitRefusal};

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

/// The configurations of a library's declared source roots. Card
/// AUDIT-CANONICAL-PATHS: records match by located path
/// ([`crate::input_paths`]) and keep their spelling; a record spelled under
/// a root that resolves outside it is refused with the route's outside
/// diagnostic, and two spellings of one file with `input-path-alias`.
pub fn select_configurations(
    entries: &[CompileEntry],
    checkout: &Path,
    library: &LibraryProfile,
) -> Result<Vec<CompileEntry>, String> {
    use crate::input_paths::{Placement, locate, place, select_exact, select_under};
    let is_c = |entry: &CompileEntry| {
        locate(&entry.file)
            .path()
            .extension()
            .is_some_and(|ext| ext == "c")
    };
    let outside =
        |entry: &CompileEntry| format!("source outside current checkout: {}", entry.file.display());
    let mut selected = Vec::new();
    for root in &library.source_roots {
        selected.extend(match &root.selection {
            UnitSelection::TopLevel => select_under(
                entries,
                &qcore_contract::profile_loader::source_root_path(checkout, &root.root),
                |entry, relative| relative.components().count() == 1 && is_c(entry),
                outside,
            )?,
            UnitSelection::Recursive => select_under(
                entries,
                &qcore_contract::profile_loader::source_root_path(checkout, &root.root),
                |entry, _| is_c(entry),
                outside,
            )?,
            UnitSelection::Exact { file } => {
                let target = checkout.join(file);
                if place(&target, checkout) == Placement::Escapes {
                    return Err(format!("source outside current checkout: {file}"));
                }
                select_exact(entries, &target)?
                    .into_iter()
                    .filter(|entry| is_c(entry))
                    .collect()
            }
        });
    }
    crate::input_paths::refuse_aliases(selected.iter().copied())?;
    Ok(distinct_configurations(selected.into_iter().cloned()))
}

pub struct CurrentAudit {
    checkout: PathBuf,
    revision: String,
    compiler: ClangIdentity,
    contracts: ResourceContracts,
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the loaded contracts before any
    /// refutation table, and the table the published phase applied.
    loaded_contracts: ResourceContracts,
    refuted: qcore_schemas::contract_refutation::RefutationTable,
    pins: crate::contract_revision::PinEvaluation,
    dependencies: BTreeMap<String, String>,
    source_profile_sha256: String,
    assumptions: Value,
    report: AuditReport,
    sources: BTreeSet<PathBuf>,
    counts: AuditCounts,
    /// Card MEMBER-PREPASS-OTHER-ROUTES (rule 12n, R1): the run's member
    /// facts the pre-pass read over the route's compile database, installed
    /// on every analysing thread, with its evidence section.
    member_prepass: Option<crate::member_prepass::Prepass>,
    cross_unit_homes: Vec<qcore_c_gateway::legacy_c::CrossUnitHomeEvidence>,
}

/// One configuration's console lines and, unless it failed, its products.
pub struct EntryOutcome {
    pub output: String,
    pub result: Result<EntryProducts, String>,
}

/// What one configuration contributes to a [`CurrentAudit`]: its checkout
/// source path, its report rows and its tally.
pub struct EntryProducts {
    source: Option<PathBuf>,
    report: AuditReport,
    counts: AuditCounts,
    /// Card W45-AUDIT-CHECKPOINTS: what preparation established beyond the
    /// compile-task identity, for the unit's checkpoint fingerprint.
    preparation: crate::checkpoint::PreparationFacts,
}

impl EntryProducts {
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: this configuration's unit rows,
    /// from which a phase's refutation table is read.
    #[must_use]
    pub fn unit_rows(&self) -> &[serde_json::Value] {
        self.report.unit_rows()
    }

    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: this
    /// configuration's bounds body evidence, from which a phase's bounds
    /// refutation table is read.
    #[must_use]
    pub fn bounds_evidence(&self) -> &[serde_json::Value] {
        self.report.bounds_evidence()
    }
}

type CrossUnitHomeScan = (
    Vec<(String, qcore_c_gateway::legacy_c::CrossUnitExport)>,
    Vec<(String, String)>,
    bool,
);

impl CurrentAudit {
    /// First-phase W4 exports for a claimed home. The source and compilation
    /// options are read through the same configured request as the row.
    pub fn export_cross_unit_home(
        &self,
        library: &str,
        expected_source: &Path,
        entry: &CompileEntry,
        links: &crate::cross_unit_links::LinkMap,
    ) -> Result<CrossUnitHomeScan, String> {
        // Card AUDIT-CANONICAL-PATHS: the home is placed by its located path,
        // so a record spelled through a moved or symlinked checkout finds its
        // unit key; one that resolves outside the checkout is still refused.
        let file = crate::input_paths::relative_within(expected_source, &self.checkout)
            .ok_or("cross-unit home lies outside checkout")?;
        let file = file.to_string_lossy();
        let unit_key = format!("{library}/{file}");
        let requested = links.names_for(&unit_key);
        let bytes = std::fs::read(&entry.file).map_err(|error| error.to_string())?;
        crate::verify_pinned_bytes(&self.checkout, &self.revision, expected_source, &bytes)
            .map_err(UnitRefusal::into_reason)?;
        let source = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
        let options = compiler_options(entry)?;
        let unit = C11TranslationUnit {
            source,
            include_paths: options.include_paths,
            definitions: options.definitions,
            preprocessor_options: options.preprocessor_options,
            dialect: options.dialect,
            function_scope: LegacyFunctionScope::MainSourceDefinitions,
            input: options.input,
        };
        let prepared = prepare_translation_unit(&unit);
        let complete = prepared.inventory().is_complete();
        let requested_names = links.requested();
        let definitions = prepared
            .inventory()
            .definitions()
            .iter()
            .filter(|name| requested_names.contains(*name))
            .map(|name| (unit_key.to_string(), name.clone()))
            .collect();
        let exports = if requested.is_empty() {
            Vec::new()
        } else {
            qcore_c_gateway::legacy_c::export_cross_unit_homes(
                &prepared,
                &self.contracts,
                &requested,
                &links.digest,
            )?
        };
        Ok((
            exports
                .into_iter()
                .map(|export| (unit_key.to_string(), export))
                .collect(),
            definitions,
            complete,
        ))
    }

    pub fn new(checkout: &Path, profile: &LoadedProfile, manifest: &Path) -> Result<Self, String> {
        Self::with_contracts(
            checkout,
            profile,
            &ContractInput::manifest(manifest)?,
            EvidenceMode::Configured,
        )
    }

    /// Card **W44-QUIRIN-AUDIT-PROVIDERS**: the contracts come from a manifest
    /// or a lowered provider selection, loaded by one loader, and a
    /// contracts-only run records its mode. A manifest input with the
    /// configured mode writes exactly what [`Self::new`] always wrote.
    pub fn with_contracts(
        checkout: &Path,
        profile: &LoadedProfile,
        input: &ContractInput,
        mode: EvidenceMode,
    ) -> Result<Self, String> {
        let checkout = checkout.canonicalize().map_err(|error| error.to_string())?;
        let revision = crate::checkout_head(&checkout)?;
        let mut contracts = input.load()?;
        // Card W45-CONTRACT-REVISION-MODE: every pinned contract is compared
        // with its lock over the checkout's HEAD before any unit is analysed.
        let pins = crate::contract_revision::apply_pins(
            &mut contracts,
            &crate::contract_revision::ModuleOwners::for_manifest(
                input.manifest_path(),
                input.manifest_text(),
            )?,
            &checkout,
        )?;
        // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds environment.
        crate::bounds_selection::select_for_route(
            "configured",
            input.manifest_path(),
            input.manifest_text(),
            &checkout,
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
            "configured",
            input.manifest_path(),
            input.manifest_text(),
            &checkout,
        )?;
        let manifest_text = input.manifest_text().to_owned();
        let source_profile_sha256 = profile.manifest_digest().to_owned();
        let dependencies = BTreeMap::from([
            (
                "source_revision_selection".into(),
                sha256_hex(format!("checkout_head:{revision}").as_bytes()),
            ),
            ("source_profile".into(), source_profile_sha256.clone()),
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
        let mut assumptions = json!({
            "schema_version": 1, "source_revision": revision,
            "source_revision_policy": "checkout_head", "profile": profile.assumptions(),
            "contract_manifest": manifest_text, "contracts": contracts.source_assumptions(),
            "scope": "current-revision resource projection per distinct compilation configuration; Unsafe is provisional; declared contracts remain assumptions",
            "historical_evidence": "Unknown: historical source summaries, native observations and provider evidence are not admitted",
            "profile_pin_role": "historical reference only; historical file digests and construct witnesses are not admitted",
        });
        // Card CONTRACT-INHERIT: inheritance chains travel with the sources.
        if let Some(provenance) = qcore_contract::contract_loader::inclusion_provenance(&contracts)
        {
            assumptions["contract_inclusion"] = provenance;
        }
        // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13h): the declarations'
        // premises, recorded before any unit so a composer has them.
        assumptions[crate::contract_dependencies::PREMISES_KEY] =
            crate::contract_dependencies::premises(&contracts)?;
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the declarations
        // whose member closure the run folds, recorded before any unit so a
        // checkpoint composer refuses the run unless its checkpoints carry
        // the run's facts (card MEMBER-PREPASS-OTHER-ROUTES).
        let invoking = crate::callback_member_fold::member_declarations(&contracts);
        if !invoking.is_empty() {
            assumptions[crate::callback_member_fold::DECLARATIONS_KEY] =
                serde_json::json!(invoking.keys().collect::<Vec<_>>());
        }
        if let Some(record) = input.provider_record() {
            assumptions["provider_selection"] = record.clone();
        }
        let mut report = AuditReport::new(&revision);
        // Card W45-USER-CONTRACT-PLUGINS: every checker binding of a user load
        // path runs once, before any unit, and every unit folds its links.
        if let Some(plugins) = crate::user_plugins::run(
            input.extension(),
            crate::user_plugins::PluginCheckers::default(),
        ) {
            report.set_plugin_links(plugins.links);
            assumptions["plugin_checks"] = plugins.section;
        }
        if mode.is_contracts_only() {
            let record = crate::evidence::contracts_only_record(&contracts);
            assumptions["evidence"] = record.clone();
            report.set_evidence(record);
        }
        Ok(Self {
            report,
            checkout,
            revision,
            compiler: ClangIdentity::capture()?,
            loaded_contracts: contracts.clone(),
            refuted: qcore_schemas::contract_refutation::RefutationTable::new(),
            contracts,
            pins,
            dependencies,
            source_profile_sha256,
            assumptions,
            sources: BTreeSet::new(),
            counts: AuditCounts::default(),
            member_prepass: None,
            cross_unit_homes: Vec::new(),
        })
    }

    /// Card MEMBER-PREPASS-OTHER-ROUTES (rule 12n, R1): every later unit is
    /// analysed with the run's member facts installed, and the fold reads
    /// them. Set before any unit is analysed.
    pub fn set_member_prepass(&mut self, prepass: crate::member_prepass::Prepass) {
        self.member_prepass = Some(prepass);
    }

    /// The run's member facts, when the pre-pass ran.
    #[must_use]
    pub fn member_prepass(&self) -> Option<&crate::member_prepass::Prepass> {
        self.member_prepass.as_ref()
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: analyse every later unit under
    /// the loaded contracts with `table` recorded (the empty table restores
    /// them as loaded).
    pub fn set_refuted_table(
        &mut self,
        table: &qcore_schemas::contract_refutation::RefutationTable,
    ) {
        self.contracts = crate::refuted_contracts::with_table(&self.loaded_contracts, table);
        self.cross_unit_homes.clear();
        if let Some(assumptions) = self.assumptions.as_object_mut() {
            assumptions.remove("cross_unit_link_resolution");
        }
        self.refuted = table.clone();
        // Recorded before any unit, so a checkpoint's run-wide inputs carry
        // the table to the composer; `finish` replaces it with the section.
        self.assumptions[crate::refuted_contracts::APPLIED_KEY] = json!({ "applied": table });
    }

    /// Install only homes replayed under the final refutation environment.
    /// The linked-program claim is retained as an assumption in every row.
    pub fn set_cross_unit(
        &mut self,
        environment: ResourceContracts,
        homes: Vec<qcore_c_gateway::legacy_c::CrossUnitHomeEvidence>,
        links: &crate::cross_unit_links::LinkMap,
    ) {
        self.contracts = environment;
        self.cross_unit_homes = homes;
        self.assumptions["cross_unit_link_resolution"] = json!({
            "scope": "conditional: each external call to an imported symbol resolves to its named home definition with compatible function type and calling convention in the actual link and interposition environment",
            "premise": links.premise,
            "map_sha256": self.contracts.link_resolution_sha256(),
            "imported": self.cross_unit_homes.iter().map(|home| json!({
                "function": home.function,
                "home_unit": links.home_for(&home.function),
                "home_raw_source_sha256": home.input_identity.raw_source_sha256,
                "home_preprocessed_source_sha256": home.input_identity.preprocessed_source_sha256,
                "home_environment_sha256": home.environment_sha256,
            })).collect::<Vec<_>>(),
        });
    }

    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the refutation table later
    /// units are analysed under.
    #[must_use]
    pub fn refuted_table(&self) -> &qcore_schemas::contract_refutation::RefutationTable {
        &self.refuted
    }

    /// Rebind this audit to one library slice's scoped [`ContractInput`],
    /// releasing the previously cached premises/hypothesis graphs and updating
    /// compile-task identity dependencies for the slice.
    pub fn activate_slice_contracts(&mut self, slice_input: &ContractInput) -> Result<(), String> {
        crate::contract_dependencies::clear_premises_cache();
        let mut contracts = slice_input.load()?;
        let pins = crate::contract_revision::apply_pins(
            &mut contracts,
            &crate::contract_revision::ModuleOwners::for_manifest(
                slice_input.manifest_path(),
                slice_input.manifest_text(),
            )?,
            &self.checkout,
        )?;
        crate::bounds_selection::select_for_route(
            "configured",
            slice_input.manifest_path(),
            slice_input.manifest_text(),
            &self.checkout,
            &contracts,
        )?;
        // Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the pin-derived result
        // sets of the installed bounds environment, read by rule 38 while in force.
        let contracts = {
            let mut contracts = contracts;
            contracts.set_result_ranges(crate::bounds_selection::result_range_table());
            contracts
        };
        crate::outer_selection::select_for_route(
            "configured",
            slice_input.manifest_path(),
            slice_input.manifest_text(),
            &self.checkout,
        )?;
        let manifest_text = slice_input.manifest_text();
        self.dependencies.insert(
            "resource_contract_manifest".into(),
            sha256_hex(manifest_text.as_bytes()),
        );
        self.dependencies.insert(
            "resource_contract_sources".into(),
            sha256_hex(
                serde_json::to_string(&contracts.source_assumptions())
                    .map_err(|error| error.to_string())?
                    .as_bytes(),
            ),
        );
        self.loaded_contracts = contracts.clone();
        self.contracts = contracts;
        self.pins = pins;
        self.refuted = qcore_schemas::contract_refutation::RefutationTable::new();
        self.cross_unit_homes.clear();
        Ok(())
    }

    /// Restore the run-wide union contract environment and accumulated
    /// refutation table after sequential per-library slices complete so the
    /// unified sidecar sections (`contract_revision`, `contract_dependencies`,
    /// `pending_human_review`, `refuted_contracts`) are folded over all
    /// consulted declarations, and record `library_provider_selection`.
    pub fn restore_run_contracts(
        &mut self,
        union_input: &ContractInput,
        table: &qcore_schemas::contract_refutation::RefutationTable,
        library_selection_record: Value,
    ) -> Result<(), String> {
        self.activate_slice_contracts(union_input)?;
        self.set_refuted_table(table);
        self.assumptions["library_provider_selection"] = library_selection_record;
        Ok(())
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
        let EntryOutcome { output, result } =
            self.evaluate_entry(library, expected_source, entry, unresolved);
        print!("{output}");
        self.absorb_entry(result?)
    }

    /// Analyse one selected configuration without touching the audit's
    /// accumulated state (card **W44-AUDIT-PARALLEL-UNITS**): the unit's report
    /// rows, tally, source path and console lines are returned for
    /// [`Self::absorb_entry`], which a concurrent caller applies in selection
    /// order. The console lines are returned on failure too, exactly as far as
    /// a sequential run would have printed them.
    pub fn evaluate_entry(
        &self,
        library: &str,
        expected_source: &Path,
        entry: &CompileEntry,
        unresolved: Option<&str>,
    ) -> EntryOutcome {
        let mut output = String::new();
        let mut products = EntryProducts {
            source: None,
            report: self.report.fragment(),
            counts: AuditCounts::default(),
            preparation: Default::default(),
        };
        // Card MEMBER-PREPASS-OTHER-ROUTES (R1): the unit is analysed with
        // the run's member facts installed, when the pre-pass ran.
        let facts = self
            .member_prepass
            .as_ref()
            .map(|prepass| prepass.facts.clone());
        let result = qcore_c_gateway::with_audit_unit_lifetimes(|| {
            qcore_c_gateway::member_run_facts::scoped(facts, || {
                self.evaluate_entry_into(
                    library,
                    expected_source,
                    entry,
                    unresolved,
                    &mut products,
                    &mut output,
                )
            })
        })
        .map(|()| products);
        EntryOutcome { output, result }
    }

    /// Card **W45-FB-SYSTEM-SELECTION**: name the explicit unit selection in
    /// the `.contracts.json` assumptions sidecar. It is not a dependency of any
    /// unit's identity: the selection chooses units, it does not change them.
    pub fn record_selection(&mut self, selection: Value) {
        self.assumptions["unit_selection"] = selection;
    }

    /// Card **W45-FB-SYSTEM-SELECTION**: a selected file with no compilation
    /// record stays in the denominator as one `Unknown` unit, with an
    /// environment reason, its checkout definitions as `not_projected`, and a
    /// configuration record whose `compile_record` is null. It is never
    /// dropped and never analysed under a guessed command.
    pub fn evaluate_absent(&self, library: &str, relative: &str) -> EntryOutcome {
        let mut output = String::new();
        let mut products = EntryProducts {
            source: None,
            report: self.report.fragment(),
            counts: AuditCounts::default(),
            preparation: Default::default(),
        };
        let path = self.checkout.join(relative);
        let bytes = fs::read(&path);
        let refusal = match &bytes {
            Ok(bytes) => {
                match crate::verify_pinned_bytes(&self.checkout, &self.revision, &path, bytes) {
                    Ok(()) => {
                        products.source = path.canonicalize().ok();
                        UnitRefusal::environment(format!(
                            "selected source has no compilation database record: {relative}"
                        ))
                    }
                    Err(refusal) => refusal,
                }
            }
            Err(error) => UnitRefusal::environment(format!(
                "selected source is absent from the checkout: {relative}: {error}"
            )),
        };
        let functions = bytes
            .as_ref()
            .ok()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .map(legacy_c_source_functions)
            .unwrap_or_default();
        products.counts.units += 1;
        products.counts.unknown_units += 1;
        products.counts.source_functions += functions.len();
        products
            .report
            .record_unknown(library, relative, &functions, &refusal);
        let reason = refusal.reason();
        {
            use std::fmt::Write as _;
            let _ = writeln!(output, "  {library}/{relative}: Unknown: {reason}");
        }
        let mut configuration = json!({
            "source_revision": self.revision, "file": Value::Null, "expected_source": path,
            "directory": Value::Null, "command": Value::Null, "compile_record": Value::Null,
            "raw_source_sha256": bytes.as_ref().ok().map(|bytes| sha256_hex(bytes)),
            "compiler": self.compiler, "dependencies": self.dependencies,
            "unresolved": reason,
        });
        let result = serde_json::to_vec(&configuration)
            .map_err(|error| error.to_string())
            .map(|bytes| {
                configuration["inventory_sha256"] = json!(sha256_hex(&bytes));
                products.report.record_configuration(configuration);
                products
            });
        EntryOutcome { output, result }
    }

    /// Fold one [`Self::evaluate_entry`] result into the audit.
    pub fn absorb_entry(&mut self, products: EntryProducts) -> Result<(), String> {
        let EntryProducts {
            source,
            report,
            counts,
            preparation: _,
        } = products;
        if let Some(source) = source {
            self.sources.insert(source);
        }
        self.report.absorb(report)?;
        self.counts.absorb(counts);
        Ok(())
    }

    fn evaluate_entry_into(
        &self,
        library: &str,
        expected_source: &Path,
        entry: &CompileEntry,
        unresolved: Option<&str>,
        products: &mut EntryProducts,
        output: &mut String,
    ) -> Result<(), String> {
        let expected_source = expected_source
            .canonicalize()
            .map_err(|error| error.to_string())?;
        products.source = Some(expected_source.clone());
        let relative = expected_source
            .strip_prefix(&self.checkout)
            .map_err(|_| "source outside current checkout")?;
        let file = crate::repository_path(relative);
        // Card REASON-CODES-B1: raw `std::io::Error` text carries no marker the
        // closed vocabulary can recognise, so this site names its category
        // instead of leaving the row uncoded. It is an environment failure,
        // which says nothing about the analysed source.
        let bytes =
            fs::read(&entry.file).map_err(|error| UnitRefusal::environment(error.to_string()));
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
        let prepared = (|| -> Result<C11TranslationUnit<'_>, UnitRefusal> {
            let bytes = bytes.as_ref().map_err(Clone::clone)?;
            crate::verify_pinned_bytes(&self.checkout, &self.revision, &expected_source, bytes)?;
            let source = source.ok_or_else(|| UnitRefusal::classified("source is not UTF-8"))?;
            if let Some(reason) = unresolved {
                return Err(UnitRefusal::classified(reason));
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
                let prepared = prepare_translation_unit(&unit);
                products.preparation = crate::checkpoint::PreparationFacts::of(&prepared);
                let preparation_error = prepared.validation_error().map(ToString::to_string);
                let identity = compile_task_identity(
                    entry,
                    &unit,
                    match prepared.preprocessed_source() {
                        Ok(source) => EffectiveInput::Accepted(source),
                        Err(_) => EffectiveInput::Rejected(
                            preparation_error
                                .as_deref()
                                .expect("rejected preparation has an error"),
                        ),
                    },
                    &self.compiler,
                    self.dependencies.clone(),
                )?;
                configuration["compile_task_identity"] =
                    serde_json::to_value(&identity).map_err(|error| error.to_string())?;
                crate::projection::project_prepared_unit_into_with_cross_unit(
                    library,
                    &file,
                    &prepared,
                    &self.contracts,
                    &self.cross_unit_homes,
                    PreparedAuditCertificateContext {
                        compile_task_identity: &identity,
                        source_profile_sha256: &self.source_profile_sha256,
                    },
                    &mut products.report,
                    &mut products.counts,
                    output,
                )?;
            }
            Err(refusal) => {
                products.counts.units += 1;
                products.counts.unknown_units += 1;
                products.counts.source_functions += functions.len();
                products
                    .report
                    .record_unknown(library, &file, &functions, &refusal);
                let reason = refusal.reason();
                configuration["unresolved"] = json!(reason);
                {
                    use std::fmt::Write as _;
                    let _ = writeln!(output, "  {library}/{file}: Unknown: {reason}");
                }
            }
        }
        // This digest is inventory identity, not admission of a rejected build.
        let digest = sha256_hex(
            serde_json::to_vec(&configuration)
                .map_err(|error| error.to_string())?
                .as_slice(),
        );
        configuration["inventory_sha256"] = json!(digest);
        products.report.record_configuration(configuration);
        Ok(())
    }

    pub fn finish(mut self, path: Option<&Path>) -> Result<AuditCounts, String> {
        if crate::checkout_head(&self.checkout)? != self.revision {
            return Err("checkout HEAD changed during audit; rerun with stable inputs".into());
        }
        // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): the run's
        // closure of every invoked member, folded over the rows before
        // anything is published; it only demotes. Card
        // MEMBER-PREPASS-OTHER-ROUTES: over the pre-pass's facts, when it ran.
        if let Some(prepass) = self.member_prepass.take() {
            self.report
                .set_member_prepass(prepass.facts, prepass.section);
        }
        let (members, from_safe, from_pending) = self.report.fold_callback_members(&self.contracts);
        self.report.release_final_fold_inputs();
        self.counts.safe_units -= from_safe;
        self.counts.pending_units -= from_pending;
        self.counts.unknown_units += from_safe + from_pending;
        if !members.is_null() {
            println!(
                "callback members: {} demoted",
                members["demoted"].as_array().map_or(0, Vec::len)
            );
            self.assumptions[crate::callback_member_fold::SECTION_KEY] = members;
        }
        println!(
            "current revision: {} source file(s), {} configuration(s): {} Safe / {} Pending / {} provisional Unsafe / {} Unknown / {} data-only / {} gateway failures",
            self.sources.len(),
            self.counts.units,
            self.counts.safe_units,
            self.counts.pending_units,
            self.counts.unsafe_units,
            self.counts.unknown_units,
            self.counts.data_only_units,
            self.counts.gateway_failures
        );
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
        self.assumptions["pending_human_review"] = pending;
        // Card W45-USER-CONTRACT-PLUGINS: printed only when a checker ran.
        if self.assumptions["plugin_checks"].is_object() {
            println!(
                "{}",
                crate::user_plugins::tally(&self.assumptions["plugin_checks"])
            );
        }
        // Card W45-CONTRACT-REVISION-MODE: revision modes, pin mismatches and
        // flexible assumptions, tallied and published the same way.
        let revision = crate::contract_revision::section(&self.contracts, &self.pins, &self.report);
        println!("{}", crate::contract_revision::tally(&revision));
        // Card VERDICT-CONTRACT-DEPENDENCIES (rule 13h).
        let dependencies = crate::contract_dependencies::section(
            &revision,
            &self.assumptions[crate::contract_dependencies::PREMISES_KEY],
            &self.report,
        );
        println!("{}", crate::contract_dependencies::tally(&dependencies));
        self.assumptions[crate::contract_dependencies::SECTION_KEY] = dependencies;
        self.assumptions["contract_revision"] = revision;
        // Card W45-REFUTED-CONTRACT-WITHDRAWAL: the refutation table, its
        // digest and the calls it held, tallied and published the same way.
        let refuted = crate::refuted_contracts::section(&self.refuted, &self.report);
        println!("{}", crate::refuted_contracts::tally(&refuted));
        self.assumptions["refuted_contracts"] = refuted;
        if let Some(path) = path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            self.assumptions["source_file_count"] = json!(self.sources.len());
            self.assumptions["configuration_count"] = json!(self.counts.units);
            // Card AUDIT-COMPOSITION: the contract bundle the components rest on.
            self.report.record_contract_bundle(&self.contracts);
            self.report
                .write_with_revision_key(path, "fluent_bit_revision")?;
            self.report.write_bounds_selection()?;
            let null_rows = self.report.write_null_selection()?;
            self.report
                .write_outer_projection(&self.contracts, null_rows.as_deref())?;
            fs::write(
                path.with_extension("contracts.json"),
                serde_json::to_vec_pretty(&self.assumptions).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        }
        crate::contract_dependencies::clear_premises_cache();
        Ok(self.counts)
    }
}

/// Card **W45-AUDIT-CHECKPOINTS**: what a checkpointed run reads of an audit
/// and of one configuration's products. Read-only; the whole-run fold above is
/// unchanged.
impl CurrentAudit {
    pub(crate) fn checkout(&self) -> &Path {
        &self.checkout
    }

    pub(crate) fn compiler(&self) -> &ClangIdentity {
        &self.compiler
    }

    pub(crate) fn contracts(&self) -> &ResourceContracts {
        &self.contracts
    }

    pub(crate) fn dependencies(&self) -> &BTreeMap<String, String> {
        &self.dependencies
    }

    pub(crate) fn source_profile_sha256(&self) -> &str {
        &self.source_profile_sha256
    }

    /// The `.contracts.json` assumptions before `finish` adds its run-final keys.
    pub(crate) fn assumptions(&self) -> &Value {
        &self.assumptions
    }

    /// The report's evidence-mode record, when the run has one.
    pub(crate) fn report_evidence(&self) -> Option<&Value> {
        self.report.evidence_record()
    }

    /// The pin evaluation of card W45-CONTRACT-REVISION-MODE.
    pub(crate) fn pins(&self) -> &crate::contract_revision::PinEvaluation {
        &self.pins
    }
}

impl EntryProducts {
    /// The configuration's source path, report rows, tally and preparation facts.
    pub(crate) fn into_parts(
        self,
    ) -> (
        Option<PathBuf>,
        AuditReport,
        AuditCounts,
        crate::checkpoint::PreparationFacts,
    ) {
        (self.source, self.report, self.counts, self.preparation)
    }
}

pub(crate) struct ParkedEntryOutcome {
    pub(crate) output: String,
    pub(crate) result: Result<ParkedEntryProducts, String>,
}

pub(crate) struct ParkedEntryProducts {
    source: Option<PathBuf>,
    pub(crate) report: crate::report::ParkedReport,
    counts: AuditCounts,
    preparation: crate::checkpoint::PreparationFacts,
}

impl EntryOutcome {
    pub(crate) fn park(self) -> ParkedEntryOutcome {
        ParkedEntryOutcome {
            output: self.output,
            result: self.result.and_then(|p| {
                Ok(ParkedEntryProducts {
                    source: p.source,
                    report: p.report.park()?,
                    counts: p.counts,
                    preparation: p.preparation,
                })
            }),
        }
    }
}

impl ParkedEntryProducts {
    pub(crate) fn restore(self) -> Result<EntryProducts, String> {
        Ok(EntryProducts {
            source: self.source,
            report: self.report.restore()?,
            counts: self.counts,
            preparation: self.preparation,
        })
    }
}
