//! The null-dereference consumer of route G (card **NULL-SAFETY-PROPERTY**,
//! gateway rules 14k–14p, contract specification C09.3).
//!
//! A separately selected property, `c.null.deref.v1`, beside custody and
//! bounds. Nothing in the custody analysis calls this module; a caller asks
//! for it ([`analyze_null_c11`], [`analyze_null_preprocessed`], or the
//! audit's `QCORE_NULL_PROPERTY` selection), and the custody result is
//! unchanged whether or not it does.
//!
//! # What is read
//!
//! Clang's typed AST of the whole translation unit: every function
//! definition (header `static inline` ones included) is compacted as it is
//! read. For each selected definition the consumer walks the body forward,
//! joining paths, and judges every **dereference** (a `*p`, `p->f` or `p[i]`
//! read or write, an atomic access, and an actual at a `quant null-deref`
//! position) through [`qcore_core::null_deref::check`]. A form it does not
//! read is a named function-level reason (`Unknown`), never skipped.
//!
//! # Sources, guards and the dominating dereference
//!
//! * **Possibly null:** an allocation result (a resource contract's owned
//!   nullable result), a nullable or lookup result, a result no contract
//!   states, a formal (unless its clause is declared or inferred), a pointer
//!   read from memory or a global, a null constant on some path, an
//!   uninitialized local.
//! * **Non-null:** an address, an array decay, a string literal, a function;
//!   a declared non-null contract result (a listed assumption); a non-null
//!   in-unit summary result; a NULL test's non-null arm (`if (p)`, `!p`,
//!   `p != NULL`, `&&`/`||`, `?:`, `__builtin_expect`), an early exit or a
//!   no-return call on the other arm; `assert(p)`'s surviving arm when the
//!   analysed text still holds it (`NDEBUG` absent: a listed premise).
//! * **User decision 1:** after a real read or write through `B`, `B` is
//!   non-null with its question moved to that check, until `B` is reassigned;
//!   `&B->G` is non-null exactly when `B` is; forming `&B->G` and an
//!   unevaluated operand (`sizeof`) is no dereference. Facts on locals survive
//!   calls (no address is taken); facts on memory paths (`s->next`, globals)
//!   are forgotten at every call and every store that could write a pointer
//!   (a non-character scalar store keeps them, C11 6.5p7, as a listed
//!   premise).
//!
//! # The clause and its inference
//!
//! A declaration's `quant null-deref #B;` (bounds contract v1, pinned like
//! every bounds declaration) makes `B` non-null at the body's entry (the row
//! is then conditional on it) and an obligation at every call: the actual
//! must be proved non-null, or the call is a candidate. A body checks its own
//! clause: a test of `B` before its first dereference, or a path without one,
//! is reported **stronger than necessary**, never wrong. In-unit, a body that
//! dereferences a formal on every terminating path before any test of it
//! infers the clause, and its callers in the unit carry the obligation; a
//! body whose every return is non-null gives its callers a non-null result.
//! Neither summary leaves the unit.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::bounds_contract::{BoundsClause, BoundsContracts};
use qcore_core::null_deref::{self as core, Nullness, Outcome, Proof, Source};
use qcore_schemas::contract::ResourceContracts;
use serde_json::Value;

/// Card NULL-CANDIDATE-TRIAGE: the core decisions the audit's run fold
/// reads (the null property's model and the triage's).
pub use qcore_core::null_deref as core_model;
pub use qcore_core::null_deref::{NULL_PROPERTY, NullVerdict, Request, compose};
/// Card NULL-MEMBER-INVARIANT-SCOPE: the scoped member decisions.
pub use qcore_core::null_member_scope;
pub use qcore_core::null_triage;

use crate::legacy_c::C11TranslationUnit;
use crate::typed_c_ast::kind;

mod ast;
mod scope;
mod walk;
mod witness;

use ast::kids;
pub use scope::StableStore;
use walk::{Declared, Entry, Env, MemberDecl, ResultDecl, Summary, Walker};
pub use walk::{ExternalCall, MemberStore};
pub use witness::{PathStep, Witness};

/// Card NULL-CANDIDATE-TRIAGE (rule 14r): the source kinds a candidate
/// carries for its path to be searched (a NULL condition the search can
/// name: an allocation, a lookup, a null constant, a test's null arm).
pub const STRONG_SOURCE_KINDS: [&str; 6] = [
    "allocation",
    "nullable-result",
    "lookup",
    "null-literal",
    "uninitialized",
    "global",
];

