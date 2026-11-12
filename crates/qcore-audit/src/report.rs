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

//! Diagnostic inventory only: never changes analysis verdicts or certificates.
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use qcore_c_gateway::legacy_c::{
    LegacyCError, LegacyDiagnostic, LegacyDiagnosticKind, SourceDefinitionInventory, SourceLineMap,
    UnrestrictedLegacyResult, unlocated_site,
};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::reason::ReasonCategory;
use serde_json::{Value, json};

/// Card ARM-OBLIGATIONS: the row key of the rule packages' carried
/// obligations.
pub const RULE_OBLIGATIONS_KEY: &str = "rule_obligations";

/// The row field of a row's carried obligations, `None` when it carries none
/// (the row keeps its bytes): `premises` and `open`, each `{rule, name}`.
#[must_use]
pub fn rule_obligations_field(
    carried: &qcore_c_gateway::arm_obligations::Carried,
) -> Option<Value> {
    if carried.is_empty() {
        return None;
    }
    let list = |set: &std::collections::BTreeSet<qcore_c_gateway::arm_obligations::Carriage>| {
        set.iter()
            .map(|carriage| json!({"rule": carriage.rule, "name": carriage.name}))
            .collect::<Vec<_>>()
    };
    Some(json!({"premises": list(&carried.premises), "open": list(&carried.open)}))
}

/// Card ARM-OBLIGATIONS: a row's verdict-contract `assumptions` with the
/// premises it carries folded in. A row that carries none keeps `base`
/// exactly, so only a row resting on a rule package's premise changes.
#[must_use]
pub fn assumptions_with_premises(base: &str, row: &Value) -> String {
    let premises = &row[RULE_OBLIGATIONS_KEY]["premises"];
    match premises.as_array() {
        Some(premises) if !premises.is_empty() => qcore_schemas::sha256_hex(
            json!({"sources": base, "rule_premises": premises})
                .to_string()
                .as_bytes(),
        ),
        _ => base.to_owned(),
    }
}

/// The closed L21.1 category of a per-unit reason, as a report value.
///
/// This is the second carrier of the reason-code vocabulary
/// (`docs/QCORE_SPEC_r0_3.md` §L21.1); the first is
/// `qcore_schemas::obligation::DerivationIncomplete::category`. It *refines*
/// the row: `status`, `reason`, `source_functions`, `functions`,
/// `not_projected` and `diagnostic_counts` keep their exact meaning and bytes.
///
/// Fail-closed in two directions:
///
/// * A unit with no reason carries no category — `null`, not a guess.
/// * A reason the closed vocabulary does not recognise also carries `null`.
///   `null` asserts nothing, so an unrecognised row can never be read as a
///   statement about the analysed source (§L21.1 rule 1). It is never
///   defaulted to a category.
///   `qcore_schemas`'s `reason_codes` test enumerates the reason families this
///   workspace emits and pins that each one classifies.
pub(crate) fn reason_code(reason: Option<&str>) -> Value {
    reason_code_value(reason.map(ReasonCategory::classify).and_then(Result::ok))
}

/// The report value for a category a producer already classified.
///
/// Rows whose producer carries a typed category — `ChunkSourceResult::Unknown`
/// since REASON-CODES-B2 — go through here instead of being re-classified from
/// their own free text, so the producer and the report cannot disagree. The
/// fail-closed shape is identical: `None` serializes as `null` and asserts
/// nothing.
pub(crate) fn reason_code_value(category: Option<ReasonCategory>) -> Value {
    match category {
        Some(category) => json!(category.as_str()),
        None => Value::Null,
    }
}

/// A refusal of one selected configuration, carrying its closed §L21.1
/// category beside its text.
///
/// Card REASON-CODES-B1. Most refusal texts on this route classify under
/// `ReasonCategory::classify`, but a few are raw OS or tool text -
/// `std::io::Error::to_string()` from reading the selected source, and the
/// `canonicalize`/`git show` failures inside `verify_pinned_bytes`. Those carry
/// no stable marker, so classifying from prose returns `null`: honest (no
/// claim) but incomplete (no code). This type lets such a site name its
/// category where the text cannot, without changing one byte of the text
/// (§L21.1 rule 2).
///
/// Naming a category here can only narrow what a row claims: the sites that use
/// [`Self::environment`] are OS and tool failures, and
/// `ReasonCategory::EnvironmentUnavailable` is the one category that says
/// nothing about the analysed source and can never contribute to a provisional
/// `Unsafe` (§L21.1 rule 1). There is deliberately no constructor that names a
/// source-subject category for opaque text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitRefusal {
    reason: String,
    category: Option<ReasonCategory>,
}

impl UnitRefusal {
    /// A refusal whose text the closed vocabulary is expected to recognise.
    /// Fail-closed exactly as before: an unrecognised text carries no category.
    pub fn classified(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        let category = ReasonCategory::classify(&reason).ok();
        Self { reason, category }
    }

    /// A refusal minted from opaque OS or tool text, whose category is named at
    /// the site because the text cannot carry it.
    pub fn environment(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            category: Some(ReasonCategory::EnvironmentUnavailable),
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn category(&self) -> Option<ReasonCategory> {
        self.category
    }

    /// Drop the category for a caller that only has a `String` channel. This is
    /// a narrowing, never a relabeling: the text is unchanged.
    pub fn into_reason(self) -> String {
        self.reason
    }
}

impl From<String> for UnitRefusal {
    fn from(reason: String) -> Self {
        Self::classified(reason)
    }
}

impl From<&str> for UnitRefusal {
    fn from(reason: &str) -> Self {
        Self::classified(reason)
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ReportField<'a> {
    Version(u32),
    Text(&'a str),
    Counts(&'a BTreeMap<String, usize>),
    Units(&'a [Value]),
    /// Card AUDIT-COMPOSITION: the unit rows derived from their components.
    DerivedUnits(&'a [std::borrow::Cow<'a, Value>]),
    Json(&'a Value),
}

/// Card CORPUS-GROUP-CHECKPOINTS: what one unit's report fragment records,
/// in unit order, as [`AuditReport::absorb`] appends it.
pub(crate) struct FragmentParts {
    pub units: Vec<Value>,
    pub diagnostic_counts: BTreeMap<String, usize>,
    pub bounds: Vec<Value>,
    pub bounds_evidence: Vec<Value>,
    pub null: Vec<Value>,
    pub null_evidence: Vec<Value>,
    pub callback_members: Vec<crate::callback_member_fold::UnitFacts>,
}

pub struct AuditReport {
    revision: String,
    units: Vec<Value>,
    diagnostic_counts: BTreeMap<String, usize>,
    source_contracts: Option<Value>,
    /// Card W44-QUIRIN-AUDIT-PROVIDERS: a contracts-only run's evidence mode
    /// and the contract modules its verdicts rest on. Absent otherwise, so
    /// every other report keeps its shape.
    evidence: Option<Value>,
    /// Card W45-USER-CONTRACT-PLUGINS: the checker links every recorded unit
    /// folds (`crate::user_plugins::fold_unit`). Absent without a binding,
    /// so every other report keeps its shape.
    plugin_links: Option<std::sync::Arc<crate::user_plugins::PluginLinks>>,
    /// Card CG16-INDEX-AND-OFFSET-EXTENTS: the bounds rows of a run that
    /// selected the bounds property (`QCORE_BOUNDS_PROPERTY`), in unit order.
    /// Never part of the custody document: [`Self::document`] does not read it.
    bounds: Vec<Value>,
    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: each unit's body
    /// evidence for bounds declarations ([`crate::bounds_evidence`]), in unit
    /// order. Never part of the custody document.
    bounds_evidence: Vec<Value>,
    /// Card NULL-SAFETY-PROPERTY: the null rows of a run that selected the
    /// null-dereference property (`QCORE_NULL_PROPERTY`), in unit order.
    /// Never part of the custody document.
    null: Vec<Value>,
    /// Card NULL-CANDIDATE-TRIAGE: each unit's null evidence (member stores,
    /// definition exports, external calls), in unit order, folded over the
    /// run before the null rows are written. Never part of the custody
    /// document.
    null_evidence: Vec<Value>,
    /// Card W45-FB-ENGINE-PROGRESS-MEASURE (rule 44): the loop-measure rows
    /// of a run that selected the property (`QCORE_LOOP_MEASURE_PROPERTY`),
    /// in unit order. Never part of the custody document.
    loop_measures: Vec<Value>,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): each analysed
    /// unit's function-pointer member facts, in unit order, recorded only
    /// when a declaration invokes a member; folded over the run by
    /// [`Self::fold_callback_members`] before anything is published.
    callback_members: Vec<crate::callback_member_fold::UnitFacts>,
    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, R1): the run's member
    /// facts the pinned route's pre-pass read, with its evidence section.
    member_prepass: Option<(
        std::sync::Arc<qcore_c_gateway::member_run_facts::RunMemberFacts>,
        Value,
    )>,
    /// Card AUDIT-COMPOSITION: the digests of the contract bundles this run's
    /// rows rest on, for the components' provenance; `Err` when a digest
    /// could not be taken, which refuses every component (the rows are then
    /// `Unknown`). Whole-report data, like the evidence record.
    contract_bundles: Result<BTreeMap<String, String>, String>,
}

impl AuditReport {
    pub fn new(revision: &str) -> Self {
        Self {
            revision: revision.into(),
            units: Vec::new(),
            diagnostic_counts: BTreeMap::new(),
            source_contracts: None,
            evidence: None,
            plugin_links: None,
            bounds: Vec::new(),
            bounds_evidence: Vec::new(),
            null: Vec::new(),
            null_evidence: Vec::new(),
            loop_measures: Vec::new(),
            callback_members: Vec::new(),
            member_prepass: None,
            contract_bundles: Ok(BTreeMap::new()),
        }
    }

    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): one analysed
    /// unit's member facts (`None` when it recorded none).
    pub(crate) fn record_callback_members(
        &mut self,
        library: &str,
        file: &str,
        facts: Option<qcore_c_gateway::callback_member_closure::MemberFacts>,
        state: Option<qcore_c_gateway::constructed_state::StateFacts>,
    ) {
        self.callback_members
            .push(crate::callback_member_fold::UnitFacts {
                library: library.to_owned(),
                file: file.to_owned(),
                facts,
                state,
            });
    }

    /// Card MEMBER-PREPASS-OTHER-ROUTES: the member and state facts every
    /// unit recorded so far, in unit order (what a checkpoint carries per
    /// unit).
    #[must_use]
    pub fn callback_member_facts(&self) -> &[crate::callback_member_fold::UnitFacts] {
        &self.callback_members
    }

    /// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (rule 12n, R1): the run's member
    /// facts the pre-pass read, which the fold reads instead of the units'.
    pub fn set_member_prepass(
        &mut self,
        facts: std::sync::Arc<qcore_c_gateway::member_run_facts::RunMemberFacts>,
        section: Value,
    ) {
        self.member_prepass = Some((facts, section));
    }

    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12n): fold the run's
    /// member closure over the rows, demoting what it does not keep; the
    /// section, `Value::Null` when no declaration invokes a member, and how
    /// many unit rows it moved from `Safe` and from `Pending` to `Unknown`.
    pub fn fold_callback_members(
        &mut self,
        contracts: &ResourceContracts,
    ) -> (Value, usize, usize) {
        let before = self
            .units
            .iter()
            .map(|row| row["status"].as_str().unwrap_or("").to_owned())
            .collect::<Vec<_>>();
        let mut section = crate::callback_member_fold::fold_with(
            &mut self.units,
            &self.callback_members,
            contracts,
            &mut self.diagnostic_counts,
            self.member_prepass
                .as_ref()
                .map(|(facts, _)| facts.as_ref()),
        );
        // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: the pre-pass's evidence.
        if let (Some((_, prepass)), false) = (&self.member_prepass, section.is_null()) {
            section["prepass"] = prepass.clone();
        }
        let (lifetimes, lifetime_demoted) = crate::loop_lifetime_fold::fold(
            &mut self.units,
            contracts,
            self.member_prepass
                .as_ref()
                .map(|(facts, _)| facts.as_ref()),
            &mut self.diagnostic_counts,
        );
        if !lifetimes.is_null() {
            if section.is_null() {
                section = serde_json::json!({});
            }
            section["lifetime_protocols"] = lifetimes;
        }
        // Card OP-CONSTRUCTOR-DECLARATIONS: a function the fold demoted is
        // demoted in the property rows' custody column too, and their
        // combined verdict recomputed, so no combined row reads a custody
        // verdict the published custody row no longer has.
        crate::loop_lifetime_fold::demote_properties(
            &mut self.null,
            &mut self.bounds,
            &lifetime_demoted,
        );
        let demoted = crate::callback_member_fold::demoted_functions(&section);
        if !demoted.is_empty() {
            crate::callback_member_fold::demote_property_rows(
                &mut self.null,
                &mut self.bounds,
                &demoted,
            );
        }
        let mut from_safe = 0;
        let mut from_pending = 0;
        for (row, before) in self.units.iter().zip(before) {
            if row["status"].as_str() == Some("Unknown") {
                match before.as_str() {
                    "Safe" => from_safe += 1,
                    "Pending" => from_pending += 1,
                    _ => {}
                }
            }
        }
        (section, from_safe, from_pending)
    }

