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

//! Proposed logical flow dependencies follow actual contract consultations.
#[path = "../../qcore-contract/examples/support/premise_flow_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use qcore_c_gateway::contract_dependencies::{Recording, hypothesis_closure, row_dependencies};
use qcore_c_gateway::legacy_c::analyze_legacy_c_translation_unit;
use qcore_contract::{contract_loader::load_resource_contract_sources, premise_flow};
use qcore_schemas::hypothesis::Disposition;

#[test]
fn actual_direct_and_summary_consultations_retain_all_or_and_forbidden_dependencies() {
    let mut f = fixture::Fixture::new(
        include_bytes!("../../../proof/QCore/PremiseFlow.lean"),
        include_bytes!("../../../proof/AuditPremiseFlow.lean"),
    );
    let contracts = load_resource_contract_sources(&f.sources).unwrap();
    let source = "int direct(void *p) { return observed(p); } int caller(void *p) { return direct(p); } int unrelated(void) { return 7; }";
    // Independently held actual fixture/function identity; the logical
    // refutation remains an assigned model premise, never a C violation.
    f.context.artifact = "fixture.c#direct".into();
    f.context.source_identity = qcore_schemas::sha256_hex(source.as_bytes());
    f.context.scope = "direct,caller,unrelated; assigned logical witness".into();
    f.packet.hypotheses.context = f.context.clone();
    for evidence in &mut f.packet.hypotheses.evidence {
        evidence.context = f.context.clone();
    }
    let recording = Recording::start();
    let summary = analyze_legacy_c_translation_unit(source, "caller", &contracts).unwrap();
    let record = recording.finish();
    let rows = row_dependencies(&record, &summary, &contracts);
    for function in ["direct", "caller"] {
        let closure = hypothesis_closure(&rows[function], &f.graphs);
        assert!(closure["observed"].contains("a"));
        assert!(closure["observed"].contains("c"));
    }
    assert!(rows["caller"].summaries.contains("direct"));
    assert!(hypothesis_closure(&rows["unrelated"], &f.graphs).is_empty());
    let before = summary.clone();
    let mut refuted = f.packet.clone();
    refuted.hypotheses.evidence[0].disposition = Disposition::Refuted;
    let evaluated = premise_flow::evaluate_proposed(&refuted, f.held(), "not_flow").unwrap();
    assert_eq!(evaluated.disposition, Disposition::Refuted);
    assert_eq!(evaluated.atom_dependencies, ["a", "c"]);
    let impacted = rows
        .iter()
        .filter_map(|(name, row)| {
            let closure = hypothesis_closure(row, &f.graphs);
            closure
                .get("observed")
                .is_some_and(|atoms| atoms.contains("a"))
                .then_some(name.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(impacted, ["caller", "direct"]);
    // These are potential dependency users. A refutation held for one exact
    // Context is never silently relabeled as evidence in another caller.
    let mut foreign = f.context.clone();
    foreign.artifact = "fixture.c#caller".into();
    let held = premise_flow::Expected {
        context: &foreign,
        ..f.held()
    };
    assert!(premise_flow::evaluate_proposed(&refuted, held, "not_flow").is_err());
    assert_eq!(summary, before); // proposed refutation never mutates a verdict
    println!("FLOW_IMPACT_JSON={}",serde_json::to_string(&serde_json::json!({
        "version":1,"source_sha256":qcore_schemas::sha256_hex(source.as_bytes()),
        "document_sha256":f.packet.document_sha256,"graph_sha256":f.packet.hypotheses.graph_sha256,
        "refutation":"assigned finite model premise; no validated reachable C violation",
        "context":f.context,"atom":"a","flow":"not_flow","disposition":evaluated.disposition,
        "all_flow_dependencies":evaluated.atom_dependencies,"potential_dependent_rows":impacted,
        "exact_context_affected":["direct"],"dependent_other_context_unchanged":["caller"],
        "unrelated_unchanged":["unrelated"],"rewritten_verdicts":0
    })).unwrap());
}