/// Card NULL-CANDIDATE-TRIAGE: the triage of one strong-source candidate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TriageRecord {
    /// The check's id.
    pub check: usize,
    /// `finding`, `infeasible`, `unresolved` or `indeterminate`.
    pub status: &'static str,
    /// A finding's class (`null-on-allocation-failure`,
    /// `null-on-normal-path`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<&'static str>,
    /// The NULL condition (a finding) or why there is none.
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<Witness>,
    /// Paths the search ran.
    pub paths: usize,
}

/// Card NULL-CANDIDATE-TRIAGE (rule 14v): what one analysed definition
/// exports for cross-unit travel.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DefinitionSummary {
    pub function: String,
    /// External linkage (not `static`, not an `inline` definition).
    pub external: bool,
    pub parameters: usize,
    /// The inferred clauses: position and formal name.
    pub inferred: Vec<(usize, String)>,
    pub returns_non_null: bool,
    pub returns_null_on_a_path: bool,
    /// The declarations its summary rests on.
    pub dependencies: Vec<String>,
}

/// One judged check.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NullCheck {
    pub id: usize,
    /// `member-access`, `dereference`, `element-access`, `atomic-access`,
    /// `call-precondition`.
    pub kind: &'static str,
    /// Line in the analysed (preprocessed) text.
    pub line: usize,
    /// The pointer expression checked.
    pub subject: String,
    /// `proved`, `moved`, `held`, `candidate`, `violation`,
    /// `violation-reachability-unresolved`, `excluded`.
    pub outcome: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub proofs: Vec<String>,
}

/// A declared clause's standing against the body that defines it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ClauseStanding {
    pub clause: String,
    /// `necessary` or `stronger-than-necessary`.
    pub standing: &'static str,
}

/// The null row of one function.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NullFunctionRow {
    pub function: String,
    pub property: &'static str,
    pub verdict: &'static str,
    pub reasons: Vec<String>,
    /// `unconditional`, or the function's own null-deref preconditions.
    pub scope: String,
    /// Every declared, inferred or asserted fact a judgement relied on.
    pub assumptions: Vec<String>,
    pub checks: Vec<NullCheck>,
    /// The formals this unit's inference found dereferenced before any test
    /// on every terminating path (`#B`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inferred_preconditions: Vec<String>,
    /// The function's own declared clauses against its body.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub clause_standing: Vec<ClauseStanding>,
    /// Every return is non-null (the in-unit summary callers read).
    pub returns_non_null: bool,
    /// The declared names (and in-unit inferences) the row consulted.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contract_dependencies: Vec<String>,
    /// Calls through a function pointer, whose own null-ness is not checked
    /// (a registered residual).
    #[serde(skip_serializing_if = "is_zero")]
    pub indirect_calls: usize,
    /// Card NULL-CANDIDATE-TRIAGE (rule 14r): every strong-source
    /// candidate's path search.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub triage: Vec<TriageRecord>,
}

fn is_zero(count: &usize) -> bool {
    *count == 0
}

/// The null report of one translation unit.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NullUnitReport {
    pub property: &'static str,
    pub model_version: &'static str,
    pub source_sha256: String,
    /// The bounds environment's digest (the clauses' source).
    pub contracts_sha256: String,
    /// The resource environment's digest (result nullability).
    pub resource_contracts_sha256: String,
    pub functions: Vec<NullFunctionRow>,
    /// Card NULL-CANDIDATE-TRIAGE: every store to a declared member in the
    /// unit's analysed definitions (rule 14t's evidence).
    pub member_stores: Vec<MemberStore>,
    /// Card NULL-MEMBER-INVARIANT-SCOPE: every store to a stable member in
    /// the unit's analysed definitions (rule 14x's evidence).
    pub stable_stores: Vec<StableStore>,
    /// Card NULL-CANDIDATE-TRIAGE: every analysed definition's export
    /// (rule 14v), and each declared result's body (rule 14u).
    pub definitions: Vec<DefinitionSummary>,
    /// Card NULL-CANDIDATE-TRIAGE: the pointer actuals of calls to functions
    /// the unit neither defines nor declares (rule 14v).
    pub external_calls: Vec<ExternalCall>,
}

