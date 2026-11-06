//! Conditional transport of one W4 summary from a different pinned unit.
//!
//! The external-linkage resolution is a named *premise*, not a consequence of
//! finding one definition in an audit's corpus.  Replay validates the home
//! body's W4 result and binds the exact imported contract; a caller must also
//! bind the premise to its independently held link-map identity.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

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

/// Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): the most homes one
/// caller unit holds. Certificate issue and the certificate check each replay
/// every home the caller holds (a whole-unit W4 analysis of the home), so a
/// caller's import work is bounded by this many home analyses per check. A
/// caller that may reach more holds none: its calls to those names stay
/// uncontracted (`Unknown`), and [`CallerImports::withheld`] names why.
pub const REPLAY_BUDGET_PER_UNIT: usize = 24;

/// What one caller unit holds of a run's cross-unit imports.
#[derive(Clone, Debug)]
pub struct CallerImports {
    /// The run environment with only the held imports.
    pub environment: ResourceContracts,
    /// The held homes, replayed at certificate issue and check.
    pub homes: Vec<CrossUnitHomeEvidence>,
    /// Run homes the caller may reach: no definition in the caller, and the
    /// name spelled in the caller's preprocessed unit or named as a value by
    /// the run's member facts.
    pub reachable: usize,
    /// Of `reachable`, the homes the caller never uses and only the run's
    /// member facts name.
    pub named_only: usize,
    /// Set when `reachable` exceeds [`REPLAY_BUDGET_PER_UNIT`]: the caller
    /// then holds no import.
    pub withheld: Option<String>,
}

/// Every identifier the unit uses outside a file-scope declarator: inside
/// any brace (a function body, a record or enumeration body, an aggregate
/// initializer) or after a file-scope `=` (a scalar initializer). A
/// file-scope prototype, which a header spells for every function of its
/// library, is not a use: the analysis reads a callee only from a body.
fn used_identifiers(tokens: &[super::Token]) -> BTreeSet<&str> {
    let mut used = BTreeSet::new();
    let mut depth = 0usize;
    let mut initializer = false;
    for token in tokens {
        match token.text.as_str() {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            ";" if depth == 0 => initializer = false,
            "=" if depth == 0 => initializer = true,
            text => {
                if depth > 0 || initializer {
                    used.insert(text);
                }
            }
        }
    }
    used
}

/// Restrict a run-wide import table to the homes this caller may reach. A
/// name defined in the caller's preprocessed unit takes local precedence (a
/// header static always does). A name neither used in the caller's unit
/// (outside a file-scope prototype, [`used_identifiers`]) nor named as a
/// value by the run's installed member facts
/// ([`crate::member_run_facts::RunMemberFacts::named_functions`]) is never
/// looked up by the caller's analysis, so its home is not held and not
/// replayed. Holding fewer imports can only leave a call uncontracted, never
/// supply a summary (fail-closed); a caller that may reach more than
/// [`REPLAY_BUDGET_PER_UNIT`] homes holds none.
pub fn for_caller(
    preprocessed: &str,
    run_environment: &ResourceContracts,
    run_homes: &[CrossUnitHomeEvidence],
) -> CallerImports {
    let (tokens, conditionals) = tokenize_with_conditionals(preprocessed);
    let local = function_names(&tokens, &conditionals)
        .definitions
        .into_iter()
        .collect::<BTreeSet<_>>();
    let spelled = used_identifiers(&tokens);
    let run_facts = crate::member_run_facts::installed();
    let names = run_homes
        .iter()
        .map(|home| home.function.as_str())
        .collect::<Vec<_>>();
    let named = |name: &str| {
        run_facts
            .as_ref()
            .is_some_and(|facts| facts.named_functions().contains(name))
    };
    let scope = scope(
        &names,
        |name| local.contains(name),
        |name| spelled.contains(name),
        named,
        REPLAY_BUDGET_PER_UNIT,
    );
    let named_only = names
        .iter()
        .filter(|name| !local.contains(**name) && !spelled.contains(**name) && named(name))
        .count();
    let withheld = scope.exhausted.then(|| {
        format!(
            "cross-unit replay budget exhausted: {} reachable home(s) exceed the per-unit \
             budget of {REPLAY_BUDGET_PER_UNIT}; no import held",
            scope.reachable
        )
    });
    let homes = scope
        .held
        .iter()
        .map(|&index| run_homes[index].clone())
        .collect::<Vec<_>>();
    let held = homes
        .iter()
        .map(|home| home.function.as_str())
        .collect::<BTreeSet<_>>();
    let mut environment = run_environment.clone();
    let imports = environment
        .inferred_imports()
        .iter()
        .filter(|(name, _)| held.contains(name.as_str()))
        .map(|(name, contract)| (name.clone(), contract.clone()))
        .collect();
    environment.set_inferred_imports(imports);
    environment.set_lifetime_imports(
        environment
            .lifetime_imports()
            .iter()
            .filter(|(name, _)| held.contains(name.as_str()))
            .map(|(name, summary)| (name.clone(), summary.clone()))
            .collect(),
    );
    let identities = environment
        .cross_unit_import_identities()
        .iter()
        .filter(|(name, _)| held.contains(name.as_str()))
        .map(|(name, identity)| (name.clone(), identity.clone()))
        .collect();
    environment.set_cross_unit_import_identities(identities);
    CallerImports {
        environment,
        homes,
        reachable: scope.reachable,
        named_only,
        withheld,
    }
}

