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

use qcore_contract::{hypothesis, premise_flow};
use qcore_schemas::hypothesis::*;
use qcore_schemas::premise_flow::*;
use std::collections::BTreeMap;
pub const SOURCE: &str = include_str!("premise_flow.qv");
pub struct Fixture {
    pub sources: BTreeMap<String, String>,
    pub graphs: BTreeMap<String, Graph>,
    pub models: BTreeMap<String, Model>,
    pub artifacts: BTreeMap<String, Vec<u8>>,
    pub document: Document,
    pub packet: FlowInstance,
    pub context: Context,
}
impl Fixture {
    pub fn new(model_bytes: &[u8], audit_bytes: &[u8]) -> Self {
        let sources = BTreeMap::from([("example.qv".into(), SOURCE.into())]);
        let graphs = hypothesis::replay(&sources).unwrap();
        let graph = graphs["observed"].clone();
        let artifacts = BTreeMap::from([
            ("model.lean".into(), model_bytes.to_vec()),
            ("audit.lean".into(), audit_bytes.to_vec()),
        ]);
        let environment = EnvironmentDefinition {
            id: "assigned_finite_propositions_v1".into(),
            variables: BTreeMap::from([(
                "valuation".into(),
                "three assigned propositions; no C/source truth".into(),
            )]),
        };
        let atoms = ["a", "b", "c"]
            .into_iter()
            .map(|id| {
                (
                    id.into(),
                    FormalBinding {
                        atom_sha256: qcore_schemas::sha256_hex(
                            &serde_json::to_vec(&graph.atoms[id]).unwrap(),
                        ),
                        proposition: "QCore.PremiseFlow.proposition".into(),
                        model: ArtifactReference {
                            path: "model.lean".into(),
                            sha256: qcore_schemas::sha256_hex(model_bytes),
                        },
                        audit: ArtifactReference {
                            path: "audit.lean".into(),
                            sha256: qcore_schemas::sha256_hex(audit_bytes),
                        },
                        environment: environment.id.clone(),
                        quantifiers: vec![format!("assigned atom {id}; valuation : Nat -> Prop")],
                        prerequisites: vec![environment.id.clone()],
                        conclusion: format!("the assigned valuation of {id}"),
                    },
                )
            })
            .collect();
        let models = BTreeMap::from([("observed".into(), Model { environment, atoms })]);
        let document =
            premise_flow::compile(&sources, &graphs, &models, &artifacts).unwrap()["observed"]
                .clone();
        let context = Context {
            artifact: "premise-flow-example".into(),
            property: "proposed-logical-expression".into(),
            semantic_profile: "flow-v1".into(),
            scope: "three assigned finite logical propositions".into(),
            bounds: "a,b,c; no source truth".into(),
            assumptions: "assigned A and B; positively refuted C".into(),
            source_identity: qcore_schemas::sha256_hex(SOURCE.as_bytes()),
            build_identity: "example-assignment-model-v1".into(),
            evidence_version: "1".into(),
        };
        let evidence = [
            ("a", Disposition::Supported),
            ("b", Disposition::Supported),
            ("c", Disposition::Refuted),
        ]
        .into_iter()
        .map(|(id, disposition)| Evidence {
            atom: id.into(),
            atom_sha256: models["observed"].atoms[id].atom_sha256.clone(),
            context: context.clone(),
            disposition,
            warrant: Warrant::Declared,
            reference: "explicit finite logical assignment, not C evidence".into(),
        })
        .collect();
        let packet = FlowInstance {
            document: document.clone(),
            document_sha256: premise_flow::document_digest(&document).unwrap(),
            hypotheses: Instance {
                graph: graph.clone(),
                graph_sha256: hypothesis::digest(&graph).unwrap(),
                context: context.clone(),
                evidence,
            },
        };
        Self {
            sources,
            graphs,
            models,
            artifacts,
            document,
            packet,
            context,
        }
    }
    pub fn held(&self) -> premise_flow::Expected<'_> {
        premise_flow::Expected {
            sources: &self.sources,
            graphs: &self.graphs,
            models: &self.models,
            artifacts: &self.artifacts,
            document: &self.document,
            context: &self.context,
        }
    }
}
