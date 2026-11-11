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

//! Actual reader replay, not a C execution or universal discharge.
use qcore_c_gateway::{
    contract_dependencies::Recording,
    legacy_c::{CCompilerDialect, analyze_legacy_c_translation_unit},
    reading_site_evidence::*,
};
use qcore_contract::{contract_loader::load_resource_contract_sources, hypothesis};
use qcore_schemas::{
    contract::{
        ResourceContracts, ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
    },
    hypothesis::Context,
};
use std::collections::BTreeMap;

const SCALAR: &str = "int *__errno_location(void);\nint read_errno(void) { if (*__errno_location() == 1) return 1; return 0; }\n";
fn contracts() -> ResourceContracts {
    let mut c = load_resource_contract_sources(&BTreeMap::from([(
        "errno.qv".into(),
        include_str!("../../../providers/quirin-provider-libc/contracts/libc/errno.qv").into(),
    )]))
    .unwrap();
    c.insert_contract(
        "consume",
        ResourceFunctionContract::new(
            [
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Borrowed,
            ],
            ResourceReturnMode::None,
        ),
    );
    c
}
fn artifacts() -> BTreeMap<String, Vec<u8>> {
    let manifest: serde_json::Value = serde_json::from_str(BINDINGS).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    manifest["artifacts"]
        .as_object()
        .unwrap()
        .values()
        .map(|r| {
            let path = r["path"].as_str().unwrap();
            (path.to_owned(), std::fs::read(root.join(path)).unwrap())
        })
        .collect()
}
fn context(source: &str, owner: &str) -> Context {
    Context {
        artifact: format!("independent/unit.c#{owner}"),
        property: "resource-custody.v1".into(),
        semantic_profile: "profile".into(),
        scope: "selected".into(),
        bounds: "bounds".into(),
        assumptions: "assumptions".into(),
        source_identity: qcore_schemas::sha256_hex(source.as_bytes()),
        build_identity: "build".into(),
        evidence_version: "independent-version".into(),
    }
}
fn observe(source: &str, entry: &str, c: &ResourceContracts) -> Vec<Observation> {
    let recording = Recording::start();
    let _ = analyze_legacy_c_translation_unit(source, entry, c);
    recording.finish().reading_sites
}
#[test]
fn actual_scalar_site_replays_and_all_context_coordinates_are_required() {
    let c = contracts();
    let observations = observe(SCALAR, "read_errno", &c);
    assert!(
        !observations.is_empty(),
        "actual accepted scalar reader captured"
    );
    let graph = hypothesis::replay(c.source_assumptions()).unwrap();
    let ctx = context(SCALAR, "read_errno");
    let receipts = observations
        .iter()
        .map(|o| bind(o, &ctx, &graph[&o.declaration]).unwrap())
        .collect::<Vec<_>>();
    replay(
        SCALAR,
        "read_errno",
        &c,
        None,
        CCompilerDialect::C11,
        &graph,
        &ctx,
        &receipts,
        BINDINGS.as_bytes(),
        &artifacts(),
    )
    .unwrap();
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
        let mut value = serde_json::to_value(&ctx).unwrap();
        value[key] = format!("changed-{key}").into();
        let wrong = serde_json::from_value(value).unwrap();
        assert!(
            replay(
                SCALAR,
                "read_errno",
                &c,
                None,
                CCompilerDialect::C11,
                &graph,
                &wrong,
                &receipts,
                BINDINGS.as_bytes(),
                &artifacts()
            )
            .is_err(),
            "{key}"
        );
    }
    let mut wrong_graph = graph.clone();
    wrong_graph
        .get_mut("__errno_location")
        .unwrap()
        .atoms
        .remove("scalar_declaration");
    assert!(
        replay(
            SCALAR,
            "read_errno",
            &c,
            None,
            CCompilerDialect::C11,
            &wrong_graph,
            &ctx,
            &receipts,
            BINDINGS.as_bytes(),
            &artifacts()
        )
        .is_err()
    );
    assert!(
        replay(
            &SCALAR.replace("== 1", "== 2"),
            "read_errno",
            &c,
            None,
            CCompilerDialect::C11,
            &graph,
            &ctx,
            &receipts,
            BINDINGS.as_bytes(),
            &artifacts()
        )
        .is_err()
    );
    for receipt in &receipts {
        assert_eq!(receipt.universal_disposition, "open");
        assert_eq!(receipt.coverage, "selected-observations-only");
    }
}
#[test]
fn siblings_and_spans_replay_against_actual_source_not_a_self_consistent_receipt() {
    let c = contracts();
    let source = "char *strerror(int n);\nvoid consume(char *a, char *b);\nvoid use_errors(void) { consume(strerror(1), strerror(2)); }\n";
    let observations = observe(source, "use_errors", &c);
    let spans = observations
        .iter()
        .map(|o| (o.call.start, o.call.end))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(spans.len(), 2, "both physical sibling sites captured");
    // Distinct effective environments retain distinct observations; repeated
    // replay in the same environment is deduplicated by the recorder.
    assert!(
        observations
            .iter()
            .enumerate()
            .all(|(i, o)| !observations[..i].contains(o))
    );
    assert!(observations.iter().all(|o| o.siblings.len() == 2));
    let graph = hypothesis::replay(c.source_assumptions()).unwrap();
    let ctx = context(source, "use_errors");
    let mut receipts = observations
        .iter()
        .map(|o| bind(o, &ctx, &graph["strerror"]).unwrap())
        .collect::<Vec<_>>();
    replay(
        source,
        "use_errors",
        &c,
        None,
        CCompilerDialect::C11,
        &graph,
        &ctx,
        &receipts,
        BINDINGS.as_bytes(),
        &artifacts(),
    )
    .unwrap();
    // Recompute the entire receipt after removing its sibling: digest consistency
    // is insufficient because independent capture still has the real sibling.
    receipts[0].observation.siblings.pop();
    receipts[0] = bind(&receipts[0].observation, &ctx, &graph["strerror"]).unwrap();
    assert!(
        replay(
            source,
            "use_errors",
            &c,
            None,
            CCompilerDialect::C11,
            &graph,
            &ctx,
            &receipts,
            BINDINGS.as_bytes(),
            &artifacts()
        )
        .is_err()
    );
}
#[test]
fn rejected_shapes_and_incompatible_declarations_publish_no_success() {
    let c = contracts();
    for source in [
        SCALAR.replace("== 1", "== *p"),
        SCALAR.replace("int *__errno_location", "struct x *__errno_location"),
        SCALAR.replace("__errno_location() ==", "__errno_location(1) =="),
    ] {
        assert!(observe(&source, "read_errno", &c).is_empty(), "{source}");
    }
    for expression in ["strerror(strerror(1))", "strerror(1) + 1", "strerror(1, 2)"] {
        let source = format!(
            "char *strerror(int n);\nvoid consume(char *a, char *b);\nvoid use_errors(void) {{ consume({expression}, 0); }}\n"
        );
        assert!(
            observe(&source, "use_errors", &c).is_empty(),
            "{expression}"
        );
    }
    let without_recording = analyze_legacy_c_translation_unit(SCALAR, "read_errno", &c).unwrap();
    let recording = Recording::start();
    let with_recording = analyze_legacy_c_translation_unit(SCALAR, "read_errno", &c).unwrap();
    let _ = recording.finish();
    assert_eq!(without_recording, with_recording);
}

