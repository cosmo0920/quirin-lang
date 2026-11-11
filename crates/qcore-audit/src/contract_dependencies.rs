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

//! Card **VERDICT-CONTRACT-DEPENDENCIES** (gateway rules 13f–13h): every
//! published row's contract dependencies, and the run's dependency section.
//!
//! # The row field (rule 13f)
//!
//! Each function row of a unit gains `contract_dependencies`: the declared
//! names its derivation consulted
//! ([`qcore_c_gateway::contract_dependencies::row_dependencies`]), split by
//! their standing at the consultation (`declarations` in force, `held`,
//! `withdrawn`) with the in-unit `summaries` whose own dependencies were
//! added. The unit row gains `contract_body_evidence` when the unit examined
//! the body of a declared function: what each body made of its declaration.
//! Both keys are written after the unit's verdict is settled, from a record
//! no analysis reads, so the row's status and diagnostics are those it had
//! without them.
//!
//! # The section (rule 13h)
//!
//! The contract sidecar's `contract_dependencies` section is a fold of the
//! rows ([`Facts`]) under the `contract_revision` inventory and the declared
//! premises: per declaration its digest, mode and status, its **body
//! standing** for the run, its premises and how many rows (and `Safe` rows)
//! consulted it; and the reviewer's roll-up of the `Safe` rows. A
//! declaration's evidence standing is `validated` only when every unit that
//! examined one of its bodies validated it and at least one did; one held for
//! a human act is `held`; every other is a `declared-assumption` (contract
//! specification C06): no body, or a body that did not validate it.
//!
//! # Per clause (card VERDICT-BODY-STANDING-CLAUSE-SCOPE, rules 13i–13k)
//!
//! Evidence validates the clauses it covers, read clause by clause by
//! [`qcore_c_gateway::contract_clause_standing`]. The unit field carries, for
//! every declaration the unit validated, each clause with the evidence that
//! covered it there (`clauses`). A clause is `validated` for the run when the
//! declaration's evidence standing is `validated` and the clause was covered in
//! every unit that validated the declaration; the declaration is `validated`
//! only when **every** clause is, `partly-validated` when some but not all are
//! (its `uncovered_clauses` listed), and otherwise a `declared-assumption`
//! ([`clause_standing_of`]). The roll-up's "only validated" bucket counts rows
//! whose every declaration is fully validated; a row resting on a
//! partly-validated declaration and no declared assumption is counted apart.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use qcore_c_gateway::contract_dependencies::{RowDependencies, UnitRecord, declared};
use qcore_c_gateway::legacy_c::{LegacyProgramSummary, UnrestrictedLegacyResult};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::retained_by::RetentionHolder;
use serde_json::{Value, json};

use crate::report::AuditReport;

/// The row key.
pub const ROW_KEY: &str = "contract_dependencies";

/// The unit key.
pub const UNIT_KEY: &str = "contract_body_evidence";

/// The sidecar key of the section, and of the premises a checkpointed run
/// records before its units.
pub const SECTION_KEY: &str = "contract_dependencies";
pub const PREMISES_KEY: &str = "contract_premises";

/// What the section means, carried in the section itself.
pub const MEANING: &str = "the contract declarations each published row's derivation consulted (rule 13f): every name a lookup of the row's own work asked for, every callee its solved CFG applies a contract of, and transitively those of every in-unit summary it consulted; over-approximated, never omitted. `body_standing` (rules 13g, 13i-13k): evidence covers the clauses it compares or reads (an equal W4 summary, and the body under the declaration's entry custody matching every exit and Safe, on a reported row (`entry`) or on a body whose W4 summary only refines the declaration (rule 13m, which then records no conflict failure): every parameter and result mode with its kind; rule 11r: every exit class; 12w: the guards of retentions; 13a: slot moves, derived out slots and a borrowed result; 13c: reference keeps and drops; 13d: a returned reference); no evidence covers a container or counted-user clause, a retention, a holder reset, a named premise, a variadic tail or a no-return clause. A clause is `validated` where every unit that examined a body of the declared function validated the declaration, at least one did, and every such unit's evidence covered the clause; a declaration is `validated` only when every clause is, `partly-validated` when some but not all are (`uncovered_clauses`), `held` when held for a human act, and otherwise `declared-assumption` (contract specification C06): there is no analysed body, a body did not validate it, or no clause is covered everywhere. `resting_only_on_validated_declarations` counts Safe rows whose every declaration is fully validated. Premises are the named assumptions a declaration states (`resource_premise`), its literal-argument premises, and the premises a rule states for its clause (rule 13e for a result holder). A row's `null_premises` (rule 14n) are the named null premises (`null-deref:FUNCTION:LINE:VARIABLE`) its derivation and its in-unit summaries' relied on, decision 1's moved NULL question; `resting_only_on_a_null_premise` counts Safe rows with no declaration and at least one, `carrying_a_null_premise` every Safe row with one, and neither is ever counted as resting on nothing. Evidence only: no verdict reads this section or the row fields";

