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

//! Card **AUDIT-COMPOSITION**: per-row components beside a report, and the
//! report's unit rows as a deterministic view derived from them.
//!
//! The report writer ([`crate::report::AuditReport::write_with_revision_key`])
//! turns every recorded unit row into one [`Component`] of the integration
//! semigroup (`qcore_schemas::composition`), writes the components to a
//! sidecar beside the report (`<report>.components.json`), and writes the
//! report's `units` as [`render`]ed from that sidecar. Nothing here reads or
//! changes a verdict, a route or an analysis: a row's component carries the
//! row's own status as its verdict, and a row with exactly one component and
//! no conflict renders as that component's `detail`, which is the row.
//!
//! Fail-closed in both directions:
//!
//! * a row from which no component can be built (no unit identity, no status)
//!   becomes a component whose verdict is `Unknown` and whose detail says why,
//!   so the published row is `Unknown`, never the unchecked row;
//! * a presented row with no component renders as `Unknown`, and a row whose
//!   components conflict renders at least `Unknown` with the
//!   `composition-conflict` reason.
//!
//! [`check`] recomputes every published row from a sidecar and compares.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::composition::{
    COMPOSITION_SCHEMA_VERSION, Component, ComponentSidecar, Contribution, Provenance, ReasonLeaf,
    RowKey, RuleSet, UnitRef, VerdictTuple,
};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::reason::ReasonCategory;
use qcore_schemas::verdict::Verdict;
use serde_json::{Map, Value, json};

use crate::checkpoint::unit_verdict;

/// How a unit row is rendered from its derivation.
pub const VIEW: &str = "qcore-audit.report-units.v1";

/// The property a unit row decides (as `projection::hypothesis_context`
/// names it).
pub const PROPERTY: &str = "resource-custody.v1";

/// The scope, bounds and evidence version of a unit row, as the checkpointed
/// route's run identity spells them (`checkpoint_run::run_identity`).
const SCOPE: &str = "selected-function-inventory; intraprocedural custody with validated summaries";
const BOUNDS: &str = "existing gateway budgets; no new hypothesis discharge";
const EVIDENCE_VERSION: &str = "qcore-audit-report.v1";

/// The gateway's built-in route table, generated from its rule registry
/// (`crates/gateway-rules/registry.toml`, card GATEWAY-RULE-API). The name is
/// unchanged from before the registry, so a run with no rule off records the
/// same provenance as before it.
const RULE_REGISTRY: &str = "qcore-c-gateway:built-in-route-map";

/// The bundle name of the loaded resource-contract environment.
pub const RESOURCE_CONTRACTS_BUNDLE: &str = "resource-contracts";

/// The sidecar path beside a report.
#[must_use]
pub fn sidecar_path(report: &std::path::Path) -> std::path::PathBuf {
    report.with_extension("components.json")
}

/// This analyzer's revision: package, version and the build commit when the
/// build recorded one.
#[must_use]
pub fn analyzer_revision() -> String {
    format!(
        "{}@{}:{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        option_env!("QCORE_AUDIT_BUILD_COMMIT").unwrap_or("unrecorded-build")
    )
}

/// The digest of a loaded resource-contract environment, as
/// `certificate_context` takes it.
///
/// # Errors
///
/// When the environment cannot be serialized.
pub fn contracts_digest(contracts: &ResourceContracts) -> Result<String, String> {
    Ok(qcore_schemas::sha256_hex(
        &serde_json::to_vec(contracts).map_err(|error| error.to_string())?,
    ))
}

/// The provenance of this run's components.
#[must_use]
pub fn provenance(contract_bundles: BTreeMap<String, String>) -> Provenance {
    Provenance {
        analyzer_revision: analyzer_revision(),
        rule_set: rule_set(qcore_c_gateway::gateway_rules::effective_off()),
        contract_bundles,
        external_checker: None,
    }
}

/// The rule set of a run that switched `off` off (`QCORE_GATEWAY_RULES_OFF`,
/// as the gateway applied it: `quirin-audit measure --rules-off` checks every
/// component against the list it asked for).
#[must_use]
pub fn rule_set(off: BTreeSet<String>) -> RuleSet {
    RuleSet {
        registry: RULE_REGISTRY.into(),
        off,
    }
}

