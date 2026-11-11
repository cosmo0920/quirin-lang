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

//! W2 profile-manifest loading: the design's §3 fail-closed table, the
//! shipped profiles, and the guards that keep the sites still holding Rust
//! tables from drifting away from the data.
//!
//! Every rejection here ends in Unknown at the call site: a rejected profile
//! grants no mechanism, classifies no symbol as accepted and admits no build.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use qcore_compiler::profile_loader::{
    FileAuditDiagnostic, LoadedProfile, PinDiagnostic, ProfileDiagnostic, load_audited_bundle,
    load_profile_directory, load_source_set_profile, sha256_hex, verify_pin,
};
use qcore_compiler::source_profiles::{
    FLUENT_BIT_SOURCE_SET, LIBEVENT_SOURCE_SET, load_profile, profiles_root,
};
use qcore_compiler::verifier_plugins::libevent::LIBEVENT_SUBMODULE_PATH;
use qcore_compiler::{PINNED_FLUENT_BIT_REVISION, ProfileDisposition, classify_libevent_symbol};
use qcore_schemas::profile::{
    ApiDisposition, PROFILE_MANIFEST_VERSION, SUPPORTED_PROFILE_MANIFEST_VERSIONS, WitnessCheck,
};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler crate must be inside the repository")
        .to_path_buf()
}

/// Precondition for the tests that read the Libevent checkout itself. In a
/// fresh worktree the submodule directory exists but is empty, and `git -C
/// <dir> rev-parse HEAD` then answers from the *superproject*, so the pin
/// check would report a revision mismatch against whatever commit this
/// repository is on — a misleading invariant-5 alarm for a checkout that is
/// merely uninitialised (residual `DOCS-COMPACT-3-WORKTREE-SUBMODULE`,
/// `docs/reports/docs-compact-3/README.md` §11.1). The test still fails in
/// that state; only the message changes. The pin assertion itself is
/// untouched: an initialised checkout on the wrong revision reports
/// `SourceRevisionMismatch` exactly as before.
fn initialized_libevent_checkout() -> PathBuf {
    let checkout = repository_root().join(LIBEVENT_SUBMODULE_PATH);
    assert!(
        checkout.join(".git").exists(),
        "the `{LIBEVENT_SUBMODULE_PATH}` submodule at {} is not initialized; \
         run `git submodule update --init --recursive` first",
        checkout.display()
    );
    checkout
}

/// Write a mutated copy of a shipped profile into a scratch directory whose
/// name is the source set, so the directory-name rule still applies.
fn scratch_profile(
    name: &str,
    source_set: &str,
    mutate: impl Fn(&mut serde_json::Value),
) -> PathBuf {
    let directory = std::env::temp_dir()
        .join("qcore-w2-profile-tests")
        .join(name)
        .join(source_set);
    let _ = fs::remove_dir_all(directory.parent().expect("scratch parent"));
    fs::create_dir_all(directory.join("providers")).expect("scratch directory");
    let original = profiles_root().join(LIBEVENT_SOURCE_SET);
    let mut document: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(original.join("profile.json")).expect("profile"))
            .expect("valid JSON");
    mutate(&mut document);
    fs::write(
        directory.join("profile.json"),
        serde_json::to_string_pretty(&document).expect("serializable"),
    )
    .expect("write profile");
    fs::copy(
        original.join("providers/qcore.libevent.json"),
        directory.join("providers/qcore.libevent.json"),
    )
    .expect("copy provider manifest");
    directory
}

// ---------------------------------------------------------------- digests

