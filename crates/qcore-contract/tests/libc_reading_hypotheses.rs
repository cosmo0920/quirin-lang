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

//! Production graph controls, not tests of C-library bodies.
use qcore_contract::{contract_loader::*, hypothesis::*};
use qcore_schemas::hypothesis::*;
use std::collections::BTreeMap;

const SOURCE: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/errno.qv");
fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([("errno.qv".into(), SOURCE.into())])
}
#[test]
fn migrated_production_graph_replays_and_keeps_the_runtime_premise_name() {
    let s = sources();
    let c = load_resource_contract_sources(&s).unwrap();
    let graphs = replay(c.source_assumptions()).unwrap();
    check_replay(&s, &graphs).unwrap();
    for (name, count) in [("__errno_location", 3), ("strerror", 5)] {
        let g = &graphs[name];
        assert_eq!(g.atoms.len(), count);
        assert_eq!(g.premises.len(), 1);
        assert_eq!(g.premises["nested_call_admission"].children.len(), count);
        assert!(
            g.atoms
                .values()
                .all(|a| matches!(a.proposition, Proposition::Formula { .. }))
        );
        assert_eq!(erasure(g).unwrap()[0].name, "nested_call_admission");
        assert_eq!(decode(&serde_json::to_vec(g).unwrap()).unwrap(), *g);
    }
    let Proposition::Formula { definition } =
        &graphs["__errno_location"].atoms["checked_reading_route"].proposition
    else {
        panic!()
    };
    use qcore_schemas::quant_proposition::Formula as F;
    let F::Forall { body, .. } = &definition.formula else {
        panic!()
    };
    let F::Implies { right, .. } = body.as_ref() else {
        panic!()
    };
    assert!(matches!(right.as_ref(), F::Or { .. }));
    assert_eq!(graphs["__errno_location"].version, 3);
    assert!(
        !graphs["__errno_location"]
            .atoms
            .contains_key("nested_route")
    );
}
#[test]
fn production_inheritance_and_stale_atom_controls_preserve_every_obligation() {
    let mut s = sources();
    s.get_mut("errno.qv").unwrap().push_str(
        "\nextern fn inherited_errno() -> ptr<mut, u8> contract inherit __errno_location;\n",
    );
    use qcore_contract::contract_inclusion::{
        ContractLoadPath, PublishedContractCrate, derive_contract_namespace,
    };
    let name = "quirin-provider-libc-reading";
    let krate = PublishedContractCrate {
        name: name.into(),
        namespace: derive_contract_namespace(name).unwrap(),
        version: "0.1.0".into(),
        dependencies: Default::default(),
        modules: s.clone(),
        published: Default::default(),
    };
    let path = ContractLoadPath::new(vec![krate]).unwrap();
    let c = load_published_resource_contracts(&path, &[PublishedModuleRef::new(name, "errno.qv")])
        .unwrap();
    let g = replay(c.source_assumptions()).unwrap();
    assert_eq!(g["inherited_errno"], g["__errno_location"]);
    let mut forged = g.clone();
    forged
        .get_mut("__errno_location")
        .unwrap()
        .atoms
        .remove("scalar_declaration");
    assert!(check_replay(c.source_assumptions(), &forged).is_err());
    let changed = c
        .source_assumptions()
        .iter()
        .map(|(name, text)| {
            (
                name.clone(),
                text.replace(
                    "#nested_route(#env, #s) || #scalar_route(#env, #s)",
                    "#nested_route(#env, #s) && #scalar_route(#env, #s)",
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let changed_graphs = replay(&changed).unwrap();
    assert_ne!(changed_graphs, g);
    assert!(check_replay(&changed, &g).is_err());
}
#[test]
fn refutation_and_context_mismatch_remain_evidence_only() {
    let g = replay(&sources())
        .unwrap()
        .remove("__errno_location")
        .unwrap();
    let ctx = Context {
        artifact: "unit".into(),
        property: "custody".into(),
        semantic_profile: "v1".into(),
        scope: "site".into(),
        bounds: "bounded".into(),
        assumptions: "libc".into(),
        source_identity: "source".into(),
        build_identity: "build".into(),
        evidence_version: "v1".into(),
    };
    let evidence = g
        .atoms
        .values()
        .map(|a| Evidence {
            atom: a.id.clone(),
            atom_sha256: qcore_schemas::sha256_hex(&serde_json::to_vec(a).unwrap()),
            context: ctx.clone(),
            disposition: Disposition::Supported,
            warrant: Warrant::ProviderProposed,
            reference: "synthetic conditional control; no C finding".into(),
        })
        .collect();
    let mut i = Instance {
        graph_sha256: digest(&g).unwrap(),
        graph: g.clone(),
        context: ctx.clone(),
        evidence,
    };
    assert_eq!(
        compose(&i, &ctx, &g, "nested_call_admission").unwrap(),
        Disposition::Supported
    );
    i.evidence[0].disposition = Disposition::Refuted;
    assert_eq!(
        compose(&i, &ctx, &g, "nested_call_admission").unwrap(),
        Disposition::Refuted
    );
    i.evidence[0].context.build_identity = "another-build".into();
    assert!(compose(&i, &ctx, &g, "nested_call_admission").is_err());
}