    /// Card NULL-SAFETY-PROPERTY: one function's null row.
    pub(crate) fn record_null(&mut self, row: Value) {
        self.null.push(row);
    }

    /// Card NULL-CANDIDATE-TRIAGE: one unit's null evidence.
    pub(crate) fn record_null_evidence(&mut self, unit: Value) {
        self.null_evidence.push(unit);
    }

    /// Card NULL-SAFETY-PROPERTY: write the null selection's rows (JSON
    /// lines) and their tally beside them, when the run selected the
    /// property. The custody report is written elsewhere and never reads
    /// these. Returns the folded rows written (card OUTER-CONTRACT: the
    /// runtime projection reads them), `None` when the property is not
    /// selected.
    pub fn write_null_selection(&self) -> Result<Option<Vec<Value>>, String> {
        let Some(path) = std::env::var_os(crate::projection::NULL_SELECTION) else {
            return Ok(None);
        };
        let path = std::path::PathBuf::from(path);
        // Card NULL-CANDIDATE-TRIAGE (rules 14t-14v): the run's fold of the
        // declarations' standing, the withdrawals and cross-unit travel.
        let mut rows = self.null.clone();
        let fold = crate::null_fold::fold(
            &mut rows,
            &self.null_evidence,
            &crate::bounds_selection::null_environment(),
        );
        let hypothesis_view = crate::null_hypotheses::View::new(
            &crate::bounds_selection::null_environment(),
            &crate::bounds_selection::null_hypothesis_sources()?,
        )?;
        // Bind the null property's own consulted premise records after its fold.
        // Contexts follow the same unit/configuration order as the custody rows.
        let mut contexts = std::collections::BTreeMap::<
            (String, String, String),
            std::collections::VecDeque<Value>,
        >::new();
        for unit in &self.units {
            for function in unit["functions"].as_array().into_iter().flatten() {
                if let Some(context) = function.get("hypothesis_context") {
                    let key = (
                        unit["library"].as_str().unwrap_or_default().to_owned(),
                        unit["file"].as_str().unwrap_or_default().to_owned(),
                        function["function"].as_str().unwrap_or_default().to_owned(),
                    );
                    contexts.entry(key).or_default().push_back(context.clone());
                }
            }
        }
        for row in &mut rows {
            let records = row["null"]["premises"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| qcore_schemas::hypothesis::ErasedPremise {
                    name: p["name"].as_str().unwrap_or_default().to_owned(),
                    statement: p["statement"].as_str().unwrap_or_default().to_owned(),
                    origin: "declared".into(),
                })
                .collect::<Vec<_>>();
            let mut graph = qcore_contract::hypothesis::legacy_records(&records)?;
            let references = hypothesis_view.attach(row, &mut graph)?;
            if !references.is_empty() {
                row["hypothesis_binding_requirements"] = json!(references);
            }
            row["hypothesis_graph"] = serde_json::to_value(&graph).map_err(|e| e.to_string())?;
            row["hypothesis_graph_sha256"] = qcore_contract::hypothesis::digest(&graph)?.into();
            let key = (
                row["library"].as_str().unwrap_or_default().to_owned(),
                row["file"].as_str().unwrap_or_default().to_owned(),
                row["function"].as_str().unwrap_or_default().to_owned(),
            );
            if let Some(mut context) = contexts.get_mut(&key).and_then(|q| q.pop_front()) {
                context["property"] = qcore_c_gateway::null_deref::NULL_PROPERTY.into();
                context["scope"] =
                    json!({"scope":row["null"]["scope"],"assumptions":row["null"]["assumptions"]})
                        .to_string()
                        .into();
                context["assumptions"] = qcore_schemas::sha256_hex(
                    json!({"custody":context["assumptions"],"null_identity":row["identity"]})
                        .to_string()
                        .as_bytes(),
                )
                .into();
                row["hypothesis_context"] = context;
            } else if !records.is_empty() {
                return Err(
                    "hypothesis: null premise row lacks its caller-held compile context".into(),
                );
            }
        }
        let mut lines = String::new();
        for row in &rows {
            lines.push_str(&serde_json::to_string(row).map_err(|e| e.to_string())?);
            lines.push('\n');
        }
        fs::write(&path, lines).map_err(|e| format!("could not write {}: {e}", path.display()))?;
        let mut summary = crate::null_tally::summary(&rows);
        summary["fold"] = fold;
        println!("{}", crate::null_tally::line(&summary));
        let mut bytes = serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let summary_path = path.with_extension("summary.json");
        fs::write(&summary_path, bytes)
            .map_err(|e| format!("could not write {}: {e}", summary_path.display()))?;
        Ok(Some(rows))
    }

    /// Card OUTER-CONTRACT (gateway rule 31): write the runtime projection
    /// of the published rows (custody, and `null_rows` when the null
    /// property was selected) beside them, when the run selected it
    /// (`QCORE_OUTER_PROJECTION`). The static report is written before and
    /// is never read back: no static byte moves.
    pub fn write_outer_projection(
        &self,
        contracts: &ResourceContracts,
        null_rows: Option<&[Value]>,
    ) -> Result<(), String> {
        if !crate::outer_selection::selected() {
            return Ok(());
        }
        let premises = crate::contract_dependencies::premises(contracts)?;
        let Some(projection) = crate::outer_selection::project(&self.units, null_rows, &premises)
        else {
            return Err("the outer projection is selected but no outer environment was installed on this route".into());
        };
        crate::outer_selection::write(&projection)
    }

    /// Card CG16-INDEX-AND-OFFSET-EXTENTS: one function's bounds row.
    pub(crate) fn record_bounds(&mut self, row: Value) {
        self.bounds.push(row);
    }

    /// Card W45-FB-ENGINE-PROGRESS-MEASURE (rule 44): one function's
    /// loop-measure row.
    pub(crate) fn record_loop_measures(&mut self, row: Value) {
        self.loop_measures.push(row);
    }