/// The component store of a report's unit rows, in report order.
///
/// `provenance` is `Err` when the run's provenance could not be established;
/// then no component can be built and every row is the `Unknown` refusal.
#[must_use]
pub fn sidecar(rows: &[Value], provenance: &Result<Provenance, String>) -> ComponentSidecar {
    let mut occurrences = BTreeMap::<UnitRef, usize>::new();
    let mut order = Vec::with_capacity(rows.len());
    let mut contribution = Contribution::new();
    for row in rows {
        let unit = unit_of(row);
        let ordinal = occurrences.entry(unit.clone()).or_default();
        let key = RowKey {
            unit: unit.clone(),
            row: format!("unit#{ordinal}"),
            property: PROPERTY.into(),
            profile: unit.library.clone(),
        };
        *ordinal += 1;
        contribution.insert(component(row, &key, provenance));
        order.push(key);
    }
    ComponentSidecar {
        schema_version: COMPOSITION_SCHEMA_VERSION,
        view: VIEW.into(),
        order,
        contribution,
    }
}

/// The unit a row names; empty spellings when it names none, which
/// [`component`] then refuses.
fn unit_of(row: &Value) -> UnitRef {
    let text = |key: &str| {
        row.get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    UnitRef {
        library: text("library"),
        file: text("file"),
    }
}

/// One row's component, or the `Unknown` refusal naming why none could be
/// built.
fn component(row: &Value, key: &RowKey, provenance: &Result<Provenance, String>) -> Component {
    let provenance_value = match provenance {
        Ok(provenance) => provenance.clone(),
        Err(error) => {
            return refusal(
                row,
                key,
                self::provenance(BTreeMap::new()),
                &format!("the run's provenance is unavailable: {error}"),
            );
        }
    };
    let Some(object) = row.as_object() else {
        return refusal(row, key, provenance_value, "the row is not an object");
    };
    if !matches!(object.get("library"), Some(Value::String(_)))
        || !matches!(object.get("file"), Some(Value::String(_)))
    {
        return refusal(row, key, provenance_value, "the row names no unit");
    }
    let Some(status) = object.get("status").and_then(Value::as_str) else {
        return refusal(row, key, provenance_value, "the row carries no status");
    };
    let reasons = match object.get("reason") {
        Some(Value::String(text)) => vec![ReasonLeaf {
            category: object
                .get("reason_code")
                .and_then(Value::as_str)
                .and_then(|code| code.parse::<ReasonCategory>().ok()),
            text: text.clone(),
        }],
        _ => Vec::new(),
    };
    Component {
        unit: key.unit.clone(),
        row: key.row.clone(),
        property: key.property.clone(),
        profile: key.profile.clone(),
        provenance: provenance_value,
        verdict: VerdictTuple {
            verdict: unit_verdict(status),
            scope: SCOPE.into(),
            bounds: BOUNDS.into(),
            assumptions: assumptions(object),
            evidence_version: EVIDENCE_VERSION.into(),
        },
        reasons,
        detail: row.clone(),
    }
}

/// The text a refused row carries.
fn refusal_text(why: &str) -> String {
    format!("composition: no component could be built for this row: {why}")
}

/// The component of a row from which none could be built: `Unknown`, with the
/// row kept beside the refusal.
fn refusal(row: &Value, key: &RowKey, provenance: Provenance, why: &str) -> Component {
    let text = refusal_text(why);
    let mut detail = match row {
        Value::Object(object) => object.clone(),
        other => {
            let mut object = Map::new();
            object.insert("row".into(), other.clone());
            object
        }
    };
    detail.insert("status".into(), json!(Verdict::Unknown.as_str()));
    detail.insert("reason".into(), json!(text));
    detail.insert("reason_code".into(), Value::Null);
    Component {
        unit: key.unit.clone(),
        row: key.row.clone(),
        property: key.property.clone(),
        profile: key.profile.clone(),
        provenance,
        verdict: VerdictTuple {
            verdict: Verdict::Unknown,
            scope: SCOPE.into(),
            bounds: BOUNDS.into(),
            assumptions: BTreeSet::new(),
            evidence_version: EVIDENCE_VERSION.into(),
        },
        reasons: vec![ReasonLeaf {
            category: None,
            text,
        }],
        detail: Value::Object(detail),
    }
}

/// The named assumptions a unit row's verdict holds under, as the row records
/// them: the unit's assumed configuration and cross-unit link premise (by
/// digest), and per function the declared contracts and summaries its verdict
/// rests on (rule 13h) and its named null premises (rule 14n). This is a
/// projection of the row; the row itself stays the component's detail.
fn assumptions(row: &Map<String, Value>) -> BTreeSet<String> {
    let digest = |value: &Value| {
        qcore_schemas::sha256_hex(
            &serde_json::to_vec(value).expect("a report value is plain serializable data"),
        )
    };
    let mut assumptions = BTreeSet::new();
    if let Some(value) = row.get("assumptions") {
        assumptions.insert(format!("unit-assumptions:{}", digest(value)));
    }
    if let Some(value) = row.get("cross_unit_link_resolution") {
        assumptions.insert(format!("cross-unit-link-resolution:{}", digest(value)));
    }
    let names = |value: Option<&Value>| {
        value
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    for function in row
        .get("functions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let dependencies = function.get("contract_dependencies");
        for name in names(dependencies.and_then(|value| value.get("declarations"))) {
            assumptions.insert(format!("contract:{name}"));
        }
        for name in names(dependencies.and_then(|value| value.get("summaries"))) {
            assumptions.insert(format!("summary:{name}"));
        }
        for name in names(dependencies.and_then(|value| value.get("null_premises"))) {
            assumptions.insert(format!("null-premise:{name}"));
        }
        for premise in function
            .get("null_premises")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(name) = premise.get("name").and_then(Value::as_str) {
                assumptions.insert(format!("null-premise:{name}"));
            }
        }
    }
    assumptions
}

/// The report's unit rows, derived from a sidecar in its presentation order.
///
/// A row with one component and no conflict is that component's detail,
/// borrowed. Otherwise the row is built from the derivation: the detail of a
/// component carrying the joined verdict, its status replaced by the joined
/// verdict when they differ, and on a conflict the `composition-conflict`
/// reason. A presented row with no component is `Unknown`.
#[must_use]
pub fn render(sidecar: &ComponentSidecar) -> Vec<Cow<'_, Value>> {
    sidecar
        .order
        .iter()
        .map(|key| render_row(&sidecar.contribution, key))
        .collect()
}

