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

use qcore_contract::quant_theorem::{OpenBinding, instantiate};
use qcore_frontend::quant_theorem::parse;
use qcore_schemas::hypothesis::ArtifactReference;
use qcore_schemas::quant_theorem::TheoremSchema;
fn schema() -> TheoremSchema {
    let formal = parse("#formal(#x: plugin, #p: predicate<plugin>, #q: predicate<plugin>) using QCore.Plugin.role(#x, #p, #q) requires (#p(#x)) yields #q(#x)").unwrap();
    let artifact = ArtifactReference {
        path: "proof/QCore/Plugin.lean".into(),
        sha256: "a".repeat(64),
    };
    TheoremSchema {
        reference: formal.reference,
        parameters: formal.parameters,
        prerequisites: formal.prerequisites,
        conclusion: formal.conclusion,
        model: artifact.clone(),
        audit: artifact,
    }
}
const APPLICATION: &str = "#input_role(#instance: plugin, #ready: predicate<plugin>, #valid: predicate<plugin>) using QCore.Plugin.role(#instance, #ready, #valid) requires (#ready(#instance)) yields #valid(#instance)";
#[test]
fn application_retains_proof_and_source_obligations_for_all_roles() {
    for role in ["input", "output", "filter", "processor", "custom"] {
        let app = parse(&APPLICATION.replace("input_role", &format!("{role}_role"))).unwrap();
        let obligations = instantiate(&schema(), &app).unwrap();
        assert_eq!(obligations.prerequisites(), app.prerequisites);
        assert_eq!(obligations.conclusion(), &app.conclusion);
        assert_eq!(obligations.model_proof_binding(), OpenBinding::Open);
        assert_eq!(obligations.source_binding(), OpenBinding::Open);
        let json = serde_json::to_value(obligations).unwrap();
        assert_eq!(json["source_binding"], "open");
        assert_eq!(json["model_proof_binding"], "open");
    }
}
#[test]
fn changed_reference_missing_premise_conclusion_and_sort_refuse() {
    let expected = schema();
    let mut app = parse(APPLICATION).unwrap();
    app.reference.push_str("Other");
    assert!(instantiate(&expected, &app).is_err());
    let mut app = parse(APPLICATION).unwrap();
    app.prerequisites.clear();
    assert!(instantiate(&expected, &app).is_err());
    let mut app = parse(APPLICATION).unwrap();
    app.conclusion = app.prerequisites[0].clone();
    assert!(instantiate(&expected, &app).is_err());
    let mut app = parse(APPLICATION).unwrap();
    app.arguments.swap(0, 1);
    assert!(instantiate(&expected, &app).is_err());
}
#[test]
fn predicate_and_function_symbols_substitute_and_captures_refuse() {
    let formal=parse("#f(#x: integer, #f: function<integer,integer>) using QCore.identity(#x,#f) requires (#f(#x) == #x) yields forall(#b: integer) { #f(#b) == #b }").unwrap();
    let mut expected = schema();
    expected.reference = formal.reference;
    expected.parameters = formal.parameters;
    expected.prerequisites = formal.prerequisites;
    expected.conclusion = formal.conclusion;
    let app=parse("#g(#y: integer, #g: function<integer,integer>) using QCore.identity(#y,#g) requires (#g(#y) == #y) yields forall(#b: integer) { #g(#b) == #b }").unwrap();
    assert!(instantiate(&expected, &app).is_ok());
    let app=parse("#g(#b: integer, #g: function<integer,integer>) using QCore.identity(#b,#g) requires (#g(#b) == #b) yields forall(#c: integer) { #g(#c) == #c }").unwrap();
    assert!(instantiate(&expected, &app).is_err());
}
#[test]
fn malformed_trailing_undeclared_and_unbounded_source_refuse() {
    for suffix in [";", " garbage", " yields #valid(#instance)"] {
        assert!(parse(&format!("{APPLICATION}{suffix}")).is_err());
    }
    assert!(parse(&APPLICATION.replace("#ready(#instance)", "#unknown(#instance)")).is_err());
    assert!(parse(&APPLICATION.replace("QCore.Plugin.role", "QCore..role")).is_err());
    assert!(
        parse(&APPLICATION.replace("#instance, #ready, #valid", "#instance + 1, #ready, #valid"))
            .is_err()
    );
    assert!(parse(&" ".repeat(65537)).is_err());
    let mut expected = schema();
    expected.audit.sha256 = "bad".into();
    assert!(instantiate(&expected, &parse(APPLICATION).unwrap()).is_err());
}
#[test]
fn total_premise_budget_and_every_branch_are_checked() {
    let formulas = std::iter::repeat_n("#ready(#instance)", 33)
        .collect::<Vec<_>>()
        .join(",");
    assert!(
        parse(&APPLICATION.replace(
            "requires (#ready(#instance))",
            &format!("requires ({formulas})")
        ))
        .is_err()
    );
    let formula = std::iter::repeat_n("#ready(#instance)", 32)
        .fold("#ready(#instance)".to_string(), |a, b| {
            format!("({a} && {b})")
        });
    assert!(
        parse(&APPLICATION.replace("#ready(#instance)) yields", &format!("{formula}) yields")))
            .is_err()
    );
    assert!(
        parse(&APPLICATION.replace(
            "#valid(#instance)",
            "(#valid(#instance) || #missing(#instance))"
        ))
        .is_err()
    );
}

