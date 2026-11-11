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

use qcore_c_gateway::{
    contract_dependencies::Recording,
    legacy_c::{CCompilerDialect, analyze_legacy_c_translation_unit},
    reading_site_domain::{self, HeldDomain},
    reading_site_evidence::{BINDINGS, Receipt, bind},
};
use qcore_schemas::hypothesis::Context;
use std::collections::{BTreeMap, BTreeSet};
const SOURCE: &str = "int *__errno_location(void);\nint f(void) { if (*__errno_location() == 1) return 1; if (*__errno_location() == 2) return 2; return 0; }\n";
#[test]
fn complete_recorded_domain_replays_and_truncated_context_source_or_environment_refuses() {
    let contracts =
        qcore_contract::contract_loader::load_resource_contract_sources(&BTreeMap::from([(
            "errno.qv".into(),
            include_str!("../../../providers/quirin-provider-libc/contracts/libc/errno.qv").into(),
        )]))
        .unwrap();
    let recording = Recording::start();
    analyze_legacy_c_translation_unit(SOURCE, "f", &contracts).unwrap();
    let record = recording.finish();
    let graphs = qcore_contract::hypothesis::replay(contracts.source_assumptions()).unwrap();
    let context = Context {
        artifact: "unit.c#f".into(),
        property: "resource-custody.v1".into(),
        semantic_profile: "p".into(),
        scope: "f".into(),
        bounds: "b".into(),
        assumptions: "a".into(),
        source_identity: qcore_schemas::sha256_hex(SOURCE.as_bytes()),
        build_identity: "compile".into(),
        evidence_version: "v1".into(),
    };
    let expected: Vec<Receipt> = record
        .reading_sites
        .iter()
        .map(|o| bind(o, &context, &graphs[&o.declaration]).unwrap())
        .collect();
    assert!(expected.len() >= 2);
    let manifest: serde_json::Value = serde_json::from_str(BINDINGS).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let artifacts = manifest["artifacts"]
        .as_object()
        .unwrap()
        .values()
        .map(|r| {
            let p = r["path"].as_str().unwrap();
            (p.to_owned(), std::fs::read(root.join(p)).unwrap())
        })
        .collect();
    let contexts = BTreeMap::from([("f".into(), context)]);
    let owners = BTreeSet::from(["f".into()]);
    let declarations = BTreeSet::from(["__errno_location".into()]);
    let sha = qcore_schemas::sha256_hex(SOURCE.as_bytes());
    let mut held = HeldDomain {
        source: SOURCE,
        raw_source: SOURCE.as_bytes(),
        raw_sha256: &sha,
        compile_configuration: "actual-compile",
        expected_compile_configuration: "actual-compile",
        entry: "f",
        contracts: &contracts,
        selected: None,
        dialect: CCompilerDialect::C11,
        graphs: &graphs,
        contexts: &contexts,
        owners: &owners,
        declarations: &declarations,
        expected: &expected,
        binding_manifest: BINDINGS.as_bytes(),
        formal_artifacts: &artifacts,
    };
    assert_eq!(
        reading_site_domain::check(&held, &expected)
            .unwrap()
            .receipt_ids()
            .len(),
        expected.len()
    );
    let checked = reading_site_domain::check(&held, &expected).unwrap();
    let mut other = contexts["f"].clone();
    other.artifact = "other.c#caller".into();
    let rows = vec![
        reading_site_domain::DependentRecord {
            id: "direct-control".into(),
            context: contexts["f"].clone(),
            declarations: declarations.clone(),
        },
        reading_site_domain::DependentRecord {
            id: "summary-other-context-control".into(),
            context: other.clone(),
            declarations: declarations.clone(),
        },
        reading_site_domain::DependentRecord {
            id: "unrelated".into(),
            context: contexts["f"].clone(),
            declarations: BTreeSet::new(),
        },
    ];
    for kind in [
        reading_site_domain::FailureKind::SourceMismatch,
        reading_site_domain::FailureKind::RefutedPrerequisite,
        reading_site_domain::FailureKind::FinitePredicateCounterexample,
    ] {
        let impact = serde_json::to_value(
            checked
                .impact(&expected[0].id, &contexts["f"], kind, &rows)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            impact["affected_records"],
            serde_json::json!(["direct-control"])
        );
        assert_eq!(
            impact["dependent_other_contexts_unchanged"],
            serde_json::json!(["summary-other-context-control"])
        );
        assert_eq!(
            impact["unrelated_records"],
            serde_json::json!(["unrelated"])
        );
        assert_eq!(impact["verdict_mutations"], 0);
    }
    assert!(
        checked
            .impact(
                &expected[0].id,
                &other,
                reading_site_domain::FailureKind::SourceMismatch,
                &rows
            )
            .is_err()
    );
    let mut truncated = expected.clone();
    truncated.pop();
    assert!(reading_site_domain::check(&held, &truncated).is_err());
    held.expected = &truncated;
    assert!(
        reading_site_domain::check(&held, &truncated).is_err(),
        "fresh original reader detects denominator omission even if both claims agree"
    );
    held.expected = &expected;
    held.raw_source = b"changed";
    assert!(reading_site_domain::check(&held, &expected).is_err());
    held.raw_source = SOURCE.as_bytes();
    held.compile_configuration = "foreign";
    assert!(reading_site_domain::check(&held, &expected).is_err());
    held.compile_configuration = "actual-compile";
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
        let mut value = serde_json::to_value(&contexts["f"]).unwrap();
        value[key] = "foreign".into();
        let bad = BTreeMap::from([("f".into(), serde_json::from_value(value).unwrap())]);
        let bad_held = HeldDomain {
            contexts: &bad,
            ..held
        };
        assert!(
            reading_site_domain::check(&bad_held, &expected).is_err(),
            "{key}"
        );
    }
    held.contexts = &contexts;
    let mut wrong = expected.clone();
    wrong[0].observation.environment_sha256 = "foreign".into();
    held.expected = &wrong;
    assert!(reading_site_domain::check(&held, &wrong).is_err());
}