impl NullUnitReport {
    pub fn row(&self, function: &str) -> Option<&NullFunctionRow> {
        self.functions.iter().find(|row| row.function == function)
    }
}

/// The verdict of a row, as the core's four values.
pub fn row_verdict(row: &NullFunctionRow) -> NullVerdict {
    match row.verdict {
        "Safe" => NullVerdict::Safe,
        "Pending" => NullVerdict::Pending,
        "Unsafe" => NullVerdict::Unsafe,
        _ => NullVerdict::Unknown,
    }
}

fn render_source(source: &Source) -> String {
    source.render()
}

fn render_proof(proof: &Proof) -> String {
    match proof {
        Proof::Tested(line) => format!("tested:{line}"),
        Proof::Asserted(line) => format!("asserted:{line}"),
        Proof::Precondition(text) => format!("precondition:{text}"),
        Proof::NonNullResult(name) => format!("result:{name}"),
        Proof::Address => "address".into(),
        Proof::Moved(site) => format!("moved:{site}"),
    }
}

fn record(check: walk::CheckRecord) -> NullCheck {
    let (sources, proofs) = match &check.outcome {
        Outcome::Proved(proofs) | Outcome::Moved(proofs) => {
            (Vec::new(), proofs.iter().map(render_proof).collect())
        }
        Outcome::Candidate(sources) => (sources.iter().map(render_source).collect(), Vec::new()),
        Outcome::Held(hold) => (Vec::new(), vec![format!("held:{hold}")]),
        Outcome::Violation { .. } | Outcome::Excluded => (Vec::new(), Vec::new()),
    };
    NullCheck {
        id: check.id,
        kind: check.kind,
        line: check.line,
        subject: check.subject,
        outcome: check.outcome.class(),
        sources,
        proofs,
    }
}

/// The declared clauses of the bounds environment, by function.
fn declared_clauses(contracts: &BoundsContracts) -> BTreeMap<String, Declared> {
    contracts
        .null_derefs()
        .map(|(name, contract)| {
            let positions = contract
                .clauses
                .iter()
                .filter_map(|entry| match &entry.clause {
                    BoundsClause::NullDeref { parameter } => contract
                        .parameter(parameter)
                        .map(|(index, _)| (index, entry.text.clone())),
                    _ => None,
                })
                .collect();
            let held = contract.held().then(|| contract.holds().join("; "));
            (
                name.clone(),
                Declared {
                    parameters: contract.parameters.len(),
                    positions,
                    held,
                },
            )
        })
        .collect()
}

/// Card NULL-CANDIDATE-TRIAGE: the declared non-null members and results.
fn declared_members_and_results(
    contracts: &BoundsContracts,
) -> (
    BTreeMap<(String, String), MemberDecl>,
    BTreeMap<String, ResultDecl>,
) {
    let mut members = BTreeMap::new();
    let mut results = BTreeMap::new();
    for (name, contract) in contracts.null_declarations() {
        let held = contract.held().then(|| contract.holds().join("; "));
        for entry in &contract.clauses {
            match &entry.clause {
                BoundsClause::NullMember { tag, field } => {
                    let decl = MemberDecl {
                        host: name.clone(),
                        held: held.clone(),
                    };
                    // Two hosts of one member: held when either is.
                    members
                        .entry((tag.clone(), field.clone()))
                        .and_modify(|existing: &mut MemberDecl| {
                            if existing.held.is_none() {
                                existing.held.clone_from(&decl.held);
                            }
                        })
                        .or_insert(decl);
                }
                BoundsClause::NullResult => {
                    results.insert(name.clone(), ResultDecl { held: held.clone() });
                }
                _ => {}
            }
        }
    }
    (members, results)
}

