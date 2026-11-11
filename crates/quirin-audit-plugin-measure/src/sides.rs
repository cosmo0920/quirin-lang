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

//! Measurement sides: what each side measures and how it is prepared and run.
//!
//! A side is `NAME=SPEC` ([`SideSpec::parse`]):
//!
//! | Spec | Side |
//! |---|---|
//! | `rev:<rev>` (or any other text git resolves) | a `git archive` snapshot of the revision, its submodules cloned at their pinned commits, built and run at one fixed path |
//! | an existing directory (`.`) | that checkout, built in place (`cargo build --release -p qcore-audit`) |
//! | an existing file | that `quirin-audit` binary, run from the checkout it was built in (`<checkout>/target/release/quirin-audit`) |
//! | `baseline:<sha>` | a published baseline, reused after identity verification ([`crate::baseline`]) |
//! | `run:<dir>` | a side directory a previous `measure` wrote, read as it is |
//!
//! The snapshot hazards of ledger entry MEASURE-SNAPSHOT-HAZARDS are handled
//! here: (1) every gitlink of the revision is cloned at its pinned commit
//! (`git archive` omits `third_party/libevent`); (2) every route runs with a
//! private working directory, so transient retained products never land in
//! the snapshot or in another route's inputs; (3) VCS metadata is excluded from
//! the before/after input identity; (4) the analyzer build commit is set
//! explicitly (`QCORE_AUDIT_BUILD_COMMIT` = the measured revision, or
//! `--build-commit`), never read from whatever repository encloses the
//! snapshot; (5) a snapshot is built and run in a private mount namespace
//! (`unshare -rm`) with its tree bound at one fixed path (`<mount
//! root>/src`, and a re-spelled mirror at `<mount root>/mirror`), so sides
//! that differ only in source spell every contract-source path alike.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use crate::inputs::{Route, RouteKind};
use crate::mirror::Spelled;
use crate::util::{
    capture, identity_digest, is_full_revision, logged, now, sha_file, tree_identity, write_json,
};

/// The environment variable a rule-switch-capable analyzer reads
/// (card RULE-SWITCHES, plan seam B).
pub const RULES_OFF_VARIABLE: &str = "QCORE_GATEWAY_RULES_OFF";

/// Top-level directories of a source tree that are never inputs (build
/// output and private measurement directories).
pub const NOT_INPUTS: [&str; 3] = ["target", "private", "measure"];

/// Clang expands `__DATE__`/`__TIME__` from this variable when it is set.
/// Fluent Bit's `lib/monkey/mk_bin/monkey.c` expands both, and the
/// lifetime-source pre-pass keys each unit by a digest of its preprocessed
/// text, so without one fixed value that key changes with the clock (ledger
/// entries LIFETIME-SOURCE-IDENTITY-TIME-MACROS, PREPASS-CLOCK-DEPENDENT-KEY;
/// CFG-RELEASE measured the fix). Every side of a measurement gets the same
/// value; it is recorded and part of a baseline's inputs.
pub const SOURCE_DATE_EPOCH: &str = "SOURCE_DATE_EPOCH";

/// The default `SOURCE_DATE_EPOCH`: 2026-01-01T00:00:00Z.
pub const DEFAULT_SOURCE_DATE_EPOCH: u64 = 1_767_225_600;

/// The slot script's location variables (`scripts/gate_slot.py`), passed to
/// every route when the plugin received them.
pub const GATE_VARIABLES: [&str; 3] = [
    "QUIRIN_GATE_DIR",
    "QUIRIN_GATE_LEGACY_LOCK",
    "QUIRIN_GATE_MEMINFO",
];

/// How often a running route's slot-wait record is read.
const WAIT_POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// The `audit` slot budget every route takes (`gate_slot.py --memory-budget
/// 7G`), in the audit's MiB planning units.
pub const SLOT_BUDGET_MIB: u32 = 7168;

/// A side's specification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SideSpec {
    Rev(String),
    Checkout(PathBuf),
    Binary(PathBuf),
    Baseline(String),
    Run(PathBuf),
}

