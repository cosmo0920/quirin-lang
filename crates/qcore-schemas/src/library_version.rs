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

//! Library version ranges of declared contracts (card
//! **OPENSSL-PROVIDER-PLUGIN**, user decisions of 2026-11-16; contract spec
//! §C06.2, "Library version ranges").
//!
//! A declaration about an external library's function may state the library
//! versions it holds for, with the version its official documentation says
//! the function is supported since and, when the documentation records one,
//! the version that removed it:
//!
//! ```text
//! requires resource_library_version(openssl, since "1.1.0");
//! requires resource_library_version(openssl, since "1.1.0", removed "4.0.0");
//! requires resource_library_version(openssl, documented "1.0.2");
//! ```
//!
//! `documented` is the lower bound when the documentation records no
//! HISTORY entry for the function: it is the oldest documentation series the
//! author checked that documents the function, never an invented `since`.
//! Both read as an inclusive lower bound; the removal is exclusive.
//!
//! The version a run is checked against is the one the audited build's own
//! headers report, read by the audit (`qcore_audit::library_version`) and
//! pinned in the crate's lock by `quirin-audit pin`. A contract is **in
//! force** only when that version is readable, unambiguous, equal to the
//! pinned one and within the range ([`LibraryVersionBound::admits`]). The
//! Lean model is `QCore.LibraryVersionRange`.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A library version: three numeric components, compared lexicographically.
/// A written version with two components has patch 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LibraryVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl LibraryVersion {
    /// Parse `MAJOR.MINOR` or `MAJOR.MINOR.PATCH` (decimal components, no
    /// sign, no leading `+`, no suffix).
    ///
    /// # Errors
    ///
    /// Any other spelling.
    pub fn parse(text: &str) -> Result<Self, String> {
        let parts = text.split('.').collect::<Vec<_>>();
        let number = |part: &str| -> Result<u32, String> {
            if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(format!(
                    "`{text}` is not a library version (MAJOR.MINOR[.PATCH])"
                ));
            }
            part.parse::<u32>()
                .map_err(|_| format!("`{text}` has a component out of range"))
        };
        match parts.as_slice() {
            [major, minor] => Ok(Self {
                major: number(major)?,
                minor: number(minor)?,
                patch: 0,
            }),
            [major, minor, patch] => Ok(Self {
                major: number(major)?,
                minor: number(minor)?,
                patch: number(patch)?,
            }),
            _ => Err(format!(
                "`{text}` is not a library version (MAJOR.MINOR[.PATCH])"
            )),
        }
    }
}

impl fmt::Display for LibraryVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// How the lower bound was established.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LowerBoundSource {
    /// The documentation's HISTORY section names the version that added the
    /// function (`since`).
    Since,
    /// The documentation records no HISTORY entry; the bound is the oldest
    /// documentation series checked that documents the function
    /// (`documented`, user decision 2026-11-16 "Documented-present").
    Documented,
}

impl LowerBoundSource {
    /// The clause keyword.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Since => "since",
            Self::Documented => "documented",
        }
    }
}

/// One declaration's library version range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryVersionBound {
    /// The library, an identifier naming an entry of the declaring crate's
    /// `library-versions.json` (the version source).
    pub library: String,
    /// The inclusive lower bound.
    pub lower: LibraryVersion,
    pub lower_source: LowerBoundSource,
    /// The exclusive upper bound: the version that removed the function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub removed: Option<LibraryVersion>,
}

impl LibraryVersionBound {
    /// Whether `version` lies in the range: at or above the lower bound and,
    /// when a removal is stated, below it.
    #[must_use]
    pub fn admits(&self, version: LibraryVersion) -> bool {
        self.lower <= version && self.removed.is_none_or(|removed| version < removed)
    }

    /// The clause as written (normalised).
    #[must_use]
    pub fn clause(&self) -> String {
        let removed = self
            .removed
            .map(|removed| format!(", removed \"{removed}\""))
            .unwrap_or_default();
        format!(
            "resource_library_version({}, {} \"{}\"{removed})",
            self.library,
            self.lower_source.keyword(),
            self.lower
        )
    }
}

/// Why a declaration carrying a library version range is not in force at the
/// audited version. Every reason but [`Self::Changed`] is a decision of the
/// tool, so the calls it holds are `Unknown`; a version that differs from the
/// pinned one is resolved by a person's review and re-pin, so its calls are
/// `Pending` (`contract-pin-mismatch`), the user's split of 2026-11-16.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LibraryVersionFailure {
    /// The audited headers report a version other than the lock's pinned one
    /// (or the lock pins none for the library).
    Changed,
    /// The version is outside the declaration's range.
    OutOfRange,
    /// The version source is missing or the headers do not report a version.
    Unreadable,
    /// The audited build's include configurations report more than one
    /// version or header.
    Ambiguous,
}

impl LibraryVersionFailure {
    /// The stable reason key carried by every held call.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Changed => "library-version-changed",
            Self::OutOfRange => "library-version-out-of-range",
            Self::Unreadable => "library-version-unreadable",
            Self::Ambiguous => "library-version-ambiguous",
        }
    }

    /// Whether a person resolves it (review and re-pin), so a call held for
    /// it is `Pending`; otherwise the call is `Unknown`.
    #[must_use]
    pub const fn is_human_resolvable(self) -> bool {
        matches!(self, Self::Changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> LibraryVersion {
        LibraryVersion::parse(text).unwrap()
    }

    #[test]
    fn versions_parse_and_order() {
        assert_eq!(
            v("3.5.5"),
            LibraryVersion {
                major: 3,
                minor: 5,
                patch: 5
            }
        );
        assert_eq!(v("1.1"), v("1.1.0"));
        assert!(v("1.1.0") < v("1.1.1"));
        assert!(v("1.10.0") > v("1.9.9"));
        for bad in ["", "1", "1.", "1.1.1.1", "v1.1", "1.1a", "1.-1", "+1.0"] {
            assert!(LibraryVersion::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_range_admits_exactly_its_versions() {
        let bound = LibraryVersionBound {
            library: "openssl".into(),
            lower: v("1.1.0"),
            lower_source: LowerBoundSource::Since,
            removed: Some(v("4.0.0")),
        };
        assert!(bound.admits(v("1.1.0")));
        assert!(bound.admits(v("3.5.5")));
        assert!(!bound.admits(v("1.0.2")));
        assert!(!bound.admits(v("4.0.0")));
        assert_eq!(
            bound.clause(),
            "resource_library_version(openssl, since \"1.1.0\", removed \"4.0.0\")"
        );
        let open = LibraryVersionBound {
            removed: None,
            lower_source: LowerBoundSource::Documented,
            ..bound
        };
        assert!(open.admits(v("99.0.0")));
        assert!(LibraryVersionFailure::Changed.is_human_resolvable());
        assert!(!LibraryVersionFailure::OutOfRange.is_human_resolvable());
        assert!(!LibraryVersionFailure::Unreadable.is_human_resolvable());
        assert!(!LibraryVersionFailure::Ambiguous.is_human_resolvable());
    }
}