#[test]
fn source_retains_exact_plugin_bindings_and_strict_consumers_refuse() {
    use qcore_frontend::frontend::{SourceContractClauseKind as K, parse_source_program};
    use std::collections::BTreeMap;
    // Each role needs its own theorem schema and concrete source obligations.
    // These symbolic applications intentionally do not provide that proof.
    for role in ["input", "output", "filter", "processor", "custom"] {
        let expression = format!(
            "#{role}_instance(#instance: plugin, #descriptor: descriptor, #config: configuration, #callback: callback, #binds: predicate<plugin,descriptor,configuration,callback>, #abi: predicate<descriptor,callback>, #lifecycle: predicate<plugin,callback>, #custody: predicate<plugin,callback>, #effects: predicate<plugin,callback>, #valid: predicate<plugin>) using PluginRoles.{role}(#instance,#descriptor,#config,#callback,#binds,#abi,#lifecycle,#custody,#effects,#valid) requires (#binds(#instance,#descriptor,#config,#callback),#abi(#descriptor,#callback),#lifecycle(#instance,#callback),#custody(#instance,#callback),#effects(#instance,#callback)) yields #valid(#instance)"
        );
        let source = format!(
            "module contract; extern fn plugin() -> unit contract {{ quant theorem {expression}; }};"
        );
        let parsed = parse_source_program(&source).unwrap();
        let clause = &parsed.declarations[0]
            .value
            .function_contract
            .as_ref()
            .unwrap()
            .clauses[0]
            .value;
        assert_eq!(clause.kind, K::QuantTheorem);
        assert_eq!(clause.expression_source, expression);
        let app = parse(&clause.expression_source).unwrap();
        assert_eq!(app.prerequisites.len(), 5);
        let mut role_schema = schema();
        role_schema.reference = app.reference.clone();
        role_schema.parameters = app.parameters.clone();
        role_schema.prerequisites = app.prerequisites.clone();
        role_schema.conclusion = app.conclusion.clone();
        let obligations = instantiate(&role_schema, &app).unwrap();
        assert_eq!(obligations.source_binding(), OpenBinding::Open);
        assert_eq!(obligations.model_proof_binding(), OpenBinding::Open);
        for marker in ["resource_contract_v1", "bounds_contract_v1"] {
            let selected = source.replace("contract {", &format!("contract {{ requires {marker};"));
            let sources = BTreeMap::from([("role.qv".into(), selected)]);
            assert!(
                qcore_contract::contract_loader::load_resource_contract_sources(&sources).is_err()
            );
            assert!(
                qcore_contract::bounds_contract::load_bounds_contract_sources(&sources).is_err()
            );
        }
    }
}