#[test]
fn sha256_matches_the_published_test_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    // A multi-block input exercises the length padding across a 64-byte edge.
    assert_eq!(
        sha256_hex(&[b'a'; 1_000_000]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

// ------------------------------------------------------- shipped profiles

#[test]
fn the_shipped_profiles_load_and_are_internally_consistent() {
    for source_set in [FLUENT_BIT_SOURCE_SET, LIBEVENT_SOURCE_SET] {
        let profile = load_profile(source_set).unwrap_or_else(|error| {
            panic!("{source_set} profile must load: {error}");
        });
        // Both shipped profiles are byte-identical version-1 manifests. G03-F
        // defined version 2 (the checkout-root source root) without migrating
        // them: neither names a root spelling version 1 refuses, so moving
        // them would change bytes without changing meaning. The exact declared
        // version stays pinned so a silent migration is a test failure.
        assert_eq!(profile.manifest().schema_version, 1);
        assert!(
            SUPPORTED_PROFILE_MANIFEST_VERSIONS.contains(&profile.manifest().schema_version),
            "{source_set} declares an unsupported version"
        );
        assert_eq!(profile.manifest().source_set, source_set);
        assert_eq!(profile.manifest_digest().len(), 64);
        assert_eq!(
            profile.assumptions().get("source_set").map(String::as_str),
            Some(source_set)
        );
        for library in profile.manifest().libraries.values() {
            assert!(profile.manifest().pins.contains_key(&library.pin));
        }
    }
    assert_eq!(SUPPORTED_PROFILE_MANIFEST_VERSIONS, &[1, 2]);
    assert_eq!(PROFILE_MANIFEST_VERSION, 2);
}

#[test]
fn declared_pins_match_the_checkouts_and_the_retained_rust_constant() {
    let fluent_bit = load_profile(FLUENT_BIT_SOURCE_SET).expect("fluent-bit profile loads");
    // The ChunkIO provider modules still carry their own revision constant
    // (not migrated by W2); the data and that constant must not drift apart.
    assert_eq!(
        fluent_bit.manifest().pins[FLUENT_BIT_SOURCE_SET].git_commit,
        PINNED_FLUENT_BIT_REVISION
    );

    let libevent = load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads");
    let pin = libevent
        .pin_for(LIBEVENT_SOURCE_SET)
        .expect("libevent binds a pin");
    let checkout = initialized_libevent_checkout();
    assert_eq!(
        verify_pin(LIBEVENT_SOURCE_SET, pin, &checkout).expect("the submodule matches the pin"),
        pin.git_commit
    );
}

#[test]
fn the_libevent_api_table_reproduces_the_pre_w2_dispositions() {
    let profile = load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads");
    let library = profile
        .library(LIBEVENT_SOURCE_SET)
        .expect("libevent library");
    let dispositions = &library.api_dispositions;
    // The pre-W2 `LIBEVENT_API_PROFILE` held 45 rows: 37 accepted and 8
    // explicitly Unknown. Pinning the shape keeps a silent widening visible.
    assert_eq!(dispositions.len(), 45);
    let accepted = dispositions
        .values()
        .filter(|disposition| matches!(disposition, ApiDisposition::Accepted))
        .count();
    assert_eq!(accepted, 37);
    for symbol in [
        "event_base_new",
        "event_del_noblock",
        "evutil_closesocket",
        "evutil_socket_geterror",
        "evutil_socketpair",
    ] {
        assert_eq!(
            classify_libevent_symbol(dispositions, symbol),
            ProfileDisposition::Accepted
        );
    }
    for symbol in [
        "evdns_base_new",
        "evhttp_new",
        "evrpc_init",
        "bufferevent_openssl_socket_new",
        "evconnlistener_new",
        "event_set_mem_functions",
        "evthread_set_lock_callbacks",
        "event_reinit",
        // Absence classifies exactly like an explicit Unknown.
        "private_event_magic",
    ] {
        assert_eq!(
            classify_libevent_symbol(dispositions, symbol),
            ProfileDisposition::Unknown
        );
    }
    // No entry may be represented as anything other than model coverage.
    for (symbol, disposition) in dispositions {
        if let ApiDisposition::Unknown { reason } = disposition {
            assert!(!reason.trim().is_empty(), "{symbol} has an empty reason");
        }
    }
}

/// Design Q3: the H1 fragment tables became digest witnesses; only the
/// Libevent public-header declarations keep substring needles, and each needs
/// a rationale.
#[test]
fn only_public_header_witnesses_keep_substring_needles() {
    let fluent_bit = load_profile(FLUENT_BIT_SOURCE_SET).expect("fluent-bit profile loads");
    for (name, library) in &fluent_bit.manifest().libraries {
        for witness in &library.source_witnesses {
            assert!(
                matches!(witness.check, WitnessCheck::Digest { .. }),
                "{name}/{} must be a digest witness",
                witness.name
            );
        }
    }

    let libevent = load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads");
    let library = libevent
        .library(LIBEVENT_SOURCE_SET)
        .expect("libevent library");
    let mut needled = BTreeSet::new();
    for witness in &library.source_witnesses {
        if let WitnessCheck::Substrings { needles, rationale } = &witness.check {
            assert!(!needles.is_empty() && !rationale.trim().is_empty());
            needled.insert(witness.file.clone());
        }
    }
    assert_eq!(
        needled,
        BTreeSet::from([
            "include/event2/event.h".to_owned(),
            "include/event2/thread.h".to_owned(),
            "include/event2/util.h".to_owned(),
        ])
    );
}

/// H5 guard: the ChunkIO Boolean field importer still holds its own
/// `ChunkSourceField` pairs in Rust (not migrated by W2). The declared models
/// must stay exactly those pairs so the data cannot drift.
#[test]
fn declared_boolean_field_models_match_the_retained_rust_pairs() {
    let profile = load_profile(FLUENT_BIT_SOURCE_SET).expect("fluent-bit profile loads");
    let declared = profile
        .manifest()
        .libraries
        .values()
        .flat_map(|library| &library.boolean_field_models)
        .map(|model| (model.record.clone(), model.field.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        declared,
        BTreeSet::from([
            ("cio_chunk".to_owned(), "lock".to_owned()),
            ("flb_input_chunk".to_owned(), "busy".to_owned()),
        ])
    );
}

/// H6 parity artifact: the on-disk provider manifests are exactly the
/// first-party manifests previously built from Rust literals. The frozen
/// pre-discovery archive makes this an independent parity expectation.
#[test]
fn on_disk_provider_manifests_equal_the_archived_pre_discovery_manifests() {
    let mut on_disk = Vec::new();
    for source_set in [FLUENT_BIT_SOURCE_SET, LIBEVENT_SOURCE_SET] {
        let profile = load_profile(source_set).expect("profile loads");
        on_disk.extend(profile.providers().iter().cloned());
    }
    on_disk.sort_by(|left, right| left.plugin_id.as_str().cmp(right.plugin_id.as_str()));

    let mut literals: Vec<qcore_compiler::PluginManifest> = serde_json::from_str(include_str!(
        "../../docs/reports/w2-provider-discovery/baseline-manifests.json"
    ))
    .expect("archived literal manifests");
    literals.sort_by(|left, right| left.plugin_id.as_str().cmp(right.plugin_id.as_str()));

    assert_eq!(on_disk, literals);
}

/// Every library that binds a provider must agree with that provider's own
/// declared theorem family, and unbound libraries stay unbound.
#[test]
fn provider_bindings_agree_with_their_manifests() {
    let mut bound = BTreeMap::new();
    for source_set in [FLUENT_BIT_SOURCE_SET, LIBEVENT_SOURCE_SET] {
        let profile = load_profile(source_set).expect("profile loads");
        for (name, library) in &profile.manifest().libraries {
            if let Some(binding) = &library.provider {
                let manifest = profile
                    .providers()
                    .iter()
                    .find(|manifest| manifest.plugin_id.as_str() == binding.plugin_id)
                    .expect("a bound provider is declared");
                assert_eq!(manifest.lean_theorem_family, binding.lean_theorem_family);
                bound.insert(name.clone(), binding.plugin_id.clone());
            }
        }
    }
    assert_eq!(
        bound.get("librdkafka").map(String::as_str),
        Some("qcore.librdkafka")
    );
    // CFL and the input-chunk wrapper have no first-party provider; nothing
    // invents one for them.
    assert!(!bound.contains_key("cfl"));
    assert!(!bound.contains_key("fluent_bit"));
}

// ------------------------------------------------ fail-closed table (F1-F10)

#[test]
fn f1_absent_profile_is_not_found() {
    assert_eq!(
        load_source_set_profile(&profiles_root(), "no-such-source-set").unwrap_err(),
        ProfileDiagnostic::ProfileNotFound {
            source_set: "no-such-source-set".to_owned()
        }
    );
    // A traversal attempt is a not-found, never an escape.
    assert!(matches!(
        load_source_set_profile(&profiles_root(), "../libevent").unwrap_err(),
        ProfileDiagnostic::ProfileNotFound { .. }
    ));
}

#[test]
fn f3_unknown_schema_version_rejects_the_whole_profile() {
    // The probe moved from 2 to 3 when G03-F defined version 2. A version this
    // loader does not define still rejects the whole profile; the supported
    // set is reported verbatim rather than negotiated.
    let directory = scratch_profile("version", LIBEVENT_SOURCE_SET, |document| {
        document["schema_version"] = serde_json::json!(3);
    });
    assert_eq!(
        load_profile_directory(&directory).unwrap_err(),
        ProfileDiagnostic::UnsupportedSchemaVersion {
            found: 3,
            supported: vec![1, 2],
        }
    );
}

#[test]
fn f4_unknown_fields_reject_the_whole_profile() {
    let directory = scratch_profile("unknown-field", LIBEVENT_SOURCE_SET, |document| {
        document["libraries"]["libevent"]["accept_everything"] = serde_json::json!(true);
    });
    assert!(matches!(
        load_profile_directory(&directory).unwrap_err(),
        ProfileDiagnostic::ProfileUnreadable { .. }
    ));
}

#[test]
fn f5_broken_internal_references_reject_the_whole_profile() {
    let unknown_pin = scratch_profile("pin-reference", LIBEVENT_SOURCE_SET, |document| {
        document["libraries"]["libevent"]["pin"] = serde_json::json!("not-a-pin");
    });
    assert!(matches!(
        load_profile_directory(&unknown_pin).unwrap_err(),
        ProfileDiagnostic::ProfileInconsistent { .. }
    ));

    let unaudited = scratch_profile("witness-file", LIBEVENT_SOURCE_SET, |document| {
        document["libraries"]["libevent"]["source_witnesses"][0]["file"] =
            serde_json::json!("not/audited.c");
    });
    assert!(matches!(
        load_profile_directory(&unaudited).unwrap_err(),
        ProfileDiagnostic::ProfileInconsistent { .. }
    ));

    let renamed = scratch_profile("source-set", "libevent-renamed", |_document| {});
    assert!(matches!(
        load_profile_directory(&renamed).unwrap_err(),
        ProfileDiagnostic::ProfileInconsistent { .. }
    ));

    let undeclared = scratch_profile("provider-binding", LIBEVENT_SOURCE_SET, |document| {
        document["libraries"]["libevent"]["provider"]["plugin_id"] =
            serde_json::json!("qcore.absent");
    });
    assert!(matches!(
        load_profile_directory(&undeclared).unwrap_err(),
        ProfileDiagnostic::ProfileInconsistent { .. }
    ));
}

#[test]
fn f6_substring_needles_without_a_rationale_reject_the_whole_profile() {
    let directory = scratch_profile("rationale", LIBEVENT_SOURCE_SET, |document| {
        document["libraries"]["libevent"]["source_witnesses"][0]["check"]["substrings"]["rationale"] =
            serde_json::json!("   ");
    });
    assert!(matches!(
        load_profile_directory(&directory).unwrap_err(),
        ProfileDiagnostic::NeedleWithoutRationale { .. }
    ));

    let empty = scratch_profile("needles", LIBEVENT_SOURCE_SET, |document| {
        document["libraries"]["libevent"]["source_witnesses"][0]["check"]["substrings"]["needles"] =
            serde_json::json!([]);
    });
    assert!(matches!(
        load_profile_directory(&empty).unwrap_err(),
        ProfileDiagnostic::NeedleWithoutRationale { .. }
    ));
}

#[test]
fn f7_and_f8_pin_failures_reject_the_checkout() {
    let profile = load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads");
    let mut pin = profile
        .pin_for(LIBEVENT_SOURCE_SET)
        .expect("libevent binds a pin")
        .clone();
    let checkout = repository_root().join(LIBEVENT_SUBMODULE_PATH);
    pin.git_commit = "0".repeat(40);
    assert!(matches!(
        verify_pin(LIBEVENT_SOURCE_SET, &pin, &checkout).unwrap_err(),
        PinDiagnostic::SourceRevisionMismatch { .. }
    ));
    // git is not available in an empty scratch directory: fail-closed, never
    // "the checkout is fine".
    let empty = std::env::temp_dir().join("qcore-w2-not-a-checkout");
    fs::create_dir_all(&empty).expect("scratch directory");
    assert!(verify_pin(LIBEVENT_SOURCE_SET, &pin, &empty).is_err());
}

#[test]
fn f9_and_f10_missing_files_and_digest_drift_reject_the_inventory() {
    let profile = load_profile(LIBEVENT_SOURCE_SET).expect("libevent profile loads");
    let library = profile
        .library(LIBEVENT_SOURCE_SET)
        .expect("libevent library");
    let revision = &profile
        .pin_for(LIBEVENT_SOURCE_SET)
        .expect("libevent pin")
        .git_commit;
    let checkout = initialized_libevent_checkout();
    let bundle = load_audited_bundle(library, &checkout, revision).expect("the checkout audits");
    assert_eq!(bundle.files.len(), library.audited_files.len());

    let mut drifted = library.clone();
    drifted.audited_files[0].sha256 = "0".repeat(64);
    let diagnostics = load_audited_bundle(&drifted, &checkout, revision).unwrap_err();
    assert!(matches!(
        diagnostics.as_slice(),
        [FileAuditDiagnostic::DigestMismatch { .. }]
    ));

    let mut absent = library.clone();
    absent.audited_files[0].path = "not/present.c".to_owned();
    let diagnostics = load_audited_bundle(&absent, &checkout, revision).unwrap_err();
    assert!(matches!(
        diagnostics.as_slice(),
        [FileAuditDiagnostic::MissingFile { .. }]
    ));
}

/// The merged operation table is total-or-reject, and unknown libraries never
/// contribute a default entry.
#[test]
fn operation_summaries_merge_without_conflict_and_reject_unknown_libraries() {
    let profile: LoadedProfile = load_profile(FLUENT_BIT_SOURCE_SET).expect("profile loads");
    let merged = profile
        .merged_operation_summaries(&["chunkio", "fluent_bit"])
        .expect("no conflict");
    assert_eq!(merged.len(), 17);
    assert!(matches!(
        profile
            .merged_operation_summaries(&["chunkio", "no-such-library"])
            .unwrap_err(),
        ProfileDiagnostic::ProfileInconsistent { .. }
    ));
}

#[test]
fn unknown_forbidden_build_features_never_admit_a_build() {
    use qcore_compiler::LibeventBuildProfile;
    let profile = load_profile(LIBEVENT_SOURCE_SET).unwrap();
    let mut selectors = profile
        .library(LIBEVENT_SOURCE_SET)
        .unwrap()
        .accepted_builds
        .clone();
    for selector in &mut selectors {
        selector.forbid.insert("future_feature".into());
    }
    assert!(!LibeventBuildProfile::LINUX_EPOLL.accepted(&selectors));
    for selector in &mut selectors {
        selector.forbid.remove("future_feature");
        selector.forbid.insert("debug_mode".into());
    }
    assert!(!LibeventBuildProfile::LINUX_EPOLL.accepted(&selectors));
}

#[test]
fn the_build_matrix_matches_every_pre_w2_combination() {
    use qcore_compiler::{EventBackend, LibeventBuildProfile, ThreadMode};
    let profile = load_profile(LIBEVENT_SOURCE_SET).unwrap();
    let selectors = &profile
        .library(LIBEVENT_SOURCE_SET)
        .unwrap()
        .accepted_builds;
    for backend in [
        EventBackend::Epoll,
        EventBackend::Poll,
        EventBackend::Select,
        EventBackend::Kqueue,
        EventBackend::Iocp,
    ] {
        for threads in [
            ThreadMode::Disabled,
            ThreadMode::Pthreads,
            ThreadMode::WindowsThreads,
        ] {
            for openssl in [false, true] {
                for debug_mode in [false, true] {
                    let expected = matches!(
                        (backend, threads),
                        (
                            EventBackend::Epoll
                                | EventBackend::Poll
                                | EventBackend::Select
                                | EventBackend::Kqueue,
                            ThreadMode::Pthreads
                        ) | (
                            EventBackend::Select | EventBackend::Iocp,
                            ThreadMode::WindowsThreads
                        )
                    ) && !openssl;
                    assert_eq!(
                        LibeventBuildProfile {
                            backend,
                            threads,
                            openssl,
                            debug_mode
                        }
                        .accepted(selectors),
                        expected
                    );
                }
            }
        }
    }
}