    /// Card W45-FB-ENGINE-PROGRESS-MEASURE (rule 44): write the loop-measure
    /// rows (JSON lines) and their tally beside them, when the run selected
    /// the property. Each row carries the custody status the report's own
    /// unit row publishes, read here after every fold, and never changes it:
    /// the custody report is written elsewhere and never reads these rows.
    pub fn write_loop_measure_selection(&self) -> Result<(), String> {
        let Some(path) = std::env::var_os(crate::projection::LOOP_MEASURE_SELECTION) else {
            return Ok(());
        };
        let path = std::path::PathBuf::from(path);
        let mut custody: BTreeMap<(String, String, String), String> = BTreeMap::new();
        for unit in &self.units {
            let library = unit["library"].as_str().unwrap_or_default();
            let file = unit["file"].as_str().unwrap_or_default();
            for function in unit["functions"].as_array().into_iter().flatten() {
                if let (Some(name), Some(status)) =
                    (function["function"].as_str(), function["status"].as_str())
                {
                    custody.insert(
                        (library.to_owned(), file.to_owned(), name.to_owned()),
                        status.to_owned(),
                    );
                }
            }
        }
        let mut lines = String::new();
        let mut loops: BTreeMap<String, usize> = BTreeMap::new();
        let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for row in &self.loop_measures {
            let mut row = row.clone();
            let key = (
                row["library"].as_str().unwrap_or_default().to_owned(),
                row["file"].as_str().unwrap_or_default().to_owned(),
                row["function"].as_str().unwrap_or_default().to_owned(),
            );
            row["custody"] = custody
                .get(&key)
                .map_or(Value::Null, |status| Value::String(status.clone()));
            for fact in row["loops"].as_array().into_iter().flatten() {
                let outcome = if fact["measure"].is_object() {
                    *kinds
                        .entry(
                            fact["measure"]["bound_kind"]
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                        )
                        .or_default() += 1;
                    "measured"
                } else {
                    *refusals
                        .entry(fact["refusal"].as_str().unwrap_or("unread").to_owned())
                        .or_default() += 1;
                    "refused"
                };
                *loops.entry(outcome.into()).or_default() += 1;
            }
            if row["error"].is_string() {
                *loops.entry("functions-unread".into()).or_default() += 1;
            }
            lines.push_str(&serde_json::to_string(&row).map_err(|e| e.to_string())?);
            lines.push('\n');
        }
        fs::write(&path, lines).map_err(|e| format!("could not write {}: {e}", path.display()))?;
        let summary = json!({
            "schema_version": 1,
            "property": qcore_c_gateway::loop_measure::LOOP_MEASURE_PROPERTY,
            "model_version": qcore_c_gateway::loop_measure::MODEL_VERSION,
            "scope": "a bound fact per loop of every projected function (gateway rule 44): a checked measure for an admitted counted `for` loop, or a named refusal; recorded beside the custody status, which it never reads or changes; no progress verdict",
            "functions": self.loop_measures.len(),
            "loops": loops,
            "measured_by_bound_kind": kinds,
            "refused_by_reason": refusals,
        });
        let mut bytes = serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let summary_path = path.with_extension("summary.json");
        fs::write(&summary_path, bytes)
            .map_err(|e| format!("could not write {}: {e}", summary_path.display()))
    }

    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: one unit's body
    /// evidence for bounds declarations.
    pub(crate) fn record_bounds_evidence(&mut self, unit: Value) {
        self.bounds_evidence.push(unit);
    }

    /// Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: every unit's body
    /// evidence recorded so far, from which a phase's bounds refutation table
    /// is read.
    #[must_use]
    pub fn bounds_evidence(&self) -> &[Value] {
        &self.bounds_evidence
    }

    /// Card CG16-INDEX-AND-OFFSET-EXTENTS: write the bounds selection's rows
    /// (JSON lines) and their tally beside them, when the run selected the
    /// property. The custody report is written elsewhere and never reads these.
    pub fn write_bounds_selection(&self) -> Result<(), String> {
        let Some(path) = std::env::var_os(crate::projection::BOUNDS_SELECTION) else {
            return Ok(());
        };
        let path = std::path::PathBuf::from(path);
        let mut lines = String::new();
        let mut tally: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
        for row in &self.bounds {
            lines.push_str(&serde_json::to_string(row).map_err(|e| e.to_string())?);
            lines.push('\n');
            for key in ["custody", "combined"] {
                let value = row[key].as_str().unwrap_or("absent").to_owned();
                *tally
                    .entry(key.into())
                    .or_default()
                    .entry(value)
                    .or_default() += 1;
            }
            let bounds = row["bounds"]["verdict"]
                .as_str()
                .unwrap_or("absent")
                .to_owned();
            *tally
                .entry("bounds".into())
                .or_default()
                .entry(bounds)
                .or_default() += 1;
            let mut codes = std::collections::BTreeSet::new();
            for reason in row["bounds"]["reasons"].as_array().into_iter().flatten() {
                let code = reason
                    .as_str()
                    .unwrap_or("")
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .to_owned();
                codes.insert(code);
            }
            for code in codes {
                *reasons.entry(code).or_default() += 1;
            }
        }
        fs::write(&path, lines).map_err(|e| format!("could not write {}: {e}", path.display()))?;
        let summary = json!({
            "schema_version": 1,
            "property": qcore_c_gateway::index_extent::BOUNDS_PROPERTY,
            "scope": "the bounds property of every projected function, reported separately from custody; `combined` joins the custody and bounds rows of a request naming both, a missing row counting Unknown; Unknown and Pending stay in every denominator",
            "functions": self.bounds.len(),
            "tally": tally,
            "functions_by_reason_code": reasons,
        });
        let mut bytes = serde_json::to_vec_pretty(&summary).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        let summary_path = path.with_extension("summary.json");
        fs::write(&summary_path, bytes)
            .map_err(|e| format!("could not write {}: {e}", summary_path.display()))?;
        // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the bounds environment the
        // route installed (selection, modules, pins), beside the rows.
        if let Some(mut record) = crate::bounds_selection::installed_record() {
            // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: the body
            // evidence of every declaration and the bounds Safe roll-up.
            let evidence = crate::bounds_evidence::section(
                &self.bounds_evidence,
                &self.bounds,
                &crate::bounds_selection::environment(),
            );
            println!("{}", crate::bounds_evidence::tally(&evidence));
            record["body_evidence"] = evidence;
            let mut bytes = serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?;
            bytes.push(b'\n');
            let contracts_path = path.with_extension("contracts.json");
            fs::write(&contracts_path, bytes)
                .map_err(|e| format!("could not write {}: {e}", contracts_path.display()))?;
        }
        Ok(())
    }

    /// An empty report over the same revision, into which one unit's rows are
    /// recorded apart from every other unit's (card
    /// **W44-AUDIT-PARALLEL-UNITS**). Fold it back with [`Self::absorb`].
    /// It folds the same checker links (card W45-USER-CONTRACT-PLUGINS).
    pub fn fragment(&self) -> Self {
        let mut fragment = Self::new(&self.revision);
        fragment.plugin_links.clone_from(&self.plugin_links);
        fragment
    }

    /// Card W45-USER-CONTRACT-PLUGINS: fold these checker links into every
    /// unit recorded from now on.
    pub fn set_plugin_links(&mut self, links: std::sync::Arc<crate::user_plugins::PluginLinks>) {
        self.plugin_links = Some(links);
    }

    /// Append a [`Self::fragment`]'s rows after this report's rows and add its
    /// diagnostic counts.
    ///
    /// Recording units into fragments and absorbing the fragments in unit order
    /// yields exactly the report that recording every unit here in that order
    /// would: rows are appended in order, the only cross-unit aggregate is a
    /// per-category sum, and every attachment (`record_enumeration`,
    /// `record_configuration`) targets the row its own unit just recorded,
    /// which lives in the same fragment. A fragment of another revision, or one
    /// carrying source-contract evidence (which is whole-report data, never
    /// per-unit), is refused rather than merged.
    pub fn absorb(&mut self, fragment: Self) -> Result<(), String> {
        let Self {
            revision,
            units,
            diagnostic_counts,
            source_contracts,
            evidence,
            plugin_links: _,
            bounds,
            bounds_evidence,
            null,
            null_evidence,
            loop_measures,
            callback_members,
            member_prepass: _,
            contract_bundles: _,
        } = fragment;
        if revision != self.revision {
            return Err(format!(
                "report fragment for revision {revision} cannot join a report for {}",
                self.revision
            ));
        }
        if source_contracts.is_some() {
            return Err("a per-unit report fragment carries no source-contract evidence".into());
        }
        if evidence.is_some() {
            return Err("a per-unit report fragment carries no evidence-mode record".into());
        }
        self.units.extend(units);
        self.bounds.extend(bounds);
        self.bounds_evidence.extend(bounds_evidence);
        self.null.extend(null);
        self.null_evidence.extend(null_evidence);
        self.loop_measures.extend(loop_measures);
        self.callback_members.extend(callback_members);
        for (category, count) in diagnostic_counts {
            *self.diagnostic_counts.entry(category).or_default() += count;
        }
        Ok(())
    }

    /// Card CORPUS-GROUP-CHECKPOINTS: a per-unit fragment's contents, which
    /// a pinned group checkpoint stores and [`Self::from_fragment_parts`]
    /// restores. Refused for a fragment carrying whole-report data, exactly
    /// as [`Self::absorb`] refuses it.
    pub(crate) fn into_fragment_parts(self) -> Result<FragmentParts, String> {
        let Self {
            revision: _,
            units,
            diagnostic_counts,
            source_contracts,
            evidence,
            plugin_links,
            bounds,
            bounds_evidence,
            null,
            null_evidence,
            loop_measures,
            callback_members,
            member_prepass,
            contract_bundles: _,
        } = self;
        if source_contracts.is_some() || evidence.is_some() || member_prepass.is_some() {
            return Err("a per-unit report fragment carries no whole-report data".into());
        }
        if plugin_links.is_some() {
            return Err(
                "a checkpointed fragment carries no checker links (no pinned-route binding)".into(),
            );
        }
        // Card W45-FB-ENGINE-PROGRESS-MEASURE: a checkpoint carries no
        // loop-measure rows, so a fragment holding some is refused rather
        // than stored without them.
        if !loop_measures.is_empty() {
            return Err("a checkpointed fragment carries no loop-measure rows".into());
        }
        Ok(FragmentParts {
            units,
            diagnostic_counts,
            bounds,
            bounds_evidence,
            null,
            null_evidence,
            callback_members,
        })
    }

    /// Card CORPUS-GROUP-CHECKPOINTS: the fragment [`Self::into_fragment_parts`]
    /// took apart, over `revision`.
    pub(crate) fn from_fragment_parts(revision: &str, parts: FragmentParts) -> Self {
        let FragmentParts {
            units,
            diagnostic_counts,
            bounds,
            bounds_evidence,
            null,
            null_evidence,
            callback_members,
        } = parts;
        let mut fragment = Self::new(revision);
        fragment.units = units;
        fragment.diagnostic_counts = diagnostic_counts;
        fragment.bounds = bounds;
        fragment.bounds_evidence = bounds_evidence;
        fragment.null = null;
        fragment.null_evidence = null_evidence;
        fragment.callback_members = callback_members;
        fragment
    }

