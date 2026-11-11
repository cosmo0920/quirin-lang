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

//! Tip identities of pinned contracts (card **CONTRACT-PIN-TIP-MODE**,
//! contract spec §C06.2).
//!
//! A pinned declaration's lock entry holds either reviewed identities
//! ([`crate::contract_revision::PinnedEntry`]) or one **tip identity**
//! ([`TipEntry`]): a repository, a ref (a branch) and the commit a person
//! reviewed. Before any unit is analysed the audit resolves the ref's current
//! commit ([`TipResolver::resolve`]) and the entry is in force exactly when
//! the contract text is the pinned one, the tip resolved, and the sites read
//! from that commit's blobs equal the audited ones ([`tip_decision`]). Nothing
//! is in force by default: a repository not located, a missing ref, a tip not
//! descended from the reviewed commit, an unreadable blob or a tip with no
//! site of the function leave it out of force.
//!
//! The repository is located through [`TIP_REPOSITORIES_VARIABLE`]
//! (`NAME=PATH` entries in the platform's path-list form), then the lock's
//! recorded path; its URL is never fetched. Only read-only Git commands run
//! in it (`rev-parse`, `cat-file`, `merge-base --is-ancestor`, `ls-tree`).
//!
//! Every tip entry carries a [`Notice`]: the contract targets a future,
//! unreleased release, with severity `info` exactly when the entry is in
//! force for the audited source and `warning` otherwise. The notice is
//! presentation and evidence; no verdict reads it. The Lean model is
//! `QCore.ContractTipMode`, whose `tipVectors` are [`TIP_VECTORS`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::c_identity::{SourceSite, SourceTree};

/// The environment variable that locates tip repositories on a host.
pub const TIP_REPOSITORIES_VARIABLE: &str = "QUIRIN_TIP_REPOSITORIES";

/// The prefix of a tip entry's named assumption.
pub const TIP_ASSUMPTION_PREFIX: &str = "tip-tracked:";

/// What every notice states.
pub const NOTICE_TARGET: &str =
    "targets a future, unreleased release (a release candidate), not a released version";

/// What a tip entry's named assumption states.
pub const TIP_STATEMENT: &str = "the contract tracks the tip of a branch rather than a reviewed revision: it is in force because the audited source's sites equal those at the branch's current commit, which no person may have reviewed when the tip moved since review; the contract targets a future, unreleased release (a release candidate), not a released version";

/// A notice's severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeSeverity {
    Info,
    Warning,
}

impl NoticeSeverity {
    /// The spelling, identical to the serde representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
        }
    }
}

/// The decision for one tip entry: whether its contract text is the pinned
/// one, whether the tip resolved and whether the tip's sites equal the
/// audited ones, to (in force, notice severity). The Lean `decision`.
#[must_use]
pub const fn tip_decision(
    text_matches: bool,
    resolved: bool,
    sites_equal: bool,
) -> (bool, NoticeSeverity) {
    if text_matches && resolved && sites_equal {
        (true, NoticeSeverity::Info)
    } else {
        (false, NoticeSeverity::Warning)
    }
}

/// `TIP_VECTORS` = the Lean `tipVectors`: (text matches, resolved, sites
/// equal, in force, severity).
pub const TIP_VECTORS: [(bool, bool, bool, bool, NoticeSeverity); 8] = [
    (false, false, false, false, NoticeSeverity::Warning),
    (false, false, true, false, NoticeSeverity::Warning),
    (false, true, false, false, NoticeSeverity::Warning),
    (false, true, true, false, NoticeSeverity::Warning),
    (true, false, false, false, NoticeSeverity::Warning),
    (true, false, true, false, NoticeSeverity::Warning),
    (true, true, false, false, NoticeSeverity::Warning),
    (true, true, true, true, NoticeSeverity::Info),
];

/// The repository a tip entry tracks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TipRepository {
    /// The name [`TIP_REPOSITORIES_VARIABLE`] locates it by.
    pub name: String,
    /// Where it is published; informational, never fetched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The absolute local path where `pin` found it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// What the person reviewed: a commit and the function's sites there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TipReview {
    pub commit: String,
    pub sites: Vec<SourceSite>,
}

