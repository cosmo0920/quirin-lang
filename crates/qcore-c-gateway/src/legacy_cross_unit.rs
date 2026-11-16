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
/// caller's import work is bounded by this many home analyses per check.
/// Card **LINK-STANDING-IMPORT-LIMITERS** (rule 4c, amended): a caller that
/// may reach more holds a subset of this many ([`select`]); its calls to the
/// withheld names stay uncontracted (`Unknown`), and
/// [`CallerImports::withheld`] names them.
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
    /// Card LINK-STANDING-IMPORT-LIMITERS (rule 40): member-named homes left
    /// out of `reachable` under a derived resolution because their name has
    /// no token in the caller's unit ([`for_linked_caller`]). Not held, not
    /// probed, not counted toward the budget and not a caller refusal.
    pub unprobed: usize,
    /// Set when `reachable` exceeds [`REPLAY_BUDGET_PER_UNIT`]: the caller
    /// then holds the budget's subset ([`select`]) and this names the
    /// withheld homes.
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

/// Card **LINK-STANDING-IMPORT-LIMITERS** (rule 4c): each function the unit
/// defines (`NAME ( ... ) {` at file scope) with the identifiers its body
/// uses, closed over the unit's own functions it names: what the function's
/// analysis can look up directly or through a local callee's summary. Read
/// only to order the budget's subset ([`select`]); a function this scan
/// misses (a K&R definition, an attribute after the declarator) leaves its
/// homes to the fill by name.
fn function_uses(tokens: &[super::Token]) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut direct = BTreeMap::<&str, BTreeSet<&str>>::new();
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < tokens.len() {
        let text = tokens[index].text.as_str();
        match text {
            "{" => depth += 1,
            "}" => depth = depth.saturating_sub(1),
            _ if depth == 0
                && is_identifier(text)
                && !matches!(text, "if" | "while" | "for" | "switch")
                && tokens.get(index + 1).map(|next| next.text.as_str()) == Some("(") =>
            {
                if let Some(close) = super::matching(tokens, index + 1, "(", ")")
                    && tokens.get(close + 1).map(|next| next.text.as_str()) == Some("{")
                    && let Some(end) = super::matching(tokens, close + 1, "{", "}")
                {
                    direct.entry(text).or_default().extend(
                        tokens[close + 2..end]
                            .iter()
                            .map(|token| token.text.as_str())
                            .filter(|token| is_identifier(token)),
                    );
                    index = end + 1;
                    continue;
                }
            }
            _ => {}
        }
        index += 1;
    }
    direct
        .keys()
        .map(|&function| {
            let mut seen = BTreeSet::from([function]);
            let mut stack = vec![function];
            let mut uses = BTreeSet::new();
            while let Some(next) = stack.pop() {
                for &name in &direct[next] {
                    uses.insert(name);
                    if direct.contains_key(name) && seen.insert(name) {
                        stack.push(name);
                    }
                }
            }
            (function, uses)
        })
        .collect()
}

fn is_identifier(text: &str) -> bool {
    text.bytes()
        .next()
        .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic())
        && text
            .bytes()
            .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
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
/// [`REPLAY_BUDGET_PER_UNIT`] homes holds the budget's subset ([`select`]).
pub fn for_caller(
    preprocessed: &str,
    run_environment: &ResourceContracts,
    run_homes: &[CrossUnitHomeEvidence],
) -> CallerImports {
    caller_scope(preprocessed, run_environment, run_homes, false)
}

/// Card **LINK-STANDING-IMPORT-LIMITERS** (rule 40): [`for_caller`] under a
/// derived resolution. A home only the run's member facts name, whose name
/// has no token anywhere in the caller's preprocessed unit, leaves the scope
/// before the budget: the unit never declares the name, so Clang gives it
/// no type there and condition (14) cannot hold. Leaving it out holds fewer
/// imports (fail-closed); it is counted in [`CallerImports::unprobed`]. A
/// name the unit declares or uses stays in scope, and rule 40 still refuses
/// it when it cannot be typed.
pub fn for_linked_caller(
    preprocessed: &str,
    run_environment: &ResourceContracts,
    run_homes: &[CrossUnitHomeEvidence],
) -> CallerImports {
    caller_scope(preprocessed, run_environment, run_homes, true)
}

