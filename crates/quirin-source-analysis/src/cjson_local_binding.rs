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

//! Finite native cJSON heaps plus full typed source-body correspondence.
//! No universal representation, interference, custody or safety authority.
use qcore_c_gateway::{
    legacy_c::{C11TranslationUnit, prepare_translation_unit},
    typed_source_match::{TypedBodyComparison, compare_typed_body},
};
use qcore_schemas::hypothesis::{Context, Graph};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MODEL: &str = include_str!("../../../proof/QCore/CjsonRepresentationPremises.lean");
const EXPECTED_GRAPH: &str =
    include_str!("../../../docs/reports/premise-evidence-contracts/expected-graph.json");
pub const ENVIRONMENT: &str = "cjson-native-single-thread-allocated-ledger.v1";
pub const PROPERTY: &str = "cjson.native-local-representation.v1";
pub const SCOPE: &str = "lengths0..4;all-valid-detach-indices;11cases";
pub const BOUNDS: &str = "single-thread;finite-live-ledger;no-callback-hooks";
pub const ASSUMPTIONS: &str =
    "native-execution-and-ledger-observed;allocator-and-extraction-not-proved";
fn evidence_version() -> String {
    format!(
        "premise-local-source-bindings.v1;checker:{};model:{}",
        qcore_schemas::sha256_hex(include_bytes!("cjson_local_binding.rs")),
        qcore_schemas::sha256_hex(include_bytes!(
            "../../../proof/QCore/LocalSourceBindings.lean"
        ))
    )
}
fn source_artifact(sha: &str) -> String {
    format!("cjson-source-sha256:{sha}#cJSON_DetachItemViaPointer")
}

/// Fixed reference is compared with the whole actual typed definition. The
/// source's field representations are checked separately by the gateway.
const REFERENCE: &str = r#"
cJSON *qcore_source_reference(cJSON *parent, cJSON * const item) {
 if ((parent == ((void *)0)) || (item == ((void *)0)) || (item != parent->child && item->prev == ((void *)0))) { return ((void *)0); }
 if (item != parent->child) { item->prev->next = item->next; }
 if (item->next != ((void *)0)) { item->next->prev = item->prev; }
 if (item == parent->child) { parent->child = item->next; }
 else if (item->next == ((void *)0)) { parent->child->prev = item->prev; }
 item->prev = ((void *)0); item->next = ((void *)0); return item;
}
"#;
/// Sealed typed comparison, context and graph. No public deserializer.
///
/// ```compile_fail
/// use quirin_source_analysis::cjson_local_binding::CheckedSource;
/// let _: CheckedSource = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug, Serialize)]
pub struct CheckedSource {
    context: Context,
    graph_sha256: String,
    atom_sha256: BTreeMap<String, String>,
    model_sha256: String,
    environment: String,
    raw_sha256: String,
    prepared_sha256: String,
    comparison: TypedBodyComparison,
    native_directory: Option<std::path::PathBuf>,
}
/// The held Context/source/configuration is the consumer's independently
/// selected target. An arbitrary revision label never authenticates a source.
pub fn check_source(
    unit: &C11TranslationUnit<'_>,
    expected_raw_sha256: &str,
    held_context: &Context,
    presented_context: &Context,
    graph: &Graph,
    model: &[u8],
    environment: &str,
) -> Result<CheckedSource, String> {
    if held_context != presented_context
        || model != MODEL.as_bytes()
        || environment != ENVIRONMENT
        || held_context.property != PROPERTY
        || held_context.semantic_profile != ENVIRONMENT
        || held_context.scope != SCOPE
        || held_context.bounds != BOUNDS
        || held_context.assumptions != ASSUMPTIONS
        || held_context.artifact != source_artifact(expected_raw_sha256)
        || held_context.evidence_version != evidence_version()
        || [
            &held_context.artifact,
            &held_context.semantic_profile,
            &held_context.scope,
            &held_context.bounds,
            &held_context.assumptions,
            &held_context.build_identity,
            &held_context.evidence_version,
        ]
        .iter()
        .any(|s| s.is_empty())
        || qcore_schemas::sha256_hex(unit.source.as_bytes()) != expected_raw_sha256
    {
        return Err("cjson local source: held source/context/model/environment mismatch".into());
    }
    let expected: Graph = serde_json::from_str(EXPECTED_GRAPH).map_err(|e| e.to_string())?;
    if *graph != expected {
        return Err("cjson local source: graph/atom mismatch".into());
    }
    let prepared = prepare_translation_unit(unit);
    let pp = prepared.preprocessed_source().map_err(|e| e.to_string())?;
    let prepared_sha256 = qcore_schemas::sha256_hex(pp.as_bytes());
    if prepared_sha256 != held_context.source_identity {
        return Err("cjson local source: held preprocessing differs".into());
    }
    let comparison = compare_typed_body(
        unit,
        expected_raw_sha256,
        "cJSON_DetachItemViaPointer",
        REFERENCE,
        &[(
            "cJSON",
            &[
                ("next", "struct cJSON *"),
                ("prev", "struct cJSON *"),
                ("child", "struct cJSON *"),
            ],
        )],
    )?;
    Ok(CheckedSource {
        context: held_context.clone(),
        graph_sha256: qcore_contract::hypothesis::digest(graph)?,
        atom_sha256: graph
            .atoms
            .iter()
            .map(|(id, atom)| {
                Ok((
                    id.clone(),
                    qcore_schemas::sha256_hex(
                        &serde_json::to_vec(atom).map_err(|e| e.to_string())?,
                    ),
                ))
            })
            .collect::<Result<_, String>>()?,
        model_sha256: qcore_schemas::sha256_hex(model),
        environment: environment.into(),
        raw_sha256: expected_raw_sha256.into(),
        prepared_sha256,
        comparison,
        native_directory: match &unit.input {
            qcore_c_gateway::legacy_c::CSourceInput::DeclaredFile { spelled, directory }
                if spelled == "cJSON.c"
                    && unit.include_paths == vec![directory.to_string_lossy().into_owned()]
                    && unit.definitions.is_empty()
                    && unit.preprocessor_options.is_empty()
                    && unit.dialect == qcore_c_gateway::legacy_c::CCompilerDialect::C11 =>
            {
                Some(directory.clone())
            }
            _ => None,
        },
    })
}