/// One tip entry of a lock (format version 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TipEntry {
    pub function: String,
    pub module: String,
    pub contract_sha256: String,
    /// Card CONTRACT-SUPERSESSION: when the entry's current text was last
    /// reviewed (lock version 4 only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<String>,
    pub repository: TipRepository,
    /// The full refname tracked (`refs/heads/BRANCH`).
    #[serde(rename = "ref")]
    pub reference: String,
    /// A free-text label of the release the contract targets; shown, never
    /// read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_label: Option<String>,
    pub reviewed: TipReview,
}

/// Whether a repository name is well formed.
#[must_use]
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn hex_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn portable_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    Path::new(path).is_absolute()
        || path.starts_with('/')
        || path.starts_with("\\\\")
        || path.starts_with("//")
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

impl TipEntry {
    /// Validate the entry's shape (the lock's reader calls it).
    ///
    /// # Errors
    ///
    /// A malformed entry, naming the function.
    pub fn validate(&self) -> Result<(), String> {
        let malformed = |what: &str| format!("malformed tip entry for `{}`: {what}", self.function);
        let hex = |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
        if !hex(&self.contract_sha256) {
            return Err(malformed("contract digest"));
        }
        if !valid_name(&self.repository.name) {
            return Err(malformed("repository name"));
        }
        if self
            .repository
            .path
            .as_deref()
            .is_some_and(|path| !portable_absolute(path))
        {
            return Err(malformed("repository path is not absolute"));
        }
        if !self.reference.starts_with("refs/") || self.reference.len() <= "refs/".len() {
            return Err(malformed("ref is not a full refname"));
        }
        if !hex_commit(&self.reviewed.commit) {
            return Err(malformed("reviewed commit"));
        }
        if self.reviewed.sites.is_empty()
            || self.reviewed.sites.windows(2).any(|pair| pair[0] > pair[1])
            || self.reviewed.sites.iter().any(|site| {
                !hex(&site.declaration_sha256)
                    || site.body_sha256.as_deref().is_some_and(|body| !hex(body))
            })
        {
            return Err(malformed("reviewed sites"));
        }
        Ok(())
    }
}

/// Why a tip could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TipFailure {
    /// No repository located, or the location is not a Git repository.
    RepositoryMissing,
    /// The ref does not name a commit there.
    RefMissing,
    /// The tip does not descend from the reviewed commit.
    Diverged,
    /// A Git read failed.
    Unreadable,
    /// The tip has no site of the function.
    NoSite,
}

impl TipFailure {
    /// The spelling, identical to the serde representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RepositoryMissing => "repository-missing",
            Self::RefMissing => "ref-missing",
            Self::Diverged => "diverged",
            Self::Unreadable => "unreadable",
            Self::NoSite => "no-site",
        }
    }
}

/// What resolving one tip entry found, recorded in the sidecar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TipResolution {
    pub repository: TipRepository,
    #[serde(rename = "ref")]
    pub reference: String,
    pub release_label: Option<String>,
    /// `environment` or `lock`, when located.
    pub located_by: Option<String>,
    /// The path used, when located.
    pub located_at: Option<String>,
    /// The ref's current commit, when resolved.
    pub resolved_commit: Option<String>,
    /// The function's sites at the resolved commit (empty when unresolved).
    pub tip_sites: Vec<SourceSite>,
    /// The commit the person reviewed.
    pub reviewed_commit: String,
    /// Whether the tip moved since review (when resolved).
    pub moved_since_review: Option<bool>,
    /// Whether the tip's sites changed since review (when resolved).
    pub sites_changed_since_review: Option<bool>,
    pub failure: Option<TipFailure>,
    pub failure_detail: Option<String>,
}

impl TipResolution {
    /// Whether the tip resolved.
    #[must_use]
    pub const fn resolved(&self) -> bool {
        self.failure.is_none()
    }

    /// `ref` in `name` (URL).
    fn place(&self) -> String {
        let mut place = format!(
            "the tip of `{}` in repository `{}`",
            self.reference, self.repository.name
        );
        if let Some(url) = &self.repository.url {
            place.push_str(&format!(" ({url})"));
        }
        place
    }
}

/// Locates tip repositories and resolves tip entries, caching per entry.
#[derive(Clone, Debug, Default)]
pub struct TipResolver {
    locations: BTreeMap<String, PathBuf>,
    cache: BTreeMap<(String, String, String, String), TipResolution>,
}