/// Callees first: a post-order of the in-unit call graph (a recursive edge
/// is cut, so its callee's summary is not yet known: no clause, no result).
fn bottom_up(unit: &ast::Unit) -> Vec<String> {
    fn visit(name: &str, unit: &ast::Unit, seen: &mut BTreeSet<String>, out: &mut Vec<String>) {
        if !seen.insert(name.to_owned()) {
            return;
        }
        if let Some(definition) = unit.definitions.get(name) {
            let mut callees = BTreeSet::new();
            ast::callees(definition, &mut callees);
            for callee in callees {
                if unit.definitions.contains_key(&callee) {
                    visit(&callee, unit, seen, out);
                }
            }
        }
        out.push(name.to_owned());
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for name in unit.definitions.keys() {
        visit(name, unit, &mut seen, &mut out);
    }
    out
}

fn formal_names(definition: &Value) -> Vec<String> {
    kids(definition)
        .filter(|c| kind(c) == "ParmVarDecl")
        .map(|c| c["name"].as_str().unwrap_or("").to_owned())
        .collect()
}

/// What one definition's analysis produced besides its row.
struct Analysed {
    row: NullFunctionRow,
    summary: Summary,
    member_stores: Vec<MemberStore>,
    stable_stores: Vec<StableStore>,
    external_calls: Vec<ExternalCall>,
    export: DefinitionSummary,
}

/// Analyse one definition: its row and its in-unit summary.
fn analyze_function(env: &Env<'_>, name: &str, definition: &Value) -> Analysed {
    let names = formal_names(definition);
    let pointers = kids(definition)
        .filter(|c| kind(c) == "ParmVarDecl")
        .map(|c| ast::is_pointer(ast::ty(c)))
        .collect::<Vec<_>>();
    let declared = env.declared.get(name);
    let mut entries = BTreeMap::new();
    let mut scope_clauses = Vec::new();
    let mut reasons_before = Vec::new();
    // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14z): the function's declared
    // scope entries are preconditions of its row.
    let member_scope = env.scopes.get(name);
    if let Some(member_scope) = member_scope {
        if member_scope.parameters == names.len() {
            scope_clauses.extend(member_scope.entries.iter().map(|e| e.text.clone()));
        } else {
            reasons_before.push(format!(
                "null-declaration-withdrawn: `{name}`'s scope declaration does not match its C definition"
            ));
        }
    }
    if let Some(declared) = declared {
        if declared.parameters == names.len() {
            for (position, text) in &declared.positions {
                let entry = match &declared.held {
                    Some(hold) => Entry::Held(hold.clone()),
                    None => Entry::Declared(text.clone()),
                };
                entries.insert(*position, entry);
                scope_clauses.push(text.clone());
            }
        } else {
            reasons_before.push(format!(
                "null-declaration-withdrawn: `{name}`'s declaration does not match its C definition"
            ));
        }
    }
    // The inference pass (rule 14p): the body under its declared clauses
    // only; what every terminating path did first with each formal.
    let first = Walker::walk(env, name, definition, &entries);
    let mut inferred = Vec::new();
    for (index, exits) in first.exits.iter().enumerate() {
        if pointers.get(index) == Some(&true)
            && core::inferred(exits)
            && !entries.contains_key(&index)
        {
            let text = format!("quant null-deref #{}", names[index]);
            inferred.push((index, text));
        }
    }
    let clause_standing = declared
        .map(|declared| {
            declared
                .positions
                .iter()
                .map(|(position, text)| ClauseStanding {
                    clause: text.clone(),
                    standing: core::clause_standing(
                        first.exits.get(*position).unwrap_or(&BTreeSet::new()),
                    ),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let walked = if inferred.is_empty() {
        first
    } else {
        for (index, text) in &inferred {
            entries.insert(*index, Entry::Inferred(text.clone()));
        }
        Walker::walk(env, name, definition, &entries)
    };
    let outcomes = walked
        .checks
        .iter()
        .map(|check| check.outcome.clone())
        .collect::<Vec<_>>();
    let mut reasons = reasons_before;
    reasons.extend(walked.reasons.iter().cloned());
    let (verdict, mut reasons) = core::null_verdict(&outcomes, &reasons);
    // Card NULL-CANDIDATE-TRIAGE (rule 14r): each strong-source candidate's
    // path search; a validated path is a finding (provisional `Unsafe`).
    let mut triage = Vec::new();
    for check in &walked.checks {
        let Outcome::Candidate(sources) = &check.outcome else {
            continue;
        };
        if !sources
            .iter()
            .any(|s| STRONG_SOURCE_KINDS.contains(&s.kind()))
        {
            continue;
        }
        let found = witness::search(env, name, definition, &entries, &check.key);
        triage.push(TriageRecord {
            check: check.id,
            status: found.triage.as_str(),
            class: match found.triage {
                qcore_core::null_triage::Triage::Finding(class) => Some(class.as_str()),
                _ => None,
            },
            reason: found.reason,
            witness: found.witness,
            paths: found.runs,
        });
    }
    let findings = triage.iter().filter(|t| t.status == "finding").count();
    let verdict = qcore_core::null_triage::triaged_verdict(verdict, findings);
    if findings > 0 {
        reasons.push(format!(
            "{}: {findings} validated path(s) to a NULL dereference (provisional; never an upstream defect)",
            qcore_core::null_triage::FINDING
        ));
    }
    let mut scope = String::from("unconditional");
    let mut conditions = scope_clauses
        .iter()
        .map(|text| format!("declared {text}"))
        .collect::<Vec<_>>();
    conditions.extend(inferred.iter().map(|(_, text)| format!("inferred {text}")));
    if !conditions.is_empty() {
        scope = format!(
            "conditional on `{name}`'s null-deref preconditions: {}",
            conditions.join("; ")
        );
    }
    let returns_non_null = walked.returns.as_ref().is_some_and(Nullness::is_non_null);
    let returns_null_on_a_path = walked.returns_null_on_a_path;
    let mut dependencies = walked.consulted.iter().cloned().collect::<Vec<_>>();
    if declared.is_some() || member_scope.is_some() {
        dependencies.push(name.to_owned());
    }
    dependencies.sort();
    dependencies.dedup();
    let summary = Summary {
        derefs: inferred
            .iter()
            .map(|(index, _)| *index)
            .chain(
                declared
                    .filter(|d| d.held.is_none() && d.parameters == names.len())
                    .map(|d| d.positions.keys().copied().collect::<Vec<_>>())
                    .unwrap_or_default(),
            )
            .collect(),
        returns_non_null,
        dependencies: walked.consulted.clone(),
    };
    let external =
        definition["storageClass"].as_str() != Some("static") && definition["inline"] != true;
    let export = DefinitionSummary {
        function: name.to_owned(),
        external,
        parameters: names.len(),
        inferred: inferred
            .iter()
            .map(|(index, _)| (*index, names[*index].clone()))
            .collect(),
        returns_non_null,
        returns_null_on_a_path,
        dependencies: walked.consulted.iter().cloned().collect(),
    };
    let finding_ids = triage
        .iter()
        .filter(|t| t.status == "finding")
        .map(|t| t.check)
        .collect::<BTreeSet<_>>();
    let row = NullFunctionRow {
        function: name.to_owned(),
        property: NULL_PROPERTY,
        verdict: verdict.as_str(),
        reasons,
        scope,
        assumptions: walked.assumptions.into_iter().collect(),
        checks: walked
            .checks
            .into_iter()
            .map(|check| {
                let finding = finding_ids.contains(&check.id);
                let mut recorded = record(check);
                if finding {
                    recorded.outcome = "finding";
                }
                recorded
            })
            .collect(),
        inferred_preconditions: inferred
            .iter()
            .map(|(index, _)| format!("#{}", names[*index]))
            .collect(),
        clause_standing,
        returns_non_null,
        contract_dependencies: dependencies,
        indirect_calls: walked.indirect_calls,
        triage,
    };
    Analysed {
        row,
        summary,
        member_stores: walked.member_stores,
        stable_stores: walked.stable_stores,
        external_calls: walked.external_calls,
        export,
    }
}

/// Keep each top-level declaration, compacted, as it is read.
struct TopSeed<'a> {
    unit: &'a mut ast::Unit,
}

impl<'de> serde::de::DeserializeSeed<'de> for TopSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> serde::de::Visitor<'de> for TopSeed<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a Clang translation-unit AST")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == "inner" {
                map.next_value_seed(InnerSeed {
                    unit: &mut *self.unit,
                })?;
            } else {
                map.next_value::<serde::de::IgnoredAny>()?;
            }
        }
        Ok(())
    }
}

