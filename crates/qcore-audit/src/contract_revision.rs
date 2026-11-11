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

//! Revision modes at audit time (card **W45-CONTRACT-REVISION-MODE**, contract
//! spec §C06.2): contract locks, the pin evaluation that withholds every
//! pinned contract the audited source does not match, and the
//! `contract_revision` report section.
//!
//! # Locks
//!
//! A pinned contract's reviewed identity lives in a generated **lock**,
//! `contracts.lock`, never in the `.qcore` text. There is one per provider
//! crate, beside its `Cargo.toml` (the crate root holding its `contracts/`
//! tree); a contract module outside every crate is covered by the nearest
//! `contracts.lock` in its own directory or above it, up to the first
//! directory that holds a `Cargo.toml` or `.git`. A lock records, for every
//! pinned declaration, the digest of the contract text it
//! was reviewed with and one or more **reviewed identities**: the function's
//! sites ([`crate::c_identity`]) as they were at each named source revision
//! (revisions with equal sites share an identity). `quirin-audit pin` adds or
//! refreshes the identity of one checkout after human review, never dropping
//! those reviewed at other revisions; nothing writes a lock during an audit.
//! Format version 1 (one source per lock) is still read, as one identity per
//! entry.
//!
//! # Evaluation
//!
//! [`apply_pins`] runs once, after the contracts are loaded and before any
//! unit is analysed. Each pinned declaration in force is compared with its
//! lock ([`pin_standing`]): it stays in force when its contract text is the
//! pinned one and the audited sites equal those of some reviewed identity;
//! each mismatch — no lock, no entry or no identity, changed contract text
//! (which voids every identity), sites matching no identity — is withheld
//! ([`qcore_schemas::contract::ResourceContracts::withhold_for_pin_mismatch`]),
//! so every call relying on it is held with reason code
//! `contract-pin-mismatch`, never `Safe` or `Unsafe`: a function whose only
//! unresolved diagnostics are held calls is `Pending`, which a re-pin after
//! review resolves (card W45-USER-CONTRACT-PLUGINS; `Unknown` before). A flexible declaration is
//! never compared: it binds by name and declared signature only.
//!
//! Card **CONTRACT-PIN-TIP-MODE**: a lock entry may instead track the tip of a
//! ref (format version 3, [`crate::contract_tip`]); it is decided by the same
//! [`pin_standing`], and the section records its resolution, notice and
//! named assumption only when a run has one.
//!
//! Card **CONTRACT-SUPERSESSION** (contract spec §C06.6, user decision 9a):
//! format **version 4** adds, per entry (pinned, flexible or tip), the
//! **pin stamp** `pinned_at` — when `quirin-audit pin` recorded the review of
//! the entry's current text — and, per lock, the **Fluent Bit revision the
//! lock set is tied to** (`fluent_bit_revision`). Every lock `pin` writes is
//! version 4. Versions 1 to 3 are still read, for history (a frozen report's
//! inputs, a fixture), and carry no stamp: their declarations are in force
//! exactly as before under C06.2, but never supersede another declaration
//! ([`LockStamps`]). `pin --verify-contracts` requires every provider lock to
//! be version 4 with every entry stamped.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use qcore_c_gateway::legacy_c::pin_hold::{CONTRACT_PIN_MISMATCH, held_call};
use qcore_contract::load_path_manifest::{LoadPathManifest, load_contract_load_path_manifest};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_revision::{PinMismatchReason, RevisionMode};
use qcore_schemas::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::c_identity::{NORMALISATION, SourceSite, SourceTree};
use crate::contract_tip::{
    Notice, NoticeStanding, TIP_ASSUMPTION_PREFIX, TIP_STATEMENT, TipEntry, TipFailure,
    TipResolution, TipResolver, notice, tip_decision,
};
use crate::report::AuditReport;

/// The lock's file name.
pub const LOCK_FILE: &str = "contracts.lock";

/// Card CONTRACT-SUPERSESSION: the lock format's version, with pin stamps
/// and the lock set's Fluent Bit revision. Every lock `pin` writes has it.
pub const LOCK_VERSION: u32 = 4;

/// The historical version of a lock without a tip entry and without stamps,
/// still read.
pub const LOCK_VERSION_UNSTAMPED: u32 = 2;

/// Card CONTRACT-PIN-TIP-MODE: the historical version of a lock holding a tip
/// entry, without stamps, still read.
pub const LOCK_VERSION_TIP: u32 = 3;

/// The provider crate whose flexible contracts form the one standing libc
/// assumption.
pub const LIBC_CRATE: &str = "quirin-provider-libc";

/// The name of the standing libc assumption.
pub const LIBC_ASSUMPTION: &str = "libc-c-standard-abi";

/// What the standing libc assumption states.
pub const LIBC_STATEMENT: &str = "the C library behaves as the C standard and the platform ABI specify for every function the libc provider declares flexible: its custody statements are trusted at any revision of the audited project, and no libc body is analysed; which C library and version is in use is the build's provenance (its compiler identity and system headers), not part of this assumption";

/// What the section means, carried in the section itself.
pub const MEANING: &str = "revision modes of the declared contracts (contract spec C06.2): a pinned contract (the default) is in force only where its contract text is the one pinned and the audited source matches one of the identities its lock records, each reviewed at a named revision, and every mismatch is withheld, so each call relying on it is held with reason code `contract-pin-mismatch`, and a function whose only unresolved diagnostics are held calls is Pending, never Safe; a flexible contract binds by name and declared signature only: on a function whose body this run analysed it is checked against that body as usual, and on a trusted function it is an explicit assumption of this run; every flexible libc contract without an analysed body is one standing assumption; held counts are recorded held-call refusals, a lower bound on textual call sites";

/// One lock file (format version 4; versions 1 to 3 are read).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractLock {
    pub lock_version: u32,
    pub generated_by: String,
    pub note: String,
    /// The crate the lock belongs to, or the directory of a module set.
    pub owner: String,
    /// The normalisation of every digest ([`NORMALISATION`]).
    pub normalisation: String,
    /// Card CONTRACT-SUPERSESSION: the Fluent Bit revision the lock set this
    /// lock belongs to is tied to (user decision 9a; [`crate::pin_window`]).
    /// Version 4 only; a module-set lock outside the provider tree may have
    /// none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fluent_bit_revision: Option<String>,
    /// Every pinned declaration, sorted by function.
    pub pinned: Vec<PinnedEntry>,
    /// Every flexible declaration, sorted by function: listed so that a
    /// change of mode is a change of the lock.
    pub flexible: Vec<FlexibleEntry>,
    /// Card CONTRACT-PIN-TIP-MODE: every pinned declaration that tracks the
    /// tip of a ref instead of reviewed identities, sorted by function
    /// (format version 3 only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tip: Vec<TipEntry>,
}