pub(crate) fn git_output(root: &Path, args: &[&str]) -> Result<(i32, String), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| format!("git {}: {error}", args.join(" ")))?;
    Ok((
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
    ))
}

/// Whether `path` is the top of a Git repository (a checkout or a bare
/// repository), so a Git command there never reads an enclosing repository.
pub(crate) fn is_repository(path: &Path) -> bool {
    path.join(".git").exists() || (path.join("objects").is_dir() && path.join("HEAD").is_file())
}

impl TipResolver {
    /// The resolver this process's environment configures.
    ///
    /// # Errors
    ///
    /// A malformed [`TIP_REPOSITORIES_VARIABLE`] refuses.
    pub fn from_environment() -> Result<Self, String> {
        match std::env::var_os(TIP_REPOSITORIES_VARIABLE) {
            None => Ok(Self::default()),
            Some(value) if value.is_empty() => Ok(Self::default()),
            Some(value) => {
                let mut locations = BTreeMap::new();
                for entry in std::env::split_paths(&value) {
                    let text = entry.to_string_lossy().into_owned();
                    let malformed = || {
                        format!(
                            "malformed {TIP_REPOSITORIES_VARIABLE} entry `{text}`: expected \
                             NAME=ABSOLUTE-PATH"
                        )
                    };
                    let (name, path) = text.split_once('=').ok_or_else(malformed)?;
                    if !valid_name(name) || !Path::new(path).is_absolute() {
                        return Err(malformed());
                    }
                    if locations
                        .insert(name.to_owned(), PathBuf::from(path))
                        .is_some()
                    {
                        return Err(format!(
                            "{TIP_REPOSITORIES_VARIABLE} names repository `{name}` twice"
                        ));
                    }
                }
                Ok(Self {
                    locations,
                    cache: BTreeMap::new(),
                })
            }
        }
    }

    /// Resolve one tip entry (cached).
    pub fn resolve(&mut self, entry: &TipEntry) -> TipResolution {
        let key = (
            entry.function.clone(),
            entry.repository.name.clone(),
            entry.reference.clone(),
            entry.reviewed.commit.clone(),
        );
        if let Some(found) = self.cache.get(&key) {
            return found.clone();
        }
        let resolution = self.resolve_uncached(entry);
        self.cache.insert(key, resolution.clone());
        resolution
    }

