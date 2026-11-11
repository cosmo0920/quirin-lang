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

//! Source-set profile manifests: the published, versioned data format that
//! replaces the hardcoded pinned-source tables of Cycle 18/19.
//!
//! Layer L0 of `docs/GENERALIZATION_PLAN.md` §3.1 and the schema defined by
//! the approved design `docs/design/W2_PROFILE_MANIFEST_SCHEMA.md`. This
//! module carries serde data types only: no analysis logic, no filesystem
//! access, no workspace dependencies. The total-or-reject loader and every
//! validation rule live in `qcore-contract` (L1).
//!
//! A profile is a **declared trusted assumption**, never a verified fact
//! (plan invariant 6). Nothing in this schema can express a promotion: there
//! is no `Safe` disposition, no "accept all symbols", no "any revision", and
//! no way to skip a pin check. Every closed vocabulary below is a
//! compatibility surface — extending one is simultaneously a schema-crate
//! semver event and a [`PROFILE_MANIFEST_VERSION`] bump (design §2).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::mechanism::LibraryMechanismSummary;

/// The newest profile-manifest version this schema defines.
///
/// Version 2 changes exactly one admission rule: a [`SourceRootSelection::root`]
/// may be [`CHECKOUT_ROOT_SOURCE_ROOT`], naming the pinned checkout root itself.
/// Nothing else moves — no field is added, removed or renamed, no closed
/// vocabulary is extended, and no existing field changes meaning. The design's
/// versioning policy (`docs/design/W2_PROFILE_MANIFEST_SCHEMA.md` §2) still
/// makes it a version event, because the set of admissible manifests changed.
pub const PROFILE_MANIFEST_VERSION: u32 = 2;

/// The versions a loader built against this schema may accept. Any other
/// declared version is a whole-profile rejection (design §3, F3): a data file
/// has no negotiation counterpart, so it either is a supported version or it
/// is not readable.
///
/// Version 1 stays supported because the v1 → v2 interpretation is exact and
/// total in the direction the policy requires: v1 rejects every root a v2
/// loader would read differently, so a v1 manifest that loads at all means the
/// same thing under both. The relation is *not* symmetric, and deliberately so
/// — a manifest that names the checkout root must declare version 2, and the
/// same bytes under `"schema_version": 1` still reject.
pub const SUPPORTED_PROFILE_MANIFEST_VERSIONS: &[u32] = &[1, PROFILE_MANIFEST_VERSION];

/// The one spelling that names the pinned checkout root itself as a source
/// root, admitted from manifest version 2 onwards.
///
/// It is an exact string match, not a path-normalisation rule: `"./src"`,
/// `"a/."`, `".."` and every absolute spelling stay refused, so admitting it
/// opens no traversal beyond the checkout the pin already fixes.
pub const CHECKOUT_ROOT_SOURCE_ROOT: &str = ".";

/// Root of `contract/profiles/<source-set>/profile.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ProfileManifest {
    /// Must appear in [`SUPPORTED_PROFILE_MANIFEST_VERSIONS`]; any other
    /// value is a load rejection (fail-closed), never a best-effort parse.
    pub schema_version: u32,
    /// Source-set name; must match the directory name (checked at load).
    pub source_set: String,
    /// Named pinned checkouts. Keys are referenced by [`LibraryProfile::pin`].
    pub pins: BTreeMap<String, SourcePin>,
    /// One entry per audited library in the set. Keys are the library names
    /// used in `SupportedTarget::library` / `TargetProfile::library`.
    pub libraries: BTreeMap<String, LibraryProfile>,
    /// Relative path to the W1 resource-contract manifest that supplies
    /// `ResourceContracts` for this source set. `None` declares that no
    /// resource-contract manifest is bound: resource projection for this set
    /// has no contracts and therefore stays Unknown (fail-closed). It is not
    /// a permission to proceed without contracts.
    ///
    /// Deviation from the design (§1.2, which made this field mandatory): the
    /// Libevent source set has no resource-contract manifest, and pointing it
    /// at another set's manifest would declare an untrue binding.
    pub resource_contract_manifest: Option<String>,
    /// Provider manifests bound to this source set: relative paths under the
    /// profile directory. Loaded as [`crate::plugin::PluginManifest`];
    /// registry ordering is the sorted plugin-id order (deterministic).
    pub provider_manifests: Vec<String>,
}