fn caller_scope(
    preprocessed: &str,
    run_environment: &ResourceContracts,
    run_homes: &[CrossUnitHomeEvidence],
    linked: bool,
) -> CallerImports {
    let (tokens, conditionals) = tokenize_with_conditionals(preprocessed);
    let local = function_names(&tokens, &conditionals)
        .definitions
        .into_iter()
        .collect::<BTreeSet<_>>();
    let spelled = used_identifiers(&tokens);
    let present = tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<BTreeSet<_>>();
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
    let groups = function_uses(&tokens)
        .into_values()
        .zip(0..)
        .map(|(uses, order)| (order, uses))
        .collect::<Vec<_>>();
    let scope = scope(
        &names,
        &Facts {
            local: &|name| local.contains(name),
            spelled: &|name| spelled.contains(name),
            named: &named,
            present: &|name| !linked || present.contains(name),
        },
        &groups,
        REPLAY_BUDGET_PER_UNIT,
    );
    let named_only = scope
        .reachable
        .iter()
        .filter(|&&index| !spelled.contains(names[index]))
        .count();
    let withheld = scope.exhausted.then(|| {
        let held = scope.held.iter().copied().collect::<BTreeSet<_>>();
        let withheld = scope
            .reachable
            .iter()
            .filter(|index| !held.contains(index))
            .map(|&index| names[index])
            .collect::<BTreeSet<_>>();
        format!(
            "cross-unit replay budget exceeded: {} reachable home(s) exceed the per-unit \
             budget of {REPLAY_BUDGET_PER_UNIT}; {} held by whole function sets, then by \
             name; withheld: {}",
            scope.reachable.len(),
            scope.held.len(),
            withheld.into_iter().collect::<Vec<_>>().join(", ")
        )
    });
    let homes = scope
        .held
        .iter()
        .map(|&index| run_homes[index].clone())
        .collect::<Vec<_>>();
    let environment = restrict_environment(run_environment.clone(), &homes);
    CallerImports {
        environment,
        homes,
        reachable: scope.reachable.len(),
        named_only,
        unprobed: scope.unprobed,
        withheld,
    }
}

/// Card **LINK-STANDING** (rule 40): drop every held home `keep` refuses,
/// with its import, lifetime import and expected identity. Holding fewer
/// homes never supplies a summary (fail-closed); the scope's counts are
/// unchanged.
pub fn retain_held(
    imports: CallerImports,
    keep: impl Fn(&CrossUnitHomeEvidence) -> bool,
) -> CallerImports {
    let CallerImports {
        environment,
        homes,
        reachable,
        named_only,
        unprobed,
        withheld,
    } = imports;
    let homes = homes
        .into_iter()
        .filter(|home| keep(home))
        .collect::<Vec<_>>();
    CallerImports {
        environment: restrict_environment(environment, &homes),
        homes,
        reachable,
        named_only,
        unprobed,
        withheld,
    }
}

/// The environment with only the imports, lifetime imports and identities
/// of `homes`.
fn restrict_environment(
    mut environment: ResourceContracts,
    homes: &[CrossUnitHomeEvidence],
) -> ResourceContracts {
    let held = homes
        .iter()
        .map(|home| home.function.as_str())
        .collect::<BTreeSet<_>>();
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
    environment
}

/// The pure scope decision of [`for_caller`]
/// (`QCore.CrossUnitReplayBudget.held`): the indices of the homes a caller
/// holds, the reachable ones, how many member-named homes were left out
/// for want of a token, and whether the reachable homes exceed the budget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scope {
    pub(crate) held: Vec<usize>,
    pub(crate) reachable: Vec<usize>,
    pub(crate) unprobed: usize,
    pub(crate) exhausted: bool,
}

/// What the scope reads of one caller, per home name.
pub(crate) struct Facts<'a> {
    /// The caller defines the name.
    pub(crate) local: &'a dyn Fn(&str) -> bool,
    /// The caller uses the name ([`used_identifiers`]).
    pub(crate) spelled: &'a dyn Fn(&str) -> bool,
    /// The run's member facts name it as a value.
    pub(crate) named: &'a dyn Fn(&str) -> bool,
    /// The name can be probed in the caller: always, except under a derived
    /// resolution, where its token must occur in the unit.
    pub(crate) present: &'a dyn Fn(&str) -> bool,
}