/// A source a pinned identity was reviewed at: a Git revision, or a
/// directory tree (which has no revision).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedSource {
    /// `git` or `directory`, as [`SourceTree::kind`].
    pub kind: String,
    /// The Git revision, for a Git checkout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

impl ReviewedSource {
    /// The source a tree was read from.
    #[must_use]
    pub fn of(tree: &SourceTree) -> Self {
        Self {
            kind: tree.kind.to_owned(),
            revision: tree.revision.clone(),
        }
    }

    /// `git 66910c10…` or `directory`.
    #[must_use]
    pub fn label(&self) -> String {
        match &self.revision {
            Some(revision) => format!("{} {revision}", self.kind),
            None => self.kind.clone(),
        }
    }

    /// Whether `spelling` names this source: its full revision, a revision
    /// prefix of at least seven characters, or `directory`.
    #[must_use]
    pub fn is_named_by(&self, spelling: &str) -> bool {
        match &self.revision {
            Some(revision) => spelling.len() >= 7 && revision.starts_with(spelling),
            None => spelling == self.kind,
        }
    }
}

/// One reviewed identity of a pinned function: its sites as they were at
/// every source listed in `reviewed_at` (sources whose sites were equal share
/// one identity).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewedIdentity {
    /// Every source this identity was reviewed at, sorted, at least one.
    pub reviewed_at: Vec<ReviewedSource>,
    /// The function's sites, sorted ([`crate::c_identity`]).
    pub sites: Vec<SourceSite>,
}

/// One pinned declaration: the contract text it was reviewed with and every
/// reviewed identity of the function's source. It is in force where the
/// contract text is this one and the audited sites equal those of **some**
/// identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedEntry {
    pub function: String,
    pub module: String,
    pub contract_sha256: String,
    /// Card CONTRACT-SUPERSESSION: when the entry's current text was last
    /// reviewed and pinned (`YYYY-MM-DDTHH:MM:SSZ`; version 4 only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<String>,
    /// The reviewed identities, sorted by their first source; no source
    /// appears in two identities and no two identities have equal sites. An
    /// entry whose identities were all dropped has none and is never in force.
    pub identities: Vec<ReviewedIdentity>,
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (C09.4, rule 38e): the result
    /// derivation `pin --bounds --derive-results` checked the declaration's
    /// range against, when it did. Its helpers' sites are part of every
    /// identity's sites. Omitted otherwise, so every other lock is
    /// byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivation: Option<ResultDerivation>,
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED (contract specification C09.4,
/// gateway rule 38e): a pinned `nonnull_result_range` derived at pin time
/// from the configured unit's body chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultDerivation {
    /// The derived set's canonical spelling (G05j), inside the declared range;
    /// absent for a declaration that states no range (card
    /// CELL-RESULT-SET-WRITTEN-SET-DERIVATION: written sets only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_set: Option<String>,
    /// Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION (rule 38e as amended):
    /// each declared `written_set(SLOT[K], SET) when #result == C` derived
    /// from the body chain, sorted by slot and class.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub written_sets: Vec<WrittenDerivation>,
    /// The in-unit helpers whose bodies the derivation read, sorted; the
    /// definition and declaration sites of each join every identity's sites,
    /// so the range is in force only where they are the reviewed ones.
    pub helpers: Vec<String>,
    /// The C library result sets the derivation rested on, sorted by
    /// function; a changed or missing one holds the range.
    pub rests_on: Vec<RestedOn>,
}

/// Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION: one derived written set.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WrittenDerivation {
    /// The declared slot, `PARAMETER[K]`.
    pub slot: String,
    /// The class `C` of `when #result == C`.
    pub class: i64,
    /// The derived set's canonical spelling, inside the declared one.
    pub written_set: String,
}

/// One C library result set a derivation read.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestedOn {
    pub function: String,
    /// The declaring module, `crate:module`.
    pub module: String,
    /// The declaration's normalised text digest at derivation time.
    pub contract_sha256: String,
}

/// Card CELL-GUARD-RESULT-RANGE-CHECKED: `function`'s identity extended by
/// the sites of every helper its result derivation read, sorted and without
/// duplicates.
///
/// # Errors
///
/// A helper with no site in the tree (the derivation read a body this tree
/// does not have).
pub fn with_helper_sites(
    found: &BTreeMap<String, Vec<SourceSite>>,
    mut sites: Vec<SourceSite>,
    helpers: &[String],
) -> Result<Vec<SourceSite>, String> {
    for helper in helpers {
        let helper_sites = found.get(helper).cloned().unwrap_or_default();
        if helper_sites.is_empty() {
            return Err(format!(
                "the result derivation's helper `{helper}` has no site in the source tree"
            ));
        }
        sites.extend(helper_sites);
    }
    sites.sort();
    sites.dedup();
    Ok(sites)
}

/// One flexible declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlexibleEntry {
    pub function: String,
    pub module: String,
    pub contract_sha256: String,
    /// Card CONTRACT-SUPERSESSION: when the entry's current text was last
    /// reviewed (version 4 only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<String>,
}

/// Lock format version 1 (one source per lock), read and migrated.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LockV1 {
    #[allow(dead_code)]
    lock_version: u32,
    generated_by: String,
    note: String,
    owner: String,
    source: LockSourceV1,
    pinned: Vec<PinnedEntryV1>,
    flexible: Vec<FlexibleEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LockSourceV1 {
    kind: String,
    #[serde(default)]
    revision: Option<String>,
    normalisation: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PinnedEntryV1 {
    function: String,
    module: String,
    contract_sha256: String,
    sites: Vec<SourceSite>,
    #[serde(default)]
    revision: Option<String>,
}

impl LockV1 {
    /// A version-1 lock as version 2: each entry has the one identity it was
    /// reviewed at (its own `revision` when `--function` recorded one).
    fn migrate(self) -> ContractLock {
        let source_kind = self.source.kind.clone();
        let source_revision = self.source.revision.clone();
        ContractLock {
            lock_version: LOCK_VERSION_UNSTAMPED,
            generated_by: self.generated_by,
            note: self.note,
            owner: self.owner,
            normalisation: self.source.normalisation,
            fluent_bit_revision: None,
            pinned: self
                .pinned
                .into_iter()
                .map(|entry| PinnedEntry {
                    function: entry.function,
                    module: entry.module,
                    contract_sha256: entry.contract_sha256,
                    pinned_at: None,
                    identities: vec![ReviewedIdentity {
                        reviewed_at: vec![ReviewedSource {
                            kind: if entry.revision.is_some() {
                                "git".to_owned()
                            } else {
                                source_kind.clone()
                            },
                            revision: entry.revision.or_else(|| source_revision.clone()),
                        }],
                        sites: entry.sites,
                    }],
                    derivation: None,
                })
                .collect(),
            flexible: self.flexible,
            tip: Vec::new(),
        }
    }
}

/// How one pinned declaration stands against its lock at one source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinStanding {
    /// In force: the text matches and the sites equal the identity reviewed
    /// at these sources.
    InForce { reviewed_at: Vec<ReviewedSource> },
    /// Card CONTRACT-PIN-TIP-MODE: in force through a tip entry: the text
    /// matches and the sites equal those at the tip's resolved commit.
    InForceAtTip { tip: Box<TipResolution> },
    /// Not in force, for this reason, with a one-line account; for a tip
    /// entry, what resolving its tip found.
    Mismatch {
        reason: PinMismatchReason,
        detail: String,
        tip: Option<Box<TipResolution>>,
    },
}

