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

//! Revision modes of declared contracts (card **W45-CONTRACT-REVISION-MODE**,
//! contract spec §C06.2).
//!
//! A resource-v1 declaration, or one clause of it, may say how it relates to
//! revisions of the audited project:
//!
//! ```text
//! requires resource_revision(flexible);
//! ensures normal resource_revision(resource_released(p, nonnull), pinned);
//! ```
//!
//! **Pinned is the default** when nothing is written. A pinned contract is in
//! force only where the audited source matches the identity it was reviewed
//! against, which a generated lock file records (never the `.qcore` text); a
//! flexible contract binds by name and declared signature only. A contract is
//! applied at a call site as one conjunction, so it is flexible only when
//! every clause is: one pinned clause (the unmarked selector of a contract
//! with no flexible marker included) pins the whole contract.
//!
//! These are the data types only. The loader
//! (`qcore_contract::contract_loader`) reads the markers, and the audit
//! (`qcore_audit::contract_revision`) compares pinned identities with their
//! lock and withholds every mismatch through
//! [`crate::ResourceContracts::withhold_for_pin_mismatch`].

use serde::{Deserialize, Serialize};

use crate::contract::ResourceFunctionContract;

/// How a declared contract relates to revisions of the audited source.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RevisionMode {
    /// In force only where the audited source matches the identity recorded
    /// in the contract's lock. The default when no marker is written.
    #[default]
    Pinned,
    /// Binds by name and declared signature only, at any revision. On a
    /// function whose body is audited the body is checked against it as
    /// usual; on a trusted function it is an explicit per-run assumption.
    Flexible,
}

impl RevisionMode {
    /// The spelling a marker uses, identical to the serde representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Flexible => "flexible",
        }
    }
}

/// Which part of a declaration a revision marker speaks for.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "scope")]
pub enum RevisionScope {
    /// `requires resource_revision(MODE);` — the contract, and every clause
    /// that carries no marker of its own.
    Contract,
    /// `ensures normal resource_revision(CLAUSE, MODE);` — one clause,
    /// recorded as its canonical text (`ensures normal CLAUSE`).
    Clause { clause: String },
}

/// One revision marker as written.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RevisionMarker {
    #[serde(flatten)]
    pub scope: RevisionScope,
    pub mode: RevisionMode,
}

/// The revision markers one declaration carries, and the mode they give it.
///
/// Only declarations that carry at least one marker are recorded; every other
/// declaration is [`RevisionMode::Pinned`] by default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredRevision {
    /// The declaration's effective mode: [`RevisionMode::Flexible`] exactly
    /// when every clause is flexible (see [`Self::effective`]).
    pub mode: RevisionMode,
    /// Every marker, in clause order.
    pub markers: Vec<RevisionMarker>,
}

impl DeclaredRevision {
    /// The effective mode of a declaration with these markers.
    ///
    /// A clause without a marker takes the contract-level mode, which is
    /// [`RevisionMode::Pinned`] unless a contract-level marker says otherwise.
    /// The selector and every `requires` clause cannot carry a marker of their
    /// own, so the contract-level mode always applies to some clause, and a
    /// declaration is flexible exactly when the contract-level mode is
    /// flexible and no clause is marked pinned. A clause marked flexible in a
    /// pinned contract therefore leaves the contract pinned.
    #[must_use]
    pub fn effective(markers: &[RevisionMarker]) -> RevisionMode {
        let contract = markers
            .iter()
            .find(|marker| marker.scope == RevisionScope::Contract)
            .map_or(RevisionMode::Pinned, |marker| marker.mode);
        let pinned_clause = markers.iter().any(|marker| {
            matches!(marker.scope, RevisionScope::Clause { .. })
                && marker.mode == RevisionMode::Pinned
        });
        if contract == RevisionMode::Flexible && !pinned_clause {
            RevisionMode::Flexible
        } else {
            RevisionMode::Pinned
        }
    }
}

/// Why a pinned contract is not in force.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PinMismatchReason {
    /// No lock covers the module that declares the contract.
    NoLock,
    /// The lock covering the module has no entry for the declaration.
    NotInLock,
    /// The lock's entry was taken for different contract text.
    ContractChanged,
    /// The audited source's declarations or definitions of the function
    /// differ from the identity the lock records (for a tip identity: from
    /// the sites at the tracked ref's current commit).
    SourceChanged,
    /// Card CONTRACT-PIN-TIP-MODE: the lock entry tracks the tip of a ref
    /// that cannot be resolved on this host (repository not located, ref
    /// missing, tip not descended from the reviewed commit, unreadable blob,
    /// or no site of the function at the tip).
    TipUnresolved,
}

impl PinMismatchReason {
    /// The stable spelling, identical to the serde representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoLock => "no-lock",
            Self::NotInLock => "not-in-lock",
            Self::ContractChanged => "contract-changed",
            Self::SourceChanged => "source-changed",
            Self::TipUnresolved => "tip-unresolved",
        }
    }
}

/// A pinned contract withheld because the audited source does not match its
/// lock (card W45-CONTRACT-REVISION-MODE).
///
/// Like a declaration awaiting human review, it is **not** an entry of the
/// function map, so a consumer that knows nothing of revision modes sees an
/// uncontracted callee: the fail-closed default. Consumers that know of it
/// attribute the refusal to the mismatch (`contract-pin-mismatch`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinMismatch {
    /// The retained source-assumption key of the module that declares it.
    pub module: String,
    /// The contract the declaration states, withheld from every consumer.
    pub contract: ResourceFunctionContract,
    /// Why it is not in force.
    pub reason: PinMismatchReason,
    /// The lock consulted, when one covers the module.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock: Option<String>,
    /// A one-line account of the difference.
    pub detail: String,
    /// Card CALLEE-RETENTION-CLASS1: the withheld declaration's return-code
    /// disposition, when it states one (`contract` is then its first class
    /// and never an entry of the function map). Omitted when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relational: Option<crate::relational_post::RelationalContract>,
    /// Card CALLEE-RETENTION-CLASS1: the withheld declaration's retention
    /// clauses, when it states some (`contract` is then its custody contract
    /// and never an entry of the function map). Omitted when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retaining: Option<crate::retained_by::RetainingContract>,
}