impl SideSpec {
    /// Parse `SPEC` (relative paths against `invocation`).
    ///
    /// # Errors
    ///
    /// An empty specification.
    pub fn parse(text: &str, invocation: &Path) -> Result<Self, String> {
        if text.is_empty() {
            return Err("an empty side specification".into());
        }
        if let Some(sha) = text.strip_prefix("baseline:") {
            return Ok(Self::Baseline(sha.into()));
        }
        if let Some(directory) = text.strip_prefix("run:") {
            return Ok(Self::Run(invocation.join(directory)));
        }
        if let Some(revision) = text.strip_prefix("rev:") {
            return Ok(Self::Rev(revision.into()));
        }
        let path = invocation.join(text);
        if path.is_file() {
            Ok(Self::Binary(path))
        } else if path.is_dir() {
            Ok(Self::Checkout(path))
        } else {
            Ok(Self::Rev(text.into()))
        }
    }

    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Rev(_) => "rev",
            Self::Checkout(_) => "checkout",
            Self::Binary(_) => "binary",
            Self::Baseline(_) => "baseline",
            Self::Run(_) => "run",
        }
    }

    /// Whether the side needs a build and a run.
    #[must_use]
    pub const fn runs(&self) -> bool {
        matches!(self, Self::Rev(_) | Self::Checkout(_) | Self::Binary(_))
    }
}

/// What every side shares.
#[derive(Clone, Debug)]
pub struct Context {
    pub repository: PathBuf,
    pub inputs_base: PathBuf,
    pub output: PathBuf,
    pub mount_root: PathBuf,
    pub jobs: u32,
    pub build_jobs: u32,
    /// `scripts/gate_slot.py` of the repository, or `None` (`--no-slots`).
    pub slot_script: Option<PathBuf>,
    pub build_commit: Option<String>,
    pub rules_off: Vec<String>,
    /// `SOURCE_DATE_EPOCH` for every route (`None`: unset).
    pub source_date_epoch: Option<u64>,
}

/// A side ready to run its routes.
#[derive(Clone, Debug)]
pub struct Built {
    pub name: String,
    pub spec: SideSpec,
    pub directory: PathBuf,
    /// The source tree on disk, and as the analyzer sees it.
    pub source_real: PathBuf,
    pub source_run: PathBuf,
    pub binary_real: PathBuf,
    pub binary_run: PathBuf,
    /// Whether the source is bound at the fixed path.
    pub snapshot: bool,
    pub revision: Option<String>,
    pub build_commit: Option<String>,
    pub record: Value,
}

/// Resolve a revision to its full commit name in `repository`.
///
/// # Errors
///
/// A revision git does not resolve to a commit.
pub fn resolve_revision(repository: &Path, revision: &str) -> Result<String, String> {
    let spelled = format!("{revision}^{{commit}}");
    let full = capture(
        "git",
        &["rev-parse", "--verify", "--quiet", &spelled],
        Some(repository),
    )
    .map_err(|_| format!("`{revision}` is not a commit of {}", repository.display()))?;
    let full = full.trim().to_owned();
    if !is_full_revision(&full) {
        return Err(format!(
            "`{revision}` resolved to `{full}`, not a commit name"
        ));
    }
    Ok(full)
}

/// The gitlinks (submodules) of a revision: `(path, commit)`.
///
/// # Errors
///
/// A failing `git ls-tree`.
pub fn gitlinks(repository: &Path, revision: &str) -> Result<Vec<(String, String)>, String> {
    let listing = capture("git", &["ls-tree", "-r", revision], Some(repository))?;
    Ok(listing
        .lines()
        .filter_map(|line| {
            let (meta, path) = line.split_once('\t')?;
            let mut parts = meta.split_whitespace();
            (parts.next()? == "160000" && parts.next()? == "commit")
                .then(|| (path.to_owned(), parts.next().unwrap_or_default().to_owned()))
        })
        .collect())
}