fn render_row<'a>(contribution: &'a Contribution, key: &RowKey) -> Cow<'a, Value> {
    let Some(derivation) = contribution.derive(key) else {
        return Cow::Owned(json!({
            "library": key.unit.library, "file": key.unit.file,
            "status": Verdict::Unknown.as_str(),
            "reason": refusal_text("the row is presented without a component"),
            "reason_code": Value::Null,
        }));
    };
    if let ([only], false) = (derivation.components.as_slice(), derivation.is_conflicted()) {
        return Cow::Borrowed(&only.detail);
    }
    let base = derivation
        .components
        .iter()
        .find(|component| component.verdict.verdict == derivation.verdict)
        .unwrap_or(&derivation.components[0]);
    let mut row = match &base.detail {
        Value::Object(object) => object.clone(),
        other => {
            let mut object = Map::new();
            object.insert("row".into(), other.clone());
            object
        }
    };
    let label = row.get("status").and_then(Value::as_str).unwrap_or("");
    if unit_verdict(label) != derivation.verdict {
        row.insert("status".into(), json!(derivation.verdict.as_str()));
    }
    if let Some(conflict) = derivation
        .reasons
        .iter()
        .find(|leaf| leaf.category == Some(ReasonCategory::CompositionConflict))
    {
        row.insert("reason".into(), json!(conflict.text));
        row.insert(
            "reason_code".into(),
            json!(ReasonCategory::CompositionConflict.as_str()),
        );
    }
    Cow::Owned(Value::Object(row))
}

/// What [`check`] compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckSummary {
    pub rows: usize,
    pub components: usize,
    pub conflicts: usize,
    pub verdicts: BTreeMap<Verdict, usize>,
}