/// The premises rule 13e states for a declaration whose result holds a
/// retained pointer.
const RESULT_HOLDER_PREMISES: [(&str, &str); 2] = [
    (
        "P-RESULT-HOLDER-LAST-DROP",
        "every reference the function could see on an object it obtained itself is dropped; a reference a callee took through a position declared resource_borrowed is that declaration's assumption (rule 13e)",
    ),
    (
        "P-RETAINED-PREDEFINED",
        "__func__, __FUNCTION__ and __PRETTY_FUNCTION__ at a retained position are static storage (C11 6.4.2.2p1; rule 13e)",
    ),
];

/// Card MK-EVENT-DESCRIPTOR-QUANT (rule 38): the declarations carrying a
/// descriptor-cell state.
fn cell_declarations(contracts: &ResourceContracts) -> BTreeSet<String> {
    contracts
        .retaining()
        .iter()
        .filter(|(_, declaration)| !declaration.descriptor_cells.states.is_empty())
        .map(|(name, _)| name.clone())
        .collect()
}

/// The analysed summary of a unit result, when it has rows.
fn summary_of(
    result: &Result<UnrestrictedLegacyResult, impl Sized>,
) -> Option<&LegacyProgramSummary> {
    match result {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => Some(&certificate.summary),
        Ok(UnrestrictedLegacyResult::Unsafe(summary)) => Some(summary),
        Ok(UnrestrictedLegacyResult::Unknown { partial, .. }) => partial.as_ref(),
        Err(_) => None,
    }
}

fn names(set: &BTreeSet<String>) -> Value {
    json!(set.iter().collect::<Vec<_>>())
}

/// The row field of one row's dependencies: `declarations` always, the
/// other lists only when nonempty.
#[must_use]
pub fn row_field(dependencies: &RowDependencies) -> Value {
    let mut field = json!({ "declarations": names(&dependencies.declarations) });
    if !dependencies.held.is_empty() {
        field["held"] = names(&dependencies.held);
    }
    if !dependencies.withdrawn.is_empty() {
        field["withdrawn"] = names(&dependencies.withdrawn);
    }
    if !dependencies.summaries.is_empty() {
        field["summaries"] = names(&dependencies.summaries);
    }
    // Card NULL-SAFETY-PROPERTY (rule 14n): the named null premises the row
    // rests on (its own and its in-unit summaries'), only when there is one.
    if !dependencies.null_premises.is_empty() {
        field[NULL_PREMISES_KEY] = names(&dependencies.null_premises);
    }
    if !dependencies.lifetime_members.is_empty() {
        field[LIFETIME_MEMBERS_KEY] = names(&dependencies.lifetime_members);
    }
    if !dependencies.lifetime_prerequisites.is_empty() {
        field[LIFETIME_PREREQUISITES_KEY] = names(&dependencies.lifetime_prerequisites);
    }
    if !dependencies.lifetime_effects.is_empty() {
        field[LIFETIME_EFFECTS_KEY] = serde_json::to_value(&dependencies.lifetime_effects)
            .expect("lifetime effects contain only JSON data");
        field[LIFETIME_ROOTS_KEY] = names(&dependencies.lifetime_roots);
    }
    field
}

pub const LIFETIME_ROOTS_KEY: &str = "lifetime_roots";
pub const LIFETIME_EFFECTS_KEY: &str = "lifetime_effects";
pub const LIFETIME_PREREQUISITES_KEY: &str = "lifetime_prerequisites";

/// S3 member/stability facts. Loaded code must be covered by a kept envelope.
pub const LIFETIME_MEMBERS_KEY: &str = "lifetime_members";

/// The row field's key of the named null premises (rule 14n).
pub const NULL_PREMISES_KEY: &str = "null_premises";

/// Card OUTER-CONTRACT (rule 31): the row field's key of the run-set member
/// keys (`TAG.FIELD`) a declaration the row rests on invokes (rule 12n): the
/// closed-world facts a runtime projection reads.
pub const RUN_SET_MEMBERS_KEY: &str = "run_set_members";

/// Card OUTER-CONTRACT (rule 31): the row field's key of the member keys a
/// constructor the row rests on states a result state for (rule 14e), kept
/// only because the run's stores keep them (premise P-OPCTOR-STORES-SEEN).
pub const STATED_MEMBERS_KEY: &str = "stated_members";

/// Add the closed-world member keys of the declarations `dependencies` names
/// to the row field, each list only when nonempty (a row resting on none
/// keeps its bytes).
fn add_member_keys(
    field: &mut Value,
    dependencies: &RowDependencies,
    invoked: &BTreeMap<String, BTreeSet<String>>,
    constructed: &BTreeMap<String, BTreeMap<String, bool>>,
) {
    let mut run_set = BTreeSet::new();
    let mut stated = BTreeSet::new();
    for name in dependencies
        .declarations
        .iter()
        .chain(&dependencies.held)
        .chain(&dependencies.withdrawn)
    {
        if let Some(members) = invoked.get(name) {
            run_set.extend(members.iter().cloned());
        }
        if let Some(members) = constructed.get(name) {
            stated.extend(members.keys().cloned());
        }
    }
    if !run_set.is_empty() {
        field[RUN_SET_MEMBERS_KEY] = names(&run_set);
    }
    if !stated.is_empty() {
        field[STATED_MEMBERS_KEY] = names(&stated);
    }
}

