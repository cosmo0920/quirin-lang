//! Configured source import and process dispatch over the actual pinned audit
//! inputs. Function contracts never change a resource-projection unit verdict.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use qcore_c_gateway::boolean_source::{
    ChunkFrameSummary, ChunkSourceResult, import_chunk_source_with_profile,
};
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_contract::profile_loader::LoadedProfile;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::compile_db::CompileEntry;
use crate::source_contract::{
    SourceContractRegistry, import_configured_frame_helpers, read_bounded,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContractAuditConfig {
    pub configuration: PathBuf,
    pub source_rules: PathBuf,
    pub libraries: Vec<SourceContractLibrary>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContractLibrary {
    pub library: String,
    /// Relative to the checkout; configured helper paths resolve beneath this.
    pub source_root: PathBuf,
}

pub(crate) struct SourceContractAudit {
    registry: SourceContractRegistry,
    rules: String,
    helpers: BTreeMap<String, Vec<ChunkFrameSummary>>,
    helper_evidence: Vec<Value>,
    selections: Vec<Value>,
    units: Vec<Value>,
    checked: usize,
    unknown: usize,
}

impl SourceContractAudit {
    pub(crate) fn prepare(
        config: &SourceContractAuditConfig,
        base: &Path,
        checkout: &Path,
        entries: &[CompileEntry],
        profile: &LoadedProfile,
        selected_libraries: &[String],
    ) -> Result<Self, String> {
        if config.configuration.as_os_str().is_empty() || config.source_rules.as_os_str().is_empty()
        {
            return Err("source-contract configuration and rule paths must be nonempty".into());
        }
        let names = config
            .libraries
            .iter()
            .map(|entry| entry.library.as_str())
            .collect::<BTreeSet<_>>();
        if names.is_empty()
            || names.len() != config.libraries.len()
            || names.iter().any(|name| {
                name.is_empty() || !selected_libraries.iter().any(|selected| selected == name)
            })
        {
            return Err("source-contract libraries must be distinct nonempty selections from the resource audit".into());
        }
        let registry = SourceContractRegistry::load(crate::resolve(base, &config.configuration))
            .map_err(|error| error.to_string())?;
        if names != registry.bound_libraries().collect::<BTreeSet<_>>() {
            return Err("source-contract bindings must match the explicitly selected source-contract libraries".into());
        }
        let rules = String::from_utf8(
            read_bounded(&crate::resolve(base, &config.source_rules), 8_388_608)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if rules.trim().is_empty() {
            return Err("source-contract rules must be nonempty".into());
        }
        let mut helpers = BTreeMap::new();
        let mut helper_evidence = Vec::new();
        let mut selections = Vec::new();
        for selected in &config.libraries {
            let library = profile
                .library(&selected.library)
                .ok_or("source-contract library is absent from the profile")?;
            if library.boolean_field_models.is_empty() {
                return Err(format!(
                    "source-contract library `{}` has no declared Boolean field models",
                    selected.library
                ));
            }
            if selected.source_root.as_os_str().is_empty() || selected.source_root.is_absolute() {
                return Err(
                    "source-contract source_root must be a nonempty checkout-relative directory"
                        .into(),
                );
            }
            let source_root = checkout
                .join(&selected.source_root)
                .canonicalize()
                .map_err(|error| error.to_string())?;
            if !source_root.starts_with(checkout) || !source_root.is_dir() {
                return Err(
                    "source-contract source_root escapes its checkout or is not a directory".into(),
                );
            }
            let revision = &profile
                .pin_for(&selected.library)
                .ok_or("source-contract library has no pin")?
                .git_commit;
            let policies = registry.frame_helpers_for_library(&selected.library);
            let mut pinned = BTreeMap::new();
            for policy in policies {
                let path = source_root
                    .join(policy.source())
                    .canonicalize()
                    .map_err(|error| error.to_string())?;
                if !path.starts_with(&source_root) {
                    return Err("helper source escapes its configured source root".into());
                }
                let bytes = read_bounded(&path, 8_388_608).map_err(|error| error.to_string())?;
                crate::verify_pinned_bytes(checkout, revision, &path, &bytes)?;
                pinned.insert(
                    policy.function().to_owned(),
                    qcore_schemas::sha256_hex(&bytes),
                );
            }
            let checked = import_configured_frame_helpers(
                policies,
                &source_root,
                entries,
                revision,
                profile,
                &selected.library,
                &rules,
            )
            .map_err(|error| error.to_string())?;
            for helper in &checked {
                let identity = helper.summary.input_identity();
                if pinned.get(helper.summary.function()).map(String::as_str)
                    != Some(identity.raw_source_digest())
                {
                    return Err(
                        "helper import bytes differ from their verified pinned snapshot".into(),
                    );
                }
                helper_evidence.push(json!({
                    "library": selected.library, "function": helper.summary.function(),
                    "status": "checked_declared_frame_policy", "checked_postconditions": helper.checked_postconditions,
                    "import_identity": identity,
                }));
            }
            selections.push(json!({"library": selected.library, "source_root": selected.source_root, "revision": revision}));
            helpers.insert(
                selected.library.clone(),
                checked.into_iter().map(|helper| helper.summary).collect(),
            );
        }
        Ok(Self {
            registry,
            rules,
            helpers,
            helper_evidence,
            selections,
            units: Vec::new(),
            checked: 0,
            unknown: 0,
        })
    }

    pub(crate) fn analyze_unit(
        &mut self,
        library: &str,
        file: &str,
        unit: &C11TranslationUnit<'_>,
        raw_functions: &[String],
        profile: &LoadedProfile,
    ) {
        let Some(helpers) = self.helpers.get(library) else {
            return;
        };
        let revision = &profile
            .pin_for(library)
            .expect("prepared library pin")
            .git_commit;
        let imports = import_chunk_source_with_profile(
            unit,
            revision,
            profile,
            &[library],
            helpers,
            &self.rules,
        );
        let mut functions = Vec::new();
        let mut seen = BTreeSet::new();
        let mut import_failure = None;
        match imports {
            Ok(results) => {
                for result in results {
                    match result {
                        ChunkSourceResult::Summary(summary) => {
                            seen.insert(summary.function().to_owned());
                            match self.registry.verify(&summary, &format!("{library}/{file}")) {
                                Ok(verified) => {
                                    self.checked += 1;
                                    let certificate = verified.certificate;
                                    functions.push(json!({
                                    "function": summary.function(), "status": "checked_function_contract",
                                    "checked_states": verified.evaluation.checked_states(),
                                    "task": certificate.task(), "witness": certificate.witness(),
                                    "evidence": certificate.evidence(), "bounds": certificate.obligation_bounds(),
                                    "import_identity": summary.input_identity(),
                                }));
                                }
                                Err(error) => {
                                    self.unknown += 1;
                                    functions.push(json!({"function": summary.function(), "status": "unknown", "reason": error.to_string(), "import_identity": summary.input_identity()}));
                                }
                            }
                        }
                        ChunkSourceResult::Unknown { function, reason } => {
                            seen.insert(function.clone());
                            self.unknown += 1;
                            functions.push(json!({"function": function, "status": "unknown", "reason": reason}));
                        }
                    }
                }
            }
            Err(error) => import_failure = Some(error),
        }
        for function in raw_functions {
            if seen.insert(function.clone()) {
                self.unknown += 1;
                functions.push(json!({"function": function, "status": "unknown", "reason": import_failure.as_deref().unwrap_or("raw source definition absent from the selected typed import; possibly build-excluded")}));
            }
        }
        functions.sort_by(|a, b| a["function"].as_str().cmp(&b["function"].as_str()));
        self.units.push(json!({
            "library": library, "file": file, "source_sha256": qcore_schemas::sha256_hex(unit.source.as_bytes()),
            "raw_source_functions": raw_functions, "functions": functions,
            "import_failure": import_failure,
        }));
    }

    pub(crate) fn finish(self) -> Value {
        println!(
            "source contracts: {} checked function contract(s) / {} Unknown; no whole-unit promotion",
            self.checked, self.unknown
        );
        json!({
            "schema_version": 1,
            "scope": "source-derived Boolean-field function contracts under explicit live/exclusive/domain and source/build/ABI assumptions; no whole-unit promotion",
            "selection": self.selections, "configuration": self.registry.audit_inventory(),
            "source_rules": self.rules, "source_rules_sha256": qcore_schemas::sha256_hex(self.rules.as_bytes()),
            "frame_helpers": self.helper_evidence, "units": self.units,
            "checked_function_contracts": self.checked, "unknown_functions": self.unknown,
        })
    }
}