/// Recompute every published row of `report` from `sidecar` and compare:
/// the sidecar is valid, it presents exactly as many rows as the report has,
/// every rendered row equals the published row, and every row's joined
/// verdict is the verdict its published status denotes.
///
/// # Errors
///
/// Names the first disagreement.
pub fn check(report: &Value, sidecar: &ComponentSidecar) -> Result<CheckSummary, String> {
    sidecar.validate()?;
    if sidecar.view != VIEW {
        return Err(format!("sidecar view `{}` is not `{VIEW}`", sidecar.view));
    }
    let published = report
        .get("units")
        .and_then(Value::as_array)
        .ok_or("the report has no `units` array")?;
    if published.len() != sidecar.order.len() {
        return Err(format!(
            "the report has {} unit rows and the sidecar presents {}",
            published.len(),
            sidecar.order.len()
        ));
    }
    let mut verdicts = BTreeMap::new();
    for (index, ((key, rendered), row)) in sidecar
        .order
        .iter()
        .zip(render(sidecar))
        .zip(published)
        .enumerate()
    {
        if rendered.as_ref() != row {
            return Err(format!("unit row {index} ({key:?}) does not re-derive"));
        }
        let derivation = sidecar
            .contribution
            .derive(key)
            .ok_or_else(|| format!("unit row {index} ({key:?}) has no component"))?;
        let status = row.get("status").and_then(Value::as_str).unwrap_or("");
        if derivation.verdict != unit_verdict(status) {
            return Err(format!(
                "unit row {index} ({key:?}) joins to {} but publishes `{status}`",
                derivation.verdict
            ));
        }
        *verdicts.entry(derivation.verdict).or_default() += 1;
    }
    Ok(CheckSummary {
        rows: published.len(),
        components: sidecar.contribution.len(),
        conflicts: sidecar.contribution.conflicts().count(),
        verdicts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(library: &str, file: &str, status: &str) -> Value {
        json!({
            "library": library, "file": file, "status": status,
            "reason": if status == "Safe" { Value::Null } else { json!("resource projection unresolved: 1 unsupported operation(s), 0 unresolved alias(es), 0 custody summary conflict(s)") },
            "reason_code": if status == "Safe" { Value::Null } else { json!("unsupported-syntax") },
            "functions": [{"function": "f", "status": status, "diagnostics": [],
                "contract_dependencies": {"declarations": ["flb_free"], "summaries": ["mk_list_add"], "null_premises": ["null-deref:f:3:p"]},
                "null_premises": [{"name": "null-deref:f:3:p", "line": 3, "origin": null, "variable": "p"}]}],
            "cross_unit_link_resolution": {"premise": "linked"},
        })
    }

    fn bundles() -> Result<Provenance, String> {
        Ok(provenance(BTreeMap::from([(
            RESOURCE_CONTRACTS_BUNDLE.to_owned(),
            "ab".repeat(32),
        )])))
    }

    #[test]
    fn the_rendered_rows_are_the_recorded_rows() {
        let rows = vec![
            row("l", "a.c", "Safe"),
            row("l", "b.c", "Unknown"),
            row("l", "b.c", "Unknown"),
            row("m", "a.c", "Unsafe"),
            json!({"library": "l", "file": "d.c", "status": "data_only", "reason": null,
                   "reason_code": null, "functions": []}),
            json!({"library": "l", "file": "e.c", "status": "gateway_failure",
                   "reason": "x", "reason_code": null, "functions": []}),
        ];
        let sidecar = sidecar(&rows, &bundles());
        sidecar.validate().unwrap();
        let rendered = render(&sidecar);
        assert_eq!(
            rendered.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            rows.iter().collect::<Vec<_>>()
        );
        assert!(rendered.iter().all(|row| matches!(row, Cow::Borrowed(_))));
        // The same unit twice is two rows, not a conflict.
        assert_eq!(sidecar.order[1].row, "unit#0");
        assert_eq!(sidecar.order[2].row, "unit#1");
        assert_eq!(sidecar.contribution.conflicts().count(), 0);
        let report = json!({"units": rows});
        let summary = check(&report, &sidecar).unwrap();
        assert_eq!(summary.rows, 6);
        assert_eq!(summary.components, 6);
        assert_eq!(summary.verdicts[&Verdict::Safe], 2, "Safe and data_only");
        assert_eq!(summary.verdicts[&Verdict::Unknown], 3);
        // The sidecar round-trips through its wire form.
        let wire = serde_json::to_vec(&sidecar).unwrap();
        let read: ComponentSidecar = serde_json::from_slice(&wire).unwrap();
        assert_eq!(read, sidecar);
        assert_eq!(check(&report, &read).unwrap(), summary);
    }

    #[test]
    fn a_component_carries_the_rows_named_assumptions_and_reason() {
        let sidecar = sidecar(&[row("l", "b.c", "Unknown")], &bundles());
        let component = sidecar.contribution.components().next().unwrap();
        assert_eq!(component.verdict.verdict, Verdict::Unknown);
        assert_eq!(
            component.verdict.assumptions,
            BTreeSet::from([
                "contract:flb_free".to_owned(),
                format!(
                    "cross-unit-link-resolution:{}",
                    qcore_schemas::sha256_hex(br#"{"premise":"linked"}"#)
                ),
                "null-premise:null-deref:f:3:p".to_owned(),
                "summary:mk_list_add".to_owned(),
            ])
        );
        assert_eq!(
            component.reasons,
            vec![ReasonLeaf {
                category: Some(ReasonCategory::UnsupportedSyntax),
                text: "resource projection unresolved: 1 unsupported operation(s), 0 unresolved \
                       alias(es), 0 custody summary conflict(s)"
                    .into()
            }]
        );
        assert_eq!(component.provenance.rule_set.off, BTreeSet::new());
        let switched = super::rule_set(BTreeSet::from(["35".to_owned()]));
        assert_eq!(switched.registry, component.provenance.rule_set.registry);
        assert_eq!(switched.off, BTreeSet::from(["35".to_owned()]));
        assert!(
            component
                .provenance
                .analyzer_revision
                .starts_with("qcore-audit@")
        );
    }

    #[test]
    fn a_row_without_a_component_is_unknown_never_the_unchecked_row() {
        for (row, why) in [
            (json!({"file": "a.c", "status": "Safe"}), "names no unit"),
            (json!({"library": "l", "file": "a.c"}), "carries no status"),
            (json!("Safe"), "not an object"),
        ] {
            let sidecar = sidecar(std::slice::from_ref(&row), &bundles());
            let rendered = render(&sidecar);
            assert_eq!(rendered[0]["status"], "Unknown", "{row}");
            assert!(
                rendered[0]["reason"].as_str().unwrap().contains(why),
                "{row}"
            );
        }
        // An unavailable provenance refuses every row.
        let sidecar = sidecar(
            &[row("l", "a.c", "Safe")],
            &Err("contract digest failed".into()),
        );
        let rendered = render(&sidecar);
        assert_eq!(rendered[0]["status"], "Unknown");
        assert!(
            rendered[0]["reason"]
                .as_str()
                .unwrap()
                .contains("contract digest failed")
        );
        // A presented row with no component.
        let mut sidecar = sidecar;
        sidecar.contribution = Contribution::new();
        assert_eq!(render(&sidecar)[0]["status"], "Unknown");
        assert!(sidecar.validate().is_err());
    }

    #[test]
    fn a_conflicted_row_renders_unknown_with_the_conflict_reason_and_check_refuses_drift() {
        let rows = vec![row("l", "a.c", "Safe")];
        let mut sidecar = sidecar(&rows, &bundles());
        let mut other = sidecar.contribution.components().next().unwrap().clone();
        other.detail["functions"] = json!([]);
        other.verdict.assumptions.clear();
        sidecar.contribution.insert(other);
        let rendered = render(&sidecar);
        assert_eq!(rendered[0]["status"], "Unknown");
        assert_eq!(rendered[0]["reason_code"], "composition-conflict");
        assert_eq!(
            ReasonCategory::classify(rendered[0]["reason"].as_str().unwrap()),
            Ok(ReasonCategory::CompositionConflict)
        );
        // The published row no longer re-derives from the conflicted sidecar.
        assert!(check(&json!({"units": rows}), &sidecar).is_err());
        // A second, non-conflicting producer that says Unknown moves the row.
        let mut sidecar = self::sidecar(&rows, &bundles());
        let mut plugin = sidecar.contribution.components().next().unwrap().clone();
        plugin.provenance.external_checker = Some(qcore_schemas::composition::CheckerIdentity {
            id: "acme.check".into(),
            version: "1".into(),
            digest: "cd".repeat(32),
        });
        plugin.verdict.verdict = Verdict::Unknown;
        plugin.detail["status"] = json!("Unknown");
        sidecar.contribution.insert(plugin);
        let rendered = render(&sidecar);
        assert_eq!(rendered[0]["status"], "Unknown");
        assert_eq!(rendered[0]["reason_code"], Value::Null);
    }
}
