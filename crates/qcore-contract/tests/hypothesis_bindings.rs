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

use qcore_contract::{hypothesis, hypothesis_bindings};
use qcore_schemas::hypothesis::{BindingManifest, Graph};
use std::collections::BTreeMap;

const SOURCE: &str =
    include_str!("../../../providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv");
const MANIFEST: &[u8] =
    include_bytes!("../../../docs/reports/premise-evidence-contracts/binding-manifest.json");
const MODEL: &[u8] = include_bytes!("../../../proof/QCore/CjsonRepresentationPremises.lean");
const AUDIT: &[u8] = include_bytes!("../../../proof/AuditPremiseRefinement.lean");
struct Inputs {
    sources: BTreeMap<String, String>,
    graphs: BTreeMap<String, Graph>,
    expected: BTreeMap<String, BindingManifest>,
    presented: BTreeMap<String, Vec<u8>>,
    artifacts: BTreeMap<String, Vec<u8>>,
}
fn inputs() -> Inputs {
    let sources = BTreeMap::from([("cjson.qv".into(), SOURCE.into())]);
    let graphs = hypothesis::replay(&sources).unwrap();
    let frozen = hypothesis::decode(include_bytes!(
        "../../../docs/reports/premise-evidence-contracts/expected-graph.json"
    ))
    .unwrap();
    assert_eq!(graphs["cJSON_DetachItemViaPointer"], frozen);
    let sha = qcore_schemas::sha256_hex(MANIFEST);
    Inputs {
        sources,
        graphs,
        expected: BTreeMap::from([(sha.clone(), serde_json::from_slice(MANIFEST).unwrap())]),
        presented: BTreeMap::from([(sha, MANIFEST.to_vec())]),
        artifacts: BTreeMap::from([
            (
                "proof/QCore/CjsonRepresentationPremises.lean".into(),
                MODEL.to_vec(),
            ),
            ("proof/AuditPremiseRefinement.lean".into(), AUDIT.to_vec()),
        ]),
    }
}
fn resolve(i: &Inputs) -> Result<BTreeMap<String, BindingManifest>, String> {
    hypothesis_bindings::resolve(
        &i.sources,
        &i.graphs,
        &i.expected,
        &i.presented,
        &i.artifacts,
    )
}
#[test]
fn production_bindings_resolve_only_from_explicit_independent_inputs() {
    let i = inputs();
    let resolved = resolve(&i).unwrap();
    assert_eq!(resolved, i.expected);
    let m = resolved.values().next().unwrap();
    assert_eq!(m.atoms.len(), 9);
    assert_eq!(m.source_binding, "open");
    let requirements = hypothesis_bindings::requirements(&i.sources, &i.graphs).unwrap();
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].graph_sha256, m.graph_sha256);
    assert_eq!(
        m.atoms["detach_no_writer"].proposition,
        "QCore.CjsonRepresentationPremises.noWriter"
    );
    assert_eq!(
        m.atoms["detach_no_writer"].prerequisites.as_slice(),
        std::slice::from_ref(&m.environment.id)
    );
}
#[test]
fn stale_missing_or_changed_bindings_artifacts_contexts_and_domains_refuse() {
    let mut i = inputs();
    i.presented.clear();
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    i.artifacts.clear();
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    i.presented.values_mut().next().unwrap().push(b' ');
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    i.artifacts.values_mut().next().unwrap()[0] ^= 1;
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    i.expected
        .values_mut()
        .next()
        .unwrap()
        .atoms
        .get_mut("detach_chain")
        .unwrap()
        .proposition
        .push_str("_other");
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    i.graphs
        .get_mut("cJSON_DetachItemViaPointer")
        .unwrap()
        .atoms
        .get_mut("detach_no_writer")
        .unwrap()
        .proposition = qcore_schemas::hypothesis::Proposition::Opaque {
        statement: "different".into(),
    };
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    let text = i.sources.values_mut().next().unwrap();
    *text = text
        .lines()
        .filter(|l| !l.starts_with("// qcore-premise-binding-manifest-v1"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    let text = i.sources.values_mut().next().unwrap();
    *text = text.replace("manifest-v1", "manifest-v9");
    assert!(resolve(&i).is_err());
    let mut i = inputs();
    let text = i.sources.values_mut().next().unwrap();
    let pin = text
        .lines()
        .find(|l| l.starts_with("// qcore-premise-binding-manifest-v1"))
        .unwrap()
        .to_owned();
    text.push_str(&format!("\n{pin}\n"));
    assert!(resolve(&i).is_err());
}
#[test]
fn even_independently_proposed_malformed_manifest_never_grants_source_truth() {
    for mutation in 0..9 {
        let mut i = inputs();
        let old = i.expected.keys().next().unwrap().clone();
        let mut m = i.expected[&old].clone();
        match mutation {
            0 => m.version = 2,
            1 => m.source_binding = "supported".into(),
            2 => m.atoms.get_mut("detach_chain").unwrap().atom_sha256 = "forged".into(),
            3 => m.atoms.get_mut("detach_chain").unwrap().environment = "other".into(),
            4 => m
                .atoms
                .get_mut("detach_chain")
                .unwrap()
                .prerequisites
                .clear(),
            5 => m.atoms.get_mut("detach_chain").unwrap().model.path = "../model.lean".into(),
            6 => {
                m.atoms.remove("detach_no_writer");
            }
            7 => m.environment.variables.clear(),
            8 => m.graph_sha256 = "other".into(),
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&m).unwrap();
        let sha = qcore_schemas::sha256_hex(&bytes);
        for source in i.sources.values_mut() {
            *source = source.replace(&old, &sha);
        }
        i.expected = BTreeMap::from([(sha.clone(), m)]);
        i.presented = BTreeMap::from([(sha, bytes)]);
        assert!(resolve(&i).is_err(), "mutation{mutation}");
    }
}