impl PinStanding {
    /// The tip entry's resolution, when the entry tracks a tip.
    #[must_use]
    pub fn tip(&self) -> Option<&TipResolution> {
        match self {
            Self::InForce { .. } => None,
            Self::InForceAtTip { tip } => Some(tip),
            Self::Mismatch { tip, .. } => tip.as_deref(),
        }
    }

    /// The notice of a tip entry (`None` for any other entry).
    #[must_use]
    pub fn notice(&self, function: &str) -> Option<Notice> {
        let standing = match self {
            Self::InForce { .. } => return None,
            Self::InForceAtTip { .. } => NoticeStanding::InForce,
            Self::Mismatch { detail, .. } => NoticeStanding::NotInForce(detail),
        };
        self.tip().map(|tip| notice(function, tip, standing))
    }
}

/// Compare one pinned declaration (its contract digest and the audited
/// sites) with its lock entry. The single decision both an audit and
/// `quirin-audit pin --check` use. A tip entry (card CONTRACT-PIN-TIP-MODE)
/// is resolved through `tips` and decided by [`tip_decision`].
#[must_use]
pub fn pin_standing(
    lock: Option<&ContractLock>,
    function: &str,
    contract_sha256: &str,
    sites: &[SourceSite],
    module: &str,
    tips: &mut TipResolver,
) -> PinStanding {
    let Some(lock) = lock else {
        return PinStanding::Mismatch {
            reason: PinMismatchReason::NoLock,
            detail: format!("no {LOCK_FILE} covers `{module}`"),
            tip: None,
        };
    };
    if let Some(entry) = lock.tip.iter().find(|e| e.function == function) {
        return tip_standing(entry, contract_sha256, sites, tips);
    }
    let Some(entry) = lock.pinned.iter().find(|e| e.function == function) else {
        return PinStanding::Mismatch {
            reason: PinMismatchReason::NotInLock,
            detail: format!("the lock has no pinned entry for `{function}`"),
            tip: None,
        };
    };
    if entry.contract_sha256 != contract_sha256 {
        return PinStanding::Mismatch {
            reason: PinMismatchReason::ContractChanged,
            detail: format!(
                "the contract text changed since it was pinned (lock {}, now {}): every \
                 reviewed identity is void",
                short(&entry.contract_sha256),
                short(contract_sha256)
            ),
            tip: None,
        };
    }
    if entry.identities.is_empty() {
        return PinStanding::Mismatch {
            reason: PinMismatchReason::NotInLock,
            detail: format!("the lock records no reviewed identity for `{function}`"),
            tip: None,
        };
    }
    match entry
        .identities
        .iter()
        .find(|identity| identity.sites == sites)
    {
        Some(identity) => PinStanding::InForce {
            reviewed_at: identity.reviewed_at.clone(),
        },
        None => PinStanding::Mismatch {
            reason: PinMismatchReason::SourceChanged,
            detail: site_difference(&entry.identities, sites),
            tip: None,
        },
    }
}

/// Card CONTRACT-PIN-TIP-MODE: how a tip entry stands. The tip is resolved
/// whatever the contract text, so a changed text still reports where the tip
/// is; [`tip_decision`] decides.
fn tip_standing(
    entry: &TipEntry,
    contract_sha256: &str,
    sites: &[SourceSite],
    tips: &mut TipResolver,
) -> PinStanding {
    let tip = tips.resolve(entry);
    let text_matches = entry.contract_sha256 == contract_sha256;
    let resolved = tip.resolved();
    let sites_equal = resolved && tip.tip_sites == sites;
    let (in_force, _) = tip_decision(text_matches, resolved, sites_equal);
    if in_force {
        return PinStanding::InForceAtTip { tip: Box::new(tip) };
    }
    let (reason, detail) = if !text_matches {
        (
            PinMismatchReason::ContractChanged,
            format!(
                "the contract text changed since its tip was tracked (lock {}, now {}): track \
                 the tip again after review",
                short(&entry.contract_sha256),
                short(contract_sha256)
            ),
        )
    } else if !resolved {
        (
            PinMismatchReason::TipUnresolved,
            format!(
                "the tip cannot be resolved ({}): {}",
                tip.failure.map_or("unresolved", TipFailure::as_str),
                tip.failure_detail.as_deref().unwrap_or_default()
            ),
        )
    } else {
        let paths = tip
            .tip_sites
            .iter()
            .chain(sites)
            .filter(|site| !(tip.tip_sites.contains(site) && sites.contains(site)))
            .map(|site| site.path.as_str())
            .collect::<BTreeSet<_>>();
        (
            PinMismatchReason::SourceChanged,
            format!(
                "the audited source's sites differ from those at the tip of `{}` in `{}` at {}: \
                 differs in {}",
                entry.reference,
                entry.repository.name,
                tip.resolved_commit.as_deref().unwrap_or_default(),
                paths.into_iter().collect::<Vec<_>>().join(", ")
            ),
        )
    };
    PinStanding::Mismatch {
        reason,
        detail,
        tip: Some(Box::new(tip)),
    }
}