/// A home is reachable when the caller does not define its name and either
/// spells it or the run's member facts name it as a value and it can be
/// probed in the caller. The caller holds every reachable home when they fit
/// the budget and otherwise the subset [`select`] chooses.
pub(crate) fn scope(
    homes: &[&str],
    facts: &Facts<'_>,
    groups: &[(usize, BTreeSet<&str>)],
    budget: usize,
) -> Scope {
    let mut reachable = Vec::new();
    let mut unprobed = 0;
    for (index, name) in homes.iter().enumerate() {
        if (facts.local)(name) {
            continue;
        }
        if (facts.spelled)(name) {
            reachable.push(index);
        } else if (facts.named)(name) {
            if (facts.present)(name) {
                reachable.push(index);
            } else {
                unprobed += 1;
            }
        }
    }
    let exhausted = reachable.len() > budget;
    Scope {
        held: select(homes, &reachable, groups, budget),
        reachable,
        unprobed,
        exhausted,
    }
}

/// Card **LINK-STANDING-IMPORT-LIMITERS** (rule 4c,
/// `QCore.CrossUnitReplayBudget.select`): the homes a caller holds of its
/// reachable ones. Within the budget, all of them. Above it, at most
/// `budget`: each function's reachable home set (`groups`, keyed by the
/// function's position in name order), smallest first with the function
/// order breaking ties, is held whole when the union still fits; then the
/// remaining reachable homes by name while room is left. The result depends
/// only on the sets and names, never on the order `groups` is given in.
/// Every subset is fail-closed: a withheld home's calls stay uncontracted.
pub(crate) fn select(
    homes: &[&str],
    reachable: &[usize],
    groups: &[(usize, BTreeSet<&str>)],
    budget: usize,
) -> Vec<usize> {
    if reachable.len() <= budget {
        return reachable.to_vec();
    }
    let index_of = reachable
        .iter()
        .map(|&index| (homes[index], index))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = groups
        .iter()
        .map(|(order, uses)| {
            let set = uses
                .iter()
                .filter_map(|name| index_of.get(name).copied())
                .collect::<BTreeSet<_>>();
            (set.len(), *order, set)
        })
        .filter(|(size, _, _)| *size > 0)
        .collect::<Vec<_>>();
    ordered.sort();
    let mut held = Vec::new();
    let mut chosen = BTreeSet::new();
    for (_, _, set) in ordered {
        let added = set
            .iter()
            .filter(|index| !chosen.contains(*index))
            .copied()
            .collect::<Vec<_>>();
        if chosen.len() + added.len() <= budget {
            chosen.extend(added.iter().copied());
            held.extend(added);
        }
    }
    for &index in index_of.values() {
        if held.len() >= budget {
            break;
        }
        if chosen.insert(index) {
            held.push(index);
        }
    }
    held
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
        record: Box<qcore_schemas::contract::ResourceSummaryRecord>,
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
                                    record: Box::new(record.clone()),
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

/// One `SCOPE_VECTORS` row: run homes (in name order), the caller's local
/// definitions, its spellings, the run's member-fact names, whether the run
/// is derived (rule 40) and the names with a token in the unit, the
/// per-function home sets (by function position) and the budget; then the
/// held homes, the reachable count, the unprobed count and whether the
/// budget is exceeded.
#[cfg(test)]
pub(crate) type ScopeVector = (
    &'static [&'static str],
    &'static [&'static str],
    &'static [&'static str],
    &'static [&'static str],
    bool,
    &'static [&'static str],
    &'static [&'static [&'static str]],
    usize,
    &'static [&'static str],
    usize,
    usize,
    bool,
);