/// `cmd` wrapped to run in a private mount namespace with each `(real,
/// fixed)` bound, from `work`.
#[must_use]
pub fn namespaced(binds: &[(PathBuf, PathBuf)], work: &Path, command: &[String]) -> Vec<String> {
    if binds.is_empty() {
        return command.to_vec();
    }
    let mut script = String::from("set -e\n");
    let mut arguments = Vec::new();
    for (index, (real, fixed)) in binds.iter().enumerate() {
        script.push_str(&format!(
            "mount --bind \"${}\" \"${}\"\n",
            2 * index + 1,
            2 * index + 2
        ));
        arguments.push(real.display().to_string());
        arguments.push(fixed.display().to_string());
    }
    let work_index = 2 * binds.len() + 1;
    script.push_str(&format!(
        "cd \"${work_index}\"\nshift {work_index}\nexec \"$@\"\n"
    ));
    arguments.push(work.display().to_string());
    let mut wrapped = vec![
        "unshare".to_owned(),
        "-rm".to_owned(),
        "sh".to_owned(),
        "-c".to_owned(),
        script,
        "quirin-measure-ns".to_owned(),
    ];
    wrapped.extend(arguments);
    wrapped.extend(command.iter().cloned());
    wrapped
}

/// The environment every child starts from: the host's allow-list, as the
/// plugin received it.
#[must_use]
pub fn base_environment() -> BTreeMap<String, String> {
    quirin_audit_plugin::ENVIRONMENT_ALLOW_LIST
        .iter()
        .filter_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| ((*name).to_owned(), value))
        })
        .collect()
}

fn command_of(argv: &[String], environment: &BTreeMap<String, String>) -> Command {
    let mut command = Command::new(&argv[0]);
    command.args(&argv[1..]).env_clear().envs(environment);
    command
}