    fn resolve_uncached(&self, entry: &TipEntry) -> TipResolution {
        let mut resolution = TipResolution {
            repository: entry.repository.clone(),
            reference: entry.reference.clone(),
            release_label: entry.release_label.clone(),
            located_by: None,
            located_at: None,
            resolved_commit: None,
            tip_sites: Vec::new(),
            reviewed_commit: entry.reviewed.commit.clone(),
            moved_since_review: None,
            sites_changed_since_review: None,
            failure: None,
            failure_detail: None,
        };
        let fail = |mut resolution: TipResolution, failure: TipFailure, detail: String| {
            resolution.failure = Some(failure);
            resolution.failure_detail = Some(detail);
            resolution
        };
        let name = &entry.repository.name;
        let (by, path) = match (
            self.locations.get(name),
            entry.repository.path.as_ref().map(PathBuf::from),
        ) {
            (Some(path), _) => ("environment", path.clone()),
            (None, Some(path)) => ("lock", path),
            (None, None) => {
                return fail(
                    resolution,
                    TipFailure::RepositoryMissing,
                    format!(
                        "repository `{name}` is not located on this host (set \
                         {TIP_REPOSITORIES_VARIABLE}={name}=PATH)"
                    ),
                );
            }
        };
        resolution.located_by = Some(by.to_owned());
        resolution.located_at = Some(path.display().to_string());
        if !is_repository(&path) {
            return fail(
                resolution,
                TipFailure::RepositoryMissing,
                format!(
                    "repository `{name}` is not a Git repository at {} (located by the {by})",
                    path.display()
                ),
            );
        }
        let commit = match git_output(
            &path,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{}^{{commit}}", entry.reference),
            ],
        ) {
            Ok((0, commit)) if hex_commit(&commit) => commit,
            Ok(_) => {
                return fail(
                    resolution,
                    TipFailure::RefMissing,
                    format!(
                        "`{}` names no commit in {}",
                        entry.reference,
                        path.display()
                    ),
                );
            }
            Err(error) => return fail(resolution, TipFailure::Unreadable, error),
        };
        resolution.resolved_commit = Some(commit.clone());
        match git_output(
            &path,
            &[
                "cat-file",
                "-e",
                &format!("{}^{{commit}}", entry.reviewed.commit),
            ],
        ) {
            Ok((0, _)) => {}
            Ok(_) => {
                return fail(
                    resolution,
                    TipFailure::Diverged,
                    format!(
                        "the reviewed commit {} is not in the repository at {}",
                        entry.reviewed.commit,
                        path.display()
                    ),
                );
            }
            Err(error) => return fail(resolution, TipFailure::Unreadable, error),
        }
        match git_output(
            &path,
            &[
                "merge-base",
                "--is-ancestor",
                &entry.reviewed.commit,
                &commit,
            ],
        ) {
            Ok((0, _)) => {}
            Ok((1, _)) => {
                return fail(
                    resolution,
                    TipFailure::Diverged,
                    format!(
                        "the tip {commit} does not descend from the reviewed commit {}",
                        entry.reviewed.commit
                    ),
                );
            }
            Ok((code, _)) => {
                return fail(
                    resolution,
                    TipFailure::Unreadable,
                    format!("git merge-base --is-ancestor exited {code}"),
                );
            }
            Err(error) => return fail(resolution, TipFailure::Unreadable, error),
        }
        let names = BTreeSet::from([entry.function.clone()]);
        let sites = match SourceTree::read_sites_at(&path, &commit, &names) {
            Ok(scan) => scan.sites.get(&entry.function).cloned().unwrap_or_default(),
            Err(error) => return fail(resolution, TipFailure::Unreadable, error),
        };
        if sites.is_empty() {
            return fail(
                resolution,
                TipFailure::NoSite,
                format!("the tip {commit} has no site of `{}`", entry.function),
            );
        }
        resolution.moved_since_review = Some(commit != entry.reviewed.commit);
        resolution.sites_changed_since_review = Some(sites != entry.reviewed.sites);
        resolution.tip_sites = sites;
        resolution
    }
}

/// Where a notice stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeStanding<'a> {
    /// In force for the audited source.
    InForce,
    /// Not in force for the audited source, and why.
    NotInForce(&'a str),
    /// No audited source (`pin --verify-contracts`, `pin --track-tip`).
    NoSource,
}

/// A notice: severity and message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notice {
    pub severity: NoticeSeverity,
    pub message: String,
}

impl Notice {
    /// `notice [SEVERITY]: MESSAGE`.
    #[must_use]
    pub fn line(&self) -> String {
        format!("notice [{}]: {}", self.severity.as_str(), self.message)
    }
}