/// `SCOPE_VECTORS` (`QCore.CrossUnitReplayBudget.scopeVectors`).
#[cfg(test)]
pub(crate) const SCOPE_VECTORS: &[ScopeVector] = &[
    // spelled homes are held; an unspelled, unnamed home is not
    (
        &["a", "b", "c"],
        &[],
        &["a", "x"],
        &[],
        false,
        &[],
        &[],
        4,
        &["a"],
        1,
        0,
        false,
    ),
    // a member-fact name is held though unspelled
    (
        &["a", "b", "c"],
        &[],
        &["a"],
        &["c"],
        false,
        &[],
        &[],
        4,
        &["a", "c"],
        2,
        0,
        false,
    ),
    // a local definition takes precedence over its home
    (
        &["a", "b"],
        &["a"],
        &["a", "b"],
        &[],
        false,
        &[],
        &[],
        4,
        &["b"],
        1,
        0,
        false,
    ),
    // exactly at the budget, every reachable home is held
    (
        &["a", "b"],
        &[],
        &["a", "b"],
        &[],
        false,
        &[],
        &[],
        2,
        &["a", "b"],
        2,
        0,
        false,
    ),
    // above the budget: the smaller whole function set first, then by name
    (
        &["a", "b", "c"],
        &[],
        &["a", "b", "c"],
        &[],
        false,
        &[],
        &[&["a", "b"], &["c"]],
        2,
        &["c", "a"],
        3,
        0,
        true,
    ),
    // no home reachable
    (&["a"], &[], &[], &[], false, &[], &[], 0, &[], 0, 0, false),
    // derived run: a member-named home with no token in the unit is unprobed
    (
        &["a", "b", "c"],
        &[],
        &["a"],
        &["b", "c"],
        true,
        &["a", "b"],
        &[],
        4,
        &["a", "b"],
        2,
        1,
        false,
    ),
    // derived run: the unprobed homes do not count toward the budget
    (
        &["a", "b", "c"],
        &[],
        &["a"],
        &["b", "c"],
        true,
        &["a"],
        &[],
        1,
        &["a"],
        1,
        2,
        false,
    ),
    // equal-size function sets: the earlier function wins the tie
    (
        &["a", "b", "c", "d"],
        &[],
        &["a", "b", "c", "d"],
        &[],
        false,
        &[],
        &[&["c", "d"], &["a", "b"]],
        2,
        &["c", "d"],
        4,
        0,
        true,
    ),
    // a function set larger than the budget is never held whole
    (
        &["a", "b", "c", "d"],
        &[],
        &["a", "b", "c", "d"],
        &[],
        false,
        &[],
        &[&["a", "b", "c"], &["d"]],
        2,
        &["d", "a"],
        4,
        0,
        true,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn run(vector: &ScopeVector) -> Scope {
        let (homes, local, spelled, named, linked, present, groups, budget, ..) = *vector;
        let groups = groups
            .iter()
            .enumerate()
            .map(|(order, uses)| (order, uses.iter().copied().collect::<BTreeSet<_>>()))
            .collect::<Vec<_>>();
        scope(
            homes,
            &Facts {
                local: &|name| local.contains(&name),
                spelled: &|name| spelled.contains(&name),
                named: &|name| named.contains(&name),
                present: &|name| !linked || present.contains(&name),
            },
            &groups,
            budget,
        )
    }

    #[test]
    fn scope_vectors_hold() {
        for vector in SCOPE_VECTORS {
            let (homes, .., budget, held, reachable, unprobed, exhausted) = *vector;
            let scope = run(vector);
            let names = scope
                .held
                .iter()
                .map(|&index| homes[index])
                .collect::<Vec<_>>();
            assert_eq!(names, held, "{vector:?}");
            assert_eq!(scope.reachable.len(), reachable, "{vector:?}");
            assert_eq!(scope.unprobed, unprobed, "{vector:?}");
            assert_eq!(scope.exhausted, exhausted, "{vector:?}");
            assert!(scope.held.len() <= budget);
            assert!(
                scope
                    .held
                    .iter()
                    .all(|index| scope.reachable.contains(index))
            );
        }
        assert_eq!(SCOPE_VECTORS.len(), 10);
    }

    /// Card LINK-STANDING-IMPORT-LIMITERS: the subset depends on the sets and
    /// names, not on the order the function sets are given in.
    #[test]
    fn selection_is_deterministic() {
        let homes = ["a", "b", "c", "d", "e"];
        let reachable = (0..homes.len()).collect::<Vec<_>>();
        let sets = [
            (0, BTreeSet::from(["d", "e"])),
            (1, BTreeSet::from(["a"])),
            (2, BTreeSet::from(["b", "c", "d"])),
        ];
        let forward = select(&homes, &reachable, &sets, 3);
        let mut reversed = sets.clone();
        reversed.reverse();
        assert_eq!(select(&homes, &reachable, &reversed, 3), forward);
        let names = forward
            .iter()
            .map(|&index| homes[index])
            .collect::<Vec<_>>();
        assert_eq!(names, ["a", "d", "e"]);
    }

    /// Card LINK-STANDING-IMPORT-LIMITERS: each defined function's uses,
    /// closed over the unit's own callees; a prototype is not a definition.
    #[test]
    fn function_uses_close_over_local_callees() {
        let (tokens, _) = tokenize_with_conditionals(
            "int h1(int); int h2(int);\n\
             static int helper(int x) { return h2(x); }\n\
             int outer(int x) { return helper(h1(x)); }\n\
             int alone(void) { return 0; }\n",
        );
        let uses = function_uses(&tokens);
        assert_eq!(uses.len(), 3);
        assert!(uses["helper"].contains("h2") && !uses["helper"].contains("h1"));
        assert!(uses["outer"].contains("h1") && uses["outer"].contains("h2"));
        assert!(!uses["alone"].contains("h1"));
    }
}
