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

//! Source-derived profiles for pinned external-library custody summaries.
//!
//! W2 (`docs/GENERALIZATION_PLAN.md`, design
//! `docs/design/W2_PROFILE_MANIFEST_SCHEMA.md`) moved this module's pinned
//! path constants and `contents.contains` fragment tables into the data
//! profiles under `contract/profiles/<source-set>/`. What stays here is the
//! derivation rule, which is unchanged: the pinned revision must match, every
//! audited file must be present, and every declared construct witness must
//! pass before any mechanism summary is granted. A failed witness rejects the
//! whole library certificate, exactly as the hardcoded contract tables did.
//!
//! Source-witness structure and digests are checked again at this public
//! boundary: callers may construct both `LibraryProfile` and
//! `ExternalSourceBundle` directly, so loader provenance cannot be assumed.

use std::collections::BTreeSet;

use qcore_contract::profile_loader::{ProfileDiagnostic, validate_library_source_profile};

use qcore_schemas::profile::{LibraryProfile, WitnessCheck};

use crate::library_import::LibrarySummaryProfile;

pub use qcore_schemas::identity::ExternalSourceBundle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySourceCertificate {
    pub profile: LibrarySummaryProfile,
    pub checked_files: BTreeSet<String>,
    pub checked_contracts: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LibrarySourceDiagnostic {
    InvalidProfile(ProfileDiagnostic),
    SourceRevisionMismatch,
    MissingFile(String),
    DigestMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    ContractMismatch {
        file: String,
        contract: String,
    },
}

/// Derive a library's granted mechanism summaries from its declared profile
/// and an audited source bundle. `expected_revision` is the pin the profile
/// binds the library to; a mismatch is fail-closed, never a warning.
pub fn derive_library_summary_profile(
    library: &str,
    profile: &LibraryProfile,
    expected_revision: &str,
    source: &ExternalSourceBundle,
) -> Result<LibrarySourceCertificate, Vec<LibrarySourceDiagnostic>> {
    validate_library_source_profile(library, profile)
        .map_err(|error| vec![LibrarySourceDiagnostic::InvalidProfile(error)])?;
    let mut diagnostics = BTreeSet::new();
    if source.source_revision != expected_revision {
        diagnostics.insert(LibrarySourceDiagnostic::SourceRevisionMismatch);
    }
    for audited in &profile.audited_files {
        match source.files.get(&audited.path) {
            None => {
                diagnostics.insert(LibrarySourceDiagnostic::MissingFile(audited.path.clone()));
            }
            Some(contents) => {
                let actual = qcore_contract::profile_loader::sha256_hex(contents.as_bytes());
                if actual != audited.sha256 {
                    diagnostics.insert(LibrarySourceDiagnostic::DigestMismatch {
                        file: audited.path.clone(),
                        expected: audited.sha256.clone(),
                        actual,
                    });
                }
            }
        }
    }

    let mut summaries = BTreeSet::new();
    let mut checked_contracts = BTreeSet::new();
    for witness in &profile.source_witnesses {
        let Some(contents) = source.files.get(&witness.file) else {
            diagnostics.insert(LibrarySourceDiagnostic::MissingFile(witness.file.clone()));
            continue;
        };
        let established = match &witness.check {
            // The file's digest is rechecked at this boundary above, so
            // the construct this witness names is established by identity.
            WitnessCheck::Digest { .. } => true,
            WitnessCheck::Substrings { needles, .. } => {
                needles.iter().all(|needle| contents.contains(needle))
            }
        };
        if established {
            summaries.extend(witness.mechanisms.iter().copied());
            checked_contracts.insert(witness.name.clone());
        } else {
            diagnostics.insert(LibrarySourceDiagnostic::ContractMismatch {
                file: witness.file.clone(),
                contract: witness.name.clone(),
            });
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics.into_iter().collect());
    }

    Ok(LibrarySourceCertificate {
        profile: LibrarySummaryProfile {
            source_revision: source.source_revision.clone(),
            library: library.to_owned(),
            summaries,
        },
        checked_files: profile
            .audited_files
            .iter()
            .map(|audited| audited.path.clone())
            .collect(),
        checked_contracts,
    })
}
