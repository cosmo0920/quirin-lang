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

//! Card **SP10-FORMAL-BINDING-DESCRIPTIONS** (contract specification C04.18.2,
//! C04.18.3, C04.18.9's SP10 block, `QF005`): binding manifests and flow
//! models state typed environment references, never English descriptions.
#[path = "../../../tests/support/legacy_prose_premise.rs"]
mod legacy_prose_premise;
#[path = "../examples/support/premise_flow_fixture.rs"]
#[allow(dead_code)]
mod premise_flow_fixture;

use qcore_contract::formal_binding::{
    ENVIRONMENT_DECISION_VECTORS, PROSE_BINDING, VECTOR_DEFINITION, ValueKind, admitted, classify,
};
use qcore_contract::{hypothesis, hypothesis_bindings, premise_flow};
use qcore_frontend::premise_formula::RENDERED_TERMS;
use qcore_schemas::hypothesis::{BindingManifest, Graph};
use qcore_schemas::quant_proposition::Definition;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const LEAN: &str = include_str!("../../../proof/QCore/FormalBindingEnvironments.lean");
const RECORD: &str =
    include_str!("../../../docs/reports/sp10-formal-binding-descriptions/migration.json");
const INDEX: &str =
    include_str!("../../../docs/reports/sp10-formal-binding-descriptions/bindings/index.json");
const CJSON: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
const DETACH_MANIFEST: &[u8] = include_bytes!(
    "../../../docs/reports/sp10-formal-binding-descriptions/bindings/a1fa87c8c5bc126404bd955397ff7b1cff0b4f4feb28a436d32003c4ca01bf55.json"
);
const DETACH_ENVIRONMENT: &str = "cjson_detach_entry_and_interference_v1";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn record() -> serde_json::Value {
    serde_json::from_str(RECORD).unwrap()
}

fn occurrences() -> Vec<serde_json::Value> {
    let index: serde_json::Value = serde_json::from_str(INDEX).unwrap();
    index["occurrences"].as_array().unwrap().clone()
}

fn read_old(sha: &str) -> Vec<u8> {
    [
        "docs/reports/sp6-symbolic-hypothesis-atoms/bindings",
        "docs/reports/sp2-symbolic-librdkafka-premises/bindings",
    ]
    .iter()
    .map(|d| root().join(d).join(format!("{sha}.json")))
    .find(|p| p.exists())
    .map(|p| std::fs::read(p).unwrap())
    .unwrap()
}

fn read_new(sha: &str) -> Vec<u8> {
    std::fs::read(
        root()
            .join("docs/reports/sp10-formal-binding-descriptions/bindings")
            .join(format!("{sha}.json")),
    )
    .unwrap()
}

/// The typed parameters each live manifest's bound atoms quantify, by name.
fn parameters(manifest: &BindingManifest) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for binding in manifest.atoms.values() {
        for q in &binding.quantifiers {
            let p: qcore_schemas::quant_proposition::Parameter = serde_json::from_str(q).unwrap();
            out.insert(p.name, serde_json::to_string(&p.sort).unwrap());
        }
    }
    out
}

/// Every live manifest's environment is typed references only, and every
/// one of its bindings is derived from its typed atom.
#[test]
fn every_live_manifest_states_typed_references_only() {
    let occurrences = occurrences();
    assert_eq!(occurrences.len(), 167);
    let mut values = 0;
    for o in &occurrences {
        let sha = o["manifest_sha256"].as_str().unwrap();
        let bytes = read_new(sha);
        assert_eq!(qcore_schemas::sha256_hex(&bytes), sha);
        let manifest: BindingManifest = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&manifest).unwrap(), bytes);
        for value in manifest.environment.variables.values() {
            let terms: Vec<qcore_schemas::quant_proposition::Parameter> =
                serde_json::from_str(value).unwrap();
            assert!(!terms.is_empty());
            values += 1;
        }
    }
    assert_eq!(values, 166 * 11 + 9);
}