/// Untrusted native object observations. Addresses are opaque identities, never
/// dereferenced by Rust; the independently executed harness reads live objects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: u64,
    pub next: u64,
    pub prev: u64,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Heap {
    pub parent: u64,
    pub child: u64,
    pub nodes: Vec<Node>,
    pub allocations: Vec<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCase {
    pub length: usize,
    pub index: Option<usize>,
    pub before: Heap,
    pub after: Heap,
    pub returned: u64,
}

fn equations(heap: &Heap, sequence: &[u64]) -> Result<(), String> {
    let ids = heap.nodes.iter().map(|n| n.id).collect::<BTreeSet<_>>();
    let allocated = heap.allocations.iter().copied().collect::<BTreeSet<_>>();
    if heap.parent == 0
        || ids.len() != heap.nodes.len()
        || ids.contains(&0)
        || ids.contains(&heap.parent)
        || allocated.len() != heap.allocations.len()
        || allocated != ids.iter().copied().chain([heap.parent]).collect()
        || sequence.iter().copied().collect::<BTreeSet<_>>().len() != sequence.len()
        || sequence.iter().any(|id| !ids.contains(id))
        || heap.child != sequence.first().copied().unwrap_or(0)
    {
        return Err("cjson finite heap: allocation/distinct/parent/child domain mismatch".into());
    }
    for (i, id) in sequence.iter().enumerate() {
        let node = heap
            .nodes
            .iter()
            .find(|n| n.id == *id)
            .ok_or("cjson finite heap: missing child")?;
        let next = sequence.get(i + 1).copied().unwrap_or(0);
        let prev = if i == 0 {
            sequence.last().copied().unwrap_or(0)
        } else {
            sequence[i - 1]
        };
        if node.next != next || node.prev != prev {
            return Err("cjson finite heap: next/predecessor/endpoint equation mismatch".into());
        }
    }
    // Distinct finite sequence, exact adjacent-next equations and null last-next
    // entail acyclicity; every nonnull link target lies in the allocation ledger.
    Ok(())
}

/// Cannot be imported from a serialized receipt.
///
/// ```compile_fail
/// use quirin_source_analysis::cjson_local_binding::CheckedNativeDomain;
/// let _: CheckedNativeDomain = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Debug, Serialize)]
pub struct CheckedNativeDomain {
    schema: &'static str,
    claim: &'static str,
    source: CheckedSource,
    complete_case_domain: Vec<(usize, Option<usize>)>,
    native_observation_sha256: String,
    heap_model_views: Vec<serde_json::Value>,
    finite_equations: [&'static str; 8],
    prerequisites: [&'static str; 3],
    universal_atoms: &'static str,
    no_writer_trace: &'static str,
}
impl CheckedNativeDomain {
    pub fn case_count(&self) -> usize {
        self.complete_case_domain.len()
    }
}

/// Freshly collected native observations are a separate held input, not copied
/// from an old receipt. Exact case-domain completeness is checked before any
/// scoped result is constructed. Neither input alone authenticates an execution;
/// the native driver retains actual compiler/run commands and bytes.
fn check_native_domain(
    source: CheckedSource,
    independently_observed: &[NativeCase],
    presented: &[NativeCase],
    prerequisites: &BTreeSet<String>,
) -> Result<CheckedNativeDomain, String> {
    let required = [
        "native-driver-executed-held-source",
        "allocation-ledger-live-until-snapshot",
        "single-thread-harness-no-callback-hooks",
    ];
    if required.iter().any(|p| !prerequisites.contains(*p)) || independently_observed != presented {
        return Err("cjson finite heap: prerequisite missing or native replay differs".into());
    }
    let expected = std::iter::once((0, None))
        .chain((1..=4).flat_map(|n| (0..n).map(move |i| (n, Some(i)))))
        .collect::<BTreeSet<_>>();
    let domain = presented
        .iter()
        .map(|c| (c.length, c.index))
        .collect::<BTreeSet<_>>();
    if domain != expected || presented.len() != expected.len() {
        return Err("cjson finite heap: incomplete/duplicate case domain".into());
    }
    for c in presented {
        if c.before.nodes.len() != c.length
            || c.after.nodes.len() != c.length
            || c.before.allocations != c.after.allocations
            || c.before.parent != c.after.parent
            || c.before.nodes.iter().map(|n| n.id).collect::<Vec<_>>()
                != c.after.nodes.iter().map(|n| n.id).collect::<Vec<_>>()
        {
            return Err("cjson finite heap: incomplete before/after object domain".into());
        }
        let sequence = c.before.nodes.iter().map(|n| n.id).collect::<Vec<_>>();
        equations(&c.before, &sequence)?;
        let mut remaining = sequence.clone();
        if let Some(index) = c.index {
            let detached = remaining.remove(index);
            let n = c
                .after
                .nodes
                .iter()
                .find(|n| n.id == detached)
                .ok_or("cjson finite heap: detached object missing")?;
            if c.returned != detached || n.next != 0 || n.prev != 0 {
                return Err(
                    "cjson finite heap: returned item or detached null links mismatch".into(),
                );
            }
        } else if c.returned != 0 {
            return Err("cjson finite heap: empty result mismatch".into());
        }
        equations(&c.after, &remaining)?;
    }
    Ok(CheckedNativeDomain {
        schema: "cjson-local-source-heap.v1",
        claim: "typed-detach-body-and-complete-finite-native-heap-equations",
        source,
        complete_case_domain: domain.into_iter().collect(),
        native_observation_sha256: qcore_schemas::sha256_hex(
            &serde_json::to_vec(presented).map_err(|e| e.to_string())?,
        ),
        heap_model_views: presented.iter().map(|case| {
            let before=case.before.nodes.iter().map(|node|node.id).collect::<Vec<_>>();
            let after=before.iter().enumerate().filter(|(i,_)|Some(*i)!=case.index).map(|(_,id)|*id).collect::<Vec<_>>();
            let rank=|sequence:&[u64]|sequence.iter().enumerate().map(|(index,id)|(id.to_string(),index+1)).collect::<BTreeMap<_,_>>();
            serde_json::json!({"case":[case.length,case.index],"parent":case.before.parent,"children_before":before,"children_after":after,"rank_before":rank(&before),"rank_after":rank(&after),"live_domain":case.before.allocations,"ownedChild_interpretation":"membership in exact selected finite sequence only; no arbitrary C custody claim","next_prev_interpretation":"exact native field map; absent IDs map to None","trace_interpretation":"Open; no interleaving trace supplied"})
        }).collect(),
        finite_equations: [
            "chain",
            "distinct",
            "empty",
            "firstPrev",
            "lastNext",
            "linkTargets-relative-to-live-ledger",
            "parentOutside",
            "predecessor",
        ],
        prerequisites: required,
        universal_atoms: "open",
        no_writer_trace: "open-no-interleaving-trace-proved",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> String {
        format!(
            "#include <stddef.h>\ntypedef struct cJSON {{ struct cJSON *next,*prev,*child; }} cJSON;\n{}",
            REFERENCE.replace("qcore_source_reference", "cJSON_DetachItemViaPointer")
        )
    }
    fn ctx(source: &str) -> Context {
        let unit = C11TranslationUnit {
            source,
            ..Default::default()
        };
        Context {
            artifact: source_artifact(&qcore_schemas::sha256_hex(source.as_bytes())),
            property: PROPERTY.into(),
            semantic_profile: ENVIRONMENT.into(),
            scope: SCOPE.into(),
            bounds: BOUNDS.into(),
            assumptions: ASSUMPTIONS.into(),
            source_identity: qcore_schemas::sha256_hex(
                prepare_translation_unit(&unit)
                    .preprocessed_source()
                    .unwrap()
                    .as_bytes(),
            ),
            build_identity: "fixture-compile".into(),
            evidence_version: evidence_version(),
        }
    }
    fn checked() -> CheckedSource {
        let s = fixture();
        let c = ctx(&s);
        check_source(
            &C11TranslationUnit {
                source: &s,
                ..Default::default()
            },
            &qcore_schemas::sha256_hex(s.as_bytes()),
            &c,
            &c,
            &serde_json::from_str(EXPECTED_GRAPH).unwrap(),
            MODEL.as_bytes(),
            ENVIRONMENT,
        )
        .unwrap()
    }
    fn requirements() -> BTreeSet<String> {
        [
            "native-driver-executed-held-source",
            "allocation-ledger-live-until-snapshot",
            "single-thread-harness-no-callback-hooks",
        ]
        .map(str::to_owned)
        .into_iter()
        .collect()
    }
    fn heap(sequence: &[u64], length: usize) -> Heap {
        Heap {
            parent: 10,
            child: sequence.first().copied().unwrap_or(0),
            allocations: std::iter::once(10).chain(1..=length as u64).collect(),
            nodes: (1..=length as u64)
                .map(|id| {
                    if let Some(i) = sequence.iter().position(|v| *v == id) {
                        Node {
                            id,
                            next: sequence.get(i + 1).copied().unwrap_or(0),
                            prev: if i == 0 {
                                sequence.last().copied().unwrap_or(0)
                            } else {
                                sequence[i - 1]
                            },
                        }
                    } else {
                        Node {
                            id,
                            next: 0,
                            prev: 0,
                        }
                    }
                })
                .collect(),
        }
    }
    fn cases() -> Vec<NativeCase> {
        std::iter::once((0, None))
            .chain((1..=4).flat_map(|n| (0..n).map(move |i| (n, Some(i)))))
            .map(|(length, index)| {
                let seq = (1..=length as u64).collect::<Vec<_>>();
                let mut remain = seq.clone();
                let returned = index.map(|i| remain.remove(i)).unwrap_or(0);
                NativeCase {
                    length,
                    index,
                    before: heap(&seq, length),
                    after: heap(&remain, length),
                    returned,
                }
            })
            .collect()
    }
    #[test]
    fn typed_body_and_all_context_model_environment_graph_boundaries() {
        let s = fixture();
        let c = ctx(&s);
        let g: Graph = serde_json::from_str(EXPECTED_GRAPH).unwrap();
        let u = C11TranslationUnit {
            source: &s,
            ..Default::default()
        };
        let sha = qcore_schemas::sha256_hex(s.as_bytes());
        check_source(&u, &sha, &c, &c, &g, MODEL.as_bytes(), ENVIRONMENT).unwrap();
        for key in [
            "artifact",
            "property",
            "semantic_profile",
            "scope",
            "bounds",
            "assumptions",
            "source_identity",
            "build_identity",
            "evidence_version",
        ] {
            let mut v = serde_json::to_value(&c).unwrap();
            v[key] = "foreign".into();
            let wrong = serde_json::from_value(v).unwrap();
            assert!(
                check_source(&u, &sha, &c, &wrong, &g, MODEL.as_bytes(), ENVIRONMENT).is_err(),
                "{key}"
            );
        }
        assert!(check_source(&u, "foreign", &c, &c, &g, MODEL.as_bytes(), ENVIRONMENT).is_err());
        for key in [
            "artifact",
            "property",
            "semantic_profile",
            "scope",
            "bounds",
            "assumptions",
            "evidence_version",
        ] {
            let mut v = serde_json::to_value(&c).unwrap();
            v[key] = "self-consistent-foreign".into();
            let wrong: Context = serde_json::from_value(v).unwrap();
            assert!(
                check_source(&u, &sha, &wrong, &wrong, &g, MODEL.as_bytes(), ENVIRONMENT).is_err(),
                "independent fixed target axis{key}"
            );
        }

        assert!(check_source(&u, &sha, &c, &c, &g, b"changed model", ENVIRONMENT).is_err());
        assert!(check_source(&u, &sha, &c, &c, &g, MODEL.as_bytes(), "foreign").is_err());
        let mut wrong = g.clone();
        wrong.atoms.remove("detach_chain");
        assert!(check_source(&u, &sha, &c, &c, &wrong, MODEL.as_bytes(), ENVIRONMENT).is_err());
        // Hashes and preprocessing context honestly updated: semantic body still
        // differs from independently fixed reference and must refuse.
        let changed = s.replace("item->next = ((void *)0);", "item->next = parent;");
        assert_ne!(changed, s, "negative must alter the actual source bytes");
        let c = ctx(&changed);
        assert!(
            check_source(
                &C11TranslationUnit {
                    source: &changed,
                    ..Default::default()
                },
                &qcore_schemas::sha256_hex(changed.as_bytes()),
                &c,
                &c,
                &g,
                MODEL.as_bytes(),
                ENVIRONMENT
            )
            .is_err()
        );
    }
    #[test]
    fn complete_heap_domain_predicates_and_prerequisites_are_not_receipt_booleans() {
        let source = checked();
        let held = cases();
        let guards = requirements();
        assert_eq!(
            check_native_domain(source.clone(), &held, &held, &guards)
                .unwrap()
                .case_count(),
            11
        );
        let mut truncated = held.clone();
        truncated.pop();
        assert!(check_native_domain(source.clone(), &held, &truncated, &guards).is_err());
        assert!(
            check_native_domain(source.clone(), &truncated, &truncated, &guards).is_err(),
            "both sides truncated still fail fixed domain"
        );
        let mut wrong = held.clone();
        wrong[10].after.nodes[0].next = 999;
        assert!(check_native_domain(source.clone(), &held, &wrong, &guards).is_err());
        assert!(
            check_native_domain(source.clone(), &wrong, &wrong, &guards).is_err(),
            "self-consistent malformed heap is independently checked"
        );
        for mutation in 0..6 {
            let mut bad = held.clone();
            let c = &mut bad[10];
            match mutation {
                0 => c.before.nodes[1].id = c.before.nodes[0].id,
                1 => c.before.child = 0,
                2 => c.before.nodes[0].prev = 999,
                3 => {
                    c.before.allocations.pop();
                }
                4 => c.returned = 999,
                _ => {
                    c.after.nodes.pop();
                }
            }
            assert!(
                check_native_domain(source.clone(), &bad, &bad, &guards).is_err(),
                "heap mutation{mutation}"
            );
        }
        let checked = check_native_domain(source.clone(), &held, &held, &guards).unwrap();
        for entry in [
            "entry_and_post",
            "entry_or_post",
            "post_except_incomplete",
            "post_terminal",
        ] {
            let flow = checked
                .evaluate_scoped_flow(&source.context, entry)
                .unwrap();
            assert_eq!(
                flow.evaluation().disposition,
                qcore_schemas::hypothesis::Disposition::Supported
            );
            assert_eq!(flow.evaluation().terminated, entry == "post_terminal");
            assert!(flow.evaluation().atom_dependencies.len() >= 2);
        }
        let mut foreign = source.context.clone();
        foreign.property = "resource-custody.v1".into();
        assert!(
            checked
                .evaluate_scoped_flow(&foreign, "entry_and_post")
                .is_err()
        );
        for guard in &guards {
            let mut missing = guards.clone();
            missing.remove(guard);
            assert!(check_native_domain(source.clone(), &held, &held, &missing).is_err());
        }
    }
}

// This graph has only new finite-domain propositions. It does not name or
// rewrite any atom of the original production cJSON representation graph.
const FINITE_FLOW_SOURCE: &str = r#"
module contract;
extern fn qcore_cjson_native_domain() -> unit contract {
 requires resource_contract_v1;
 requires resource_hypothesis(finite_detach_post, "All post-field comparisons in the exact native case domain hold");
 requires resource_hypothesis(finite_entry_equations, "Eight finite representation equations hold on the recorded native entry snapshots");
 requires resource_hypothesis(incomplete_fixed_case_domain, "The exact recorded native case-key multiset differs from the fixed required eleven-case multiset");
 requires resource_premise_all(native_case_claims, finite_detach_post, finite_entry_equations, incomplete_fixed_case_domain);
 requires resource_premise_flow_v1(entry_and_post, (finite_entry_equations |-> finite_detach_post));
 requires resource_premise_flow_v1(entry_or_post, (finite_entry_equations ||-> finite_detach_post));
 requires resource_premise_flow_v1(post_except_incomplete, (finite_detach_post !-> incomplete_fixed_case_domain));
 requires resource_premise_flow_v1(post_terminal, (flow(entry_and_post) ->| finite_detach_post));
 ensures normal resource_return_none();
};
"#;

/// Sealed, source-local proposed evaluation. It supplies no safety verdict and
/// cannot be deserialized into a checked result or attached to universal atoms.
#[derive(Debug, Serialize)]
pub struct CheckedFlow {
    claim: &'static str,
    native_observation_sha256: String,
    bounded_graph_sha256: String,
    original_graph_sha256: String,
    result: qcore_schemas::premise_flow::ProposedEvaluation,
    universal_atoms: &'static str,
}
impl CheckedFlow {
    pub fn evaluation(&self) -> &qcore_schemas::premise_flow::ProposedEvaluation {
        &self.result
    }
}
impl CheckedNativeDomain {
    /// Construct only the three fixed finite assignments AFTER source and heap
    /// replay. Observed refutation of `incomplete` uses exact checked domain
    /// completeness, never absence/Unknown and never a universal writer claim.
    pub fn evaluate_scoped_flow(
        &self,
        held_context: &Context,
        entry: &str,
    ) -> Result<CheckedFlow, String> {
        use qcore_schemas::{
            hypothesis::{
                ArtifactReference, Disposition, EnvironmentDefinition, Evidence, FormalBinding,
                Instance, Warrant,
            },
            premise_flow::{FlowInstance, Model},
        };
        if held_context != &self.source.context {
            return Err("cjson local flow: foreign held Context".into());
        }
        let sources = BTreeMap::from([("native-domain.qv".into(), FINITE_FLOW_SOURCE.into())]);
        let contracts = qcore_contract::contract_loader::load_resource_contract_sources(&sources)?;
        let graphs = qcore_contract::hypothesis::replay(contracts.source_assumptions())?;
        let graph = &graphs["qcore_cjson_native_domain"];
        let model_path = "proof/QCore/LocalSourceBindings.lean";
        let audit_path = "proof/AuditLocalSourceBindings.lean";
        let model_bytes = include_bytes!("../../../proof/QCore/LocalSourceBindings.lean");
        let audit_bytes = include_bytes!("../../../proof/AuditLocalSourceBindings.lean");
        let artifacts = BTreeMap::from([
            (model_path.into(), model_bytes.to_vec()),
            (audit_path.into(), audit_bytes.to_vec()),
        ]);
        let environment=EnvironmentDefinition{id:ENVIRONMENT.into(),variables:BTreeMap::from([
            ("native_observation_sha256".into(),self.native_observation_sha256.clone()),
            ("entry_heap".into(),"Complete live native allocation identities and exact child/next/prev snapshots; finite sequence membership defines local domain only".into()),
            ("case_domain".into(),"Empty plus lengths1..4 all valid detach indices;11case keys".into()),
            ("prerequisites".into(),self.prerequisites.join(";")),
            ("context".into(),serde_json::to_string(held_context).map_err(|e|e.to_string())?),
            ("external_boundaries".into(),"Native execution/compiler/allocator correspondence is observed or assumed; arbitrary custody/writers and universal atoms remain Open".into()),
        ])};
        let atom_model = [
            (
                "finite_entry_equations",
                "QCore.LocalSourceBindings.finiteEntryEquations",
            ),
            (
                "finite_detach_post",
                "QCore.LocalSourceBindings.finiteDetachPost",
            ),
            (
                "incomplete_fixed_case_domain",
                "QCore.LocalSourceBindings.incompleteFixedCaseDomain",
            ),
        ];
        let atoms=atom_model.into_iter().map(|(id,prop)| Ok((id.into(),FormalBinding{
            atom_sha256:qcore_schemas::sha256_hex(&serde_json::to_vec(&graph.atoms[id]).map_err(|e|e.to_string())?),proposition:prop.into(),
            model:ArtifactReference{path:model_path.into(),sha256:qcore_schemas::sha256_hex(model_bytes)},
            audit:ArtifactReference{path:audit_path.into(),sha256:qcore_schemas::sha256_hex(audit_bytes)},environment:ENVIRONMENT.into(),quantifiers:vec!["Every entry/post snapshot in exactly the independently observed11case finite domain".into()],prerequisites:vec![ENVIRONMENT.into()],conclusion:prop.into(),
        }))).collect::<Result<_,String>>()?;
        let models = BTreeMap::from([(
            "qcore_cjson_native_domain".into(),
            Model { environment, atoms },
        )]);
        let documents =
            qcore_contract::premise_flow::compile(&sources, &graphs, &models, &artifacts)?;
        let document = &documents["qcore_cjson_native_domain"];
        let evidence = graph
            .atoms
            .iter()
            .map(|(id, atom)| Evidence {
                atom: id.clone(),
                atom_sha256: qcore_schemas::sha256_hex(
                    &serde_json::to_vec(atom).expect("atom serializes"),
                ),
                context: held_context.clone(),
                disposition: if id == "incomplete_fixed_case_domain" {
                    Disposition::Refuted
                } else {
                    Disposition::Supported
                },
                warrant: Warrant::Observed,
                reference: format!(
                    "native-observation:{};{}",
                    self.native_observation_sha256, ENVIRONMENT
                ),
            })
            .collect();
        let packet = FlowInstance {
            document: document.clone(),
            document_sha256: qcore_contract::premise_flow::document_digest(document)?,
            hypotheses: Instance {
                graph: graph.clone(),
                graph_sha256: qcore_contract::hypothesis::digest(graph)?,
                context: held_context.clone(),
                evidence,
            },
        };
        let result = qcore_contract::premise_flow::evaluate_proposed(
            &packet,
            qcore_contract::premise_flow::Expected {
                sources: &sources,
                graphs: &graphs,
                models: &models,
                artifacts: &artifacts,
                document,
                context: held_context,
            },
            entry,
        )?;
        Ok(CheckedFlow {
            claim: "source-bound-finite-native-proposed-flow-only",
            native_observation_sha256: self.native_observation_sha256.clone(),
            bounded_graph_sha256: qcore_contract::hypothesis::digest(graph)?,
            original_graph_sha256: self.source.graph_sha256.clone(),
            result,
            universal_atoms: "open",
        })
    }
}

/// Actual fixed native build inputs. Constructed by reading the supplied source,
/// header and resolved compiler bytes; no deserializer/unchecked constructor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NativeBuild {
    directory: std::path::PathBuf,
    compiler: String,
    compiler_prefix: Vec<String>,
    compiler_version_sha256: String,
    compiler_file_sha256: String,
    source_sha256: String,
    header_sha256: String,
    harness_sha256: String,
    arguments: Vec<String>,
}
impl NativeBuild {
    pub fn digest(&self) -> Result<String, String> {
        Ok(qcore_schemas::sha256_hex(
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        ))
    }
}
const HARNESS: &[u8] =
    include_bytes!("../../../docs/reports/premise-local-source-bindings/native/cjson_heap.c");
const NATIVE_FLAGS: [&str; 5] = [
    "-std=c11",
    "-O0",
    "-fsanitize=undefined",
    "-fno-sanitize-recover=all",
    "-I.",
];
fn read_input(path: impl AsRef<std::path::Path>) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}

fn resolved_compiler(program: &std::ffi::OsStr) -> Result<std::path::PathBuf, String> {
    let path = std::path::Path::new(program);
    let acceptable = |candidate: &std::path::Path| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            candidate.is_file()
                && candidate
                    .metadata()
                    .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        }
        #[cfg(not(unix))]
        {
            candidate.is_file()
        }
    };
    if path.components().count() > 1 || path.is_absolute() {
        if !acceptable(path) {
            return Err("cjson native: actual compiler executable absent".into());
        }
        return std::fs::canonicalize(path).map_err(|e| e.to_string());
    }
    for directory in
        std::env::split_paths(&std::env::var_os("PATH").ok_or("cjson native: PATH absent")?)
    {
        let candidate = directory.join(path);
        if acceptable(&candidate) {
            return std::fs::canonicalize(candidate).map_err(|e| e.to_string());
        }
        #[cfg(windows)]
        {
            let candidate = candidate.with_extension("exe");
            if acceptable(&candidate) {
                return std::fs::canonicalize(candidate).map_err(|e| e.to_string());
            }
        }
    }
    Err("cjson native: cannot independently resolve discovered compiler spelling".into())
}

pub fn native_build_configuration(directory: &std::path::Path) -> Result<NativeBuild, String> {
    let directory = std::fs::canonicalize(directory).map_err(|e| e.to_string())?;
    let command = qcore_c_gateway::clang_command()?;
    let compiler = resolved_compiler(command.get_program())?
        .to_string_lossy()
        .into_owned();
    let prefix = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let version = std::process::Command::new(&compiler)
        .args(&prefix)
        .arg("--version")
        .output()
        .map_err(|e| e.to_string())?;
    if !version.status.success() {
        return Err("cjson native: compiler identity refused".into());
    }
    let arguments = NATIVE_FLAGS
        .into_iter()
        .chain(["cJSON.c", "heap.c", "-lm", "-o", "heap"])
        .map(str::to_owned)
        .collect();
    Ok(NativeBuild {
        source_sha256: qcore_schemas::sha256_hex(&read_input(directory.join("cJSON.c"))?),
        header_sha256: qcore_schemas::sha256_hex(&read_input(directory.join("cJSON.h"))?),
        harness_sha256: qcore_schemas::sha256_hex(HARNESS),
        compiler_version_sha256: qcore_schemas::sha256_hex(&version.stdout),
        compiler_file_sha256: qcore_schemas::sha256_hex(&read_input(&compiler)?),
        directory,
        compiler,
        compiler_prefix: prefix,
        arguments,
    })
}
/// Derive every Context axis from actual held inputs and this checker's exact
/// bounded property/model/scope. A resource/null/universal Context cannot alias it.
pub fn native_context(
    unit: &C11TranslationUnit<'_>,
    build: &NativeBuild,
) -> Result<Context, String> {
    if qcore_schemas::sha256_hex(unit.source.as_bytes()) != build.source_sha256 {
        return Err("cjson native: raw source mismatch".into());
    }
    let prepared = prepare_translation_unit(unit);
    let pp = prepared.preprocessed_source().map_err(|e| e.to_string())?;
    Ok(Context {
        artifact: source_artifact(&build.source_sha256),
        property: PROPERTY.into(),
        semantic_profile: ENVIRONMENT.into(),
        scope: SCOPE.into(),
        bounds: BOUNDS.into(),
        assumptions: ASSUMPTIONS.into(),
        source_identity: qcore_schemas::sha256_hex(pp.as_bytes()),
        build_identity: build.digest()?,
        evidence_version: evidence_version(),
    })
}

/// This is the ONLY public constructor of checked native/flow authority. It
/// actually compiles and executes the held source, checks the compiled-input
/// preprocessing projection and all finite heap equations, and retains raw
/// output. Caller-provided serialized snapshots never mint a checked result.
pub fn execute_native_domain(
    source: CheckedSource,
    build: &NativeBuild,
) -> Result<CheckedNativeDomain, String> {
    use std::{fs, io::Write, process::Command};
    let directory = source.native_directory.as_ref().ok_or(
        "cjson native: source was not checked in exact declared native input configuration",
    )?;
    if directory != &build.directory
        || native_build_configuration(directory)? != *build
        || source.raw_sha256 != build.source_sha256
        || source.context.build_identity != build.digest()?
    {
        return Err("cjson native: stale build/compiler/source identity".into());
    }
    let raw =
        String::from_utf8(read_input(directory.join("cJSON.c"))?).map_err(|e| e.to_string())?;
    let unit = C11TranslationUnit {
        source: &raw,
        include_paths: vec![directory.to_string_lossy().into_owned()],
        dialect: qcore_c_gateway::legacy_c::CCompilerDialect::C11,
        input: qcore_c_gateway::legacy_c::CSourceInput::DeclaredFile {
            spelled: "cJSON.c".into(),
            directory: directory.clone(),
        },
        ..Default::default()
    };
    if native_context(&unit, build)? != source.context {
        return Err("cjson native: one of nine Context axes differs from independently reconstructed target".into());
    }
    for name in [
        "heap.c",
        "heap",
        "native.json",
        "commands.json",
        "native-prepared.c",
    ] {
        if directory.join(name).exists() {
            return Err(format!("cjson native: fresh output required ({name})"));
        }
    }
    let mut harness = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("heap.c"))
        .map_err(|e| e.to_string())?;
    harness.write_all(HARNESS).map_err(|e| e.to_string())?;
    drop(harness);
    let mut commands = serde_json::json!({"build":build,"context":source.context});
    let pp = Command::new(&build.compiler)
        .args(&build.compiler_prefix)
        .args(NATIVE_FLAGS)
        .args(["-E", "-P", "cJSON.c"])
        .current_dir(directory)
        .output()
        .map_err(|e| e.to_string())?;
    fs::write(directory.join("native-prepared.c"), &pp.stdout).map_err(|e| e.to_string())?;
    fs::write(directory.join("native-prepared.stderr"), &pp.stderr).map_err(|e| e.to_string())?;
    commands["preprocessing_exit"] = serde_json::json!(pp.status.code());
    commands["native_preprocessed_sha256"] =
        serde_json::json!(qcore_schemas::sha256_hex(&pp.stdout));
    fs::write(
        directory.join("commands.json"),
        serde_json::to_vec_pretty(&commands).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    if !pp.status.success() || qcore_schemas::sha256_hex(&pp.stdout) != source.prepared_sha256 {
        return Err(
            "cjson native: actual compile preprocessing differs from typed checked source".into(),
        );
    }
    let compiled = Command::new(&build.compiler)
        .args(&build.compiler_prefix)
        .args(&build.arguments)
        .current_dir(directory)
        .output()
        .map_err(|e| e.to_string())?;
    fs::write(directory.join("compile.stdout"), &compiled.stdout).map_err(|e| e.to_string())?;
    fs::write(directory.join("compile.stderr"), &compiled.stderr).map_err(|e| e.to_string())?;
    commands["compile_exit"] = serde_json::json!(compiled.status.code());
    fs::write(
        directory.join("commands.json"),
        serde_json::to_vec_pretty(&commands).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    if !compiled.status.success() {
        return Err("cjson native: actual compile failed".into());
    }
    let executed = Command::new(directory.join("heap"))
        .current_dir(directory)
        .output()
        .map_err(|e| e.to_string())?;
    fs::write(directory.join("native.json"), &executed.stdout).map_err(|e| e.to_string())?;
    fs::write(directory.join("native.stderr"), &executed.stderr).map_err(|e| e.to_string())?;
    commands["native_exit"] = serde_json::json!(executed.status.code());
    commands["native_stderr_bytes"] = serde_json::json!(executed.stderr.len());
    commands["executable_sha256"] = serde_json::json!(qcore_schemas::sha256_hex(&read_input(
        directory.join("heap")
    )?));
    commands["native_stdout_sha256"] =
        serde_json::json!(qcore_schemas::sha256_hex(&executed.stdout));
    fs::write(
        directory.join("commands.json"),
        serde_json::to_vec_pretty(&commands).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    if !executed.status.success() || !executed.stderr.is_empty() {
        return Err("cjson native: execution or sanitizer refused".into());
    }
    if native_build_configuration(directory)? != *build
        || read_input(directory.join("heap.c"))? != HARNESS
    {
        return Err("cjson native: actual inputs changed during execution".into());
    }
    let observed: Vec<NativeCase> =
        serde_json::from_slice(&executed.stdout).map_err(|e| e.to_string())?;
    let guards = [
        "native-driver-executed-held-source",
        "allocation-ledger-live-until-snapshot",
        "single-thread-harness-no-callback-hooks",
    ]
    .map(str::to_owned)
    .into_iter()
    .collect();
    check_native_domain(source, &observed, &observed, &guards)
}