    /// Every unit row recorded so far, in order (card
    /// W45-CONTRACT-DEFERRED-REVIEW: read by the pending-review inventory).
    #[must_use]
    pub fn unit_rows(&self) -> &[Value] {
        &self.units
    }

    /// The most recent unit row, to attach what its own unit derives (card
    /// W47-REVIEW-HOLD-CONCRETE-REASON: `review_holds`).
    pub(crate) fn last_unit_mut(&mut self) -> Option<&mut Value> {
        self.units.last_mut()
    }

    /// The four-value verdict the most recent unit row publishes, `None` when
    /// there is no row or its status is not a verdict (a gateway failure, a
    /// data-only unit). Card W45-USER-CONTRACT-PLUGINS.
    #[must_use]
    pub fn last_unit_verdict(&self) -> Option<qcore_schemas::verdict::Verdict> {
        self.units.last()?["status"].as_str()?.parse().ok()
    }

    /// Card AUDIT-COMPOSITION: record the digest of the resource-contract
    /// environment this run's rows rest on, as a contract bundle of the
    /// components' provenance. A digest that cannot be taken refuses every
    /// component, so the rows are `Unknown`, never published unprovenanced.
    pub(crate) fn record_contract_bundle(&mut self, contracts: &ResourceContracts) {
        match (
            &mut self.contract_bundles,
            crate::report_components::contracts_digest(contracts),
        ) {
            (Ok(bundles), Ok(digest)) => {
                bundles.insert(
                    crate::report_components::RESOURCE_CONTRACTS_BUNDLE.into(),
                    digest,
                );
            }
            (bundles, Err(error)) => *bundles = Err(error),
            (Err(_), Ok(_)) => {}
        }
    }

    pub(crate) fn set_source_contracts(&mut self, evidence: Value) {
        self.source_contracts = Some(evidence);
    }

    /// Record the evidence mode of the whole run (card W44-QUIRIN-AUDIT-PROVIDERS).
    pub(crate) fn set_evidence(&mut self, evidence: Value) {
        self.evidence = Some(evidence);
    }

    /// Retain a selected configuration whose source/build premises could not
    /// be established. Missing evidence is never a data-only or Safe unit.
    pub fn record_unknown(
        &mut self,
        library: &str,
        file: &str,
        functions: &[String],
        refusal: &UnitRefusal,
    ) {
        let reason = refusal.reason();
        self.units.push(json!({
            "library": library, "file": file, "status": "Unknown", "reason": reason,
            "reason_code": reason_code_value(refusal.category()),
            "source_functions": functions, "functions": [], "not_projected": functions,
            "diagnostic_counts": {},
        }));
    }

    /// Attach the enumerated main-source definition inventory to the
    /// immediately preceding unit (unit **E01-F1**).
    ///
    /// This block *refines* the row and replaces nothing: `source_functions`
    /// is the same denominator the analyzer selected over, and
    /// `not_projected` keeps its meaning. What it adds is the honest account
    /// of that denominator — which definitions exist only after preprocessing,
    /// which ones no implemented analysis reads and why, and whether the
    /// definition set was established at all. A refused completeness claim is
    /// a scoped `Unknown` about the *inventory*, never a claim about the
    /// analysed functions.
    pub fn record_enumeration(&mut self, inventory: &SourceDefinitionInventory) {
        let unanalyzable = inventory
            .unanalyzable()
            .iter()
            .map(|definition| {
                let reason = definition.form().reason();
                json!({
                    "function": definition.function(),
                    "status": "unknown",
                    "form": definition.form().as_str(),
                    "reason": reason,
                    "reason_code": reason_code(Some(reason)),
                })
            })
            .collect::<Vec<_>>();
        let refusals = inventory
            .refusals()
            .iter()
            .map(|refusal| {
                let reason = refusal.reason();
                json!({
                    "ground": refusal.as_str(),
                    "reason": reason,
                    "reason_code": reason_code(Some(reason)),
                })
            })
            .collect::<Vec<_>>();
        let Some(unit) = self.units.last_mut() else {
            return;
        };
        unit["enumeration"] = json!({
            "completeness": if inventory.is_complete() { "established" } else { "refused" },
            "refusals": refusals,
            "absent_from_raw_scan": inventory.absent_from_raw_scan(),
            "unanalyzable": unanalyzable,
        });
    }

    /// Attach the exact configuration inventory to the immediately preceding
    /// unit, including rejected commands which cannot mint a compile-task proof.
    pub fn record_configuration(&mut self, configuration: Value) {
        if let Some(unit) = self.units.last_mut() {
            unit["compile_configuration"] = configuration;
        }
    }

    /// Attach a no-build unit's assumed configuration to the immediately
    /// preceding unit (card **W44-CONTRACTS-ONLY-NO-BUILD**). Its verdict is
    /// relative to exactly these assumptions.
    pub fn record_assumptions(&mut self, assumptions: Value) {
        if let Some(unit) = self.units.last_mut() {
            unit["assumptions"] = assumptions;
        }
    }

    /// The opt-in linked-program resolution premise conditions every unit
    /// that was evaluated with imported cross-unit W4 summaries.
    pub fn record_cross_unit_premise(&mut self, premise: &Value) {
        for unit in &mut self.units {
            unit["cross_unit_link_resolution"] = premise.clone();
        }
    }

    /// Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): the preceding
    /// unit could reach more cross-unit homes than the per-unit replay budget
    /// and held no import; its calls to them stayed uncontracted.
    pub fn record_cross_unit_withheld(&mut self, reason: &str) {
        if let Some(unit) = self.units.last_mut() {
            unit["cross_unit_imports_withheld"] = json!(reason);
        }
    }

    /// Card **LINK-STANDING** (rule 40): the held homes a derived link
    /// resolution refused for this caller, with their reasons.
    pub fn record_cross_unit_link_refusals(&mut self, refusals: &[(String, String)]) {
        if let Some(unit) = self.units.last_mut() {
            unit["cross_unit_link_refusals"] = refusals
                .iter()
                .map(|(function, reason)| json!({"function": function, "reason": reason}))
                .collect();
        }
    }

    pub fn record_data_only(&mut self, library: &str, file: &str, error: Option<&LegacyCError>) {
        let reason = error.map(ToString::to_string);
        self.units.push(json!({
            "library": library, "file": file,
            "status": if error.is_some() { "gateway_failure" } else { "data_only" },
            "reason": reason,
            "reason_code": reason_code(reason.as_deref()),
            "source_functions": [], "functions": [], "not_projected": [],
            "diagnostic_counts": {},
        }));
    }

    /// Card **GATEWAY-C-LOCATION**: render one diagnostic's site.
    ///
    /// The presented coordinate is the **original** pre-preprocessing
    /// `file:line`, because the preprocessed translation unit is a text nobody
    /// reads. The preprocessed position stays in the row's `line` key, which is
    /// the leaf identity every pinned corpus comparison is keyed on; it reaches
    /// presented text only as the explicit fallback when no original coordinate
    /// was established, where it names itself as internal.
    fn site(locations: Option<&SourceLineMap>, file: &str, line: usize) -> String {
        locations
            .and_then(|map| map.present(line, file))
            .unwrap_or_else(|| unlocated_site(line))
    }

    /// The row value for an original coordinate: the presented `file:line`, or
    /// `null` where none is established. `null` asserts nothing.
    fn origin(locations: Option<&SourceLineMap>, file: &str, line: usize) -> Value {
        match locations.and_then(|map| map.present(line, file)) {
            Some(origin) => json!(origin),
            None => Value::Null,
        }
    }