impl ContractLock {
    /// Read and validate a lock. Version 1 is read and migrated to version 2
    /// (`quirin-audit pin` then writes version 2).
    ///
    /// # Errors
    ///
    /// A malformed lock refuses; it is never read as an empty one.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text =
            fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let malformed =
            |error: String| format!("malformed contract lock {}: {error}", path.display());
        let value: Value = serde_json::from_str(&text).map_err(|e| malformed(e.to_string()))?;
        let lock = match value["lock_version"].as_u64() {
            Some(1) => serde_json::from_value::<LockV1>(value)
                .map_err(|e| malformed(e.to_string()))?
                .migrate(),
            Some(2..=4) => {
                serde_json::from_value::<Self>(value).map_err(|e| malformed(e.to_string()))?
            }
            other => return Err(malformed(format!("unsupported lock_version {other:?}"))),
        };
        lock.validate().map_err(malformed)?;
        Ok(lock)
    }

    fn validate(&self) -> Result<(), String> {
        match (self.lock_version, self.tip.is_empty()) {
            (LOCK_VERSION, _) | (LOCK_VERSION_UNSTAMPED, true) | (LOCK_VERSION_TIP, false) => {}
            (LOCK_VERSION_UNSTAMPED, false) => {
                return Err("a version-2 lock holds a tip entry (it needs version 3)".into());
            }
            (LOCK_VERSION_TIP, true) => {
                return Err("a version-3 lock holds no tip entry (it is version 2)".into());
            }
            (other, _) => return Err(format!("unsupported lock_version {other}")),
        }
        // Card CONTRACT-SUPERSESSION: stamps and the revision tie exist from
        // version 4 on. A stamp's spelling is not checked here: a malformed
        // stamp is a supersession residue (C06.6), and `pin
        // --verify-contracts` reports it.
        if self.lock_version < LOCK_VERSION {
            let stamped = self.pinned.iter().any(|e| e.pinned_at.is_some())
                || self.flexible.iter().any(|e| e.pinned_at.is_some())
                || self.tip.iter().any(|e| e.pinned_at.is_some());
            if stamped || self.fluent_bit_revision.is_some() {
                return Err(format!(
                    "a version-{} lock carries a pin stamp or a Fluent Bit revision (they need \
                     version 4)",
                    self.lock_version
                ));
            }
        }
        if let Some(revision) = &self.fluent_bit_revision
            && !(revision.len() == 40 && revision.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err(format!(
                "fluent_bit_revision `{revision}` is not a full Git revision"
            ));
        }
        if self.normalisation != NORMALISATION {
            return Err(format!(
                "unsupported normalisation `{}`",
                self.normalisation
            ));
        }
        let hex = |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        let pinned = self
            .pinned
            .iter()
            .map(|e| e.function.as_str())
            .collect::<Vec<_>>();
        let flexible = self
            .flexible
            .iter()
            .map(|e| e.function.as_str())
            .collect::<Vec<_>>();
        let tip = self
            .tip
            .iter()
            .map(|e| e.function.as_str())
            .collect::<Vec<_>>();
        for (label, names) in [("pinned", &pinned), ("flexible", &flexible), ("tip", &tip)] {
            if names.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(format!("{label} entries are not sorted and unique"));
            }
        }
        if let Some(both) = pinned.iter().find(|name| flexible.contains(name)) {
            return Err(format!("`{both}` is both pinned and flexible"));
        }
        if let Some(both) = tip
            .iter()
            .find(|name| pinned.contains(name) || flexible.contains(name))
        {
            return Err(format!(
                "`{both}` tracks a tip and is also a pinned or flexible entry"
            ));
        }
        for entry in &self.tip {
            entry.validate()?;
        }
        for entry in &self.pinned {
            let malformed = || format!("malformed entry for `{}`", entry.function);
            if !hex(&entry.contract_sha256) {
                return Err(malformed());
            }
            let mut seen = BTreeSet::new();
            for identity in &entry.identities {
                if identity.reviewed_at.is_empty()
                    || identity
                        .reviewed_at
                        .windows(2)
                        .any(|pair| pair[0] >= pair[1])
                    || identity
                        .reviewed_at
                        .iter()
                        .any(|source| !seen.insert(source.clone()))
                    || identity.sites.windows(2).any(|pair| pair[0] > pair[1])
                    || identity.sites.iter().any(|site| {
                        !hex(&site.declaration_sha256)
                            || site.body_sha256.as_deref().is_some_and(|body| !hex(body))
                    })
                {
                    return Err(malformed());
                }
            }
            if entry
                .identities
                .windows(2)
                .any(|pair| pair[0].reviewed_at[0] >= pair[1].reviewed_at[0])
            {
                return Err(format!(
                    "identities of `{}` are not sorted by their first source",
                    entry.function
                ));
            }
            let distinct = entry
                .identities
                .iter()
                .map(|identity| &identity.sites)
                .collect::<BTreeSet<_>>();
            if distinct.len() != entry.identities.len() {
                return Err(format!(
                    "two identities of `{}` have equal sites",
                    entry.function
                ));
            }
        }
        if self
            .flexible
            .iter()
            .any(|entry| !hex(&entry.contract_sha256))
        {
            return Err("malformed flexible entry".into());
        }
        Ok(())
    }

    /// The lock's canonical text: two-space pretty JSON and a final newline.
    ///
    /// # Errors
    ///
    /// Serialisation failure only.
    pub fn to_text(&self) -> Result<String, String> {
        let mut lock = self.clone();
        // Card CONTRACT-SUPERSESSION: every lock `pin` writes is version 4.
        lock.lock_version = LOCK_VERSION;
        serde_json::to_string_pretty(&lock)
            .map(|text| text + "\n")
            .map_err(|error| error.to_string())
    }

    /// Every source any identity of this lock was reviewed at.
    #[must_use]
    pub fn reviewed_sources(&self) -> BTreeSet<ReviewedSource> {
        self.pinned
            .iter()
            .flat_map(|entry| &entry.identities)
            .flat_map(|identity| identity.reviewed_at.iter().cloned())
            .collect()
    }
}

impl PinnedEntry {
    /// Record `sites` as reviewed at `source` (adding or refreshing): the
    /// source leaves any identity it was recorded in, identities left with no
    /// source go, and then it joins the identity with equal sites or starts a
    /// new one. Identities reviewed at other sources are kept.
    pub fn record(&mut self, source: &ReviewedSource, sites: Vec<SourceSite>) {
        self.drop_source(source);
        match self
            .identities
            .iter_mut()
            .find(|identity| identity.sites == sites)
        {
            Some(identity) => {
                identity.reviewed_at.push(source.clone());
                identity.reviewed_at.sort();
            }
            None => self.identities.push(ReviewedIdentity {
                reviewed_at: vec![source.clone()],
                sites,
            }),
        }
        self.identities
            .sort_by(|a, b| a.reviewed_at[0].cmp(&b.reviewed_at[0]));
    }

    /// Remove `source` from every identity; identities left with no source
    /// go. Returns whether anything was removed.
    pub fn drop_source(&mut self, source: &ReviewedSource) -> bool {
        let before = self.identities.clone();
        for identity in &mut self.identities {
            identity.reviewed_at.retain(|reviewed| reviewed != source);
        }
        self.identities
            .retain(|identity| !identity.reviewed_at.is_empty());
        self.identities
            .sort_by(|a, b| a.reviewed_at[0].cmp(&b.reviewed_at[0]));
        before != self.identities
    }
}

/// Where the modules of an environment's contracts come from: the load path
/// named by the contract manifest, when there is one.
#[derive(Clone, Debug, Default)]
pub struct ModuleOwners {
    load_path: Option<LoadPathManifest>,
}

impl ModuleOwners {
    /// The owners named by a contract manifest's text (schema 2 names its
    /// load path, relative to the manifest).
    ///
    /// # Errors
    ///
    /// A load path that does not read refuses.
    pub fn for_manifest(manifest_path: &Path, manifest_text: &str) -> Result<Self, String> {
        let manifest: Value = serde_json::from_str(manifest_text)
            .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
        let Some(load_path) = manifest["load_path"].as_str() else {
            return Ok(Self::default());
        };
        let path = manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(load_path);
        Ok(Self {
            load_path: Some(load_contract_load_path_manifest(&path)?),
        })
    }