/// The unit field: what each examined body made of its declaration, for the
/// declared names only; `None` when the unit examined none.
#[must_use]
pub fn unit_field(record: &UnitRecord, contracts: &ResourceContracts) -> Option<Value> {
    let mut validated = serde_json::Map::new();
    let mut unvalidated = Vec::new();
    let mut failed = serde_json::Map::new();
    for (name, outcome) in record.body_outcomes() {
        if !declared(contracts, &name) {
            continue;
        }
        match outcome {
            qcore_c_gateway::contract_dependencies::BodyOutcome::Validated => {
                validated.insert(name.clone(), json!(record.validated[&name]));
            }
            qcore_c_gateway::contract_dependencies::BodyOutcome::Unvalidated => {
                unvalidated.push(name);
            }
            qcore_c_gateway::contract_dependencies::BodyOutcome::Failed => {
                failed.insert(name.clone(), json!(record.failed[&name]));
            }
        }
    }
    let withdrawn = record
        .withdrawn
        .iter()
        .filter(|(name, _)| declared(contracts, name))
        .map(|(name, why)| (name.clone(), json!(why)))
        .collect::<serde_json::Map<_, _>>();
    if validated.is_empty() && unvalidated.is_empty() && failed.is_empty() && withdrawn.is_empty() {
        return None;
    }
    let mut field = json!({
        "validated": validated,
        "unvalidated": unvalidated,
        "failed": failed,
        "withdrawn": withdrawn,
    });
    // Card VERDICT-BODY-STANDING-CLAUSE-SCOPE (rule 13j): each validated
    // declaration's clauses, with the evidence that covered each here.
    let coverage = qcore_c_gateway::contract_clause_standing::unit_coverage(record, contracts);
    if !coverage.is_empty() {
        field[CLAUSES_KEY] = json!(coverage);
    }
    Some(field)
}

/// The unit field's key of the per-clause coverage (rule 13j).
pub const CLAUSES_KEY: &str = "clauses";

/// Cached derived contract artifacts for a [`ResourceContracts`] instance so
/// per-unit annotation and reading-site binding reuse the parsed premises,
/// hypothesis graphs, and callback-member declarations across units of a
/// slice instead of re-parsing the contract corpus on every translation unit.
struct CachedContractPremises {
    assumptions: BTreeMap<String, String>,
    result_holders: BTreeSet<String>,
    invoked: BTreeMap<String, BTreeSet<String>>,
    constructed: BTreeMap<String, BTreeMap<String, bool>>,
    premises: Value,
    graphs: Arc<BTreeMap<String, qcore_schemas::hypothesis::Graph>>,
    replayed_graphs: Arc<BTreeMap<String, qcore_schemas::hypothesis::Graph>>,
}

static PREMISES_CACHE: OnceLock<Mutex<BTreeMap<u64, Arc<CachedContractPremises>>>> =
    OnceLock::new();