/// The 167 frozen manifests the card replaced: each is refused as it stands
/// (`QF005`), and the legacy reader lifts it to exactly the bytes of its
/// replacement. The descriptions are the only change.
#[test]
fn every_frozen_manifest_is_refused_and_lifts_to_its_replacement() {
    let mut lifted = 0;
    for o in occurrences() {
        let old = read_old(o["previous_manifest_sha256"].as_str().unwrap());
        let new = read_new(o["manifest_sha256"].as_str().unwrap());
        let frozen: BindingManifest = serde_json::from_slice(&old).unwrap();
        let error = qcore_contract::formal_binding::check_environment(&frozen.environment, &[])
            .unwrap_err();
        assert!(error.starts_with(PROSE_BINDING), "{error}");
        let mut value: serde_json::Value = serde_json::from_slice(&old).unwrap();
        lifted += legacy_prose_premise::lift_environment(&mut value["environment"]);
        let lift: BindingManifest = serde_json::from_value(value).unwrap();
        assert_eq!(serde_json::to_vec(&lift).unwrap(), new);
        let mut unchanged = lift.clone();
        unchanged.environment = frozen.environment.clone();
        assert_eq!(unchanged, frozen);
    }
    assert_eq!(lifted, 166 * 11 + 9);
}

/// An unrecorded description is not lifted: it stays English and is refused.
#[test]
fn an_unrecorded_description_stays_refused() {
    let mut manifest: serde_json::Value = serde_json::from_slice(DETACH_MANIFEST).unwrap();
    manifest["environment"]["variables"]["live"] =
        serde_json::Value::String("Entry liveness predicate, every interpretation Open".into());
    assert_eq!(
        legacy_prose_premise::lift_environment(&mut manifest["environment"]),
        0
    );
    let manifest: BindingManifest = serde_json::from_value(manifest).unwrap();
    let error =
        qcore_contract::formal_binding::check_environment(&manifest.environment, &[]).unwrap_err();
    assert!(error.starts_with(PROSE_BINDING), "{error}");
}

struct Detach {
    graphs: BTreeMap<String, Graph>,
    artifacts: BTreeMap<String, Vec<u8>>,
}

fn detach() -> Detach {
    let sources = BTreeMap::from([("cjson.qv".into(), CJSON.into())]);
    let graphs = hypothesis::replay(&sources).unwrap();
    let artifacts = BTreeMap::from([
        (
            "proof/QCore/CjsonRepresentationPremises.lean".into(),
            include_bytes!("../../../proof/QCore/CjsonRepresentationPremises.lean").to_vec(),
        ),
        (
            "proof/AuditPremiseRefinement.lean".into(),
            include_bytes!("../../../proof/AuditPremiseRefinement.lean").to_vec(),
        ),
    ]);
    Detach { graphs, artifacts }
}

/// Resolve the detach declaration against `manifest`, its marker repointed
/// to the manifest's digest.
fn resolve_detach(d: &Detach, manifest: &BindingManifest) -> Result<(), String> {
    let bytes = serde_json::to_vec(manifest).unwrap();
    let sha = qcore_schemas::sha256_hex(&bytes);
    let current = qcore_schemas::sha256_hex(DETACH_MANIFEST);
    assert_eq!(CJSON.matches(&current).count(), 1);
    let sources = BTreeMap::from([("cjson.qv".into(), CJSON.replace(&current, &sha))]);
    hypothesis_bindings::resolve(
        &sources,
        &d.graphs,
        &BTreeMap::from([(sha.clone(), manifest.clone())]),
        &BTreeMap::from([(sha, bytes)]),
        &d.artifacts,
    )
    .map(|_| ())
}

/// The resolver route: the live manifest resolves; English in any formal
/// field is refused with `QF005`.
#[test]
fn english_in_a_manifest_is_refused_on_the_resolver_route() {
    let d = detach();
    let live: BindingManifest = serde_json::from_slice(DETACH_MANIFEST).unwrap();
    resolve_detach(&d, &live).unwrap();
    let old: BindingManifest = serde_json::from_slice(&read_old(
        "27c0736584d81d659e44dd29eae891ccdc37cfeaf019a57a620fadaa73d910a1",
    ))
    .unwrap();
    let mut cases: Vec<(&str, BindingManifest)> = Vec::new();
    // The retired descriptions, one variable at a time and all at once.
    for (variable, value) in &old.environment.variables {
        let mut m = live.clone();
        m.environment
            .variables
            .insert(variable.clone(), value.clone());
        cases.push(("description", m));
    }
    let mut m = live.clone();
    m.environment = old.environment.clone();
    cases.push(("environment", m));
    let id = live.atoms.keys().next().unwrap().clone();
    let mut m = live.clone();
    m.atoms.get_mut(&id).unwrap().quantifiers = vec!["the parent and every child".into()];
    cases.push(("quantifier", m));
    let mut m = live.clone();
    m.atoms.get_mut(&id).unwrap().conclusion = "the chain holds".into();
    cases.push(("conclusion", m));
    let mut m = live.clone();
    m.atoms.get_mut(&id).unwrap().proposition = "the chain proposition".into();
    cases.push(("proposition", m));
    // A typed reference to a term that is neither a parameter nor registered.
    let mut m = live.clone();
    m.environment.variables.insert(
        "live".into(),
        r#"[{"name":"alive","sort":{"kind":"predicate","arguments":["obj"]}}]"#.into(),
    );
    cases.push(("unresolved", m));
    for (what, manifest) in cases {
        let error = resolve_detach(&d, &manifest).unwrap_err();
        assert!(error.starts_with(PROSE_BINDING), "{what}: {error}");
    }
}