    /// Owners resolved through one load-path manifest.
    ///
    /// # Errors
    ///
    /// A load path that does not read refuses.
    pub fn for_load_path(path: &Path) -> Result<Self, String> {
        Ok(Self {
            load_path: Some(load_contract_load_path_manifest(path)?),
        })
    }

    /// The crate root of a load-path crate.
    fn crate_root(&self, name: &str) -> Option<PathBuf> {
        let root = self.load_path.as_ref()?.contract_roots.get(name)?;
        root.parent().map(Path::to_path_buf)
    }

    /// The lock covering a module key and the crate that owns it, if any.
    #[must_use]
    pub fn lock_for(&self, module: &str) -> (Option<PathBuf>, Option<String>) {
        // A qualified `crate:module` key: the crate's own lock.
        if let Some((crate_name, _)) = module.split_once(':')
            && !Path::new(module).is_absolute()
        {
            return (
                self.crate_root(crate_name)
                    .map(|root| root.join(LOCK_FILE))
                    .filter(|lock| lock.is_file()),
                Some(crate_name.to_owned()),
            );
        }
        let Ok(path) = Path::new(module).canonicalize() else {
            return (None, None);
        };
        let owner = self.load_path.as_ref().and_then(|load_path| {
            load_path
                .contract_roots
                .iter()
                .find(|(_, root)| root.canonicalize().is_ok_and(|root| path.starts_with(root)))
                .map(|(name, _)| name.clone())
        });
        let mut directory = path.parent();
        while let Some(current) = directory {
            let lock = current.join(LOCK_FILE);
            if lock.is_file() {
                return (Some(lock), owner);
            }
            if current.join("Cargo.toml").exists() || current.join(".git").exists() {
                break;
            }
            directory = current.parent();
        }
        (None, owner)
    }
}

/// Card **CONTRACT-SUPERSESSION** (contract spec §C06.6): the pin stamps of
/// the declarations a loader meets, read from the `contracts.lock` covering
/// each module ([`ModuleOwners::lock_for`]). A declaration with no lock, no
/// entry, or an unreadable lock is unpinned; an entry taken for another text
/// is stale; an entry of a lock older than version 4 has no stamp. Only a
/// function declared by more than one module ever asks.
pub struct LockStamps<'a> {
    owners: &'a ModuleOwners,
    locks: std::cell::RefCell<BTreeMap<PathBuf, Option<ContractLock>>>,
}

impl<'a> LockStamps<'a> {
    #[must_use]
    pub fn new(owners: &'a ModuleOwners) -> Self {
        Self {
            owners,
            locks: std::cell::RefCell::new(BTreeMap::new()),
        }
    }
}

impl qcore_contract::contract_supersession::DeclarationStamps for LockStamps<'_> {
    fn standing(
        &self,
        module: &str,
        function: &str,
        contract_sha256: &str,
    ) -> qcore_schemas::contract_supersession::StampStanding {
        use qcore_schemas::contract_supersession::StampStanding;
        let (Some(path), _) = self.owners.lock_for(module) else {
            return StampStanding::Unpinned;
        };
        let mut locks = self.locks.borrow_mut();
        let Some(lock) = locks
            .entry(path.clone())
            .or_insert_with(|| ContractLock::read(&path).ok())
        else {
            return StampStanding::Unpinned;
        };
        let entry = lock
            .pinned
            .iter()
            .find(|e| e.function == function)
            .map(|e| (&e.contract_sha256, e.pinned_at.as_deref()))
            .or_else(|| {
                lock.flexible
                    .iter()
                    .find(|e| e.function == function)
                    .map(|e| (&e.contract_sha256, e.pinned_at.as_deref()))
            })
            .or_else(|| {
                lock.tip
                    .iter()
                    .find(|e| e.function == function)
                    .map(|e| (&e.contract_sha256, e.pinned_at.as_deref()))
            });
        match entry {
            None => StampStanding::Unpinned,
            Some((digest, _)) if digest != contract_sha256 => StampStanding::Stale,
            Some((_, pinned_at)) => StampStanding::of_entry(pinned_at),
        }
    }
}

/// A pinned declaration awaiting comparison: function, module, owner, lock
/// and contract digest.
type PendingPin = (String, String, Option<String>, Option<PathBuf>, String);

/// Held-call refusals per callee, and the distinct (library, file, function)
/// rows holding them.
pub type HeldCalls = BTreeMap<String, (usize, BTreeSet<(String, String, String)>)>;

/// One pinned declaration's outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PinOutcome {
    function: String,
    module: String,
    owner: Option<String>,
    lock: Option<String>,
    contract_sha256: String,
    reason: Option<PinMismatchReason>,
    detail: String,
    /// The sources of the identity that matched, when in force.
    matched: Vec<ReviewedSource>,
    /// Every reviewed identity of the lock entry.
    identities: Vec<ReviewedIdentity>,
    found: Vec<SourceSite>,
    /// Card CONTRACT-PIN-TIP-MODE: what resolving a tip entry found, and its
    /// notice.
    tip: Option<TipResolution>,
    notice: Option<Notice>,
}

/// What [`apply_pins`] decided, kept for the report section.
#[derive(Clone, Debug, Default)]
pub struct PinEvaluation {
    source_kind: String,
    source_revision: Option<String>,
    locks: BTreeMap<String, Value>,
    outcomes: Vec<PinOutcome>,
    owners: BTreeMap<String, Option<String>>,
    contract_sha256: BTreeMap<String, String>,
    modules: BTreeMap<String, String>,
}

