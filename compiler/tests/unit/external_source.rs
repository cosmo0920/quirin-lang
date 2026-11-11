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

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::LazyLock;

use qcore_contract::profile_loader::LoadedProfile;
use qcore_schemas::profile::{LibraryProfile, WitnessCheck};

use super::*;
use crate::external_integration::LibraryMechanismSummary;
use crate::source_profiles::{FLUENT_BIT_SOURCE_SET, load_profile};

/// The shipped `fluent-bit` source-set profile (W2). These tests exercise the
/// declared audited-file inventory and construct witnesses, not a fixture.
static PROFILE: LazyLock<LoadedProfile> =
    LazyLock::new(|| load_profile(FLUENT_BIT_SOURCE_SET).expect("fluent-bit profile loads"));

fn library(name: &str) -> LibraryProfile {
    let mut profile = PROFILE.library(name).expect("declared library").clone();
    for audited in &mut profile.audited_files {
        audited.sha256 = qcore_contract::profile_loader::sha256_hex(b"");
    }
    profile
}

fn revision(name: &str) -> &'static str {
    &PROFILE
        .pin_for(name)
        .expect("every declared library binds a pin")
        .git_commit
}

/// Synthetic empty-file fixture pinned by the test-only profile copy. This
/// exercises derivation, and is not evidence about the upstream source.
fn bundle(name: &str) -> ExternalSourceBundle {
    let profile = library(name);
    let mut files = BTreeMap::<String, String>::new();
    for audited in &profile.audited_files {
        files.entry(audited.path.clone()).or_default();
    }
    for witness in &profile.source_witnesses {
        if let WitnessCheck::Substrings { needles, .. } = &witness.check {
            let contents = files.entry(witness.file.clone()).or_default();
            for needle in needles {
                contents.push_str(needle);
                contents.push('\n');
            }
        }
    }
    ExternalSourceBundle {
        source_revision: revision(name).to_owned(),
        files,
    }
}

#[test]
fn derives_only_the_audited_librdkafka_mechanisms() {
    let certificate = derive_library_summary_profile(
        "librdkafka",
        &library("librdkafka"),
        revision("librdkafka"),
        &bundle("librdkafka"),
    )
    .unwrap();
    assert_eq!(certificate.checked_files.len(), 3);
    assert_eq!(certificate.checked_contracts.len(), 3);
    assert!(
        certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::ReferenceCount)
    );
    assert!(
        certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::TypedQueue)
    );
    assert!(
        certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::ThreadHandoff)
    );
    assert!(
        !certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::AtomicPublication)
    );
    assert!(
        !certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::Reclamation)
    );
}

#[test]
fn derives_cfl_arena_and_chunkio_nested_teardown_profiles() {
    let cfl =
        derive_library_summary_profile("cfl", &library("cfl"), revision("cfl"), &bundle("cfl"))
            .unwrap();
    assert_eq!(
        cfl.profile.summaries,
        BTreeSet::from([
            LibraryMechanismSummary::IntrusiveList,
            LibraryMechanismSummary::Arena,
            LibraryMechanismSummary::CascadingDestructor,
        ])
    );

    let chunk = derive_library_summary_profile(
        "chunkio",
        &library("chunkio"),
        revision("chunkio"),
        &bundle("chunkio"),
    )
    .unwrap();
    assert_eq!(
        chunk.profile.summaries,
        BTreeSet::from([
            LibraryMechanismSummary::IntrusiveList,
            LibraryMechanismSummary::CascadingDestructor,
        ])
    );
}

#[test]
fn source_profiles_fail_closed_on_revision_file_or_contract_drift() {
    let mut source = bundle("librdkafka");
    source.source_revision = "unpinned".into();
    assert!(
        derive_library_summary_profile(
            "librdkafka",
            &library("librdkafka"),
            revision("librdkafka"),
            &source
        )
        .unwrap_err()
        .contains(&LibrarySourceDiagnostic::SourceRevisionMismatch)
    );

    let missing = library("cfl").audited_files[0].path.clone();
    let mut source = bundle("cfl");
    source.files.remove(&missing);
    assert!(
        derive_library_summary_profile("cfl", &library("cfl"), revision("cfl"), &source)
            .unwrap_err()
            .contains(&LibrarySourceDiagnostic::MissingFile(missing))
    );
}

/// A substring witness whose needles are absent rejects the whole library
/// certificate, exactly as the pre-W2 fragment tables did.
#[test]
fn substring_witnesses_still_reject_the_library_on_drift() {
    let mut profile = library("librdkafka").clone();
    profile.source_witnesses[0].check = WitnessCheck::Substrings {
        needles: vec!["static RD_INLINE RD_UNUSED int rd_refcnt_add0".to_owned()],
        rationale: "test".to_owned(),
    };
    let source = bundle("librdkafka");
    let diagnostics =
        derive_library_summary_profile("librdkafka", &profile, revision("librdkafka"), &source)
            .unwrap_err();
    assert!(
        diagnostics.contains(&LibrarySourceDiagnostic::ContractMismatch {
            file: profile.source_witnesses[0].file.clone(),
            contract: profile.source_witnesses[0].name.clone(),
        })
    );
}

#[test]
fn a_public_bundle_cannot_claim_digest_verification() {
    let profile = PROFILE.library("librdkafka").unwrap();
    let diagnostics = derive_library_summary_profile(
        "librdkafka",
        profile,
        revision("librdkafka"),
        &bundle("librdkafka"),
    )
    .unwrap_err();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic, LibrarySourceDiagnostic::DigestMismatch { .. }))
    );
}