/// The flow-model route: the fixture compiles; English is refused.
#[test]
fn english_in_a_flow_model_is_refused_on_the_flow_route() {
    let f = premise_flow_fixture::Fixture::new(b"model", b"audit");
    premise_flow::compile(&f.sources, &f.graphs, &f.models, &f.artifacts).unwrap();
    for case in 0..4 {
        let mut models = f.models.clone();
        let model = models.get_mut("observed").unwrap();
        match case {
            0 => {
                model.environment.variables.insert(
                    "valuation".into(),
                    "three assigned propositions; no C/source truth".into(),
                );
            }
            1 => {
                model.atoms.get_mut("a").unwrap().quantifiers =
                    vec!["assigned atom a; valuation : Nat -> Prop".into()];
            }
            2 => {
                model.atoms.get_mut("a").unwrap().conclusion = "the assigned valuation of a".into()
            }
            _ => model.environment.id = "assigned finite propositions".into(),
        }
        let error =
            premise_flow::compile(&f.sources, &f.graphs, &models, &f.artifacts).unwrap_err();
        assert!(error.starts_with(PROSE_BINDING), "{case}: {error}");
    }
}

#[test]
fn the_environment_decision_vectors_are_the_lean_table() {
    assert_eq!(PROSE_BINDING, "QF005");
    let definition: Definition = serde_json::from_str(VECTOR_DEFINITION).unwrap();
    let rows = ENVIRONMENT_DECISION_VECTORS
        .iter()
        .map(|(value, expected)| {
            let kind = classify(value, &[&definition]);
            assert_eq!(admitted(kind), *expected, "{value}");
            let tag = match kind {
                ValueKind::Digest => ".digest".to_owned(),
                ValueKind::Terms {
                    canonical,
                    resolved,
                } => format!(".terms {canonical} {resolved}"),
                ValueKind::Text => ".text".to_owned(),
            };
            format!("({tag}, {expected})")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def environmentVectors : List (EnvValue × Bool) := [{rows}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
}

#[test]
fn the_sp10_block_is_new_typed_and_bound_to_lean() {
    let sp10: Vec<_> = RENDERED_TERMS
        .iter()
        .filter(|t| t.block == "SP10")
        .collect();
    assert_eq!(sp10.len(), 22);
    for t in &sp10 {
        assert!(
            qcore_frontend::premise_formula::term(t.name).is_none(),
            "#{} is already a registered term",
            t.name
        );
        assert_eq!(
            RENDERED_TERMS.iter().filter(|u| u.name == t.name).count(),
            1,
            "#{} is in two blocks",
            t.name
        );
        assert!(!t.meaning.is_empty());
    }
    let names = sp10
        .iter()
        .map(|t| format!("\"{}\"", t.name))
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def sp10Terms : List String := [{names}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    // User decision 2026-11-24 (A′, "Prefix, then land"): every SP10 name is
    // prefixed, `plg_` for a plugin-model component over SP9's sorts, `bnd_`
    // for the binding's own terms and sorts, so no generic name is reserved.
    for t in &sp10 {
        assert!(
            t.name.starts_with("plg_") || t.name.starts_with("bnd_"),
            "#{}",
            t.name
        );
    }
    let sorts: Vec<_> = qcore_frontend::premise_formula::RENDERED_VALUE_SORTS
        .iter()
        .filter(|(s, _)| s.starts_with("bnd_"))
        .map(|(s, _)| format!("\"{s}\""))
        .collect();
    assert_eq!(sorts.len(), 2);
    let line = format!("def sp10Sorts : List String := [{}]", sorts.join(", "));
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    // Every sort an SP10 term names is a registered value sort (SP9's
    // `plg_*` or SP10's `bnd_*`) or `integer`.
    use qcore_frontend::premise_formula::RenderedSort;
    let registered = |s: &str| {
        s == "integer"
            || qcore_frontend::premise_formula::RENDERED_VALUE_SORTS
                .iter()
                .any(|(v, _)| *v == s)
    };
    for t in &sp10 {
        let names: Vec<&str> = match t.sort {
            RenderedSort::Value(v) => vec![v],
            RenderedSort::Predicate(a) => a.to_vec(),
            RenderedSort::Function(a, r) => a.iter().copied().chain([r]).collect(),
        };
        assert!(names.iter().all(|s| registered(s)), "#{}", t.name);
    }
    for generic in [
        "access",
        "forge",
        "forged",
        "api_call",
        "core_write",
        "hook_install",
        "held_context",
        "binding_source_corresponds",
        "resource_view",
        "environment",
        "event",
    ] {
        assert!(
            qcore_frontend::premise_formula::rendered_parameter_sort(generic).is_none(),
            "#{generic} is registered"
        );
    }
}

fn lean_list(names: impl IntoIterator<Item = String>) -> String {
    names
        .into_iter()
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The recorded references and the atoms' parameter names are the Lean
/// tables `every_reference_resolves` is proved over; every SP10 term an
/// environment names is one of them, in its registered sort.
#[test]
fn the_recorded_references_are_the_lean_table() {
    let occurrences = occurrences();
    let manifest = |declaration: &str| -> BindingManifest {
        let o = occurrences
            .iter()
            .find(|o| o["declaration"].as_str() == Some(declaration))
            .unwrap();
        serde_json::from_slice(&read_new(o["manifest_sha256"].as_str().unwrap())).unwrap()
    };
    let detach = manifest("cJSON_DetachItemViaPointer");
    let plugin_occurrence = occurrences
        .iter()
        .find(|o| o["premise"].as_str() == Some("rk_plugin_callback_closure"))
        .unwrap();
    let plugin = manifest(plugin_occurrence["declaration"].as_str().unwrap());
    for (name, m) in [("pluginParameters", &plugin), ("detachParameters", &detach)] {
        let line = format!(
            "def {name} : List String := [{}]",
            lean_list(parameters(m).into_keys())
        );
        assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    }
    let rows = record()["descriptions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            let environment = d["environment"].as_str().unwrap();
            let terms = d["terms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t["name"].as_str().unwrap().to_owned());
            format!(
                "(\"{environment}\", \"{}\", [{}])",
                d["variable"].as_str().unwrap(),
                lean_list(terms)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let line = format!("def references : List (String × String × List String) := [{rows}]");
    assert!(LEAN.lines().any(|l| l == line), "no `{line}`");
    // Each recorded term is a bound atom's parameter in its sort, or a
    // registered (SP7, SP10) term in its rendered sort.
    for d in record()["descriptions"].as_array().unwrap() {
        let m = if d["environment"].as_str() == Some(DETACH_ENVIRONMENT) {
            &detach
        } else {
            &plugin
        };
        let held = parameters(m);
        for t in d["terms"].as_array().unwrap() {
            let name = t["name"].as_str().unwrap();
            let sort = serde_json::to_string(&t["sort"]).unwrap();
            let sort: qcore_schemas::quant_proposition::Sort = serde_json::from_str(&sort).unwrap();
            let typed = serde_json::to_string(&sort).unwrap();
            assert!(
                held.get(name) == Some(&typed)
                    || qcore_frontend::premise_formula::rendered_parameter_sort(name).as_ref()
                        == Some(&sort),
                "{name}"
            );
        }
    }
}

/// The vector definition is canonical, and an empty list is not canonical.
#[test]
fn the_vector_definition_is_well_formed() {
    let definition: Definition = serde_json::from_str(VECTOR_DEFINITION).unwrap();
    assert_eq!(
        serde_json::to_string(&definition).unwrap(),
        VECTOR_DEFINITION
    );
    assert!(matches!(
        classify("[]", &[&definition]),
        ValueKind::Terms {
            canonical: false,
            ..
        }
    ));
}