/// Compare every pinned declaration in force with its lock over the source at
/// `source_root`, and withhold every mismatch.
///
/// # Errors
///
/// A malformed lock or an unreadable source tree refuses the audit; a
/// missing lock or entry is a mismatch, not an error.
pub fn apply_pins(
    contracts: &mut ResourceContracts,
    owners: &ModuleOwners,
    source_root: &Path,
) -> Result<PinEvaluation, String> {
    let digests =
        qcore_contract::contract_revision::declaration_digests(contracts.source_assumptions())?;
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 13l): the `@` names whose
    // enumerations join each declaration's identity.
    let c_names =
        qcore_contract::contract_revision::declaration_c_names(contracts.source_assumptions())?;
    let mut evaluation = PinEvaluation::default();
    // Card CONTRACT-PIN-TIP-MODE: a malformed repository variable refuses.
    let mut tips = TipResolver::from_environment()?;
    let mut locks: BTreeMap<PathBuf, ContractLock> = BTreeMap::new();
    let mut pending: Vec<PendingPin> = Vec::new();
    for (name, (module, digest)) in &digests {
        let (lock, owner) = owners.lock_for(module);
        evaluation.owners.insert(module.clone(), owner.clone());
        evaluation
            .contract_sha256
            .insert(name.clone(), digest.clone());
        evaluation.modules.insert(name.clone(), module.clone());
        if let Some(path) = lock.as_ref().filter(|path| !locks.contains_key(*path)) {
            let read = ContractLock::read(path)?;
            evaluation.locks.insert(
                path.display().to_string(),
                json!({
                    "owner": read.owner,
                    "sha256": sha256_hex(&fs::read(path).map_err(|e| e.to_string())?),
                    "reviewed_at": read.reviewed_sources(),
                }),
            );
            locks.insert(path.clone(), read);
        }
        // Card CALLEE-RETENTION-CLASS1: a return-code disposition and a
        // retaining declaration are pinned like any declaration in force; the
        // two tables sit beside the function map, never in it.
        let in_force = contracts.contract(name).is_some()
            || contracts.relational_contract(name).is_some()
            || contracts.retaining_contract(name).is_some();
        if !in_force || contracts.revision_mode(name) != RevisionMode::Pinned {
            continue;
        }
        pending.push((name.clone(), module.clone(), owner, lock, digest.clone()));
    }
    let wanted = pending
        .iter()
        .filter(|(name, _, _, lock, _)| {
            lock.as_ref()
                .and_then(|path| locks.get(path))
                .is_some_and(|lock| {
                    lock.pinned.iter().any(|entry| &entry.function == name)
                        || lock.tip.iter().any(|entry| &entry.function == name)
                })
        })
        .map(|(name, ..)| name.clone())
        .collect::<BTreeSet<_>>();
    // Card W45-AUDIT-CHECKPOINTS: the sites are read one file at a time, so
    // the evaluation never holds the whole checkout's source.
    let scan = SourceTree::read_sites(
        source_root,
        &crate::c_identity::identity_names(&wanted, &c_names),
    )?;
    evaluation.source_kind = scan.kind.to_owned();
    evaluation.source_revision = scan.revision;
    let found = scan.sites;
    for (name, module, owner, lock_path, digest) in pending {
        let lock = lock_path.as_ref().and_then(|path| locks.get(path));
        let entry = lock.and_then(|lock| lock.pinned.iter().find(|e| e.function == name));
        // An unresolvable `@` name leaves the identity without its
        // enumeration: a mismatch, never in force.
        let sites = crate::c_identity::identity_sites(&found, &name, &c_names)
            .unwrap_or_else(|_| found.get(&name).cloned().unwrap_or_default());
        let identities = entry.map(|e| e.identities.clone()).unwrap_or_default();
        let standing = pin_standing(lock, &name, &digest, &sites, &module, &mut tips);
        let notice = standing.notice(&name);
        let tip = standing.tip().cloned();
        let (reason, detail, matched) = match standing {
            PinStanding::InForce { reviewed_at } => (None, String::new(), reviewed_at),
            PinStanding::InForceAtTip { .. } => (None, String::new(), Vec::new()),
            PinStanding::Mismatch { reason, detail, .. } => (Some(reason), detail, Vec::new()),
        };
        let lock_display = lock_path.as_ref().map(|path| path.display().to_string());
        if let Some(reason) = reason {
            contracts.withhold_for_pin_mismatch(
                &name,
                module.clone(),
                reason,
                lock_display.clone(),
                detail.clone(),
            )?;
        }
        evaluation.outcomes.push(PinOutcome {
            function: name,
            module,
            owner,
            lock: lock_display,
            contract_sha256: digest,
            reason,
            detail,
            matched,
            identities,
            found: sites,
            tip,
            notice,
        });
    }
    Ok(evaluation)
}

fn short(digest: &str) -> &str {
    &digest[..digest.len().min(12)]
}

/// A one-line account of how the audited sites differ from every reviewed
/// identity: per identity, its sources and the files that differ.
fn site_difference(identities: &[ReviewedIdentity], found: &[SourceSite]) -> String {
    let parts = identities
        .iter()
        .map(|identity| {
            let expected = &identity.sites;
            let paths = expected
                .iter()
                .chain(found)
                .filter(|site| !(expected.contains(site) && found.contains(site)))
                .map(|site| site.path.as_str())
                .collect::<BTreeSet<_>>();
            format!(
                "reviewed at {}: differs in {}",
                identity
                    .reviewed_at
                    .iter()
                    .map(ReviewedSource::label)
                    .collect::<Vec<_>>()
                    .join(", "),
                paths.into_iter().collect::<Vec<_>>().join(", ")
            )
        })
        .collect::<Vec<_>>();
    format!(
        "the audited source's sites match no reviewed identity ({})",
        parts.join("; ")
    )
}

/// What the section reads of a finished report: the analysed functions, by
/// name, and the pin-mismatch held-call refusals per callee with the distinct
/// (library, file, function) rows holding them.
///
/// Card **W45-AUDIT-CHECKPOINTS**: both are per-row folds (a union and a
/// sum/union), so a whole report's facts are the [`ReportFacts::absorb`] of its
/// groups' facts, and the section of a composed report is the whole run's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReportFacts {
    pub analysed: BTreeSet<String>,
    pub held: HeldCalls,
}

impl ReportFacts {
    /// The facts of `rows`.
    #[must_use]
    pub fn of_rows(rows: &[Value]) -> Self {
        let mut facts = Self::default();
        for unit in rows {
            let library = unit["library"].as_str().unwrap_or_default();
            let file = unit["file"].as_str().unwrap_or_default();
            for function in unit["functions"].as_array().into_iter().flatten() {
                if let Some(name) = function["function"].as_str() {
                    facts.analysed.insert(name.to_owned());
                }
                let name = function["function"].as_str().unwrap_or_default();
                for diagnostic in function["diagnostics"].as_array().into_iter().flatten() {
                    let Some(callee) = diagnostic["description"].as_str().and_then(held_call)
                    else {
                        continue;
                    };
                    let entry = facts.held.entry(callee.to_owned()).or_default();
                    entry.0 += 1;
                    entry
                        .1
                        .insert((library.to_owned(), file.to_owned(), name.to_owned()));
                }
            }
        }
        facts
    }

    /// Unite `other`'s analysed functions and held rows, and add its sites.
    pub fn absorb(&mut self, other: Self) {
        self.analysed.extend(other.analysed);
        for (callee, (sites, rows)) in other.held {
            let entry = self.held.entry(callee).or_default();
            entry.0 += sites;
            entry.1.extend(rows);
        }
    }
}

