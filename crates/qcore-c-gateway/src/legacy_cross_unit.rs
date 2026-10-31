//! Conditional transport of one W4 summary from a different pinned unit.
//!
//! The external-linkage resolution is a named *premise*, not a consequence of
//! finding one definition in an audit's corpus.  Replay validates the home
//! body's W4 result and binds the exact imported contract; a caller must also
//! bind the premise to its independently held link-map identity.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::{
    CrossUnitImportIdentity, ResourceContracts, ResourceFunctionContract,
};

use super::{
    AnalysisStatus, CCompilerDialect, LegacyCertificateInputIdentity, LegacyCertificateRequest,
    LegacyCertificateScopeEvidence, LegacyFunctionScope, PreparedTranslationUnit,
    build_certificate_scope_from_evidence, callee_scope, function_names,
    tokenize_with_conditionals,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossUnitHomeEvidence {
    pub function: String,
    pub preprocessed_source: String,
    pub raw_source: String,
    pub line_marked_source: String,
    pub dialect: CCompilerDialect,
    pub request: LegacyCertificateRequest,
    pub environment: ResourceContracts,
    pub input_identity: LegacyCertificateInputIdentity,
    pub environment_sha256: String,
    /// Identity of the supplied linked-program resolution table. The caller
    /// must independently expect it; uniqueness in the analysed corpus is
    /// never substituted for a link resolution claim.
    pub link_resolution_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossUnitExport {
    pub home: CrossUnitHomeEvidence,
    pub contract: ResourceFunctionContract,
    pub lifetime: Option<qcore_schemas::loop_lifetime::InferredLifetimeSummary>,
}

impl CrossUnitHomeEvidence {
    pub fn import_identity(&self) -> CrossUnitImportIdentity {
        CrossUnitImportIdentity {
            raw_source_sha256: self.input_identity.raw_source_sha256.clone(),
            preprocessed_source_sha256: self.input_identity.preprocessed_source_sha256.clone(),
            scope_evidence_sha256: self.input_identity.scope_evidence_sha256.clone(),
            analysis_request_sha256: self.input_identity.analysis_request_sha256.clone(),
            environment_sha256: self.environment_sha256.clone(),
        }
    }
}

/// Derive only requested home summaries from a prepared, main-source unit.
/// A returned record is independently validated by W4 and its own row is
/// Safe. The caller must still establish unique home and linked resolution.
pub fn export_homes(
    prepared: &PreparedTranslationUnit,
    environment: &ResourceContracts,
    requested: &BTreeSet<String>,
    link_resolution_sha256: &str,
) -> Result<Vec<CrossUnitExport>, String> {
    if requested.is_empty() {
        return Ok(Vec::new());
    }
    if prepared.function_scope() != LegacyFunctionScope::MainSourceDefinitions
        || !environment.inferred_lifetimes().is_empty()
        || !environment.inferred_imports().is_empty()
        || !environment.lifetime_imports().is_empty()
        || !environment.cross_unit_import_identities().is_empty()
        || !sha256_spelling(link_resolution_sha256)
    {
        return Err(
            "cross-unit home request lacks main-source, base environment or link identity".into(),
        );
    }
    let preprocessed = prepared
        .preprocessed_source()
        .map_err(ToString::to_string)?;
    let marked = prepared
        .line_marked_source()
        .ok_or("cross-unit home lacks line-marked source")?;
    let names = prepared.inventory().definitions().to_vec();
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let request = LegacyCertificateRequest {
        include_paths: prepared.include_paths().to_vec(),
        definitions: prepared.definitions().to_vec(),
        preprocessor_options: prepared.preprocessor_options().to_vec(),
        input: prepared.input().clone(),
    };
    let scope = build_certificate_scope_from_evidence(
        prepared.dialect(),
        LegacyFunctionScope::MainSourceDefinitions,
        preprocessed,
        Some(prepared.raw_source()),
        Some(marked),
        request.clone(),
    )?;
    let result = super::analyze_legacy_c_translation_unit_selected_partial(
        preprocessed,
        &names[0],
        environment,
        Some(&names),
        (prepared.dialect(), Some(&request)),
    )
    .map_err(|error| error.to_string())?;
    let (tokens, _) = tokenize_with_conditionals(preprocessed);
    let mut exports = Vec::new();
    for record in result.summaries {
        let name = &record.function;
        if !requested.contains(name)
            || !names.contains(name)
            || callee_scope::has_internal_linkage(&tokens, name)
            || linkage_sensitive_declaration(&tokens, name)
            || !result
                .summary
                .functions
                .get(name)
                // Card ARM-OBLIGATIONS: a summary resting on a rule
                // package's obligation is not exported: an imported contract
                // carries no premise, so the caller's row would shed it.
                .is_some_and(|row| {
                    row.status == AnalysisStatus::Safe
                        && row.diagnostics.is_empty()
                        && row.rule_obligations.is_empty()
                })
        {
            continue;
        }
        exports.push(CrossUnitExport {
            home: CrossUnitHomeEvidence {
                function: name.clone(),
                preprocessed_source: preprocessed.to_owned(),
                raw_source: prepared.raw_source().to_owned(),
                line_marked_source: marked.to_owned(),
                dialect: prepared.dialect(),
                request: request.clone(),
                environment: environment.clone(),
                input_identity: scope.input_identity.clone(),
                environment_sha256: digest_environment(environment)?,
                link_resolution_sha256: link_resolution_sha256.to_owned(),
            },
            lifetime: if record.lifetime_effects.is_empty() && record.lifetime_removals.is_empty() {
                None
            } else {
                Some(qcore_schemas::loop_lifetime::InferredLifetimeSummary {
                    holder_frames: record.lifetime_holder_frames.clone(),
                    contract: record.contract.clone(),
                    effects: record.lifetime_effects,
                    removals: record.lifetime_removals,
                })
            },
            contract: record.contract,
        });
    }
    Ok(exports)
}

/// Restrict a run-wide import table to names with no definition in this
/// caller's preprocessed unit. A header static always takes local precedence.
pub fn for_caller(
    preprocessed: &str,
    run_environment: &ResourceContracts,
    run_homes: &[CrossUnitHomeEvidence],
) -> (ResourceContracts, Vec<CrossUnitHomeEvidence>) {
    let (tokens, conditionals) = tokenize_with_conditionals(preprocessed);
    let local = function_names(&tokens, &conditionals)
        .definitions
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut environment = run_environment.clone();
    let homes = run_homes
        .iter()
        .filter(|home| !local.contains(&home.function))
        .cloned()
        .collect::<Vec<_>>();
    let imports = environment
        .inferred_imports()
        .iter()
        .filter(|(name, _)| !local.contains(*name))
        .map(|(name, contract)| (name.clone(), contract.clone()))
        .collect();
    environment.set_inferred_imports(imports);
    environment.set_lifetime_imports(
        environment
            .lifetime_imports()
            .iter()
            .filter(|(name, _)| !local.contains(*name))
            .map(|(name, summary)| (name.clone(), summary.clone()))
            .collect(),
    );
    let identities = environment
        .cross_unit_import_identities()
        .iter()
        .filter(|(name, _)| !local.contains(*name))
        .map(|(name, identity)| (name.clone(), identity.clone()))
        .collect();
    environment.set_cross_unit_import_identities(identities);
    (environment, homes)
}

fn digest_environment(environment: &ResourceContracts) -> Result<String, String> {
    let bytes = serde_json::to_vec(environment).map_err(|error| error.to_string())?;
    Ok(qcore_schemas::sha256_hex(&bytes))
}

fn sha256_spelling(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn check_imports(
    caller_source: &str,
    contracts: &ResourceContracts,
    homes: &[CrossUnitHomeEvidence],
) -> Result<(), String> {
    if !contracts.inferred_lifetimes().is_empty() {
        return Err("local inferred lifetime summaries must be rederived from this unit, not supplied as authority".into());
    }
    if homes.is_empty()
        && contracts.inferred_imports().is_empty()
        && contracts.lifetime_imports().is_empty()
        && contracts.cross_unit_import_identities().is_empty()
    {
        return Ok(());
    }
    let (caller_tokens, caller_conditionals) = tokenize_with_conditionals(caller_source);
    let caller_definitions = function_names(&caller_tokens, &caller_conditionals).definitions;
    let mut validated = BTreeMap::<String, ResourceFunctionContract>::new();
    let mut validated_lifetimes = BTreeMap::new();
    let mut validated_identities = BTreeMap::<String, CrossUnitImportIdentity>::new();
    for home in homes {
        if !sha256_spelling(&home.link_resolution_sha256) {
            return Err(format!(
                "cross-unit `{}` lacks a linked-program resolution identity",
                home.function
            ));
        }
        if contracts.link_resolution_sha256() != Some(home.link_resolution_sha256.as_str()) {
            return Err(format!(
                "cross-unit `{}` link premise differs from the caller's environment",
                home.function
            ));
        }
        if caller_definitions.contains(&home.function) {
            return Err(format!(
                "cross-unit `{}` has a definition in the caller unit",
                home.function
            ));
        }
        if contracts.functions().contains_key(&home.function) {
            return Err(format!(
                "cross-unit `{}` conflicts with a declared contract",
                home.function
            ));
        }
        if !home.environment.inferred_lifetimes().is_empty()
            || !home.environment.inferred_imports().is_empty()
            || !home.environment.lifetime_imports().is_empty()
            || !home.environment.cross_unit_import_identities().is_empty()
        {
            return Err(format!(
                "cross-unit `{}` home depends on another imported summary",
                home.function
            ));
        }
        crate::contract_loader::check_source_assumptions(&home.environment)?;
        if digest_environment(&home.environment)? != home.environment_sha256 {
            return Err(format!(
                "cross-unit `{}` home environment identity differs",
                home.function
            ));
        }
        let scope = build_certificate_scope_from_evidence(
            home.dialect,
            LegacyFunctionScope::MainSourceDefinitions,
            &home.preprocessed_source,
            Some(&home.raw_source),
            Some(&home.line_marked_source),
            home.request.clone(),
        )?;
        if scope.input_identity != home.input_identity {
            return Err(format!(
                "cross-unit `{}` home source identity differs",
                home.function
            ));
        }
        let LegacyCertificateScopeEvidence::MainSourceDefinitions { .. } = &scope.evidence else {
            return Err("cross-unit home has no main-source scope".into());
        };
        if !scope
            .definitions
            .iter()
            .any(|row| row.name == home.function)
        {
            return Err(format!(
                "cross-unit `{}` is not a selected main-source definition",
                home.function
            ));
        }
        let (tokens, _) = tokenize_with_conditionals(&home.preprocessed_source);
        if callee_scope::has_internal_linkage(&tokens, &home.function)
            || linkage_sensitive_declaration(&tokens, &home.function)
        {
            return Err(format!(
                "cross-unit `{}` home is static or inline",
                home.function
            ));
        }
        // The selected names are re-derived from raw and marked home source.
        // A Safe *function* suffices; its surrounding unit may be Unknown.
        let names = scope
            .definitions
            .iter()
            .map(|row| row.name.clone())
            .collect::<Vec<_>>();
        let result = super::analyze_legacy_c_translation_unit_selected_partial(
            &home.preprocessed_source,
            &home.function,
            &home.environment,
            Some(&names),
            (home.dialect, Some(&home.request)),
        )
        .map_err(|error| format!("cross-unit home cannot be reanalysed: {error}"))?;
        let Some(row) = result.summary.functions.get(&home.function) else {
            return Err(format!("cross-unit `{}` home has no row", home.function));
        };
        if row.status != AnalysisStatus::Safe || !row.diagnostics.is_empty() {
            return Err(format!("cross-unit `{}` home is not Safe", home.function));
        }
        let Some(record) = result
            .summaries
            .iter()
            .find(|record| record.function == home.function)
        else {
            return Err(format!(
                "cross-unit `{}` home has no validated W4 summary",
                home.function
            ));
        };
        if record
            .lifetime_effects
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != row.lifetime_effects
        {
            return Err(format!(
                "cross-unit `{}` lifetime effects differ from its Safe row",
                home.function
            ));
        }
        if validated.contains_key(&home.function)
            || validated_lifetimes.contains_key(&home.function)
        {
            return Err(format!("cross-unit `{}` has multiple homes", home.function));
        }
        if !record.lifetime_removals.is_empty() {
            let parameters = super::function_parameters(&tokens, &home.function)
                .ok_or("removal home has no formal inventory")?;
            if crate::legacy_summary::ordered_removals(&row.cfg, &parameters).as_ref()
                != Some(&record.lifetime_removals)
            {
                return Err("cross-unit ordered removal trace differs from replayed home".into());
            }
        }
        if record.lifetime_effects.is_empty() && record.lifetime_removals.is_empty() {
            validated.insert(home.function.clone(), record.contract.clone());
        } else {
            validated_lifetimes.insert(
                home.function.clone(),
                qcore_schemas::loop_lifetime::InferredLifetimeSummary {
                    holder_frames: record.lifetime_holder_frames.clone(),
                    contract: record.contract.clone(),
                    effects: record.lifetime_effects.clone(),
                    removals: record.lifetime_removals.clone(),
                },
            );
        }
        validated_identities.insert(home.function.clone(), home.import_identity());
    }
    if &validated_lifetimes != contracts.lifetime_imports() {
        return Err("cross-unit imported lifetime effects differ from replayed homes".into());
    }
    if &validated != contracts.inferred_imports() {
        return Err("cross-unit imported contracts differ from replayed homes".into());
    }
    if &validated_identities != contracts.cross_unit_import_identities() {
        return Err("cross-unit imported home identities differ from replayed homes".into());
    }
    Ok(())
}

/// A prior file-scope declaration may confer internal or C99 inline semantics
/// even when the definition's own specifiers omit them. Reject both spellings
/// anywhere in the unit's file-scope declarations for this name.
fn linkage_sensitive_declaration(tokens: &[super::Token], function: &str) -> bool {
    let mut braces = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.text.as_str() {
            "{" => braces += 1,
            "}" => braces = braces.saturating_sub(1),
            _ => {}
        }
        if braces != 0
            || token.text != function
            || tokens.get(index + 1).map(|next| next.text.as_str()) != Some("(")
        {
            continue;
        }
        for earlier in tokens[..index].iter().rev() {
            match earlier.text.as_str() {
                ";" | "{" | "}" => break,
                "static" | "inline" | "__inline" | "__inline__" => return true,
                _ => {}
            }
        }
    }
    false
}