fn premises_cache() -> &'static Mutex<BTreeMap<u64, Arc<CachedContractPremises>>> {
    PREMISES_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Drops any cached contract premises and hypothesis graphs (called between
/// per-library audit slices when releasing a slice's contract environment).
pub fn clear_premises_cache() {
    if let Ok(mut guard) = premises_cache().lock() {
        guard.clear();
    }
}

fn result_holder_names(contracts: &ResourceContracts) -> BTreeSet<String> {
    contracts
        .retaining()
        .iter()
        .chain(contracts.guarded_retaining())
        .filter(|(_, retaining)| {
            retaining
                .retentions
                .iter()
                .any(|retention| retention.holder == RetentionHolder::Result)
        })
        .map(|(name, _)| name.clone())
        .collect()
}

fn cache_key(
    assumptions: &BTreeMap<String, String>,
    result_holders: &BTreeSet<String>,
    invoked: &BTreeMap<String, BTreeSet<String>>,
    constructed: &BTreeMap<String, BTreeMap<String, bool>>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    assumptions.hash(&mut hasher);
    result_holders.hash(&mut hasher);
    invoked.hash(&mut hasher);
    constructed.hash(&mut hasher);
    hasher.finish()
}

fn cached_contract_premises(
    contracts: &ResourceContracts,
) -> Result<Arc<CachedContractPremises>, String> {
    let assumptions = contracts.source_assumptions();
    let result_holders = result_holder_names(contracts);
    let invoked = crate::callback_member_fold::member_declarations(contracts);
    let constructed = crate::callback_member_fold::constructed_declarations(contracts);
    let key = cache_key(assumptions, &result_holders, &invoked, &constructed);
    if let Ok(guard) = premises_cache().lock()
        && let Some(cached) = guard.get(&key)
        && &cached.assumptions == assumptions
        && cached.result_holders == result_holders
        && cached.invoked == invoked
        && cached.constructed == constructed
    {
        return Ok(Arc::clone(cached));
    }

    let mut premises = BTreeMap::<String, Vec<Value>>::new();
    for (name, declared) in qcore_contract::contract_loader::declared_premises(contracts)? {
        premises
            .entry(name)
            .or_default()
            .extend(declared.into_iter().map(|premise| {
                json!({
                    "name": premise.name,
                    "statement": premise.statement,
                    "origin": premise.origin.as_str(),
                })
            }));
    }
    for name in &result_holders {
        premises
            .entry(name.clone())
            .or_default()
            .extend(RESULT_HOLDER_PREMISES.iter().map(|(premise, statement)| {
                json!({"name": premise, "statement": statement, "origin": "rule-13e"})
            }));
    }
    // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38): every declaration carrying a
    // descriptor-cell state publishes the rule's tracking-limit premise with
    // the rows resting on it.
    for name in cell_declarations(contracts) {
        premises.entry(name).or_default().push(json!({
            "name": qcore_schemas::descriptor_cell::NOT_FORWARDED_PREMISE,
            "statement": qcore_schemas::descriptor_cell::NOT_FORWARDED_STATEMENT,
            "origin": "rule-38",
        }));
    }
    // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): every declaration stating
    // a record's cell protocol publishes the protocol's declared-assumption
    // premise with the rows resting on it.
    for (hosts, _) in contracts.cell_protocols().into_values() {
        for name in hosts {
            premises.entry(name).or_default().push(json!({
                "name": qcore_schemas::descriptor_cell::PROTOCOL_PREMISE,
                "statement": qcore_schemas::descriptor_cell::PROTOCOL_STATEMENT,
                "origin": "rule-38c",
            }));
        }
    }
    let replayed_graphs = qcore_contract::hypothesis::replay(assumptions)?;
    let mut graphs = replayed_graphs.clone();
    for (name, records) in &premises {
        let graph =
            graphs
                .entry(name.clone())
                .or_insert_with(|| qcore_schemas::hypothesis::Graph {
                    version: 1,
                    atoms: BTreeMap::new(),
                    premises: BTreeMap::new(),
                    refinements: BTreeMap::new(),
                    requirements: Vec::new(),
                });
        for record in records {
            let record: qcore_schemas::hypothesis::ErasedPremise =
                serde_json::from_value(record.clone()).map_err(|e| e.to_string())?;
            if graph.premises.values().any(|p| p.erasure == record) {
                continue;
            }
            qcore_contract::hypothesis::append_legacy(graph, record)?;
        }
        qcore_contract::hypothesis::validate(graph)?;
    }
    let digests = graphs
        .iter()
        .map(|(name, graph)| Ok((name.clone(), qcore_contract::hypothesis::digest(graph)?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let mut value = json!(premises);
    let binding_requirements =
        qcore_contract::hypothesis_bindings::requirements(assumptions, &graphs)?;
    if !binding_requirements.is_empty() {
        value["$hypothesis_binding_requirements"] = json!(binding_requirements);
    }
    value["$hypothesis_graphs"] = json!(&graphs);
    value["$hypothesis_graph_digests"] = json!(digests);

    let cached = Arc::new(CachedContractPremises {
        assumptions: assumptions.clone(),
        result_holders,
        invoked,
        constructed,
        premises: value,
        graphs: Arc::new(graphs),
        replayed_graphs: Arc::new(replayed_graphs),
    });
    if let Ok(mut guard) = premises_cache().lock() {
        guard.insert(key, Arc::clone(&cached));
    }
    Ok(cached)
}

/// Returns the cached replayed hypothesis graphs of `contracts`' source
/// assumptions without re-parsing the contract corpus on every translation
/// unit.
///
/// # Errors
///
/// The loader's or hypothesis replay's refusal of the retained texts.
pub fn replayed_hypothesis_graphs(
    contracts: &ResourceContracts,
) -> Result<Arc<BTreeMap<String, qcore_schemas::hypothesis::Graph>>, String> {
    Ok(Arc::clone(
        &cached_contract_premises(contracts)?.replayed_graphs,
    ))
}

/// Add the row and unit fields to the unit row `row` the report just
/// recorded for `result`, from the unit's `record` under `contracts`.
pub fn annotate_unit(
    row: &mut Value,
    record: &UnitRecord,
    result: &Result<UnrestrictedLegacyResult, impl Sized>,
    contracts: &ResourceContracts,
) -> Result<(), String> {
    let Some(summary) = summary_of(result) else {
        return Ok(());
    };
    let dependencies =
        qcore_c_gateway::contract_dependencies::row_dependencies(record, summary, contracts);
    let cached = cached_contract_premises(contracts)?;
    for function in row["functions"].as_array_mut().into_iter().flatten() {
        let Some(name) = function["function"].as_str() else {
            continue;
        };
        if let Some(dependencies) = dependencies.get(name) {
            let mut field = row_field(dependencies);
            add_member_keys(
                &mut field,
                dependencies,
                &cached.invoked,
                &cached.constructed,
            );
            field["hypothesis_dependencies"] =
                serde_json::json!(qcore_c_gateway::contract_dependencies::hypothesis_closure(
                    dependencies,
                    &cached.graphs
                ));
            function[ROW_KEY] = field;
        }
    }
    if let Some(field) = unit_field(record, contracts) {
        row[UNIT_KEY] = field;
    }
    Ok(())
}

/// The premises of every declaration of `contracts`: declared and
/// literal-argument premises from the retained texts, and rule 13e's for a
/// result holder. An unreadable source set is refused, never read as none.
///
/// # Errors
///
/// The loader's refusal of the retained texts.
pub fn premises(contracts: &ResourceContracts) -> Result<Value, String> {
    Ok(cached_contract_premises(contracts)?.premises.clone())
}

/// What the section reads of the rows: a per-unit fold, so a composed
/// report's facts are the absorb of its groups' (card W45-AUDIT-CHECKPOINTS).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// Per declared name: rows consulting it, by standing at the row.
    rows: BTreeMap<String, Uses>,
    /// Per declared name: unit outcomes of its bodies, and the evidence.
    bodies: BTreeMap<String, Bodies>,
    /// `Safe` rows, each as its declarations, held and withdrawn names, and
    /// its named null premises (rule 14n).
    safe: Vec<(BTreeSet<String>, usize, usize)>,
    rows_annotated: usize,
    safe_rows: usize,
    hypothesis_contexts: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Uses {
    rows: usize,
    safe_rows: usize,
    held_rows: usize,
    withdrawn_rows: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Bodies {
    validated_units: usize,
    unvalidated_units: usize,
    failed_units: usize,
    validated_by: BTreeSet<String>,
    failed_by: BTreeSet<String>,
    withdrawn_units: usize,
    /// Card VERDICT-BODY-STANDING-CLAUSE-SCOPE (rule 13j): validated units
    /// that carried the declaration's clause coverage.
    clause_units: usize,
    /// Per clause, the validated units whose evidence covered it.
    clauses: BTreeMap<String, usize>,
}

fn strings(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

impl Facts {
    /// The facts of every unit row of `rows`.
    #[must_use]
    pub fn of_rows(rows: &[Value]) -> Self {
        let mut facts = Self::default();
        for unit in rows {
            facts.absorb_unit(unit);
        }
        facts
    }

    /// Absorb one unit row.
    pub fn absorb_unit(&mut self, unit: &Value) {
        for function in unit["functions"].as_array().into_iter().flatten() {
            let Some(field) = function.get(ROW_KEY) else {
                continue;
            };
            self.rows_annotated += 1;
            let safe = function["status"] == "Safe";
            let declarations = strings(&field["declarations"])
                .map(str::to_owned)
                .collect::<BTreeSet<_>>();
            for name in &declarations {
                let uses = self.rows.entry(name.clone()).or_default();
                uses.rows += 1;
                uses.safe_rows += usize::from(safe);
                if let Some(context) = function.get("hypothesis_context") {
                    self.hypothesis_contexts
                        .entry(name.clone())
                        .or_default()
                        .insert(context.to_string());
                }
            }
            let mut other = 0;
            for name in strings(&field["held"]) {
                let uses = self.rows.entry(name.to_owned()).or_default();
                uses.held_rows += 1;
                other += 1;
            }
            for name in strings(&field["withdrawn"]) {
                let uses = self.rows.entry(name.to_owned()).or_default();
                uses.withdrawn_rows += 1;
                other += 1;
            }
            if safe {
                self.safe_rows += 1;
                let premises = strings(&field[NULL_PREMISES_KEY]).count();
                self.safe.push((declarations, other, premises));
            }
        }
        let evidence = &unit[UNIT_KEY];
        for (name, by) in evidence["validated"].as_object().into_iter().flatten() {
            let bodies = self.bodies.entry(name.clone()).or_default();
            bodies.validated_units += 1;
            bodies.validated_by.extend(strings(by).map(str::to_owned));
            // Rule 13j: a clause counts in this unit only when some evidence
            // covered it here; a clause absent here was not covered here.
            if let Some(coverage) = evidence[CLAUSES_KEY][name].as_object() {
                bodies.clause_units += 1;
                for (clause, covering) in coverage {
                    let covered = covering.as_array().is_some_and(|by| !by.is_empty());
                    *bodies.clauses.entry(clause.clone()).or_default() += usize::from(covered);
                }
            }
        }
        for name in strings(&evidence["unvalidated"]) {
            self.bodies
                .entry(name.to_owned())
                .or_default()
                .unvalidated_units += 1;
        }
        for (name, why) in evidence["failed"].as_object().into_iter().flatten() {
            let bodies = self.bodies.entry(name.clone()).or_default();
            bodies.failed_units += 1;
            bodies.failed_by.extend(strings(why).map(str::to_owned));
        }
        for name in evidence["withdrawn"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(n, _)| n)
        {
            self.bodies.entry(name.clone()).or_default().withdrawn_units += 1;
        }
    }

    /// Unite `other`'s facts with these.
    pub fn absorb(&mut self, other: Self) {
        for (name, contexts) in other.hypothesis_contexts {
            self.hypothesis_contexts
                .entry(name)
                .or_default()
                .extend(contexts);
        }
        for (name, uses) in other.rows {
            let mine = self.rows.entry(name).or_default();
            mine.rows += uses.rows;
            mine.safe_rows += uses.safe_rows;
            mine.held_rows += uses.held_rows;
            mine.withdrawn_rows += uses.withdrawn_rows;
        }
        for (name, bodies) in other.bodies {
            let mine = self.bodies.entry(name).or_default();
            mine.validated_units += bodies.validated_units;
            mine.unvalidated_units += bodies.unvalidated_units;
            mine.failed_units += bodies.failed_units;
            mine.withdrawn_units += bodies.withdrawn_units;
            mine.validated_by.extend(bodies.validated_by);
            mine.failed_by.extend(bodies.failed_by);
            mine.clause_units += bodies.clause_units;
            for (clause, covered) in bodies.clauses {
                *mine.clauses.entry(clause).or_default() += covered;
            }
        }
        self.safe.extend(other.safe);
        self.rows_annotated += other.rows_annotated;
        self.safe_rows += other.safe_rows;
    }
}

/// The run-level body standing of one declaration (rule 13g): `held` for a
/// declaration held for a human act (`status` `pending-review` or
/// `not-in-force`); `validated` only when some unit validated an examined
/// body and none left one unvalidated or failed; otherwise
/// `declared-assumption`. `QCore.ContractDependencies.standing`.
#[must_use]
pub fn body_standing_of(
    status: Option<&str>,
    validated_units: usize,
    unvalidated_units: usize,
    failed_units: usize,
) -> &'static str {
    if matches!(status, Some("pending-review" | "not-in-force")) {
        return "held";
    }
    if validated_units > 0 && unvalidated_units == 0 && failed_units == 0 {
        "validated"
    } else {
        "declared-assumption"
    }
}

/// Card VERDICT-BODY-STANDING-CLAUSE-SCOPE (rules 13j, 13k): the run-level
/// standing of one declaration from its clauses. `clauses` maps each clause
/// to the validated units whose evidence covered it; `clause_units` counts
/// the validated units that carried coverage at all. Under the evidence
/// standing [`body_standing_of`]'s `validated`, the declaration is
/// `validated` only when it has a clause and every clause was covered in
/// every validated unit, `partly-validated` when some clause was, and a
/// `declared-assumption` otherwise; `held` and `declared-assumption` pass
/// through. `QCore.BodyClauseStanding.standing`.
#[must_use]
pub fn clause_standing_of(
    status: Option<&str>,
    validated_units: usize,
    unvalidated_units: usize,
    failed_units: usize,
    clause_units: usize,
    clauses: &BTreeMap<String, usize>,
) -> &'static str {
    let evidence = body_standing_of(status, validated_units, unvalidated_units, failed_units);
    if evidence != "validated" {
        return evidence;
    }
    let everywhere =
        |covered: &usize| clause_units == validated_units && *covered == validated_units;
    if !clauses.is_empty() && clauses.values().all(everywhere) {
        "validated"
    } else if clauses.values().any(everywhere) {
        "partly-validated"
    } else {
        "declared-assumption"
    }
}

/// Each clause's standing for the run (rule 13j): `validated` where the
/// declaration's evidence standing is `validated` and every validated unit's
/// evidence covered it, otherwise `declared-assumption`.
fn clause_standings(status: Option<&str>, bodies: &Bodies) -> BTreeMap<String, &'static str> {
    let evidence = body_standing_of(
        status,
        bodies.validated_units,
        bodies.unvalidated_units,
        bodies.failed_units,
    );
    bodies
        .clauses
        .iter()
        .map(|(clause, covered)| {
            let validated = evidence == "validated"
                && bodies.clause_units == bodies.validated_units
                && *covered == bodies.validated_units;
            (
                clause.clone(),
                if validated {
                    "validated"
                } else {
                    "declared-assumption"
                },
            )
        })
        .collect()
}

fn body_standing(status: Option<&str>, bodies: Option<&Bodies>) -> &'static str {
    match bodies {
        Some(bodies) => clause_standing_of(
            status,
            bodies.validated_units,
            bodies.unvalidated_units,
            bodies.failed_units,
            bodies.clause_units,
            &bodies.clauses,
        ),
        None => clause_standing_of(status, 0, 0, 0, 0, &BTreeMap::new()),
    }
}

/// The clauses of `clauses` that are not `validated`.
fn uncovered(clauses: &BTreeMap<String, &str>) -> Vec<String> {
    clauses
        .iter()
        .filter(|(_, standing)| **standing != "validated")
        .map(|(clause, _)| clause.clone())
        .collect()
}

/// The section for `facts` under the rendered `contract_revision` section
/// `revision` and the `premises` map ([`premises`]).
#[must_use]
pub fn render(facts: &Facts, revision: &Value, premises: &Value) -> Value {
    let inventory = &revision["contracts"];
    let names = facts
        .rows
        .keys()
        .chain(facts.bodies.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut declarations = serde_json::Map::new();
    let mut standing = BTreeMap::new();
    let mut partly = Vec::new();
    for name in &names {
        let entry = &inventory[name];
        let status = entry["status"].as_str();
        let bodies = facts.bodies.get(name);
        let body = body_standing(status, bodies);
        standing.insert(name.clone(), body);
        let uses = facts.rows.get(name).cloned().unwrap_or_default();
        let mut record = json!({
            "contract_sha256": entry["contract_sha256"],
            "mode": entry["mode"],
            "status": entry["status"],
            "module": entry["module"],
            "body_standing": body,
            "rows": uses.rows,
            "safe_rows": uses.safe_rows,
            "premises": premises.get(name).cloned().unwrap_or_else(|| json!([])),
        });
        if let Some(graph) = premises["$hypothesis_graphs"].get(name) {
            record["hypothesis_graph"] = graph.clone();
            let references = premises["$hypothesis_binding_requirements"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|r| r["declaration"].as_str() == Some(name.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            if !references.is_empty() {
                record["hypothesis_binding_requirements"] = json!(references);
            }
            record["hypothesis_evidence"] = json!({
                "disposition": "open", "warrant": "declared",
                "graph_sha256": premises["$hypothesis_graph_digests"][name],
                "standing": "declared-assumption",
                "meaning": "graph propositions remain open; body standing and warrant confer no graph authority",
                "contexts": facts.hypothesis_contexts.get(name).into_iter().flatten().map(|s| serde_json::from_str::<Value>(s).expect("recorded JSON")).collect::<Vec<_>>(),
                "dependent_rows": uses.rows, "dependent_safe_rows": uses.safe_rows,
            });
        }
        if uses.held_rows > 0 {
            record["held_rows"] = json!(uses.held_rows);
        }
        if uses.withdrawn_rows > 0 {
            record["withdrawn_rows"] = json!(uses.withdrawn_rows);
        }
        if let Some(bodies) = bodies {
            record["bodies"] = json!({
                "validated_units": bodies.validated_units,
                "unvalidated_units": bodies.unvalidated_units,
                "failed_units": bodies.failed_units,
                "withdrawn_units": bodies.withdrawn_units,
                "validated_by": bodies.validated_by,
                "failed_by": bodies.failed_by,
            });
            // Rule 13j: each clause's standing, and what is left uncovered.
            if body != "held" && !bodies.clauses.is_empty() {
                let clauses = clause_standings(status, bodies);
                if body == "partly-validated" {
                    partly.push((uses.safe_rows, name.clone(), uncovered(&clauses)));
                    record["uncovered_clauses"] = json!(uncovered(&clauses));
                }
                record["clauses"] = json!(clauses);
            }
        }
        declarations.insert(name.clone(), record);
    }
    let mut none = 0;
    let mut only_validated = 0;
    let mut on_partly_validated = 0;
    let mut on_assumption = 0;
    let mut on_held_or_withdrawn = 0;
    let mut on_null_premise_only = 0;
    let mut carrying_null_premise = 0;
    let is = |name: &String, label: &str| standing.get(name).is_some_and(|value| *value == label);
    for (rested, other, premises) in &facts.safe {
        carrying_null_premise += usize::from(*premises > 0);
        if *other > 0 {
            on_held_or_withdrawn += 1;
        } else if rested.is_empty() && *premises > 0 {
            // Card NULL-SAFETY-PROPERTY (rule 14n): no declaration, but a
            // named null premise: never counted as resting on nothing.
            on_null_premise_only += 1;
        } else if rested.is_empty() {
            none += 1;
        } else if rested.iter().all(|name| is(name, "validated")) {
            only_validated += 1;
        } else if rested
            .iter()
            .all(|name| is(name, "validated") || is(name, "partly-validated"))
        {
            // Rule 13k: every declaration validated or partly validated, at
            // least one partly, and no declared assumption.
            on_partly_validated += 1;
        } else {
            on_assumption += 1;
        }
    }
    partly.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let partly = partly
        .into_iter()
        .map(|(safe_rows, name, uncovered)| {
            json!({"function": name, "safe_rows": safe_rows, "uncovered_clauses": uncovered})
        })
        .collect::<Vec<_>>();
    let mut assumptions = facts
        .rows
        .iter()
        .filter(|(name, uses)| {
            uses.safe_rows > 0 && standing.get(*name) == Some(&"declared-assumption")
        })
        .map(|(name, uses)| (uses.safe_rows, name))
        .collect::<Vec<_>>();
    assumptions.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    let top = assumptions
        .iter()
        .take(20)
        .map(|(safe_rows, name)| {
            let record = &declarations[name.as_str()];
            json!({
                "function": name,
                "safe_rows": safe_rows,
                "mode": record["mode"],
                "status": record["status"],
                "premises": record["premises"],
            })
        })
        .collect::<Vec<_>>();
    let count = |body: &str| standing.values().filter(|value| **value == body).count();
    json!({
        "meaning": MEANING,
        "counts": {
            "rows_annotated": facts.rows_annotated,
            "declarations_consulted": declarations.len(),
            "validated": count("validated"),
            "partly_validated": count("partly-validated"),
            "declared_assumption": count("declared-assumption"),
            "held": count("held"),
        },
        "safe_rollup": {
            "safe_rows": facts.safe_rows,
            "resting_on_no_declaration": none,
            "resting_only_on_validated_declarations": only_validated,
            "resting_on_a_partly_validated_declaration": on_partly_validated,
            "resting_on_a_declared_assumption": on_assumption,
            "consulting_a_held_or_withdrawn_declaration": on_held_or_withdrawn,
            "resting_only_on_a_null_premise": on_null_premise_only,
            "carrying_a_null_premise": carrying_null_premise,
            "top_declared_assumptions": top,
            "partly_validated_declarations": partly,
        },
        "declarations": declarations,
    })
}

/// The section of one finished report.
#[must_use]
pub fn section(revision: &Value, premises: &Value, report: &AuditReport) -> Value {
    render(&Facts::of_rows(report.unit_rows()), revision, premises)
}

/// The one-line console tally of a [`section`].
#[must_use]
pub fn tally(section: &Value) -> String {
    let rollup = &section["safe_rollup"];
    let count = |key: &str| rollup[key].as_u64().unwrap_or_default();
    format!(
        "contract dependencies: {} Safe row(s): {} on no declaration, {} only on validated \
         declarations, {} on a partly validated declaration, {} on a declared assumption, {} \
         consulting a held or withdrawn declaration, {} only on a null premise ({} carrying one); \
         {} declaration(s) consulted ({} validated, {} partly validated)",
        count("safe_rows"),
        count("resting_on_no_declaration"),
        count("resting_only_on_validated_declarations"),
        count("resting_on_a_partly_validated_declaration"),
        count("resting_on_a_declared_assumption"),
        count("consulting_a_held_or_withdrawn_declaration"),
        count("resting_only_on_a_null_premise"),
        count("carrying_a_null_premise"),
        section["counts"]["declarations_consulted"]
            .as_u64()
            .unwrap_or_default(),
        section["counts"]["validated"].as_u64().unwrap_or_default(),
        section["counts"]["partly_validated"]
            .as_u64()
            .unwrap_or_default(),
    )
}

/// Erase only this card's additive graph publication, preserving every old field.
pub fn erase_hypotheses(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // These names stay legal declaration identifiers. Erase owned
            // publication fields only at their schema locations, never an
            // equally named declaration in a premise/inventory map.
            map.remove("$hypothesis_graphs");
            map.remove("$hypothesis_graph_digests");
            map.remove("$hypothesis_binding_requirements");
            map.remove("$checked_hypothesis_bindings");
            let function_row = map.get("function").is_some_and(Value::is_string)
                && (map.contains_key("status") || map.contains_key("null"));
            let declaration =
                map.contains_key("contract_sha256") && map.contains_key("body_standing");
            if function_row {
                map.remove("hypothesis_context");
                map.remove("hypothesis_graph");
                map.remove("hypothesis_graph_sha256");
            }
            if declaration {
                map.remove("hypothesis_binding_requirements");
                map.remove("hypothesis_graph");
                map.remove("hypothesis_graph_sha256");
                map.remove("hypothesis_evidence");
            }
            if map.get("declarations").is_some_and(Value::is_array) {
                map.remove("hypothesis_dependencies");
            }
            for item in map.values_mut() {
                erase_hypotheses(item);
            }
        }
        Value::Array(items) => {
            for item in items {
                erase_hypotheses(item);
            }
        }
        _ => {}
    }
}

/// Publish explicitly supplied, independently checked reference bindings beside
/// the exact existing graph. Integrity only; source correspondence stays Open.
pub fn publish_checked_bindings(
    value: &mut Value,
    sources: &BTreeMap<String, String>,
    expected_graphs: &BTreeMap<String, qcore_schemas::hypothesis::Graph>,
    expected_manifests: &BTreeMap<String, qcore_schemas::hypothesis::BindingManifest>,
    presented_manifests: &BTreeMap<String, Vec<u8>>,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    // Audit publication may also contain rule-owned legacy graphs. Bind the
    // exact attached declarations; do not reinterpret unrelated rule metadata.
    let requirements = qcore_contract::hypothesis_bindings::requirements(sources, expected_graphs)?;
    for required in &requirements {
        if value["$hypothesis_graphs"][&required.declaration]
            != json!(expected_graphs[&required.declaration])
        {
            return Err(
                "binding: independently expected attachment graph differs from publication".into(),
            );
        }
    }
    if value["$hypothesis_binding_requirements"] != json!(requirements) {
        return Err("binding: independently expected requirements differ from publication".into());
    }
    let bindings = qcore_contract::hypothesis_bindings::resolve(
        sources,
        expected_graphs,
        expected_manifests,
        presented_manifests,
        artifacts,
    )?;
    value["$checked_hypothesis_bindings"] = json!({
        "warrant": "reference-integrity-only", "source_binding": "open", "manifests": bindings
    });
    Ok(())
}

/// Recover the published named-premise view from the actual contracts sidecar
/// schema. Missing declarations/invalid premise arrays refuse; absent per-row
/// premises explicitly means that declaration publishes none.
pub fn premise_view_from_sidecar(sidecar: &Value) -> Result<Value, String> {
    let declarations = sidecar["contract_dependencies"]["declarations"]
        .as_object()
        .ok_or("sidecar: missing contract dependency declarations")?;
    let mut out = serde_json::Map::new();
    for (name, declaration) in declarations {
        let declaration = declaration
            .as_object()
            .ok_or("sidecar: invalid declaration record")?;
        let premises = match declaration.get("premises") {
            Some(value) if value.is_array() => value.clone(),
            None => json!([]),
            _ => return Err("sidecar: invalid declared premise array".into()),
        };
        out.insert(name.clone(), premises);
    }
    Ok(Value::Object(out))
}