/// Prepare a running side: snapshot (rev), build, locate the binary.
///
/// # Errors
///
/// A revision that does not resolve, a submodule that cannot be cloned at its
/// commit, a failed build, a binary outside a checkout.
pub fn build(name: &str, spec: &SideSpec, context: &Context) -> Result<Built, String> {
    let directory = context.output.join("sides").join(name);
    fs::create_dir_all(&directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    let log = directory.join("build.log");
    let started = now();
    let mut record = json!({"side": name, "kind": spec.kind(), "build_started": started});
    let built = match spec {
        SideSpec::Rev(revision) => {
            let full = resolve_revision(&context.repository, revision)?;
            let source = directory.join("src");
            fs::create_dir(&source).map_err(|error| format!("{}: {error}", source.display()))?;
            let tar = directory.join("src.tar");
            capture(
                "git",
                &[
                    "archive",
                    "--format=tar",
                    "-o",
                    &tar.display().to_string(),
                    &full,
                ],
                Some(&context.repository),
            )?;
            capture(
                "tar",
                &[
                    "-xf",
                    &tar.display().to_string(),
                    "-C",
                    &source.display().to_string(),
                ],
                None,
            )?;
            fs::remove_file(&tar).map_err(|error| format!("{}: {error}", tar.display()))?;
            let mut submodules = BTreeMap::new();
            for (path, commit) in gitlinks(&context.repository, &full)? {
                let origin = context.repository.join(&path);
                let check = format!("{commit}^{{commit}}");
                capture("git", &["cat-file", "-e", &check], Some(&origin)).map_err(|_| {
                    format!(
                        "submodule {path}: {} has no commit {commit}; run `git submodule update \
                         --init --recursive` in the repository",
                        origin.display()
                    )
                })?;
                let target = source.join(&path);
                if target.is_dir() {
                    fs::remove_dir(&target)
                        .map_err(|error| format!("{}: {error}", target.display()))?;
                }
                capture(
                    "git",
                    &[
                        "clone",
                        "--quiet",
                        "--no-checkout",
                        &origin.display().to_string(),
                        &target.display().to_string(),
                    ],
                    None,
                )?;
                capture(
                    "git",
                    &["checkout", "--quiet", "--detach", &commit],
                    Some(&target),
                )?;
                let head = capture("git", &["rev-parse", "HEAD"], Some(&target))?;
                if head.trim() != commit {
                    return Err(format!("submodule {path} checked out {head}, not {commit}"));
                }
                submodules.insert(path, commit);
            }
            let fixed = context.mount_root.join("src");
            let build_commit = context.build_commit.clone().unwrap_or_else(|| full.clone());
            let mut environment = base_environment();
            environment.insert("CARGO_BUILD_JOBS".into(), context.build_jobs.to_string());
            environment.insert("CARGO_INCREMENTAL".into(), "0".into());
            environment.insert("QCORE_AUDIT_BUILD_COMMIT".into(), build_commit.clone());
            let argv = namespaced(
                &[(source.clone(), fixed.clone())],
                &fixed,
                &[
                    "cargo".into(),
                    "build".into(),
                    "--release".into(),
                    "--locked".into(),
                    "-p".into(),
                    "qcore-audit".into(),
                    "--bin".into(),
                    "quirin-audit".into(),
                ],
            );
            let code = logged(&mut command_of(&argv, &environment), &log)?;
            record["build_command"] = json!(argv);
            record["build_exit"] = json!(code);
            if code != 0 {
                return Err(format!(
                    "side {name}: the release build failed (exit {code}); see {}",
                    log.display()
                ));
            }
            record["submodules"] = json!(submodules);
            Built {
                name: name.into(),
                spec: spec.clone(),
                directory: directory.clone(),
                binary_real: source.join("target/release/quirin-audit"),
                binary_run: fixed.join("target/release/quirin-audit"),
                source_real: source,
                source_run: fixed,
                snapshot: true,
                revision: Some(full),
                build_commit: Some(build_commit),
                record,
            }
        }
        SideSpec::Checkout(checkout) => {
            let checkout = checkout
                .canonicalize()
                .map_err(|error| format!("{}: {error}", checkout.display()))?;
            let mut environment = base_environment();
            environment.insert("CARGO_BUILD_JOBS".into(), context.build_jobs.to_string());
            environment.insert("CARGO_INCREMENTAL".into(), "0".into());
            if let Some(commit) = &context.build_commit {
                environment.insert("QCORE_AUDIT_BUILD_COMMIT".into(), commit.clone());
            }
            let argv: Vec<String> = [
                "cargo",
                "build",
                "--release",
                "--locked",
                "-p",
                "qcore-audit",
                "--bin",
                "quirin-audit",
            ]
            .map(str::to_owned)
            .to_vec();
            let mut command = command_of(&argv, &environment);
            command.current_dir(&checkout);
            let code = logged(&mut command, &log)?;
            record["build_command"] = json!(argv);
            record["build_exit"] = json!(code);
            if code != 0 {
                return Err(format!(
                    "side {name}: the release build failed (exit {code}); see {}",
                    log.display()
                ));
            }
            let (revision, _) = checkout_revision(&checkout);
            Built {
                name: name.into(),
                spec: spec.clone(),
                directory: directory.clone(),
                binary_real: checkout.join("target/release/quirin-audit"),
                binary_run: checkout.join("target/release/quirin-audit"),
                source_real: checkout.clone(),
                source_run: checkout,
                snapshot: false,
                revision,
                build_commit: context.build_commit.clone(),
                record,
            }
        }
        SideSpec::Binary(binary) => {
            let binary = binary
                .canonicalize()
                .map_err(|error| format!("{}: {error}", binary.display()))?;
            let checkout = binary
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
                .filter(|root| {
                    binary.ends_with("target/release/quirin-audit")
                        && root.join("scripts/run_pinned_audit.py").is_file()
                        && root.join("contract/profiles").is_dir()
                })
                .ok_or_else(|| {
                    format!(
                        "side {name}: {} is not <checkout>/target/release/quirin-audit of a QCore-C \
                         checkout; the routes read that checkout's runner and profiles",
                        binary.display()
                    )
                })?
                .to_path_buf();
            let (revision, _) = checkout_revision(&checkout);
            Built {
                name: name.into(),
                spec: spec.clone(),
                directory: directory.clone(),
                binary_real: binary.clone(),
                binary_run: binary,
                source_real: checkout.clone(),
                source_run: checkout,
                snapshot: false,
                revision,
                build_commit: None,
                record,
            }
        }
        SideSpec::Baseline(_) | SideSpec::Run(_) => {
            return Err(format!("side {name}: a {} side is not built", spec.kind()));
        }
    };
    let mut built = built;
    built.record["build_ended"] = json!(now());
    built.record["revision"] = json!(built.revision);
    built.record["build_commit"] = json!(built.build_commit);
    built.record["source"] = json!(built.source_real.display().to_string());
    built.record["source_as_run"] = json!(built.source_run.display().to_string());
    built.record["snapshot_at_fixed_path"] = json!(built.snapshot);
    built.record["binary"] = json!({
        "path": built.binary_real.display().to_string(),
        "path_as_run": built.binary_run.display().to_string(),
        "sha256": sha_file(&built.binary_real)?,
    });
    built.record["bundles"] = json!(bundle_digests(&built.source_real)?);
    Ok(built)
}

/// The revision a checkout measures: `HEAD` when the tree is clean, else
/// `None` (a dirty tree is not a revision). The second value is `HEAD`.
/// Only a checkout that is itself a repository's top level has a revision:
/// a directory inside some other repository does not inherit that one's
/// `HEAD` (MEASURE-SNAPSHOT-HAZARDS item 4).
#[must_use]
pub fn checkout_revision(checkout: &Path) -> (Option<String>, Option<String>) {
    let top = capture("git", &["rev-parse", "--show-toplevel"], Some(checkout))
        .ok()
        .and_then(|text| Path::new(text.trim()).canonicalize().ok());
    if top.as_deref() != checkout.canonicalize().ok().as_deref() {
        return (None, None);
    }
    let head = capture(
        "git",
        &["rev-parse", "--verify", "HEAD^{commit}"],
        Some(checkout),
    )
    .ok()
    .map(|text| text.trim().to_owned());
    let clean = capture(
        "git",
        &["status", "--porcelain", "--untracked-files=no"],
        Some(checkout),
    )
    .is_ok_and(|text| text.trim().is_empty());
    (head.clone().filter(|_| clean), head)
}

/// Whether a contract-bundle path: `providers/<crate>/contracts/...` or
/// `contract/...`. Returns the bundle name.
#[must_use]
pub fn bundle_of(relative: &str) -> Option<String> {
    let parts: Vec<&str> = relative.split('/').collect();
    match parts.as_slice() {
        ["providers", krate, "contracts", _, ..] => Some(format!("providers/{krate}/contracts")),
        ["contract", _, ..] => Some("contract".into()),
        _ => None,
    }
}

/// The contract-bundle digests of a source tree: per bundle, the
/// [`identity_digest`] of its regular files.
///
/// # Errors
///
/// An unreadable tree.
pub fn bundle_digests(source: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut bundles: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for top in ["providers", "contract"] {
        if !source.join(top).is_dir() {
            continue;
        }
        for (path, digest) in tree_identity(&source.join(top), &[])? {
            let relative = format!("{top}/{path}");
            if let Some(bundle) = bundle_of(&relative) {
                let inner = relative[bundle.len() + 1..].to_owned();
                bundles.entry(bundle).or_default().insert(inner, digest);
            }
        }
    }
    Ok(bundles
        .into_iter()
        .map(|(bundle, identity)| (bundle, identity_digest(&identity)))
        .collect())
}

/// Whether the binary advertises the run-time rule switch: it contains the
/// variable's name. A binary that does not is refused before anything runs;
/// one that does is checked again after the run, against the components'
/// recorded off-lists ([`crate::compare`]).
///
/// # Errors
///
/// An unreadable binary.
pub fn advertises_rule_switch(binary: &Path) -> Result<bool, String> {
    let bytes = fs::read(binary).map_err(|error| format!("{}: {error}", binary.display()))?;
    let needle = RULES_OFF_VARIABLE.as_bytes();
    Ok(bytes.windows(needle.len()).any(|window| window == needle))
}

/// Run one route of a built side, as the copied driver
/// (`docs/reports/w48-bu-single-leaf-bodies/scripts/measure.py`) did: the same
/// environment and output layout, inputs recorded before and after, exit 101
/// when an input or the binary changed.
///
/// # Errors
///
/// A directory that cannot be created or a record that cannot be written.
pub fn run_route(
    side: &Built,
    route: &Route,
    spelled: &Spelled,
    mirror: Option<(&Path, &Path)>,
    context: &Context,
    job: &mut crate::waits::Job,
) -> Result<Value, String> {
    let out = side.directory.join(&route.name);
    fs::create_dir(&out)
        .map_err(|error| format!("{}: {error} (every run gets fresh paths)", out.display()))?;
    let work = side.directory.join(format!("work-{}", route.name));
    fs::create_dir_all(&work).map_err(|error| format!("{}: {error}", work.display()))?;
    let link_map = context.repository.join(&route.link_map.path);
    let mut environment = base_environment();
    environment.insert(
        "QCORE_PROFILES".into(),
        side.source_run
            .join("contract/profiles")
            .display()
            .to_string(),
    );
    environment.insert(
        "QCORE_CROSS_UNIT_LINK_MAP".into(),
        link_map.display().to_string(),
    );
    environment.insert("QCORE_CLANG".into(), route.clang.clone());
    environment.insert(
        "QCORE_NULL_PROPERTY".into(),
        out.join(format!("{}-null.jsonl", route.name))
            .display()
            .to_string(),
    );
    environment.insert(
        "QCORE_OUTER_PROJECTION".into(),
        out.join(format!("{}-outer.json", route.name))
            .display()
            .to_string(),
    );
    if !context.rules_off.is_empty() {
        environment.insert(RULES_OFF_VARIABLE.into(), context.rules_off.join(","));
    }
    if let Some(epoch) = context.source_date_epoch {
        environment.insert(SOURCE_DATE_EPOCH.into(), epoch.to_string());
    }
    // The slot script's own locations, when the host passed them
    // (`--plugin-env`): a test's scratch gate, never the host's real one.
    for name in GATE_VARIABLES {
        if let Ok(value) = std::env::var(name) {
            environment.insert(name.into(), value);
        }
    }
    let jobs = context.jobs.to_string();
    let command: Vec<String> = match &route.kind {
        RouteKind::PinnedCorpus { source_commit, .. } => vec![
            "python3".into(),
            side.source_run
                .join("scripts/run_pinned_audit.py")
                .display()
                .to_string(),
            out.join("corpus").display().to_string(),
            source_commit.clone(),
            "--inputs-root".into(),
            spelled
                .inputs_root
                .as_ref()
                .ok_or("a pinned route without an inputs root")?
                .display()
                .to_string(),
            "--jobs".into(),
            jobs,
        ],
        RouteKind::ConfiguredSelection {
            selection,
            profile_directory,
            ..
        } => vec![
            side.binary_run.display().to_string(),
            "current".into(),
            "--checkout".into(),
            spelled
                .checkout
                .as_ref()
                .ok_or("a configured route without a checkout")?
                .display()
                .to_string(),
            "--profile-directory".into(),
            side.source_run
                .join(profile_directory)
                .display()
                .to_string(),
            "--compile-database".into(),
            spelled
                .compile_database
                .as_ref()
                .ok_or("a configured route without a compile database")?
                .display()
                .to_string(),
            "--selection".into(),
            side.source_run.join(selection).display().to_string(),
            "--jobs".into(),
            jobs,
            "--report".into(),
            out.join("resource-diagnostics.json").display().to_string(),
        ],
    };
    // Under a slot, `gate_slot.py` would insert `--memory-budget-mib=N`, a
    // spelling `quirin-audit current` refuses; giving the slot's budget in
    // the two-word form the CLI reads makes the slot accept it unchanged.
    let mut command = command;
    if context.slot_script.is_some() && matches!(route.kind, RouteKind::ConfiguredSelection { .. })
    {
        command.splice(
            2..2,
            [
                "--memory-budget-mib".to_owned(),
                SLOT_BUDGET_MIB.to_string(),
            ],
        );
    }
    let mut binds = Vec::new();
    if side.snapshot {
        binds.push((side.source_real.clone(), side.source_run.clone()));
    }
    if let Some((real, fixed)) = mirror {
        binds.push((real.to_path_buf(), fixed.to_path_buf()));
    }
    let mut argv = namespaced(&binds, &work, &command);
    let receipt = side.directory.join(format!("{}.slot.json", route.name));
    let wait_record = side
        .directory
        .join(format!("{}.slot-wait.json", route.name));
    if let Some(script) = &context.slot_script {
        let mut slotted = vec![
            "python3".to_owned(),
            script.display().to_string(),
            "audit".into(),
            "--memory-budget".into(),
            "7G".into(),
            "--receipt".into(),
            receipt.display().to_string(),
            "--wait-record".into(),
            wait_record.display().to_string(),
            "--".into(),
        ];
        slotted.extend(argv);
        argv = slotted;
    }
    let externals: Vec<PathBuf> = spelled
        .databases
        .iter()
        .cloned()
        .chain([link_map.clone()])
        .collect();
    let digests = |paths: &[PathBuf]| -> Result<BTreeMap<String, String>, String> {
        paths
            .iter()
            .map(|path| Ok((path.display().to_string(), sha_file(path)?)))
            .collect()
    };
    let tracked: BTreeMap<String, String> = route
        .tracked
        .iter()
        .map(|path| Ok((path.clone(), sha_file(&side.source_real.join(path))?)))
        .collect::<Result<_, String>>()?;
    let identity_before = tree_identity(&side.source_real, &NOT_INPUTS)?;
    let mut record = json!({
        "route": route.name,
        "profile": route.profile,
        "route_digest": route.digest(),
        "command": argv,
        "environment": environment.iter()
            .filter(|(name, _)| name.starts_with("QCORE_") || *name == SOURCE_DATE_EPOCH
                || GATE_VARIABLES.contains(&name.as_str()))
            .collect::<BTreeMap<_, _>>(),
        "working_directory": work.display().to_string(),
        "jobs": context.jobs,
        "binary_sha256": sha_file(&side.binary_real)?,
        "link_map": link_map.display().to_string(),
        "tracked_inputs": tracked,
        "inputs_before": {"files": identity_before.len(), "digest": identity_digest(&identity_before)},
        "externals_before": digests(&externals)?,
        "started": now(),
    });
    write_json(&out.join("invocation.json"), &record)?;
    let stdout = fs::File::create(out.join("console.log")).map_err(|error| error.to_string())?;
    let stderr = fs::File::create(out.join("stderr.log")).map_err(|error| error.to_string())?;
    let started = std::time::Instant::now();
    let mut child = command_of(&argv, &environment)
        .current_dir(&work)
        .stdin(std::process::Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|error| format!("{}: {error}", argv[0]))?;
    // While the slot script says `waiting`, this job waits (R7); a record
    // that is missing or unreadable counts as running.
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("{}: {error}", argv[0]))?
        {
            break status;
        }
        if context.slot_script.is_some() {
            job.set_waiting(crate::waits::read_state(&wait_record) == Some(true));
        }
        std::thread::sleep(WAIT_POLL);
    };
    job.set_waiting(false);
    let wall = started.elapsed();
    let mut code = status.code().unwrap_or(-1);
    let identity_after = tree_identity(&side.source_real, &NOT_INPUTS)?;
    let externals_after = digests(&externals)?;
    let binary_after = sha_file(&side.binary_real)?;
    let stable = identity_after == identity_before
        && externals_after
            == record["externals_before"]
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .map(|(key, value)| {
                            (key.clone(), value.as_str().unwrap_or_default().to_owned())
                        })
                        .collect::<BTreeMap<_, _>>()
                })
                .unwrap_or_default()
        && record["binary_sha256"] == json!(binary_after);
    record["ended"] = json!(now());
    record["exit"] = json!(code);
    record["binary_sha256_after"] = json!(binary_after);
    record["inputs_after"] =
        json!({"files": identity_after.len(), "digest": identity_digest(&identity_after)});
    record["externals_after"] = json!(externals_after);
    if !stable {
        let changed: Vec<&String> = identity_before
            .keys()
            .chain(identity_after.keys())
            .filter(|path| identity_before.get(*path) != identity_after.get(*path))
            .collect();
        record["identity_error"] = json!("an input or the binary changed during the route");
        record["changed_inputs"] = json!(changed);
        record["audit_exit"] = json!(code);
        code = 101;
        record["exit"] = json!(code);
    }
    record["stable"] = json!(stable);
    if receipt.is_file() {
        record["slot_receipt"] = json!(receipt.display().to_string());
    }
    let waited = crate::waits::waited_ms(&wait_record);
    let wall = crate::waits::millis(wall);
    record["wall_time_ms"] = json!(wall);
    record["slot_wait_ms"] = json!(waited);
    record["analysis_time_ms"] = json!(wall.saturating_sub(waited));
    if wait_record.is_file() {
        record["slot_wait_record"] = json!(wait_record.display().to_string());
    }
    write_json(&out.join("invocation.json"), &record)?;
    fs::write(
        out.join(format!("{}.exit", route.name)),
        format!("{code}\n"),
    )
    .map_err(|error| error.to_string())?;
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specifications_are_told_apart() {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(
            SideSpec::parse("baseline:abc", here).unwrap(),
            SideSpec::Baseline("abc".into())
        );
        assert_eq!(
            SideSpec::parse("run:x", here).unwrap(),
            SideSpec::Run(here.join("x"))
        );
        assert_eq!(
            SideSpec::parse("rev:HEAD", here).unwrap(),
            SideSpec::Rev("HEAD".into())
        );
        assert_eq!(
            SideSpec::parse("Cargo.toml", here).unwrap(),
            SideSpec::Binary(here.join("Cargo.toml"))
        );
        assert_eq!(
            SideSpec::parse(".", here).unwrap(),
            SideSpec::Checkout(here.join("."))
        );
        assert_eq!(
            SideSpec::parse("e282aaccc", here).unwrap(),
            SideSpec::Rev("e282aaccc".into())
        );
        assert!(SideSpec::parse("", here).is_err());
    }

    #[test]
    fn bundles_are_provider_contract_trees_and_the_contract_tree() {
        assert_eq!(
            bundle_of("providers/quirin-provider-libc/contracts/stdio.qv").as_deref(),
            Some("providers/quirin-provider-libc/contracts")
        );
        assert_eq!(
            bundle_of("contract/profiles/x.json").as_deref(),
            Some("contract")
        );
        assert_eq!(bundle_of("providers/quirin-provider-libc/src/lib.rs"), None);
        assert_eq!(bundle_of("contract"), None);
    }

    #[test]
    fn the_namespace_wrapper_binds_in_order_and_execs_the_command() {
        let argv = namespaced(
            &[(PathBuf::from("/r/src"), PathBuf::from("/fixed/src"))],
            Path::new("/w"),
            &["tool".into(), "a b".into()],
        );
        assert_eq!(&argv[..4], ["unshare", "-rm", "sh", "-c"]);
        assert!(argv[4].contains("mount --bind \"$1\" \"$2\""));
        assert!(argv[4].contains("cd \"$3\"\nshift 3"));
        assert_eq!(
            &argv[5..],
            [
                "quirin-measure-ns",
                "/r/src",
                "/fixed/src",
                "/w",
                "tool",
                "a b"
            ]
        );
        assert_eq!(
            namespaced(&[], Path::new("/w"), &["tool".into()]),
            vec!["tool".to_owned()]
        );
    }
}
