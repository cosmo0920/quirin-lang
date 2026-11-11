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

use qcore_contract::profile_loader::{load_profile_directory, sha256_hex};
use qcore_schemas::mechanism::LibraryMechanismSummary;
use qcore_schemas::profile::{
    AuditedFile, LibraryProfile, ProfileManifest, SourcePin, SourceWitness, WitnessCheck,
};
use qcore_verification::external_source::{ExternalSourceBundle, derive_library_summary_profile};
use std::collections::{BTreeMap, BTreeSet};

const REVISION: &str = "0123456789abcdef0123456789abcdef01234567";
const SOURCE: &str = "void cache_retain(unsigned *count) { ++*count; }\n";

fn fixture() -> (LibraryProfile, ExternalSourceBundle) {
    let profile = LibraryProfile {
        pin: "checkout".into(),
        source_roots: vec![],
        audited_files: vec![AuditedFile {
            path: "src/cache.c".into(),
            sha256: sha256_hex(SOURCE.as_bytes()),
        }],
        source_witnesses: vec![SourceWitness {
            name: "reference-count".into(),
            file: "src/cache.c".into(),
            check: WitnessCheck::Digest {
                construct: "declared reference-count operation".into(),
            },
            mechanisms: BTreeSet::from([LibraryMechanismSummary::ReferenceCount]),
        }],
        permitted_mechanisms: BTreeSet::from([LibraryMechanismSummary::ReferenceCount]),
        operation_summaries: BTreeMap::new(),
        api_dispositions: BTreeMap::new(),
        accepted_builds: vec![],
        required_evidence: BTreeSet::new(),
        typed_import: None,
        callback_summaries: BTreeMap::new(),
        boolean_field_models: vec![],
        provider: None,
    };
    let bundle = ExternalSourceBundle {
        source_revision: REVISION.into(),
        files: BTreeMap::from([("src/cache.c".into(), SOURCE.into())]),
    };
    (profile, bundle)
}

fn derive(profile: &LibraryProfile, bundle: &ExternalSourceBundle) -> bool {
    derive_library_summary_profile("example.cache", profile, REVISION, bundle).is_ok()
}

#[test]
fn valid_declared_digest_and_substring_witnesses_keep_exact_inventory() {
    let (mut profile, bundle) = fixture();
    profile.source_witnesses.push(SourceWitness {
        name: "declaration".into(),
        file: "src/cache.c".into(),
        check: WitnessCheck::Substrings {
            needles: vec!["void cache_retain(unsigned *count)".into()],
            rationale: "test the additional exact declaration constraint".into(),
        },
        mechanisms: BTreeSet::new(),
    });
    let certificate =
        derive_library_summary_profile("example.cache", &profile, REVISION, &bundle).unwrap();
    assert_eq!(
        certificate.checked_files,
        BTreeSet::from(["src/cache.c".into()])
    );
    assert_eq!(
        certificate.checked_contracts,
        BTreeSet::from(["reference-count".into(), "declaration".into()])
    );
    assert_eq!(certificate.profile.summaries, profile.permitted_mechanisms);
}

#[test]
fn directly_constructed_digest_witness_requires_an_audited_file() {
    let (mut profile, mut bundle) = fixture();
    profile.audited_files.clear();
    assert!(
        !derive(&profile, &bundle),
        "a present unaudited file granted a mechanism"
    );
    bundle.files.clear();
    assert!(
        !derive(&profile, &bundle),
        "a missing required witness was silently skipped"
    );
}

#[test]
fn direct_profiles_reject_vacuous_or_undocumented_witness_checks() {
    let invalid = [
        WitnessCheck::Digest {
            construct: " \n\t".into(),
        },
        WitnessCheck::Substrings {
            needles: vec![],
            rationale: "declared".into(),
        },
        WitnessCheck::Substrings {
            needles: vec![String::new()],
            rationale: "declared".into(),
        },
        WitnessCheck::Substrings {
            needles: vec!["cache_retain".into()],
            rationale: " \t".into(),
        },
    ];
    for check in invalid {
        let (mut profile, bundle) = fixture();
        profile.source_witnesses[0].check = check;
        assert!(
            !derive(&profile, &bundle),
            "accepted {:?}",
            profile.source_witnesses[0].check
        );
    }
}