/// The pure scope decision of [`for_caller`]
/// (`QCore.CrossUnitReplayBudget.held`): the indices of the homes a caller
/// holds, how many it may reach, and whether the budget is exhausted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scope {
    pub(crate) held: Vec<usize>,
    pub(crate) reachable: usize,
    pub(crate) exhausted: bool,
}

/// A home is reachable when the caller does not define its name and either
/// spells it or the run's member facts name it as a value; the caller holds
/// every reachable home when they fit the budget, and none otherwise.
pub(crate) fn scope(
    homes: &[&str],
    local: impl Fn(&str) -> bool,
    spelled: impl Fn(&str) -> bool,
    named: impl Fn(&str) -> bool,
    budget: usize,
) -> Scope {
    let reachable = homes
        .iter()
        .enumerate()
        .filter(|(_, name)| !local(name) && (spelled(name) || named(name)))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let count = reachable.len();
    let exhausted = count > budget;
    Scope {
        held: if exhausted { Vec::new() } else { reachable },
        reachable: count,
        exhausted,
    }
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
    // Card CURRENT-RUN-WORKER-SPIN: the check replays every home it is
    // given, so it refuses more than the per-unit budget before any replay.
    if homes.len() > REPLAY_BUDGET_PER_UNIT {
        return Err(format!(
            "{} cross-unit homes exceed the per-unit replay budget of {REPLAY_BUDGET_PER_UNIT}",
            homes.len()
        ));
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
        let replay = replayed(home, &scope.input_identity, &names, &tokens)?;
        let record = match replay.0.get(&home.function) {
            None | Some(Replayed::NoRow) => {
                return Err(format!("cross-unit `{}` home has no row", home.function));
            }
            Some(Replayed::NotSafe) => {
                return Err(format!("cross-unit `{}` home is not Safe", home.function));
            }
            Some(Replayed::NoSummary) => {
                return Err(format!(
                    "cross-unit `{}` home has no validated W4 summary",
                    home.function
                ));
            }
            Some(Replayed::EffectsDiffer) => {
                return Err(format!(
                    "cross-unit `{}` lifetime effects differ from its Safe row",
                    home.function
                ));
            }
            Some(Replayed::Valid { record, removals }) => (record, removals),
        };
        let (record, removals) = record;
        if validated.contains_key(&home.function)
            || validated_lifetimes.contains_key(&home.function)
        {
            return Err(format!("cross-unit `{}` has multiple homes", home.function));
        }
        if !record.lifetime_removals.is_empty() {
            match removals {
                RemovalTrace::NoFormals => {
                    return Err("removal home has no formal inventory".into());
                }
                RemovalTrace::Differs => {
                    return Err(
                        "cross-unit ordered removal trace differs from replayed home".into(),
                    );
                }
                RemovalTrace::Agrees => {}
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

/// Card CURRENT-RUN-WORKER-SPIN: what [`check_imports`] reads of one
/// function in a replayed home unit.
#[derive(Clone, Debug)]
enum Replayed {
    NoRow,
    NotSafe,
    NoSummary,
    EffectsDiffer,
    Valid {
        record: qcore_schemas::contract::ResourceSummaryRecord,
        removals: RemovalTrace,
    },
}

/// Whether a W4 record's ordered removals equal the replayed row's trace.
#[derive(Clone, Copy, Debug)]
enum RemovalTrace {
    Agrees,
    NoFormals,
    Differs,
}

/// A home unit's replay: what each function reads, and what the replay
/// recorded (for [`crate::contract_dependencies::absorb_record`]).
type ReplayTable = Result<
    Arc<(
        BTreeMap<String, Replayed>,
        crate::contract_dependencies::UnitRecord,
    )>,
    String,
>;

/// Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): the replay of one
/// home unit, run once per process and read by every check that holds one
/// of its homes.
///
/// The replay reads only the home's preprocessed source, its environment,
/// the selected names re-derived from its raw and marked source, its dialect
/// and its request. The key is the identity this check recomputed from the
/// home's own bytes (`input`: raw, preprocessed and scope-evidence digests
/// and the request digest), the environment digest this check recomputed,
/// the dialect and the names, so equal keys are equal inputs (SHA-256).
/// The requested entry fills only the summary's `entry`, which no check
/// reads. The replay is recorded once in a fresh recording
/// ([`crate::contract_dependencies::captured`]) and that record is written
/// into the caller's recording at every check that reads it, so a caller's
/// rule 13f dependencies and evidence are what a replay inside its own
/// recording wrote.
fn replayed(
    home: &CrossUnitHomeEvidence,
    input: &LegacyCertificateInputIdentity,
    names: &[String],
    tokens: &[super::Token],
) -> ReplayTable {
    static REPLAYS: OnceLock<Mutex<HashMap<String, Arc<OnceLock<ReplayTable>>>>> = OnceLock::new();
    let key = qcore_schemas::sha256_hex(
        format!(
            "{input:?}\0{}\0{:?}\0{names:?}",
            home.environment_sha256, home.dialect
        )
        .as_bytes(),
    );
    let cell = {
        let mut table = REPLAYS
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        table.entry(key).or_default().clone()
    };
    let table = cell.get_or_init(|| {
        let (replay, record) = crate::contract_dependencies::captured(|| {
            let result = super::analyze_legacy_c_translation_unit_selected_partial(
                &home.preprocessed_source,
                &home.function,
                &home.environment,
                Some(names),
                (home.dialect, Some(&home.request)),
            )
            .map_err(|error| format!("cross-unit home cannot be reanalysed: {error}"))?;
            let functions = names
                .iter()
                .map(|name| {
                    let reading = match result.summary.functions.get(name) {
                        None => Replayed::NoRow,
                        Some(row)
                            if row.status != AnalysisStatus::Safe
                                || !row.diagnostics.is_empty() =>
                        {
                            Replayed::NotSafe
                        }
                        Some(row) => {
                            match result
                                .summaries
                                .iter()
                                .find(|record| &record.function == name)
                            {
                                None => Replayed::NoSummary,
                                Some(record)
                                    if record
                                        .lifetime_effects
                                        .iter()
                                        .cloned()
                                        .collect::<BTreeSet<_>>()
                                        != row.lifetime_effects =>
                                {
                                    Replayed::EffectsDiffer
                                }
                                Some(record) => Replayed::Valid {
                                    removals: if record.lifetime_removals.is_empty() {
                                        RemovalTrace::Agrees
                                    } else {
                                        match super::function_parameters(tokens, name) {
                                            None => RemovalTrace::NoFormals,
                                            Some(parameters)
                                                if crate::legacy_summary::ordered_removals(
                                                    &row.cfg,
                                                    &parameters,
                                                )
                                                .as_ref()
                                                    == Some(&record.lifetime_removals) =>
                                            {
                                                RemovalTrace::Agrees
                                            }
                                            Some(_) => RemovalTrace::Differs,
                                        }
                                    },
                                    record: record.clone(),
                                },
                            }
                        }
                    };
                    (name.clone(), reading)
                })
                .collect::<BTreeMap<_, _>>();
            Ok::<_, String>(functions)
        });
        replay.map(|functions| Arc::new((functions, record)))
    });
    let table = table.clone()?;
    crate::contract_dependencies::absorb_record(&table.1);
    Ok(table)
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

/// `SCOPE_VECTORS` (`QCore.CrossUnitReplayBudget.scopeVectors`): run homes,
/// the caller's local definitions, its spellings, the run's member-fact
/// names and the budget, then the held homes, the reachable count and
/// whether the budget is exhausted.
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub(crate) const SCOPE_VECTORS: &[(
    &[&str],
    &[&str],
    &[&str],
    &[&str],
    usize,
    &[&str],
    usize,
    bool,
)] = &[
    (&["a", "b", "c"], &[], &["a", "x"], &[], 4, &["a"], 1, false),
    (
        &["a", "b", "c"],
        &[],
        &["a"],
        &["c"],
        4,
        &["a", "c"],
        2,
        false,
    ),
    (&["a", "b"], &["a"], &["a", "b"], &[], 4, &["b"], 1, false),
    (&["a", "b"], &[], &["a", "b"], &[], 2, &["a", "b"], 2, false),
    (
        &["a", "b", "c"],
        &[],
        &["a", "b", "c"],
        &[],
        2,
        &[],
        3,
        true,
    ),
    (&["a"], &[], &[], &[], 0, &[], 0, false),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_vectors_hold() {
        for (homes, local, spelled, named, budget, held, reachable, exhausted) in SCOPE_VECTORS {
            let scope = scope(
                homes,
                |name| local.contains(&name),
                |name| spelled.contains(&name),
                |name| named.contains(&name),
                *budget,
            );
            let names = scope
                .held
                .iter()
                .map(|&index| homes[index])
                .collect::<Vec<_>>();
            assert_eq!(names, *held, "{homes:?} {spelled:?} {named:?}");
            assert_eq!(scope.reachable, *reachable);
            assert_eq!(scope.exhausted, *exhausted);
            assert!(scope.held.len() <= *budget);
        }
        assert_eq!(SCOPE_VECTORS.len(), 6);
    }
}