/// The notice of one tip entry. Its severity is `info` exactly when the
/// entry is in force for the audited source; with no audited source, `info`
/// exactly when the tip resolves on this host.
#[must_use]
pub fn notice(function: &str, tip: &TipResolution, standing: NoticeStanding<'_>) -> Notice {
    let mut message = format!(
        "the contract for `{function}` {NOTICE_TARGET}: it tracks {}",
        tip.place()
    );
    if let Some(label) = &tip.release_label {
        message.push_str(&format!(" (release: {label})"));
    }
    match &tip.resolved_commit {
        Some(commit) if tip.resolved() => {
            message.push_str(&format!(", resolved to commit {commit}"));
            if tip.moved_since_review == Some(true) {
                message.push_str(&format!(
                    "; the tip moved since review at {}, so commit {commit} was not reviewed",
                    tip.reviewed_commit
                ));
            }
        }
        Some(commit) => message.push_str(&format!(", at commit {commit}")),
        None => message.push_str(", unresolved"),
    }
    let severity = match standing {
        NoticeStanding::InForce => {
            message.push_str("; in force for the audited source, whose sites equal the tip's");
            NoticeSeverity::Info
        }
        NoticeStanding::NotInForce(why) => {
            message.push_str(&format!("; NOT in force for the audited source: {why}"));
            NoticeSeverity::Warning
        }
        NoticeStanding::NoSource if tip.resolved() => {
            message.push_str("; in force only where the audited source's sites equal the tip's");
            NoticeSeverity::Info
        }
        NoticeStanding::NoSource => {
            message.push_str(&format!(
                "; NOT in force anywhere on this host: {}",
                tip.failure_detail
                    .as_deref()
                    .unwrap_or("the tip cannot be resolved")
            ));
            NoticeSeverity::Warning
        }
    };
    Notice { severity, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEAN: &str = include_str!("../../../proof/QCore/ContractTipMode.lean");

    #[test]
    fn recorded_repository_paths_are_absolute_on_either_host_family() {
        for path in [
            "/home/reviewer/project",
            r"C:\Users\reviewer\project",
            r"\\server\share\project",
        ] {
            assert!(portable_absolute(path), "{path}");
        }
        for path in ["project", r"C:project", "../project"] {
            assert!(!portable_absolute(path), "{path}");
        }
    }

    #[test]
    fn tip_decision_is_its_table() {
        for (text, resolved, equal, in_force, severity) in TIP_VECTORS {
            assert_eq!(tip_decision(text, resolved, equal), (in_force, severity));
        }
        let distinct = TIP_VECTORS
            .iter()
            .map(|(t, r, e, _, _)| (*t, *r, *e))
            .collect::<BTreeSet<_>>();
        assert_eq!(distinct.len(), TIP_VECTORS.len());
    }

    /// The mechanical binding: `tipVectors` = `TIP_VECTORS`.
    #[test]
    fn tip_lean_vectors_match() {
        let start = "def tipVectors : List (Bool × Bool × Bool × Bool × Severity) := [";
        let body = LEAN.split(start).nth(1).expect("tipVectors");
        let body = &body[..body.find("\n]").unwrap()];
        let rows = body
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
            .map(|line| {
                let fields = line
                    .trim_end_matches(',')
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .split(", ")
                    .collect::<Vec<_>>();
                let severity = match fields[4] {
                    ".info" => NoticeSeverity::Info,
                    ".warning" => NoticeSeverity::Warning,
                    other => panic!("{other}"),
                };
                (
                    fields[0] == "true",
                    fields[1] == "true",
                    fields[2] == "true",
                    fields[3] == "true",
                    severity,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, TIP_VECTORS.to_vec());
    }

    fn resolution(resolved: bool, moved: bool) -> TipResolution {
        TipResolution {
            repository: TipRepository {
                name: "monkey".into(),
                url: Some("https://example.invalid/monkey".into()),
                path: None,
            },
            reference: "refs/heads/fix".into(),
            release_label: Some("next release".into()),
            located_by: None,
            located_at: None,
            resolved_commit: resolved.then(|| "b".repeat(40)),
            tip_sites: Vec::new(),
            reviewed_commit: "a".repeat(40),
            moved_since_review: resolved.then_some(moved),
            sites_changed_since_review: resolved.then_some(moved),
            failure: (!resolved).then_some(TipFailure::RefMissing),
            failure_detail: (!resolved).then(|| "`refs/heads/fix` names no commit".to_owned()),
        }
    }

    #[test]
    fn a_notice_names_everything_with_its_severity() {
        let in_force = notice("f", &resolution(true, true), NoticeStanding::InForce);
        assert_eq!(in_force.severity, NoticeSeverity::Info);
        for part in [
            "`f`",
            NOTICE_TARGET,
            "`refs/heads/fix`",
            "`monkey`",
            "https://example.invalid/monkey",
            "next release",
            &"b".repeat(40),
            "was not reviewed",
        ] {
            assert!(
                in_force.message.contains(part),
                "{part}: {}",
                in_force.message
            );
        }
        let held = notice(
            "f",
            &resolution(true, false),
            NoticeStanding::NotInForce("its sites differ"),
        );
        assert_eq!(held.severity, NoticeSeverity::Warning);
        assert!(
            held.message
                .contains("NOT in force for the audited source: its sites differ")
        );
        assert!(!held.message.contains("was not reviewed"));
        let nowhere = notice("f", &resolution(false, false), NoticeStanding::NoSource);
        assert_eq!(nowhere.severity, NoticeSeverity::Warning);
        assert!(nowhere.message.contains("unresolved"));
        assert!(nowhere.message.contains("names no commit"));
        let here = notice("f", &resolution(true, false), NoticeStanding::NoSource);
        assert_eq!(here.severity, NoticeSeverity::Info);
        assert!(here.line().starts_with("notice [info]: "));
    }
}