#[test]
fn reference_resolution_rejects_missing_or_changed_independent_artifacts() {
    let mut held = artifacts();
    resolve_bindings(BINDINGS.as_bytes(), &held).unwrap();
    let path = held.keys().next().unwrap().clone();
    held.get_mut(&path).unwrap().push(b' ');
    assert!(resolve_bindings(BINDINGS.as_bytes(), &held).is_err());
    held.remove(&path);
    assert!(resolve_bindings(BINDINGS.as_bytes(), &held).is_err());
    assert!(resolve_bindings(b"{}", &artifacts()).is_err());
}

#[test]
fn detached_effective_environment_preserves_existing_receipt_shape_and_exact_bytes() {
    let contracts = contracts();
    let recording = Recording::start();
    let _ = analyze_legacy_c_translation_unit(SCALAR, "read_errno", &contracts);
    let recorded = recording.finish();
    assert!(!recorded.reading_sites.is_empty());
    let graphs = hypothesis::replay(contracts.source_assumptions()).unwrap();
    let ctx = context(SCALAR, "read_errno");
    let receipts = recorded
        .reading_sites
        .iter()
        .map(|observation| {
            validate_observation_environment(observation, &recorded.reading_environments).unwrap();
            let canonical = &recorded.reading_environments[&observation.environment_sha256];
            let environment =
                validate_environment(canonical, &observation.environment_sha256).unwrap();
            assert_eq!(serde_json::to_string(&environment).unwrap(), *canonical);
            assert_eq!(
                environment.contract(&observation.declaration),
                Some(&observation.contract)
            );
            let receipt = bind(observation, &ctx, &graphs[&observation.declaration]).unwrap();
            // Original production tuple, before this detached census extension.
            let old_id = qcore_schemas::sha256_hex(
                &serde_json::to_vec(&(
                    observation,
                    &ctx,
                    &receipt.graph_sha256,
                    &receipt.atom_sha256,
                ))
                .unwrap(),
            );
            assert_eq!(receipt.id, old_id);
            let value = serde_json::to_value(&receipt).unwrap();
            let fields = value
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                fields,
                std::collections::BTreeSet::from([
                    "schema",
                    "id",
                    "claim",
                    "coverage",
                    "observation",
                    "context",
                    "binding_manifest_sha256",
                    "graph_sha256",
                    "atom_sha256",
                    "proposition_ids",
                    "universal_disposition"
                ])
            );
            assert!(value.get("environments").is_none());
            receipt
        })
        .collect::<Vec<_>>();
    let published = publication_environments(&receipts, &recorded.reading_environments).unwrap();
    assert_eq!(
        published.len(),
        receipts
            .iter()
            .map(|r| &r.observation.environment_sha256)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    assert!(
        publication_environments(&[], &recorded.reading_environments)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn detached_effective_environment_missing_changed_noncanonical_or_foreign_contract_refuses() {
    let contracts = contracts();
    let recording = Recording::start();
    let _ = analyze_legacy_c_translation_unit(SCALAR, "read_errno", &contracts);
    let recorded = recording.finish();
    let observation = &recorded.reading_sites[0];
    let canonical = &recorded.reading_environments[&observation.environment_sha256];
    assert!(validate_observation_environment(observation, &BTreeMap::new()).is_err());
    let changed = format!("{canonical} ");
    assert!(validate_environment(&changed, &observation.environment_sha256).is_err());
    assert!(
        validate_environment(&changed, &qcore_schemas::sha256_hex(changed.as_bytes())).is_err(),
        "recomputed digest does not admit noncanonical bytes"
    );
    let mut unknown: serde_json::Value = serde_json::from_str(canonical).unwrap();
    unknown["unrecognized_authority"] = serde_json::json!(true);
    let unknown = serde_json::to_string(&unknown).unwrap();
    assert!(
        validate_environment(&unknown, &qcore_schemas::sha256_hex(unknown.as_bytes())).is_err()
    );
    let mut foreign = observation.clone();
    foreign.contract =
        ResourceFunctionContract::new([ResourceParameterMode::Borrowed], ResourceReturnMode::None);
    assert!(validate_observation_environment(&foreign, &recorded.reading_environments).is_err());
}