    pub fn record(
        &mut self,
        library: &str,
        file: &str,
        source_functions: &[String],
        result: &Result<UnrestrictedLegacyResult, LegacyCError>,
        locations: Option<&SourceLineMap>,
    ) -> Vec<String> {
        // Card W45-USER-CONTRACT-PLUGINS: the unit's status is its four-value
        // verdict; a `Pending` unit is published through the `Unknown` variant.
        let (status, summary, reason) = match result {
            Ok(verdict @ UnrestrictedLegacyResult::Safe(certificate)) => {
                (verdict.verdict().as_str(), Some(&certificate.summary), None)
            }
            Ok(verdict @ UnrestrictedLegacyResult::Unsafe(summary)) => {
                (verdict.verdict().as_str(), Some(summary), None)
            }
            Ok(
                verdict @ UnrestrictedLegacyResult::Unknown {
                    partial, reason, ..
                },
            ) => (
                verdict.verdict().as_str(),
                partial.as_ref(),
                Some(reason.clone()),
            ),
            Err(error) => ("gateway_failure", None, Some(error.to_string())),
        };
        let mut functions = Vec::new();
        let mut counts = BTreeMap::<String, usize>::new();
        let mut examples = Vec::new();
        if let Some(summary) = summary {
            for function in summary.functions.values() {
                let mut diagnostics = Vec::new();
                for diagnostic in &function.diagnostics {
                    let (category, description) = diagnostic_description(diagnostic, &|line| {
                        Self::site(locations, file, line)
                    });
                    *counts.entry(category.into()).or_default() += 1;
                    *self.diagnostic_counts.entry(category.into()).or_default() += 1;
                    if examples.len() < 3 {
                        examples.push(format!(
                            "{}: {}: {category}: {description}",
                            diagnostic.function,
                            Self::site(locations, file, diagnostic.line),
                        ));
                    }
                    diagnostics.push(json!({
                        "line": diagnostic.line,
                        "origin": Self::origin(locations, file, diagnostic.line),
                        "category": category,
                        "description": description,
                    }));
                }
                let mut row = json!({
                    "function": function.function,
                    "status": format!("{:?}", function.status),
                    "diagnostics": diagnostics,
                });
                if let Some(scope) = qcore_c_gateway::loop_lifetime_summary::body_scope(function) {
                    row["lifetime_scope"] = scope.into();
                }
                // Card NULL-SAFETY-PROPERTY (rule 14n): every dereference
                // this verdict read non-null under decision 1's moved
                // question, a named premise. Omitted when there is none, so
                // every other row is byte-identical.
                if !function.null_premises.is_empty() {
                    row["null_premises"] = function
                        .null_premises
                        .iter()
                        .map(|premise| {
                            json!({
                                "name": premise.name(&function.function),
                                "line": premise.line,
                                "origin": Self::origin(locations, file, premise.line),
                                "variable": premise.variable,
                            })
                        })
                        .collect();
                }
                // Card ARM-OBLIGATIONS: the obligations of the rule packages'
                // admissions this row's unit analysis used: premises are
                // among the row's assumptions (folded into its verdict
                // contract's `assumptions`), open obligations keep it out of
                // `Safe`. Omitted when there is none, so every other row is
                // byte-identical.
                if let Some(field) = rule_obligations_field(&function.rule_obligations) {
                    row[RULE_OBLIGATIONS_KEY] = field;
                }
                // Card W45-FB-RETRY-LIFETIME: every counted-user clause
                // checked against this body, with its verdict. Omitted when
                // there is none, so every other row is byte-identical.
                if !function.counted_users.is_empty() {
                    row["user_count_clauses"] = function
                        .counted_users
                        .iter()
                        .map(|record| {
                            json!({
                                "clause": record.clause,
                                "verdict": record.verdict.spelling(),
                                "line": record.line,
                                "origin": if record.line == 0 {
                                    Value::Null
                                } else {
                                    Self::origin(locations, file, record.line)
                                },
                                "reason": record.reason,
                            })
                        })
                        .collect::<Vec<_>>()
                        .into();
                }
                // Card W45-REFUTED-CONTRACT-WITHDRAWAL: what this body refutes
                // of its own declaration, the input of the refutation table.
                // Omitted when there is none, so every other row is
                // byte-identical.
                if !function.refutations.is_empty() {
                    row["contract_refutations"] = function
                        .refutations
                        .iter()
                        .map(|refutation| {
                            json!({
                                "kind": refutation.kind.as_str(),
                                "clause": refutation.clause,
                                "line": refutation.line,
                                "origin": if refutation.line == 0 {
                                    Value::Null
                                } else {
                                    Self::origin(locations, file, refutation.line)
                                },
                                "reason": refutation.reason,
                            })
                        })
                        .collect::<Vec<_>>()
                        .into();
                }
                functions.push(row);
            }
        }
        let not_projected = source_functions
            .iter()
            .filter(|name| summary.is_none_or(|summary| !summary.functions.contains_key(*name)))
            .collect::<Vec<_>>();
        self.units.push(json!({
            "library": library, "file": file, "status": status, "reason": reason,
            "reason_code": reason_code(reason.as_deref()),
            "source_functions": source_functions, "functions": functions,
            "not_projected": not_projected, "diagnostic_counts": counts,
            "origin_coordinates": origin_coordinates(locations),
        }));
        // Card W45-USER-CONTRACT-PLUGINS: the callee chains of this unit.
        if let (Some(links), Some(row)) = (&self.plugin_links, self.units.last_mut()) {
            crate::user_plugins::fold_unit(links, row, result);
        }
        if counts.is_empty() {
            return Vec::new();
        }
        let total: usize = counts.values().sum();
        let mut lines = vec![format!(
            "resource diagnostics: {}",
            counts
                .iter()
                .map(|(category, count)| format!("{category}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        )];
        lines.extend(examples);
        if total > 3 {
            lines.push(format!(
                "{} more diagnostic(s); full inventory in optional resource report",
                total - 3
            ));
        }
        lines
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        self.write_with_revision_key(path, "source_revision")
    }

    /// Compatibility with an existing report schema; verdict contents are unchanged.
    ///
    /// Card AUDIT-COMPOSITION: every unit row becomes a component of the
    /// integration semigroup, written to the sidecar
    /// [`crate::report_components::sidecar_path`] first; the report's `units`
    /// are then rendered from that sidecar, never written from the rows
    /// directly ([`crate::report_components`]).
    pub fn write_with_revision_key(&self, path: &Path, revision_key: &str) -> Result<(), String> {
        let provenance = self
            .contract_bundles
            .clone()
            .map(crate::report_components::provenance);
        let sidecar = crate::report_components::sidecar(&self.units, &provenance);
        let sidecar_path = crate::report_components::sidecar_path(path);
        let written = fs::File::create(&sidecar_path).and_then(|file| {
            let mut writer = BufWriter::new(file);
            serde_json::to_writer(&mut writer, &sidecar)?;
            writer.write_all(b"\n")?;
            writer.flush()
        });
        written.map_err(|error| format!("could not write {}: {error}", sidecar_path.display()))?;
        let rows = crate::report_components::render(&sidecar);
        let file = fs::File::create(path)
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
        let mut writer = BufWriter::new(file);
        self.write_fields(
            &mut writer,
            self.borrowed_document(revision_key, ReportField::DerivedUnits(&rows)),
        )
        .map_err(|error| format!("could not write {}: {error}", path.display()))
    }

    #[cfg(test)]
    fn write_document(&self, writer: &mut impl Write, revision_key: &str) -> Result<(), String> {
        self.write_fields(
            writer,
            self.borrowed_document(revision_key, ReportField::Units(&self.units)),
        )
    }

    fn write_fields(
        &self,
        writer: &mut impl Write,
        fields: BTreeMap<&str, ReportField<'_>>,
    ) -> Result<(), String> {
        serde_json::to_writer_pretty(&mut *writer, &fields).map_err(|error| error.to_string())?;
        writer.write_all(b"\n").map_err(|error| error.to_string())?;
        writer.flush().map_err(|error| error.to_string())
    }

    /// Materialize only for callers that require an owned checkpoint document.
    pub(crate) fn document(&self, revision_key: &str) -> Value {
        serde_json::to_value(self.borrowed_document(revision_key, ReportField::Units(&self.units)))
            .expect("report fields are JSON values")
    }

    fn borrowed_document<'a>(
        &'a self,
        revision_key: &'a str,
        units: ReportField<'a>,
    ) -> BTreeMap<&'a str, ReportField<'a>> {
        let mut fields = BTreeMap::new();
        fields.insert("schema_version", ReportField::Version(1));
        fields.insert(revision_key, ReportField::Text(&self.revision));
        fields.insert("scope", ReportField::Text("generic resource projection; separate chunk/callback source profiles are not certificates here; an admitted scalar access through a pointer decides custody only, never bounds, alignment, or the liveness or nullness of an object the projection does not track; an index or pointer traversal (a stepped pointer, a moved index, a typed read in a loop condition) decides custody, use after release and invalid release only: no bounds claim; an indexed collection's element ranges decide which indices owe a release, never that an index is inside the allocation; a typed read handed to a borrowed or data-tail call actual (a scalar, a pointer read, a sub-object address) decides its base's custody only, never the liveness of an object a read pointer names"));
        fields.insert("line_coordinates", ReportField::Text("`line` is the Clang -E -P preprocessed translation unit's line, not the original file; line 0 is synthetic. It is retained as the internal leaf identity every pinned comparison is keyed on, and is not a coordinate anyone can open; `origin` beside it is the original pre-preprocessing coordinate and is what diagnostic text presents"));
        fields.insert("origin_meaning", ReportField::Text("the original `file:line` of the C a person wrote (card GATEWAY-C-LOCATION), derived from the line markers of the same unit's marked preprocessing and paired content line by content line with the analysed -E -P text; a diagnostic inside a macro expansion carries the expansion site's line. No presented file spelling is ever read from a flagless marker: an included file is named by its own entering (flag 1) marker and the primary input by this report's own `file`, so a `#line` directive in analysed source can neither move the audited scope (standing constraint 12) nor name a file here. A flagless marker that renames its frame withdraws that frame's coordinates instead of adopting the name, and `null` asserts no coordinate at all; `origin_coordinates` carries the per-unit account"));
        fields.insert("not_projected_meaning", ReportField::Text("raw source definitions absent from the projection, including build-excluded definitions; see unit reason for parser failures"));
        fields.insert("enumeration_meaning", ReportField::Text("the enumerated main-source definition denominator (E01-F1): `absent_from_raw_scan` names main-source definitions the historical raw token scan did not see — produced by macro expansion, or lost by that scan for another reason — each an ordinary definition in the preprocessed unit that the unmodified analyzer reads; `unanalyzable` definitions are counted but not read by the resource analyzer and carry their own classified Unknown row; `completeness: refused` means the definition set itself could not be established on the named ground, so no whole-unit Safe claim is published over it"));
        fields.insert("reason_code_meaning", ReportField::Text("closed reason-code vocabulary of QCORE_SPEC_r0_3 L21.1 classifying the sibling reason; null asserts no category, either because there is no reason or because the closed vocabulary does not recognise it; `environment-unavailable` says nothing about the analysed source and never contributes to a provisional Unsafe"));
        fields.insert(
            "diagnostic_counts",
            ReportField::Counts(&self.diagnostic_counts),
        );
        fields.insert("units", units);
        if let Some(value) = &self.source_contracts {
            fields.insert("source_contracts", ReportField::Json(value));
        }
        if let Some(value) = &self.evidence {
            fields.insert("evidence", ReportField::Json(value));
        }
        fields
    }

    /// Card W45-AUDIT-CHECKPOINTS: a report with no rows, the given
    /// per-category tallies and evidence-mode record: the document a composed
    /// report's rows are streamed into.
    pub(crate) fn skeleton(
        revision: &str,
        diagnostic_counts: BTreeMap<String, usize>,
        evidence: Option<Value>,
    ) -> Self {
        Self {
            revision: revision.into(),
            units: Vec::new(),
            diagnostic_counts,
            source_contracts: None,
            evidence,
            plugin_links: None,
            bounds: Vec::new(),
            bounds_evidence: Vec::new(),
            null: Vec::new(),
            null_evidence: Vec::new(),
            loop_measures: Vec::new(),
            callback_members: Vec::new(),
            member_prepass: None,
            contract_bundles: Ok(BTreeMap::new()),
        }
    }

    /// Card W45-AUDIT-CHECKPOINTS: the per-category tallies recorded so far.
    pub(crate) fn diagnostic_count_map(&self) -> &BTreeMap<String, usize> {
        &self.diagnostic_counts
    }

    /// Card W45-AUDIT-CHECKPOINTS: the evidence-mode record, when there is one.
    pub(crate) fn evidence_record(&self) -> Option<&Value> {
        self.evidence.as_ref()
    }
}

/// The per-unit account of the original-coordinate derivation.
///
/// `null` means the derivation was not run for this unit, which happens exactly
/// when the unit publishes no diagnostic to attribute. A refusal is published
/// with its ground rather than left silent, and it is always an
/// `environment-unavailable` fact about this analysis machinery — never a
/// statement about the analysed source (§L21.1 rule 1).
fn origin_coordinates(locations: Option<&SourceLineMap>) -> Value {
    let Some(map) = locations else {
        return Value::Null;
    };
    if map.is_established() {
        return json!({
            "status": "established",
            "withdrawn_lines": map.relabelled_lines(),
        });
    }
    let refusals = map
        .refusals()
        .iter()
        .map(|refusal| {
            json!({
                "ground": refusal.as_str(),
                "reason": refusal.reason(),
                "reason_code": reason_code_value(Some(refusal.category())),
            })
        })
        .collect::<Vec<_>>();
    json!({ "status": "refused", "refusals": refusals })
}

/// Card **GATEWAY-C-LOCATION**: every site this text names is the original
/// pre-preprocessing coordinate, supplied by `site`. The internal preprocessed
/// position appears only through `site`'s explicit fallback.
fn diagnostic_description(
    diagnostic: &LegacyDiagnostic,
    site: &impl Fn(usize) -> String,
) -> (&'static str, String) {
    match &diagnostic.kind {
        LegacyDiagnosticKind::Unsupported { description } => ("unsupported", description.clone()),
        LegacyDiagnosticKind::UnknownAlias { variable } => (
            "unknown_alias",
            format!("unresolved resource identity `{variable}`"),
        ),
        LegacyDiagnosticKind::Leak { acquisition_line } => (
            "leak",
            format!("live obligation acquired at {}", site(*acquisition_line)),
        ),
        LegacyDiagnosticKind::DoubleRelease { acquisition_line } => (
            "double_release",
            format!(
                "repeated release of acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::ReleaseAfterEscape { acquisition_line } => (
            "release_after_escape",
            format!(
                "release after transfer of acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::CustodyConflict { acquisition_line } => (
            "custody_conflict",
            format!(
                "conflicting custody of acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        // Card W43-LIBC-RESULT-ALIAS: the two derived-pointer violation
        // candidates. Provisional, like every other defect category here.
        LegacyDiagnosticKind::InvalidRelease { acquisition_line } => (
            "invalid_release",
            format!(
                "release of a pointer into, not the identity of, the acquisition at {}",
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::UseAfterRelease { acquisition_line } => (
            "use_after_release",
            format!(
                "use of a pointer derived from the acquisition at {} after its release",
                site(*acquisition_line)
            ),
        ),
        // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the kind candidates, in the
        // same two categories, with their own wording.
        LegacyDiagnosticKind::KindMismatchedRelease {
            acquisition_line,
            acquired,
            released,
        } => (
            "invalid_release",
            format!(
                "{} release of the {} acquired at {}",
                released.spelling(),
                acquired.spelling(),
                site(*acquisition_line)
            ),
        ),
        LegacyDiagnosticKind::UseAfterClose {
            acquisition_line,
            kind,
        } => (
            "use_after_release",
            format!(
                "use of the {} acquired at {} after its release",
                kind.spelling(),
                site(*acquisition_line)
            ),
        ),
        // Card W45-FB-RETRY-LIFETIME: a refuted counted-user obligation.
        // Provisional, like every other defect category here.
        LegacyDiagnosticKind::CountedUser { description } => ("counted_user", description.clone()),
        LegacyDiagnosticKind::SummaryConflict { inferred, declared } => (
            "summary_conflict",
            format!(
                "inferred custody summary {inferred:?} contradicts the declared contract \
                 {declared:?}; the declared contract governs every call site"
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    /// Card ARM-OBLIGATIONS: a carried premise is published in the row and
    /// folded into its verdict contract's assumptions; a row carrying none
    /// keeps both its bytes and its assumptions digest.
    #[test]
    fn a_carried_premise_is_among_the_rows_assumptions() {
        use qcore_c_gateway::arm_obligations::{Carriage, Carried};
        assert_eq!(super::rule_obligations_field(&Carried::default()), None);
        let mut row = serde_json::json!({"function": "f", "status": "Safe"});
        assert_eq!(super::assumptions_with_premises("base", &row), "base");
        let carried = Carried {
            premises: [Carriage {
                rule: "37".into(),
                name: "asserted_allocation".into(),
            }]
            .into(),
            open: Default::default(),
        };
        let field = super::rule_obligations_field(&carried).expect("carried");
        assert_eq!(
            field,
            serde_json::json!({"premises": [{"rule": "37", "name": "asserted_allocation"}],
                "open": []})
        );
        row[super::RULE_OBLIGATIONS_KEY] = field;
        let assumptions = super::assumptions_with_premises("base", &row);
        assert_ne!(assumptions, "base");
        assert_eq!(
            assumptions,
            qcore_schemas::sha256_hex(
                serde_json::json!({"sources": "base", "rule_premises":
                    [{"rule": "37", "name": "asserted_allocation"}]})
                .to_string()
                .as_bytes()
            )
        );
    }

    #[test]
    fn streamed_report_matches_owned_encoding_with_optional_fields_and_writer_failures() {
        let mut report = super::AuditReport::new("revision");
        report.units.push(
            serde_json::json!({"file":"a.c", "functions":[{"function":"f", "status":"Unknown"}]}),
        );
        report.diagnostic_counts.insert("parser".into(), 3);
        for optional in [false, true] {
            if optional {
                report.evidence =
                    Some(serde_json::json!({"mode":"contracts-only", "modules":["x"]}));
                report.source_contracts = Some(serde_json::json!({"status":"Unknown"}));
            }
            for key in ["source_revision", "historical_revision"] {
                let mut expected = serde_json::to_vec_pretty(&report.document(key)).unwrap();
                expected.push(b'\n');
                let mut actual = Vec::new();
                report.write_document(&mut actual, key).unwrap();
                assert_eq!(actual, expected);
            }
        }
        struct Reject;
        impl std::io::Write for Reject {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("write refused"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(
            report
                .write_document(&mut Reject, "source_revision")
                .unwrap_err()
                .contains("write refused")
        );
        struct RejectFlush(Vec<u8>);
        impl std::io::Write for RejectFlush {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("flush refused"))
            }
        }
        assert!(
            report
                .write_document(&mut RejectFlush(Vec::new()), "source_revision")
                .unwrap_err()
                .contains("flush refused")
        );
    }

    use super::*;
    use qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit;
    use qcore_contract::contract_loader::load_resource_contract_sources;
    use std::collections::BTreeMap;

    /// One acquisition with no release, and one uncontracted call: a leak row
    /// whose site is an acquisition line, and an unsupported row whose site is
    /// its own line.
    const TWO_FUNCTIONS: &str = "void a(void) { void *p = alloc(); } void b(void) { opaque(); }";

    fn two_function_result() -> Result<UnrestrictedLegacyResult, LegacyCError> {
        // W1 literal-site migration: loaded from a `.qcore` fixture module.
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "alloc_acquire.qcore".to_owned(),
            include_str!("../tests/fixtures/alloc_acquire.qv").to_owned(),
        )]))
        .unwrap();
        Ok(UnrestrictedLegacyResult::Unsafe(
            analyze_legacy_c_translation_unit(TWO_FUNCTIONS, "a", &contracts).unwrap(),
        ))
    }

    #[test]
    fn inventory_keeps_unknown_diagnostics_inside_unsafe_units_and_unprojected_names() {
        // W1 literal-site migration: loaded from a `.qcore` fixture module.
        let contracts = load_resource_contract_sources(&BTreeMap::from([(
            "alloc_acquire.qcore".to_owned(),
            include_str!("../tests/fixtures/alloc_acquire.qv").to_owned(),
        )]))
        .unwrap();
        let summary = analyze_legacy_c_translation_unit(
            "void a(void) { void *p = alloc(); } void b(void) { opaque(); }",
            "a",
            &contracts,
        )
        .unwrap();
        let result = Ok(UnrestrictedLegacyResult::Unsafe(summary));
        let mut report = AuditReport::new("revision");
        report.record(
            "test",
            "unit.c",
            &["a".into(), "b".into(), "excluded".into()],
            &result,
            None,
        );
        let unit = &report.units[0];
        assert_eq!(unit["status"], "Unsafe");
        assert_eq!(unit["not_projected"], json!(["excluded"]));
        assert_eq!(unit["functions"][1]["status"], "Unknown");
        assert_eq!(
            unit["functions"][1]["diagnostics"][0]["description"],
            "call to uncontracted function `opaque`"
        );
        assert_eq!(report.diagnostic_counts["unsupported"], 1);
        assert_eq!(report.diagnostic_counts["leak"], 1);
    }

    // -----------------------------------------------------------------------
    // Card GATEWAY-C-LOCATION: the presented coordinate, and the internal
    // position it never replaces.
    // -----------------------------------------------------------------------

    /// A unit with no established coordinate keeps the internal position, and
    /// says it is internal. `origin` is `null`, which asserts nothing.
    #[test]
    fn a_unit_without_coordinates_names_its_internal_position_as_internal() {
        let mut report = AuditReport::new("revision");
        report.record(
            "test",
            "unit.c",
            &["a".into()],
            &two_function_result(),
            None,
        );
        let unit = &report.units[0];
        let unsupported = &unit["functions"][1]["diagnostics"][0];
        assert_eq!(unsupported["category"], "unsupported");
        assert_eq!(unsupported["line"], 1);
        assert_eq!(unsupported["origin"], Value::Null);
        let leak = &unit["functions"][0]["diagnostics"][0];
        assert_eq!(leak["category"], "leak");
        assert_eq!(
            leak["description"],
            "live obligation acquired at preprocessed line 1"
        );
        assert_eq!(unit["origin_coordinates"], Value::Null);
    }

    /// With a map in force the same rows present the original `file:line` in
    /// both the `origin` key and the diagnostic text, and **still** carry the
    /// unchanged preprocessed `line`.
    #[test]
    fn a_presented_coordinate_replaces_the_text_and_not_the_leaf_identity() {
        // A one-line unit that is line 42 of the file it was written in.
        let marked = format!("# 42 \"<stdin>\"\n{TWO_FUNCTIONS}\n");
        let map = qcore_c_gateway::legacy_c::map_marked_preprocessing(
            &marked,
            &format!("{TWO_FUNCTIONS}\n"),
        );
        assert!(map.is_established(), "{:?}", map.refusals());
        let mut report = AuditReport::new("revision");
        let plain = {
            let mut plain = AuditReport::new("revision");
            plain.record(
                "test",
                "src/unit.c",
                &["a".into()],
                &two_function_result(),
                None,
            );
            plain
        };
        report.record(
            "test",
            "src/unit.c",
            &["a".into()],
            &two_function_result(),
            Some(&map),
        );
        let unit = &report.units[0];
        let unsupported = &unit["functions"][1]["diagnostics"][0];
        assert_eq!(
            unsupported["line"], plain.units[0]["functions"][1]["diagnostics"][0]["line"],
            "the internal leaf identity is untouched by the presentation"
        );
        assert_eq!(unsupported["origin"], "src/unit.c:42");
        let leak = &unit["functions"][0]["diagnostics"][0];
        assert_eq!(
            leak["description"], "live obligation acquired at src/unit.c:42",
            "presented text names the original coordinate, never the preprocessed one"
        );
        assert_eq!(
            unit["origin_coordinates"],
            json!({"status": "established", "withdrawn_lines": 0})
        );
        // Only the presentation moved: category, status and counts are equal.
        assert_eq!(unit["status"], plain.units[0]["status"]);
        assert_eq!(
            unit["diagnostic_counts"],
            plain.units[0]["diagnostic_counts"]
        );
        assert_eq!(unsupported["category"], "unsupported");
    }

    #[test]
    fn a_refused_derivation_is_published_as_an_environment_ground() {
        let map = qcore_c_gateway::legacy_c::map_marked_preprocessing("no marker here\n", "x\n");
        assert!(!map.is_established());
        let value = origin_coordinates(Some(&map));
        assert_eq!(value["status"], "refused");
        assert_eq!(
            value["refusals"][0]["reason_code"],
            "environment-unavailable"
        );
        // L21.1 rule 1: a ground about this machinery can never support Unsafe.
        assert!(!ReasonCategory::EnvironmentUnavailable.may_support_unsafe());
    }

    #[test]
    fn inventory_retains_failure_reason_without_a_partial_summary() {
        let mut report = AuditReport::new("revision");
        report.record(
            "test",
            "unit.c",
            &["missing".into()],
            &Ok(UnrestrictedLegacyResult::Unknown {
                preprocessed_source: String::new(),
                reason: "projection failure".into(),
                partial: None,
            }),
            None,
        );
        assert_eq!(report.units[0]["reason"], "projection failure");
        assert_eq!(report.units[0]["functions"], json!([]));
        assert_eq!(report.units[0]["not_projected"], json!(["missing"]));
    }

    // -----------------------------------------------------------------------
    // Card REASON-CODES: the audit report's per-unit reason category.
    // -----------------------------------------------------------------------

    #[test]
    fn opaque_os_text_names_its_category_at_the_site_without_changing_its_text() {
        // Card REASON-CODES-B1. `std::io::Error::to_string()` carries no marker
        // the closed vocabulary recognises, so classifying from prose yields
        // null. The three reachable OS/tool sites on the resource-projection
        // route now name their category instead, and the text is byte-identical.
        let text = "No such file or directory (os error 2)";
        assert_eq!(reason_code(Some(text)), Value::Null);
        let refusal = UnitRefusal::environment(text);
        assert_eq!(refusal.reason(), text);
        assert_eq!(
            refusal.category(),
            Some(ReasonCategory::EnvironmentUnavailable)
        );
        let mut report = AuditReport::new("revision");
        report.record_unknown("test", "unit.c", &[], &refusal);
        let unit = &report.units[0];
        assert_eq!(unit["reason"], text);
        assert_eq!(unit["reason_code"], "environment-unavailable");
        // The named category is the one subject that can never be read as a
        // statement about the analysed source (L21.1 rule 1).
        assert!(!ReasonCategory::EnvironmentUnavailable.describes_analysed_source());
    }

    #[test]
    fn a_classified_refusal_still_fails_closed_on_text_nobody_recognises() {
        // `classified` is the ordinary constructor and must not acquire a
        // category from nowhere: there is no fallback, and in particular no
        // fallback onto a source-subject category.
        let refusal = UnitRefusal::classified("something nobody has ever emitted");
        assert_eq!(refusal.category(), None);
        let mut report = AuditReport::new("revision");
        report.record_unknown("test", "unit.c", &[], &refusal);
        assert_eq!(report.units[0]["reason_code"], Value::Null);
    }

    #[test]
    fn a_producer_supplied_category_is_published_without_being_reclassified() {
        // Card REASON-CODES-B2/B3: rows whose producer already classified their
        // reason (ChunkSourceResult::Unknown) publish that category rather than
        // re-deriving it here, so the two cannot disagree. The fail-closed shape
        // is unchanged: an unclassified row is null, never a defaulted category.
        assert_eq!(
            reason_code_value(Some(ReasonCategory::UnsupportedSyntax)),
            json!("unsupported-syntax")
        );
        assert_eq!(
            reason_code_value(Some(ReasonCategory::EnvironmentUnavailable)),
            json!("environment-unavailable")
        );
        assert_eq!(reason_code_value(None), Value::Null);
        // The two routes agree on a reason both can see.
        let reason = "outside single borrowed chunk / int-or-void-result source profile";
        assert_eq!(
            reason_code(Some(reason)),
            reason_code_value(ReasonCategory::classify(reason).ok())
        );
    }

    #[test]
    fn a_unit_without_a_reason_asserts_no_category() {
        assert_eq!(reason_code(None), Value::Null);
        let mut report = AuditReport::new("revision");
        report.record_data_only("test", "data.c", None);
        assert_eq!(report.units[0]["reason"], Value::Null);
        assert_eq!(report.units[0]["reason_code"], Value::Null);
    }

    #[test]
    fn an_unrecognised_reason_asserts_no_category_rather_than_a_source_defect() {
        // Fail-closed: `null` makes no claim. It is never a category, and in
        // particular never one whose subject is the analysed source.
        assert_eq!(
            reason_code(Some("No such file or directory (os error 2)")),
            Value::Null
        );
        let mut report = AuditReport::new("revision");
        report.record_unknown(
            "test",
            "unit.c",
            &[],
            &UnitRefusal::classified("No such file or directory (os error 2)"),
        );
        assert_eq!(
            report.units[0]["reason"],
            "No such file or directory (os error 2)"
        );
        assert_eq!(report.units[0]["reason_code"], Value::Null);
    }

    #[test]
    fn a_missing_clang_is_recorded_as_an_environment_failure_not_a_source_defect() {
        let reason = "audit requires the configured Clang executable (/usr/bin/clang); \
                      compilation database selects /usr/bin/cc. Prepare an explicit \
                      Clang build with quirin-audit prepare and run quirin-audit \
                      current over it (prepare --run chains both); GCC compiler \
                      equivalence is unproven";
        let mut report = AuditReport::new("revision");
        report.record_unknown(
            "test",
            "unit.c",
            &["f".into()],
            &UnitRefusal::classified(reason),
        );
        let unit = &report.units[0];
        assert_eq!(unit["status"], "Unknown");
        assert_eq!(unit["reason_code"], "environment-unavailable");
        let category = ReasonCategory::classify(reason).unwrap();
        assert!(!category.describes_analysed_source());
        assert!(!category.may_support_unsafe());
    }

    #[test]
    fn the_projection_summary_separates_unmodeled_constructs_from_open_custody() {
        let mut report = AuditReport::new("revision");
        for (reason, expected) in [
            (
                "resource projection unresolved: 2 unsupported operation(s), \
                 0 unresolved alias(es), 0 custody summary conflict(s)",
                "unsupported-syntax",
            ),
            (
                "resource projection unresolved: 0 unsupported operation(s), \
                 1 unresolved alias(es), 0 custody summary conflict(s)",
                "residual-custody",
            ),
        ] {
            report.record(
                "test",
                "unit.c",
                &[],
                &Ok(UnrestrictedLegacyResult::Unknown {
                    preprocessed_source: String::new(),
                    reason: reason.into(),
                    partial: None,
                }),
                None,
            );
            let unit = report.units.last().unwrap();
            assert_eq!(unit["reason"], reason, "the reason text is untouched");
            assert_eq!(unit["reason_code"], expected);
        }
    }

    #[test]
    fn the_category_refines_the_unit_and_replaces_nothing() {
        // L21.1 rule 2: every pre-existing key keeps its exact value.
        let reason = "resource projection unresolved: 1 unsupported operation(s), \
                      0 unresolved alias(es), 0 custody summary conflict(s)";
        let mut report = AuditReport::new("revision");
        report.record_unknown(
            "lib",
            "unit.c",
            &["a".into(), "b".into()],
            &UnitRefusal::classified(reason),
        );
        let unit = &report.units[0];
        assert_eq!(unit["library"], "lib");
        assert_eq!(unit["file"], "unit.c");
        assert_eq!(unit["status"], "Unknown");
        assert_eq!(unit["reason"], reason);
        assert_eq!(unit["source_functions"], json!(["a", "b"]));
        assert_eq!(unit["not_projected"], json!(["a", "b"]));
        assert_eq!(unit["functions"], json!([]));
        assert_eq!(unit["diagnostic_counts"], json!({}));
        assert_eq!(unit["reason_code"], "unsupported-syntax");
        // Exactly one key was added to the eight this unit shape already had.
        let keys: Vec<_> = unit.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            vec![
                "diagnostic_counts",
                "file",
                "functions",
                "library",
                "not_projected",
                "reason",
                "reason_code",
                "source_functions",
                "status",
            ]
        );
    }

    /// Card W44-AUDIT-PARALLEL-UNITS: units recorded into fragments that are
    /// computed on several threads and completed in reverse order, then
    /// absorbed in unit order, write exactly the bytes of one report recorded
    /// sequentially.
    #[test]
    fn fragments_absorbed_in_unit_order_write_the_sequential_report() {
        fn record_unit(report: &mut AuditReport, index: usize) {
            let file = format!("unit{index}.c");
            match index % 3 {
                0 => {
                    report.record(
                        "test",
                        &file,
                        &["a".into(), "b".into()],
                        &two_function_result(),
                        None,
                    );
                }
                1 => report.record_data_only("test", &file, None),
                _ => report.record_unknown(
                    "test",
                    &file,
                    &["c".into()],
                    &UnitRefusal::environment(format!("unit {index} unreadable")),
                ),
            }
            report.record_configuration(json!({ "unit": index }));
        }
        let directory = std::env::temp_dir().join(format!(
            "qcore-report-fragments-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let mut sequential = AuditReport::new("revision");
        for index in 0..12 {
            record_unit(&mut sequential, index);
        }
        sequential
            .write(&directory.join("sequential.json"))
            .unwrap();
        for jobs in [1, 4, 12] {
            let mut folded = AuditReport::new("revision");
            let template = AuditReport::new("revision");
            crate::parallel::run_ordered(
                (0..12).collect::<Vec<usize>>(),
                jobs,
                Some((0..12).rev().collect()),
                |index| {
                    std::thread::sleep(std::time::Duration::from_millis((12 - index as u64) * 3));
                    let mut fragment = template.fragment();
                    record_unit(&mut fragment, index);
                    fragment
                },
                |fragment| folded.absorb(fragment),
            )
            .unwrap();
            let path = directory.join(format!("folded-{jobs}.json"));
            folded.write(&path).unwrap();
            assert_eq!(
                fs::read(&path).unwrap(),
                fs::read(directory.join("sequential.json")).unwrap(),
                "jobs = {jobs}"
            );
        }
        fs::remove_dir_all(&directory).ok();
    }

    #[test]
    fn a_fragment_of_another_revision_is_refused() {
        let mut report = AuditReport::new("revision");
        let mut other = AuditReport::new("other");
        other.record_data_only("test", "unit.c", None);
        assert!(report.absorb(other).is_err());
        assert!(report.units.is_empty());
        let mut evidence = report.fragment();
        evidence.set_source_contracts(json!({}));
        assert!(report.absorb(evidence).is_err());
    }

    #[test]
    fn the_envelope_explains_the_vocabulary_it_now_carries() {
        let directory = std::env::temp_dir().join(format!(
            "qcore-reason-envelope-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("report.json");
        AuditReport::new("revision").write(&path).unwrap();
        let written: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let meaning = written["reason_code_meaning"].as_str().unwrap();
        assert!(meaning.contains("L21.1"));
        assert!(meaning.contains("environment-unavailable"));
        assert!(meaning.contains("null asserts no category"));
        fs::remove_dir_all(&directory).ok();
    }
}

/// Private phase retention: complete rows and member evidence on disk, small
/// derived phase-selection tables in memory. Restoration never runs analysis.
pub(crate) struct ParkedReport {
    shell: AuditReport,
    payload: crate::retained_products::Spool,
    pub(crate) refutations: qcore_schemas::contract_refutation::RefutationTable,
    pub(crate) bounds_refutations: std::collections::BTreeSet<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RetainedRows {
    units: Vec<Value>,
    bounds: Vec<Value>,
    bounds_evidence: Vec<Value>,
    null: Vec<Value>,
    null_evidence: Vec<Value>,
    callback_members: Vec<crate::callback_member_fold::UnitFacts>,
}

impl AuditReport {
    pub(crate) fn park(mut self) -> Result<ParkedReport, String> {
        let refutations = crate::refuted_contracts::table_from_rows(&self.units);
        let bounds_refutations = crate::bounds_evidence::refutation_table(&self.bounds_evidence);
        let rows = RetainedRows {
            units: std::mem::take(&mut self.units),
            bounds: std::mem::take(&mut self.bounds),
            bounds_evidence: std::mem::take(&mut self.bounds_evidence),
            null: std::mem::take(&mut self.null),
            null_evidence: std::mem::take(&mut self.null_evidence),
            callback_members: std::mem::take(&mut self.callback_members),
        };
        let payload = crate::retained_products::Spool::write(&rows)?;
        Ok(ParkedReport {
            shell: self,
            payload,
            refutations,
            bounds_refutations,
        })
    }
}

impl ParkedReport {
    pub(crate) fn restore(mut self) -> Result<AuditReport, String> {
        let rows: RetainedRows = self.payload.read()?;
        self.shell.units = rows.units;
        self.shell.bounds = rows.bounds;
        self.shell.bounds_evidence = rows.bounds_evidence;
        self.shell.null = rows.null;
        self.shell.null_evidence = rows.null_evidence;
        self.shell.callback_members = rows.callback_members;
        Ok(self.shell)
    }
}

#[cfg(test)]
mod retained_tests {
    use super::*;

    #[test]
    fn parked_report_preserves_exact_document_and_every_sidecar() {
        let mut report = AuditReport::new("revision");
        report.units = vec![
            serde_json::json!({
                "file": "a.c", "status": "Unknown", "functions": [{
                    "function": "f", "diagnostics": [{"line": 7, "description": "unmodeled"}],
                    "contract_refutations": [{"kind": "custody_duty", "clause": "release", "reason": "body", "origin": "a.c:7"}]
                }]
            }),
            serde_json::json!({"file": "b.c", "status": "Safe", "functions": []}),
        ];
        report.bounds_evidence =
            vec![serde_json::json!({"declarations": [{"function": "f", "refutes": true}]})];
        report.bounds = vec![serde_json::json!({"bound": 12})];
        report.null = vec![serde_json::json!({"null": "unknown"})];
        report.null_evidence = vec![serde_json::json!({"store": "a->b"})];
        report.record_callback_members("library", "a.c", None, None);
        report.diagnostic_counts.insert("unsupported".into(), 1);
        let expected_rows = serde_json::to_value(RetainedRows {
            units: report.units.clone(),
            bounds: report.bounds.clone(),
            bounds_evidence: report.bounds_evidence.clone(),
            null: report.null.clone(),
            null_evidence: report.null_evidence.clone(),
            callback_members: report.callback_members.clone(),
        })
        .unwrap();
        let mut before = Vec::new();
        report
            .write_document(&mut before, "source_revision")
            .unwrap();
        let parked = report.park().unwrap();
        assert!(parked.shell.units.is_empty());
        assert!(parked.shell.callback_members.is_empty());
        assert!(parked.refutations.contains_key("f"));
        assert!(parked.bounds_refutations.contains("f"));
        let restored = parked.restore().unwrap();
        let mut after = Vec::new();
        restored
            .write_document(&mut after, "source_revision")
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(
            serde_json::to_value(RetainedRows {
                units: restored.units,
                bounds: restored.bounds,
                bounds_evidence: restored.bounds_evidence,
                null: restored.null,
                null_evidence: restored.null_evidence,
                callback_members: restored.callback_members,
            })
            .unwrap(),
            expected_rows
        );
    }
}

impl AuditReport {
    /// Called only by the consuming final-run owner after callback and lifetime
    /// folds. Checkpoint APIs and the repeatable public fold retain their facts.
    pub(crate) fn release_final_fold_inputs(&mut self) {
        self.callback_members = Vec::new();
        self.member_prepass = None;
    }
}

#[cfg(test)]
mod retained_member_tests {
    use super::*;
    use crate::callback_member_fold::{FoldEnvironment, fold_in};
    use qcore_c_gateway::callback_member_closure::{closure, read_unit};
    use qcore_c_gateway::legacy_c::C11TranslationUnit;

    #[test]
    fn parked_real_member_evidence_preserves_closures_and_run_demotions() {
        let mut report = AuditReport::new("revision");
        for name in ["first", "second"] {
            let source = format!(
                "typedef void (*Callback)(void); enum Choice {{ Set = 1 }}; \
                 struct holder {{ Callback cb; }}; void {name}(void) {{}} \
                 struct holder value = {{ {name} }};"
            );
            let unit = C11TranslationUnit {
                source: &source,
                ..Default::default()
            };
            let facts =
                read_unit(&source, &unit).expect("Clang is required for source evidence controls");
            assert_eq!(
                closure(&[&facts], "holder.cb"),
                Ok(std::collections::BTreeSet::from([name.to_owned()]))
            );
            report.units.push(json!({"library":"l", "file":name, "status":"Safe",
                "reason":null, "diagnostic_counts":{}, "functions":[{"function":"f",
                "status":"Safe", "diagnostics":[], "contract_dependencies":{"declarations":["invoke"]}}]}));
            report.record_callback_members("l", name, Some(facts), None);
        }
        let original_facts = report.callback_members.clone();
        let mut original_rows = report.units.clone();
        let evidence = serde_json::to_value(&original_facts).unwrap();
        let mut restored = report.park().unwrap().restore().unwrap();
        assert_eq!(
            serde_json::to_value(&restored.callback_members).unwrap(),
            evidence
        );
        let environment = FoldEnvironment {
            declarations: BTreeMap::from([(
                "invoke".into(),
                std::collections::BTreeSet::from(["holder.cb".into()]),
            )]),
            ..Default::default()
        };
        let mut before_counts = BTreeMap::new();
        let before = fold_in(
            &environment,
            &mut original_rows,
            &original_facts,
            &mut before_counts,
            None,
        );
        let after = fold_in(
            &environment,
            &mut restored.units,
            &restored.callback_members,
            &mut restored.diagnostic_counts,
            None,
        );
        assert_eq!(before["demoted"].as_array().unwrap().len(), 2);
        assert_eq!(before, after);
        assert_eq!(original_rows, restored.units);
        assert_eq!(before_counts, restored.diagnostic_counts);
    }
}