/// Everything the section reads of the contract environment and the pin
/// evaluation (card **W45-AUDIT-CHECKPOINTS**): a function of the run's inputs
/// alone, published into each checkpoint so a composition can render the
/// section without the environment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionTemplate {
    pub source: Value,
    pub locks: Value,
    /// Card CONTRACT-SUPERSESSION: the environment's superseded and
    /// conflicted declarations, absent (and so byte-identical to before)
    /// when there are none.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub supersession: Value,
    /// Per pinned outcome, in evaluation order: its record without the
    /// report-dependent keys, and whether it is in force.
    pub pinned: Vec<PinnedTemplate>,
    /// Every flexible declaration in force, in name order.
    pub flexible: Vec<FlexibleTemplate>,
    /// Every declaration, in name order.
    pub inventory: Vec<InventoryTemplate>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinnedTemplate {
    pub function: String,
    pub in_force: bool,
    pub record: Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlexibleTemplate {
    pub function: String,
    pub module: String,
    pub owner: Option<String>,
    pub contract_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryTemplate {
    pub function: String,
    pub mode: String,
    pub module: Option<String>,
    pub contract_sha256: String,
    /// The status when it does not depend on the report (pending review, not
    /// in force, pinned in force); `None` for a flexible declaration.
    pub fixed_status: Option<String>,
}

impl PinEvaluation {
    /// Card W45-AUDIT-CHECKPOINTS: every lock read, by path, with its file's
    /// SHA-256 — the contract-set identity a checkpoint records.
    #[must_use]
    pub fn lock_digests(&self) -> BTreeMap<String, String> {
        self.locks
            .iter()
            .map(|(path, lock)| {
                (
                    path.clone(),
                    lock["sha256"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    }
}

/// The report-independent part of the section.
#[must_use]
pub fn template(contracts: &ResourceContracts, evaluation: &PinEvaluation) -> SectionTemplate {
    let module_sha256 = |module: &str| {
        contracts
            .source_assumptions()
            .get(module)
            .map(|text| sha256_hex(text.as_bytes()))
    };
    let owner_of = |module: &str| evaluation.owners.get(module).cloned().flatten();
    let pinned = evaluation
        .outcomes
        .iter()
        .map(|outcome| {
            let mut record = json!({
                "function": outcome.function,
                "module": outcome.module,
                "module_sha256": module_sha256(&outcome.module),
                "owner": outcome.owner,
                "lock": outcome.lock,
                "contract_sha256": outcome.contract_sha256,
            });
            match outcome.reason {
                None => {
                    record["sites"] = json!(outcome.found.len());
                    record["matched_reviewed_at"] = json!(outcome.matched);
                    record["identities"] = json!(outcome.identities.len());
                }
                Some(reason) => {
                    record["reason"] = json!(reason.as_str());
                    record["detail"] = json!(outcome.detail);
                    record["identities"] = json!(outcome.identities);
                    record["found_sites"] = json!(outcome.found);
                }
            }
            // Card CONTRACT-PIN-TIP-MODE: a tip entry's resolution and notice,
            // only on tip entries, so a run without one is unchanged.
            if let Some(tip) = &outcome.tip {
                record["tip"] = json!(tip);
                if outcome.reason.is_none() {
                    record["matched_tip_commit"] = json!(tip.resolved_commit);
                }
            }
            if let Some(notice) = &outcome.notice {
                record["notice"] = json!(notice);
            }
            PinnedTemplate {
                function: outcome.function.clone(),
                in_force: outcome.reason.is_none(),
                record,
            }
        })
        .collect();
    let mut flexible = Vec::new();
    for (name, contract_digest) in &evaluation.contract_sha256 {
        if contracts.revision_mode(name) != RevisionMode::Flexible
            || contracts.contract(name).is_none()
        {
            continue;
        }
        let module = evaluation.modules.get(name).cloned().unwrap_or_default();
        flexible.push(FlexibleTemplate {
            function: name.clone(),
            owner: owner_of(&module),
            module,
            contract_sha256: contract_digest.clone(),
        });
    }
    let inventory = evaluation
        .contract_sha256
        .iter()
        .map(|(name, contract_digest)| {
            let mode = contracts.revision_mode(name);
            let fixed_status = if contracts.pending_review().contains_key(name) {
                Some("pending-review")
            } else if contracts.pin_mismatch().contains_key(name) {
                Some("not-in-force")
            } else if evaluation
                .outcomes
                .iter()
                .any(|o| &o.function == name && o.reason.is_none() && o.tip.is_some())
            {
                // Card CONTRACT-PIN-TIP-MODE: never `pinned-in-force`.
                Some("tip-in-force")
            } else if mode == RevisionMode::Pinned {
                Some("pinned-in-force")
            } else {
                None
            };
            InventoryTemplate {
                function: name.clone(),
                mode: mode.as_str().to_owned(),
                module: evaluation.modules.get(name).cloned(),
                contract_sha256: contract_digest.clone(),
                fixed_status: fixed_status.map(str::to_owned),
            }
        })
        .collect();
    SectionTemplate {
        source: json!({"kind": evaluation.source_kind, "revision": evaluation.source_revision}),
        locks: json!(evaluation.locks),
        supersession: supersession_record(contracts),
        pinned,
        flexible,
        inventory,
    }
}

/// What the section says of supersession (contract spec §C06.6): `null` when
/// no function is declared twice, so the section is unchanged.
fn supersession_record(contracts: &ResourceContracts) -> Value {
    if contracts.superseded().is_empty() && contracts.contract_conflict().is_empty() {
        return Value::Null;
    }
    json!({
        "meaning": SUPERSESSION_MEANING,
        "reason_code": qcore_c_gateway::legacy_c::conflict_hold::CONTRACT_CONFLICT,
        "superseded": contracts.superseded(),
        "conflicts": contracts.contract_conflict(),
    })
}

/// What the supersession part of the section means.
pub const SUPERSESSION_MEANING: &str = "contract bundles compose by supersession (contract spec C06.6): where more than one module declares a function, the declaration whose lock entry carries the greatest pin stamp (`pinned_at`, written by `quirin-audit pin` when its current text was reviewed) is in force and every other declaration is retained here as superseded, with both digests and stamps; where the stamps do not decide (equal stamps on different texts, a missing, malformed or stale stamp, or an unpinned declaration) no declaration is in force, every candidate is retained under `conflicts`, and every call relying on the function is held under reason code `contract-conflict`, which is Unknown, never Safe, Unsafe or Pending";

/// The section for `template` over the report facts `facts`.
#[must_use]
pub fn render(template: &SectionTemplate, facts: &ReportFacts) -> Value {
    let ReportFacts { analysed, held } = facts;
    let mut in_force = Vec::new();
    let mut mismatches = Vec::new();
    for pinned in &template.pinned {
        let mut record = pinned.record.clone();
        if pinned.in_force {
            record["body_analysed"] = json!(analysed.contains(&pinned.function));
            in_force.push(record);
        } else {
            let (sites, functions) = held
                .get(&pinned.function)
                .map_or((0, 0), |(sites, functions)| (*sites, functions.len()));
            record["held_functions"] = json!(functions);
            record["held_call_sites"] = json!(sites);
            mismatches.push(record);
        }
    }
    let mut checked = Vec::new();
    let mut trusted = Vec::new();
    let mut libc = Vec::new();
    for flexible in &template.flexible {
        let record = json!({
            "function": flexible.function,
            "module": flexible.module,
            "owner": flexible.owner,
            "contract_sha256": flexible.contract_sha256,
        });
        if analysed.contains(&flexible.function) {
            checked.push(record);
        } else if flexible.owner.as_deref() == Some(LIBC_CRATE) {
            libc.push(flexible.function.clone());
        } else {
            trusted.push(record);
        }
    }
    // Every declaration, with its mode and what this run made of it: the
    // inventory `quirin-audit compare` reads.
    let mut inventory = BTreeMap::new();
    for entry in &template.inventory {
        let status = match &entry.fixed_status {
            Some(status) => status.as_str(),
            None if analysed.contains(&entry.function) => "flexible-checked",
            None if libc.contains(&entry.function) => "flexible-standing",
            None => "flexible-trusted",
        };
        inventory.insert(
            entry.function.clone(),
            json!({
                "mode": entry.mode,
                "status": status,
                "module": entry.module,
                "contract_sha256": entry.contract_sha256,
            }),
        );
    }
    let pinned_total = template.pinned.len();
    let held_functions = held
        .values()
        .flat_map(|(_, functions)| functions)
        .collect::<BTreeSet<_>>()
        .len();
    let standing = if libc.is_empty() {
        json!([])
    } else {
        json!([{
            "name": LIBC_ASSUMPTION,
            "owner": LIBC_CRATE,
            "statement": LIBC_STATEMENT,
            "contracts": libc.len(),
            "functions": libc,
        }])
    };
    let mut section = json!({
        "meaning": MEANING,
        "reason_code": CONTRACT_PIN_MISMATCH,
        "default_mode": RevisionMode::Pinned.as_str(),
        "source": template.source,
        "locks": template.locks,
        "counts": {
            "pinned": pinned_total,
            "pinned_in_force": in_force.len(),
            "pinned_not_in_force": mismatches.len(),
            "flexible_checked": checked.len(),
            "flexible_trusted": trusted.len(),
            "standing_libc": libc.len(),
            "held_functions": held_functions,
            "held_call_sites": held.values().map(|(sites, _)| sites).sum::<usize>(),
        },
        "pinned_in_force": in_force,
        "pin_mismatches": mismatches,
        "flexible_checked": checked,
        "assumptions": {
            "flexible_trusted": trusted,
            "standing": standing,
        },
        "contracts": inventory,
    });
    if !template.supersession.is_null() {
        section["supersession"] = template.supersession.clone();
    }
    add_tip_tracking(&mut section, template);
    section
}

/// What the tip part of the section means, carried in the section itself.
pub const TIP_MEANING: &str = "tip identities (card CONTRACT-PIN-TIP-MODE, contract spec C06.2): a tip entry's contract targets a future, unreleased release (a release candidate), not a released version; it is in force exactly where its contract text is the pinned one and the audited sites equal those read from the blobs of the tracked ref's current commit, which no person may have reviewed when the tip moved since review; every tip entry in force is a named assumption of this run (`assumptions.tip_tracked`); each tip entry carries a notice, info when in force and warning when not; notices are presentation and evidence and decide no verdict";

/// Card CONTRACT-PIN-TIP-MODE: the tip counts, notices and assumptions of a
/// rendered section, added only when some pinned record tracks a tip, so a
/// run without one is byte-identical.
fn add_tip_tracking(section: &mut Value, template: &SectionTemplate) {
    let tips = template
        .pinned
        .iter()
        .filter(|pinned| pinned.record.get("tip").is_some())
        .collect::<Vec<_>>();
    if tips.is_empty() {
        return;
    }
    let in_force = tips.iter().filter(|pinned| pinned.in_force).count();
    section["tip_meaning"] = json!(TIP_MEANING);
    section["counts"]["tip_tracked"] = json!(tips.len());
    section["counts"]["tip_in_force"] = json!(in_force);
    section["counts"]["tip_not_in_force"] = json!(tips.len() - in_force);
    section["tip_notices"] = json!(
        tips.iter()
            .map(|pinned| {
                let mut notice = pinned.record["notice"].clone();
                notice["function"] = json!(pinned.function);
                notice
            })
            .collect::<Vec<_>>()
    );
    section["assumptions"]["tip_tracked"] = json!(
        tips.iter()
            .filter(|pinned| pinned.in_force)
            .map(|pinned| {
                let tip = &pinned.record["tip"];
                json!({
                    "name": format!("{TIP_ASSUMPTION_PREFIX}{}", pinned.function),
                    "function": pinned.function,
                    "statement": TIP_STATEMENT,
                    "repository": tip["repository"],
                    "ref": tip["ref"],
                    "release_label": tip["release_label"],
                    "resolved_commit": tip["resolved_commit"],
                    "reviewed_commit": tip["reviewed_commit"],
                    "moved_since_review": tip["moved_since_review"],
                    "sites_changed_since_review": tip["sites_changed_since_review"],
                })
            })
            .collect::<Vec<_>>()
    );
}

/// Card CONTRACT-PIN-TIP-MODE: the console lines of a section's tip notices,
/// `contract revision notice [SEVERITY]: MESSAGE`, in evaluation order.
#[must_use]
pub fn notice_lines(section: &Value) -> Vec<String> {
    section["tip_notices"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|notice| {
            format!(
                "contract revision notice [{}]: {}",
                notice["severity"].as_str().unwrap_or_default(),
                notice["message"].as_str().unwrap_or_default()
            )
        })
        .collect()
}

/// The `contract_revision` section for one finished report.
#[must_use]
pub fn section(
    contracts: &ResourceContracts,
    evaluation: &PinEvaluation,
    report: &AuditReport,
) -> Value {
    render(
        &template(contracts, evaluation),
        &ReportFacts::of_rows(report.unit_rows()),
    )
}

/// The one-line console tally of a [`section`].
#[must_use]
pub fn tally(section: &Value) -> String {
    let count = |key: &str| section["counts"][key].as_u64().unwrap_or_default();
    // Card CONTRACT-PIN-TIP-MODE: the tip clause and one line per notice,
    // only when the run evaluated a tip entry.
    let mut tips = String::new();
    if section["counts"].get("tip_tracked").is_some() {
        tips = format!(
            "; {} tracking the tip of an unreleased branch ({} in force, {} not in force)",
            count("tip_tracked"),
            count("tip_in_force"),
            count("tip_not_in_force")
        );
        for line in notice_lines(section) {
            tips.push('\n');
            tips.push_str(&line);
        }
    }
    format!(
        "contract revision: {} pinned ({} in force, {} not in force), {} flexible checked, {} \
         flexible trusted, {} libc under the standing assumption; {} function(s) and {} call \
         site(s) held Pending ({CONTRACT_PIN_MISMATCH}){tips}",
        count("pinned"),
        count("pinned_in_force"),
        count("pinned_not_in_force"),
        count("flexible_checked"),
        count("flexible_trusted"),
        count("standing_libc"),
        count("held_functions"),
        count("held_call_sites"),
    )
}