#[test]
fn direct_profiles_reject_ambiguous_or_malformed_audited_inventories() {
    let (original, bundle) = fixture();
    let mut duplicate_file = original.clone();
    duplicate_file
        .audited_files
        .push(original.audited_files[0].clone());
    assert!(!derive(&duplicate_file, &bundle));
    let mut duplicate_name = original.clone();
    duplicate_name
        .source_witnesses
        .push(original.source_witnesses[0].clone());
    assert!(!derive(&duplicate_name, &bundle));
    for path in ["../cache.c", "/cache.c", ""] {
        let mut profile = original.clone();
        profile.audited_files[0].path = path.into();
        profile.source_witnesses[0].file = path.into();
        let mut source = bundle.clone();
        source.files = BTreeMap::from([(path.into(), SOURCE.into())]);
        assert!(!derive(&profile, &source), "accepted path {path:?}");
    }
    let mut bad_digest = original;
    bad_digest.audited_files[0].sha256 = "not-a-digest".into();
    assert!(!derive(&bad_digest, &bundle));
}

#[test]
fn a_failed_required_witness_or_source_identity_never_returns_partial_evidence() {
    let (profile, bundle) = fixture();
    let mut missing = bundle.clone();
    missing.files.clear();
    assert!(!derive(&profile, &missing));
    let mut drifted = bundle.clone();
    drifted.files.insert(
        "src/cache.c".into(),
        "void cache_retain(unsigned *count) {}".into(),
    );
    assert!(!derive(&profile, &drifted));
    let mut revision = bundle.clone();
    revision.source_revision = "different".into();
    assert!(!derive(&profile, &revision));
    let mut failed = profile;
    failed.source_witnesses.push(SourceWitness {
        name: "mandatory-contract".into(),
        file: "src/cache.c".into(),
        check: WitnessCheck::Substrings {
            needles: vec!["not present in this file".into()],
            rationale: "additional required declaration".into(),
        },
        mechanisms: BTreeSet::new(),
    });
    assert!(!derive(&failed, &bundle));
}

#[test]
fn json_loader_and_direct_derivation_agree_on_witness_structure() {
    let (valid, bundle) = fixture();
    let mut profiles = vec![valid.clone()];
    let mut unaudited = valid.clone();
    unaudited.audited_files.clear();
    profiles.push(unaudited);
    let mut duplicate = valid.clone();
    duplicate
        .source_witnesses
        .push(valid.source_witnesses[0].clone());
    profiles.push(duplicate);
    let mut empty = valid;
    empty.source_witnesses[0].check = WitnessCheck::Substrings {
        needles: vec![],
        rationale: "documented".into(),
    };
    profiles.push(empty);

    let parent = std::env::temp_dir().join(format!(
        "qcore-source-profile-boundary-{}",
        std::process::id()
    ));
    let directory = parent.join("example-cache");
    std::fs::create_dir_all(&directory).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(parent);
    for profile in profiles {
        let manifest = ProfileManifest {
            schema_version: 1,
            source_set: "example-cache".into(),
            pins: BTreeMap::from([(
                "checkout".into(),
                SourcePin {
                    git_commit: REVISION.into(),
                    clean_paths: vec![],
                    description: "test checkout".into(),
                },
            )]),
            libraries: BTreeMap::from([("example.cache".into(), profile.clone())]),
            resource_contract_manifest: None,
            provider_manifests: vec![],
        };
        std::fs::write(
            directory.join("profile.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(
            load_profile_directory(&directory).is_ok(),
            derive(&profile, &bundle)
        );
    }
}