/// A pinned checkout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SourcePin {
    /// Full lowercase hex commit hash. Exact equality; no ranges, no `"*"`.
    pub git_commit: String,
    /// Paths (relative to the checkout root) that must be free of tracked
    /// modifications. An empty list means the whole tree must be clean.
    pub clean_paths: Vec<String>,
    /// Human identity line carried into certificates and reports.
    pub description: String,
}

/// Everything the closed per-library match arms used to say about one library.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LibraryProfile {
    /// Which pin this library's files are checked against.
    pub pin: String,
    /// Where the library lives inside the pinned checkout and how audit units
    /// are selected.
    pub source_roots: Vec<SourceRootSelection>,
    /// The audited-file inventory with digests.
    pub audited_files: Vec<AuditedFile>,
    /// Construct witnesses binding mechanism summaries to source constructs.
    pub source_witnesses: Vec<SourceWitness>,
    /// A summary outside this set fails the import (`UnsupportedSummary`).
    pub permitted_mechanisms: BTreeSet<LibraryMechanismSummary>,
    /// Abstract operation kind → required summary name. An operation whose
    /// kind is absent has no derivable requirement and stays Unknown.
    pub operation_summaries: BTreeMap<OperationKind, String>,
    /// API disposition table. Absence from the map is *not* representable as
    /// accepted: symbols not listed classify Unknown.
    pub api_dispositions: BTreeMap<String, ApiDisposition>,
    /// Accepted build/backend/thread combinations. Empty = nothing accepted.
    pub accepted_builds: Vec<BuildProfileSelector>,
    /// What must be *present* for the bounded checks to run at all.
    pub required_evidence: BTreeSet<RequiredEvidenceKind>,
    /// Typed-source import surface; `None` where no library declares one.
    pub typed_import: Option<TypedImportProfile>,
    /// Declared audited callback effect summaries.
    pub callback_summaries: BTreeMap<String, CallbackEffectSummary>,
    /// Boolean single-field source models: record × field pairs the source
    /// importer may model.
    pub boolean_field_models: Vec<BooleanFieldModel>,
    /// Provider binding: which plugin id serves this library. `None` declares
    /// that no provider is bound, so every capability request that would route
    /// to one gets provider-absence handling: Unknown (design §3, F18).
    ///
    /// Deviation from the design (§1.2, which made this field mandatory): CFL
    /// and the Fluent Bit input-chunk wrapper are audited libraries with no
    /// first-party provider, and inventing one would either overstate coverage
    /// or bind them to an unrelated plugin id.
    pub provider: Option<ProviderBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SourceRootSelection {
    /// Root relative to the pin checkout, without traversal components; or,
    /// from manifest version 2, exactly [`CHECKOUT_ROOT_SOURCE_ROOT`] for a
    /// source set whose C sources sit at the checkout root.
    pub root: String,
    /// How compile-database units under the root are selected.
    pub selection: UnitSelection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum UnitSelection {
    /// Only `.c` files whose parent directory is exactly the root.
    TopLevel,
    /// Every built unit under the root, recursively.
    Recursive,
    /// One exact file, relative to the pin checkout root.
    Exact { file: String },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AuditedFile {
    /// Path relative to the pin checkout root.
    pub path: String,
    /// SHA-256 of the file's bytes at the pinned revision, lowercase hex.
    /// Mandatory: every audited file is digest-pinned (design §1.3).
    pub sha256: String,
}

/// One named construct witness: "this mechanism summary is justified by this
/// construct being present in this file".
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SourceWitness {
    /// Stable name, unique within the library.
    pub name: String,
    /// Must name a path present in `audited_files` (checked at load).
    pub file: String,
    /// The check that establishes the construct.
    pub check: WitnessCheck,
    /// Mechanism summaries this witness contributes when the check passes.
    /// May be empty: a source-contract witness that grants no mechanism still
    /// has to pass, and a failed witness rejects the library (design §3, F11).
    pub mechanisms: BTreeSet<LibraryMechanismSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum WitnessCheck {
    /// Preferred: the construct's presence is implied by the audited file's
    /// digest; the witness records the construct description for the report.
    Digest { construct: String },
    /// Exact-substring needles, admissible only with a rationale for why a
    /// digest cannot express the check (design §1.3). Empty needles or an
    /// empty/whitespace rationale reject the whole manifest.
    Substrings {
        needles: Vec<String>,
        rationale: String,
    },
}

/// Closed abstract operation vocabulary for `operation_summaries` keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum OperationKind {
    CreateContext,
    CreateStream,
    OpenChunk,
    ResidencyUp,
    ResidencyDown,
    Lock,
    Unlock,
    SetBusy,
    TaskCustody,
    RouteCustody,
    BeginTransaction,
    CommitTransaction,
    RollbackTransaction,
    CloseChunk,
    DeleteStreamContent,
    DestroyStream,
    DestroyContext,
}

/// Model coverage, never a verdict: there is deliberately no `Safe` value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ApiDisposition {
    /// Inside the bounded model.
    Accepted,
    /// Explicitly recorded as outside the model. Classification-wise
    /// identical to absence: Unknown. The reason documents the exclusion.
    Unknown { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BuildProfileSelector {
    /// `"epoll" | "kqueue" | "select" | "poll" | "iocp"`.
    pub backend: String,
    /// `"pthreads" | "windows_threads"`.
    pub threads: String,
    /// Feature names that must be absent from the build.
    pub forbid: BTreeSet<String>,
}

/// Closed evidence-kind vocabulary (extending it is a schema-version event).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum RequiredEvidenceKind {
    DispatcherObservation,
    EventMapCustody,
    NullableBaseCreation,
    NullableEventCreation,
    RegistrationFailurePath,
    NativeAllocationLedger,
    OsEndpointLedger,
    ControlFlowComplete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TypedImportProfile {
    /// Function name → role.
    pub functions: BTreeMap<String, ImportRole>,
    /// Record names the importer may reference.
    pub records: BTreeSet<String>,
    /// Types permitted under `sizeof`.
    pub sizeof_types: BTreeSet<String>,
    /// Per-function exact signature constraints. Mandatory for every Callback.
    /// A Wrapper without a signature retains opaque unrestricted calls.
    pub signature_constraints: BTreeMap<String, SignatureConstraint>,
    /// Callee allowlist for functions with exact signatures: calls stay opaque.
    /// Wrapper roles without signatures keep the original unrestricted opaque-call bound.
    pub restricted_calls: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ImportRole {
    Callback,
    Wrapper,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SignatureConstraint {
    pub result: String,
    pub parameters: Vec<String>,
}

/// Declared audited callback effects. These are *declared trusted
/// assumptions* (invariant 6): the checker still verifies the imported body
/// against them; a mismatch is a diagnostic → Unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CallbackEffectSummary {
    pub argument_borrowed: bool,
    pub writes_fired_array: bool,
    pub releases_event_map: bool,
    pub closes_descriptor: bool,
    pub no_effect: bool,
}

/// One modeled Boolean field on one record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BooleanFieldModel {
    pub record: String,
    pub field: String,
    /// Which operation kind a write setting this field maps to.
    pub set_operation: OperationKind,
    /// Which operation kind a write clearing this field maps to.
    pub clear_operation: OperationKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ProviderBinding {
    /// Must match a plugin id from `provider_manifests` (checked at load).
    pub plugin_id: String,
    /// The provider's diagnostic namespace.
    pub diagnostic_namespace: String,
    /// Lean theorem family, cross-checked against the referenced provider
    /// manifest at load; disagreement rejects the profile.
    pub lean_theorem_family: String,
}

impl ProfileManifest {
    /// The library entry under `name`, or `None` — an unmatched library is
    /// never a default profile (invariant 1).
    #[must_use]
    pub fn library(&self, name: &str) -> Option<&LibraryProfile> {
        self.libraries.get(name)
    }

    /// The pin a library is bound to, or `None` when the reference is broken.
    /// Load-time validation rejects broken references, so a loaded manifest
    /// always resolves; this accessor never invents a pin.
    #[must_use]
    pub fn pin_for(&self, library: &str) -> Option<&SourcePin> {
        self.pins.get(&self.libraries.get(library)?.pin)
    }
}

impl LibraryProfile {
    /// The declared digest of `path`, or `None` when the file is not audited.
    #[must_use]
    pub fn audited_digest(&self, path: &str) -> Option<&str> {
        self.audited_files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.sha256.as_str())
    }
}