struct InnerSeed<'a> {
    unit: &'a mut ast::Unit,
}

impl<'de> serde::de::DeserializeSeed<'de> for InnerSeed<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> serde::de::Visitor<'de> for InnerSeed<'_> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the translation unit's declarations")
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(decl) = seq.next_element::<Value>()? {
            ast::read_declaration(self.unit, &decl);
        }
        Ok(())
    }
}

const ANALYSIS_STACK: usize = 256 << 20;

fn resource_digest(resource: &ResourceContracts) -> String {
    let bytes = serde_json::to_vec(resource.functions()).unwrap_or_default();
    qcore_schemas::digest::sha256_hex(&bytes)
}

fn analyze_on_stack(
    preprocessed: &str,
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    resource: &ResourceContracts,
    bounds: &BoundsContracts,
) -> Result<NullUnitReport, String> {
    use serde::de::DeserializeSeed;
    let mut read = ast::Unit::default();
    crate::typed_c_ast::translation_unit_ast_with(preprocessed, unit, |bytes| {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        deserializer.disable_recursion_limit();
        TopSeed { unit: &mut read }.deserialize(&mut deserializer)
    })?;
    let newlines = preprocessed
        .bytes()
        .enumerate()
        .filter(|(_, b)| *b == b'\n')
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let declared = declared_clauses(bounds);
    let (members, results) = declared_members_and_results(bounds);
    let (stable, scopes) = scope::declared_stable_and_scopes(bounds);
    let mut summaries = BTreeMap::new();
    let mut rows = BTreeMap::new();
    let mut member_stores = Vec::new();
    let mut stable_stores = Vec::new();
    let mut external_calls = Vec::new();
    let mut definitions = Vec::new();
    let selected = functions.iter().collect::<BTreeSet<_>>();
    for name in bottom_up(&read) {
        let Some(definition) = read.definitions.get(&name) else {
            continue;
        };
        let env = Env {
            unit: &read,
            resource,
            declared: &declared,
            summaries: &summaries,
            newlines: &newlines,
            members: &members,
            results: &results,
            stable: &stable,
            scopes: &scopes,
        };
        let analysed = analyze_function(&env, &name, definition);
        member_stores.extend(analysed.member_stores);
        stable_stores.extend(analysed.stable_stores);
        if selected.contains(&name) {
            external_calls.extend(analysed.external_calls);
            rows.insert(name.clone(), analysed.row);
        }
        definitions.push(analysed.export);
        summaries.insert(name, analysed.summary);
    }
    let rows = functions
        .iter()
        .map(|name| {
            rows.remove(name).unwrap_or_else(|| NullFunctionRow {
                function: name.clone(),
                property: NULL_PROPERTY,
                verdict: "Unknown",
                reasons: vec![
                    "null-import-failure: no definition of the function in the unit's AST".into(),
                ],
                scope: "unconditional".into(),
                assumptions: Vec::new(),
                checks: Vec::new(),
                inferred_preconditions: Vec::new(),
                clause_standing: Vec::new(),
                returns_non_null: false,
                contract_dependencies: Vec::new(),
                indirect_calls: 0,
                triage: Vec::new(),
            })
        })
        .collect();
    Ok(NullUnitReport {
        property: NULL_PROPERTY,
        model_version: core::NULL_MODEL_VERSION,
        source_sha256: qcore_schemas::digest::sha256_hex(preprocessed.as_bytes()),
        contracts_sha256: bounds.digest(),
        resource_contracts_sha256: resource_digest(resource),
        functions: rows,
        member_stores,
        stable_stores,
        definitions,
        external_calls,
    })
}

/// The null report of the selected functions of an already preprocessed unit
/// (the text the custody analysis read), under the resource environment
/// (result nullability) and the bounds environment (the clauses).
pub fn analyze_null_preprocessed(
    preprocessed: &str,
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    resource: &ResourceContracts,
    bounds: &BoundsContracts,
) -> Result<NullUnitReport, String> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(ANALYSIS_STACK)
            .spawn_scoped(scope, || {
                analyze_on_stack(preprocessed, unit, functions, resource, bounds)
            })
            .map_err(|error| format!("cannot start the null reader: {error}"))?
            .join()
            .map_err(|_| "the null reader panicked".to_owned())?
    })
}

/// [`analyze_null_preprocessed`] after the gateway's own validation and
/// preprocessing of `unit`.
pub fn analyze_null_c11(
    unit: &C11TranslationUnit<'_>,
    functions: &[String],
    resource: &ResourceContracts,
    bounds: &BoundsContracts,
) -> Result<NullUnitReport, String> {
    let preprocessed =
        crate::legacy_c::validate_unrestricted_c11(unit).map_err(|e| format!("{e:?}"))?;
    analyze_null_preprocessed(&preprocessed, unit, functions, resource, bounds)
}
