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
/// Fixed reference is compared with the whole actual typed definition. The
/// source's field representations are checked separately by the gateway.
const REFERENCE: &str = r#"
cJSON *qcore_source_reference(cJSON *parent, cJSON * const item) {
 if ((parent == NULL) || (item == NULL) || (item != parent->child && item->prev == NULL)) { return NULL; }
 if (item != parent->child) { item->prev->next = item->next; }
 if (item->next != NULL) { item->next->prev = item->prev; }
 if (item == parent->child) { parent->child = item->next; }
 else if (item->next == NULL) { parent->child->prev = item->prev; }
 item->prev = NULL; item->next = NULL; return item;
}
"#;
/// Sealed typed comparison, context and graph. No public deserializer.
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
        || held_context.property != "resource-custody.v1"
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

#[derive(Debug, Serialize)]
pub struct CheckedNativeDomain {
    schema: &'static str,
    claim: &'static str,
    source: CheckedSource,
    complete_case_domain: Vec<(usize, Option<usize>)>,
    native_observation_sha256: String,
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
pub fn check_native_domain(
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
            artifact: "fixture#cJSON_DetachItemViaPointer".into(),
            property: "resource-custody.v1".into(),
            semantic_profile: "fixture".into(),
            scope: "11case-domain".into(),
            bounds: "4".into(),
            assumptions: "explicit-observed-live-ledger".into(),
            source_identity: qcore_schemas::sha256_hex(
                prepare_translation_unit(&unit)
                    .preprocessed_source()
                    .unwrap()
                    .as_bytes(),
            ),
            build_identity: "fixture-compile".into(),
            evidence_version: "v1".into(),
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
        assert!(check_source(&u, &sha, &c, &c, &g, b"changed model", ENVIRONMENT).is_err());
        assert!(check_source(&u, &sha, &c, &c, &g, MODEL.as_bytes(), "foreign").is_err());
        let mut wrong = g.clone();
        wrong.atoms.remove("detach_chain");
        assert!(check_source(&u, &sha, &c, &c, &wrong, MODEL.as_bytes(), ENVIRONMENT).is_err());
        // Hashes and preprocessing context honestly updated: semantic body still
        // differs from independently fixed reference and must refuse.
        let changed = s.replace("item->next = NULL;", "item->next = parent;");
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
        for guard in &guards {
            let mut missing = guards.clone();
            missing.remove(guard);
            assert!(check_native_domain(source.clone(), &held, &held, &missing).is_err());
        }
    }
}
