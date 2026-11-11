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

//! `PluginLimits::additional` is the manifest's declared limit-kind
//! vocabulary (card P02). It is a *declaration*: this crate validates its
//! shape, and the host's execution profile decides whether a kind is enforced
//! or the package is refused. These tests pin the shape rules and the
//! serialization parity that keeps existing manifests byte-identical.

use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::plugin::{
    DEFAULT_MAX_FRAME_BYTES, ManifestError, PluginEntrypoint, PluginId, PluginLimits,
    PluginManifest, PluginTrustClass, SupportedTarget, VerifierCapability,
};

fn manifest() -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new("example.limits").unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: BTreeSet::from([1]),
        entrypoint: PluginEntrypoint::Executable {
            path: "provider".into(),
        },
        package_digest: "a".repeat(64),
        targets: vec![SupportedTarget {
            library: "example".into(),
            revisions: BTreeSet::from(["source".into()]),
            profiles: BTreeSet::from(["default".into()]),
            dialects: BTreeSet::from(["c11".into()]),
            platforms: BTreeSet::from(["linux".into()]),
            backends: BTreeSet::from(["native".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ResourceContracts]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_claims".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}

#[test]
fn the_default_limit_declaration_is_empty_and_omitted_from_the_wire() {
    let limits = PluginLimits::default();
    assert!(limits.additional.is_empty());
    assert_eq!(limits.timeout_ms, 5_000);
    assert_eq!(limits.output_bytes, DEFAULT_MAX_FRAME_BYTES);
    // Byte parity: a manifest that declares no additional kind serializes
    // exactly as it did before the field existed.
    let wire = serde_json::to_string(&limits).unwrap();
    assert!(!wire.contains("additional"), "{wire}");
    let decoded: PluginLimits = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded, limits);
    // And a manifest written before the field existed still loads.
    let legacy = r#"{"timeout_ms":1,"memory_bytes":2,"output_bytes":3}"#;
    assert!(
        serde_json::from_str::<PluginLimits>(legacy)
            .unwrap()
            .additional
            .is_empty()
    );
}

#[test]
fn declared_kinds_round_trip_and_are_ordered() {
    let mut limits = PluginLimits::default();
    limits.additional.insert("max_processes".into(), 8);
    limits.additional.insert("cpu_seconds".into(), 3);
    let wire = serde_json::to_string(&limits).unwrap();
    assert!(
        wire.contains(r#""additional":{"cpu_seconds":3,"max_processes":8}"#),
        "{wire}"
    );
    assert_eq!(serde_json::from_str::<PluginLimits>(&wire).unwrap(), limits);
}

#[test]
fn malformed_or_duplicated_limit_kinds_are_rejected() {
    for kind in [
        "",
        "CPU_SECONDS",
        "cpu seconds",
        "cpu.seconds",
        "cpu-seconds",
        // A repeat of a mandatory field would be a second declaration of the
        // same kind; reject rather than silently prefer either one.
        "timeout_ms",
        "memory_bytes",
        "output_bytes",
    ] {
        let mut candidate = manifest();
        candidate.limits.additional = BTreeMap::from([(kind.to_owned(), 1)]);
        assert_eq!(
            candidate.validate(),
            Err(ManifestError::InvalidLimits),
            "accepted limit kind `{kind}`"
        );
    }
    let mut long = manifest();
    long.limits.additional = BTreeMap::from([("k".repeat(65), 1)]);
    assert_eq!(long.validate(), Err(ManifestError::InvalidLimits));

    let mut accepted = manifest();
    accepted.limits.additional = BTreeMap::from([
        ("cpu_seconds".to_owned(), 1),
        ("max_open_files".to_owned(), 64),
    ]);
    assert_eq!(accepted.validate(), Ok(()));
}

#[test]
fn an_unknown_manifest_field_is_still_rejected() {
    // `additional` is the only new key; the manifest and limits objects both
    // keep denying unknown fields, so a typo cannot become a silent no-op.
    let wire = r#"{"timeout_ms":1,"memory_bytes":2,"output_bytes":3,"additionals":{}}"#;
    assert!(serde_json::from_str::<PluginLimits>(wire).is_err());
}
