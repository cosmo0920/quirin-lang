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

//! The per-execution process quota behind the declared limit kind
//! `max_execution_processes` (card `P03-F`).
//!
//! P02 enforces a declared `max_processes` with `RLIMIT_NPROC`, which the
//! kernel counts **per real UID**, not per process tree. That makes it a bound
//! on the account, not on one execution: a value high enough to let the
//! account work is too high to bound a fork storm, and a value low enough to
//! bound the storm also fails every fork the account makes elsewhere. P03
//! recorded that as an open item, and P03-F closes it with a mechanism that
//! counts the *tree*.
//!
//! The mechanism is cgroup v2's `pids` controller: a fresh cgroup is created
//! for one execution, `pids.max` is written into it, and the child joins it in
//! `pre_exec` before it is confined and before `exec`. Every descendant it
//! forks is a member, so the bound is on the execution and nothing else of the
//! account's is counted against it.
//!
//! It needs no privilege — but it does need a *delegated* cgroup subtree the
//! invoking user may write, with the `pids` controller enabled in the parent's
//! `cgroup.subtree_control`. Where that is absent the profile **refuses before
//! spawning**; there is no downgrade to `RLIMIT_NPROC` and no unbounded run.
//!
//! What this is not: a memory or CPU cgroup bound (`RLIMIT_AS`/`RLIMIT_CPU`
//! from the composed limit profile remain per-process), a namespace, or any
//! part of an OS sandbox. It bounds how many tasks one execution's tree may
//! hold at once, nothing else.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ProcessExecutionProfile;

/// The declared limit kind this module enforces. It is deliberately *not*
/// `max_processes`: that kind names `RLIMIT_NPROC`, whose per-UID counting is
/// a different bound, and conflating them would let a manifest believe it had
/// a per-tree quota when it had an account-wide one.
pub const MAX_EXECUTION_PROCESSES: &str = "max_execution_processes";

/// The mechanism name recorded in the evidence.
pub const CGROUP_V2_PIDS: &str = "cgroup-v2-pids";

/// What enforces one declared limit kind under one profile.
///
/// There is no third answer. A kind a profile cannot place in either role is a
/// refusal at every call site, never a skipped limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclaredKindRole {
    /// Installed with `setrlimit` in the child before `exec`.
    Rlimit,
    /// Installed as a per-execution cgroup quota the child joins before `exec`.
    ProcessQuota,
}

/// The kinds a per-execution quota mechanism can enforce. One, today.
pub const PROCESS_QUOTA_KINDS: [&str; 1] = [MAX_EXECUTION_PROCESSES];

/// Whether a profile installs a per-execution process quota at all.
///
/// Only the confining profile does: the quota needs a delegated cgroup, whose
/// availability is an environment fact the confining profile already probes
/// and refuses on. Adding it to the limit profile would give that profile a
/// second, differently-shaped environment dependency without an evidence
/// record for it.
#[must_use]
pub const fn profile_installs_quota(profile: ProcessExecutionProfile) -> bool {
    matches!(profile, ProcessExecutionProfile::LinuxConfinedV1)
}

/// The role a declared kind takes under this profile, or `None` — a refusal.
///
/// `rlimit_kinds` is the profile's `setrlimit` table; passing it keeps this
/// decision in one place without duplicating the table.
#[must_use]
pub fn declared_kind_role(
    profile: ProcessExecutionProfile,
    kind: &str,
    rlimit_kinds: &[&str],
) -> Option<DeclaredKindRole> {
    if !cfg!(target_os = "linux") {
        // Off Linux no profile enforces any declared kind, so no kind has a
        // role and every declaration refuses.
        return None;
    }
    if profile.installs_rlimits() && rlimit_kinds.contains(&kind) {
        return Some(DeclaredKindRole::Rlimit);
    }
    if profile_installs_quota(profile) && PROCESS_QUOTA_KINDS.contains(&kind) {
        return Some(DeclaredKindRole::ProcessQuota);
    }
    None
}

/// Why a declared quota could not be turned into a plan. Each variant is a
/// pre-spawn refusal, never a weaker execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuotaPlanRefusal {
    /// This profile enforces no per-execution quota, so the declaration would
    /// be a bound nothing installs.
    ProfileEnforcesNoQuota {
        profile: ProcessExecutionProfile,
        kind: String,
    },
    /// A quota of zero admits no process at all, including the package's own
    /// entrypoint, so it cannot be installed as written.
    ZeroQuota { kind: String },
}

impl fmt::Display for QuotaPlanRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProfileEnforcesNoQuota { profile, kind } => write!(
                formatter,
                "execution profile `{profile}` installs no per-execution process quota, so \
                 declared limit kind `{kind}` would be a bound nothing enforces; select `{}` \
                 instead",
                ProcessExecutionProfile::LinuxConfinedV1
            ),
            Self::ZeroQuota { kind } => write!(
                formatter,
                "declared limit kind `{kind}` is zero, which admits no process at all, not even \
                 the package's own entrypoint"
            ),
        }
    }
}

/// The per-execution process quota one execution installs, with the mechanism
/// and the delegated cgroup subtree it was derived from.
///
/// Every field is measured before the spawn: the controller list is read from
/// the parent cgroup the kernel reports, and the parent is the one this process
/// actually lives beside.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessQuota {
    /// The declared limit kind this quota installs.
    pub kind: String,
    /// The kernel mechanism.
    pub mechanism: String,
    /// The maximum number of tasks the execution's whole tree may hold.
    pub max_processes: u64,
    /// The delegated cgroup the per-execution cgroup is created beneath.
    pub cgroup_parent: String,
    /// The controllers that parent enables for its children, as read.
    pub parent_subtree_control: String,
}

impl ProcessQuota {
    /// A single-line summary suitable for a diagnostic or a report row.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "{}={} via {} under {}",
            self.kind, self.max_processes, self.mechanism, self.cgroup_parent
        )
    }
}

/// What one execution's quota actually observed, read from the kernel after
/// the child exited and before the per-execution cgroup was removed.
///
/// This is measurement, not declaration: `peak_processes` is the kernel's own
/// high-water mark for the tree and `quota_rejections` is the number of forks
/// the quota refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaObservation {
    pub mechanism: String,
    /// The per-execution cgroup, which existed only for this execution.
    pub cgroup_path: String,
    pub max_processes: u64,
    /// `pids.peak`, where the kernel provides it.
    pub peak_processes: Option<u64>,
    /// The `max` counter of `pids.events`: how many times the quota refused a
    /// fork in this tree.
    pub quota_rejections: Option<u64>,
    /// Whether the per-execution cgroup was removed after the run. A cgroup
    /// that could not be removed still held a task, which is recorded rather
    /// than assumed away.
    pub released: bool,
}

impl QuotaObservation {
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "quota={}/{} peak={:?} rejections={:?} released={}",
            self.mechanism,
            self.max_processes,
            self.peak_processes,
            self.quota_rejections,
            self.released
        )
    }
}

/// Whether a mechanism is taken from the kernel or forced unavailable.
///
/// There is deliberately **no** variant that forces a mechanism *available*:
/// an override can only make a profile refuse more, never enforce less, so it
/// cannot be used — in a test or anywhere else — to obtain an execution that
/// ran with weaker enforcement than the kernel actually provided. That is the
/// whole of its safety argument, and
/// `crates/qcore-verifier-host/tests/execution_quota.rs` pins it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MechanismProbe {
    /// Ask the kernel. The production answer, and the default.
    #[default]
    AsProbed,
    /// Treat the mechanism as absent regardless of what the kernel offers.
    /// Every execution that needs it is refused before spawning.
    ForcedUnavailable,
}

impl MechanismProbe {
    /// The refusal text a forced-unavailable mechanism produces, or `None`
    /// when the kernel should be asked.
    ///
    /// The text names the override explicitly: a report must never read a
    /// forced refusal as a statement about the running kernel.
    #[must_use]
    pub fn forced_refusal(self, mechanism: &str) -> Option<String> {
        match self {
            Self::AsProbed => None,
            Self::ForcedUnavailable => Some(format!(
                "the {mechanism} mechanism is reported unavailable by an explicit availability \
                 override; this is the refusal path, not a statement about this kernel"
            )),
        }
    }
}

/// Per-mechanism probe selection for one provider.
///
/// Additive and default-`AsProbed`, so no existing caller's behavior moves and
/// the production path always asks the kernel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MechanismAvailability {
    pub landlock: MechanismProbe,
    pub user_namespace: MechanismProbe,
    pub process_quota: MechanismProbe,
    /// The parent's duplicate of the per-execution cgroup's `cgroup.procs`
    /// descriptor, which the child's `pre_exec` join writes through. Unlike the
    /// other three it is not consulted by the plan: forced unavailable, the
    /// cgroup is still created, and the duplication itself then fails with
    /// `EMFILE` at its real call site, so the refusal is the one a host out of
    /// descriptors takes (card `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`).
    pub quota_join_descriptor: MechanismProbe,
}

impl MechanismAvailability {
    /// Whether any mechanism is forced. Recorded so evidence never presents a
    /// forced refusal as a kernel fact.
    #[must_use]
    pub fn any_forced(&self) -> bool {
        [
            self.landlock,
            self.user_namespace,
            self.process_quota,
            self.quota_join_descriptor,
        ]
        .contains(&MechanismProbe::ForcedUnavailable)
    }
}

/// The descriptor-table exhaustion a forced
/// [`MechanismAvailability::quota_join_descriptor`] reports, as the kernel
/// spells it for `dup`.
#[cfg(target_os = "linux")]
const FORCED_DESCRIPTOR_ERRNO: i32 = libc::EMFILE;

#[cfg(target_os = "linux")]
pub(crate) mod linux {
    use std::io::Write;
    use std::os::fd::OwnedFd;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        CGROUP_V2_PIDS, MAX_EXECUTION_PROCESSES, MechanismProbe, ProcessQuota, QuotaObservation,
    };

    /// The delegated cgroup subtree this process may create per-execution
    /// cgroups in.
    #[derive(Clone, Debug)]
    pub(crate) struct Delegation {
        pub(crate) parent: PathBuf,
        pub(crate) subtree_control: String,
    }

    /// The unified cgroup v2 mount point, from the kernel's own mount table.
    fn cgroup2_mount() -> Result<PathBuf, String> {
        let table = std::fs::read_to_string("/proc/self/mountinfo")
            .map_err(|error| format!("could not read /proc/self/mountinfo: {error}"))?;
        for line in table.lines() {
            // ... - <fstype> <source> <options>
            let Some((prefix, suffix)) = line.split_once(" - ") else {
                continue;
            };
            let mut fields = suffix.split_whitespace();
            if fields.next() != Some("cgroup2") {
                continue;
            }
            // The mount point is the fifth field of the prefix.
            if let Some(point) = prefix.split_whitespace().nth(4) {
                return Ok(PathBuf::from(point.replace("\\040", " ")));
            }
        }
        Err(
            "this host has no cgroup v2 (unified) mount, so it offers no per-tree \
             process quota"
                .to_owned(),
        )
    }

    /// This process's own cgroup, relative to the unified hierarchy root.
    fn own_cgroup() -> Result<String, String> {
        let text = std::fs::read_to_string("/proc/self/cgroup")
            .map_err(|error| format!("could not read /proc/self/cgroup: {error}"))?;
        for line in text.lines() {
            if let Some(path) = line.strip_prefix("0::") {
                return Ok(path.to_owned());
            }
        }
        Err(
            "this process is in no cgroup v2 hierarchy, so no per-tree process quota can be \
             scoped to it"
                .to_owned(),
        )
    }

    fn probe_once() -> Result<Delegation, String> {
        let mount = cgroup2_mount()?;
        let own = own_cgroup()?;
        let own = mount.join(own.trim_start_matches('/'));
        // A per-execution cgroup must be a *sibling*: this process lives in its
        // own cgroup, and cgroup v2 forbids a cgroup from holding processes and
        // enabling controllers for children at the same time.
        let parent = own
            .parent()
            .ok_or_else(|| {
                format!(
                    "cgroup `{}` is the hierarchy root and has no delegated parent to create a \
                     per-execution cgroup beside",
                    own.display()
                )
            })?
            .to_path_buf();
        let subtree_control = std::fs::read_to_string(parent.join("cgroup.subtree_control"))
            .map_err(|error| {
                format!(
                    "could not read `{}`: {error}; this host delegates no writable cgroup subtree \
                     to the invoking user",
                    parent.join("cgroup.subtree_control").display()
                )
            })?
            .trim()
            .to_owned();
        if !subtree_control
            .split_whitespace()
            .any(|name| name == "pids")
        {
            return Err(format!(
                "cgroup `{}` does not enable the `pids` controller for its children \
                 (cgroup.subtree_control = `{subtree_control}`), so no per-tree process quota \
                 can be installed here",
                parent.display()
            ));
        }
        // Writability is checked by actually creating and removing a cgroup:
        // a permission bit that the kernel would still refuse is not evidence.
        let probe = parent.join(format!("qcore-quota-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir(&probe);
        std::fs::create_dir(&probe).map_err(|error| {
            format!(
                "could not create a cgroup under `{}`: {error}; this host delegates no writable \
                 cgroup subtree to the invoking user",
                parent.display()
            )
        })?;
        let pids_max = probe.join("pids.max");
        let writable = std::fs::write(&pids_max, b"max\n");
        let _ = std::fs::remove_dir(&probe);
        writable.map_err(|error| {
            format!(
                "could not write `{}`: {error}; the delegated cgroup subtree offers no writable \
                 `pids.max`",
                pids_max.display()
            )
        })?;
        Ok(Delegation {
            parent,
            subtree_control,
        })
    }

    /// The cached delegation probe. A failure is cached as a failure, so an
    /// unavailable mechanism refuses every execution rather than being retried
    /// into a different answer.
    pub(crate) fn probe() -> Result<Delegation, String> {
        static PROBE: OnceLock<Result<Delegation, String>> = OnceLock::new();
        PROBE.get_or_init(probe_once).clone()
    }

    /// The quota this declaration would install on this host, without creating
    /// anything. Pre-spawn: an unavailable mechanism is an error here, never a
    /// weaker execution.
    pub(crate) fn describe(
        max_processes: u64,
        availability: MechanismProbe,
    ) -> Result<ProcessQuota, String> {
        if let Some(forced) = availability.forced_refusal("per-execution process quota") {
            return Err(forced);
        }
        let delegation = probe()?;
        Ok(ProcessQuota {
            kind: MAX_EXECUTION_PROCESSES.to_owned(),
            mechanism: CGROUP_V2_PIDS.to_owned(),
            max_processes,
            cgroup_parent: delegation.parent.to_string_lossy().into_owned(),
            parent_subtree_control: delegation.subtree_control,
        })
    }

    /// One execution's cgroup: created before the spawn, joined by the child in
    /// `pre_exec`, observed after the run and removed when it ends.
    pub(crate) struct QuotaSession {
        path: PathBuf,
        procs: OwnedFd,
        quota: ProcessQuota,
    }

    /// The next per-execution sequence number of this process.
    static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);

    /// The name of one execution's cgroup: this process's id, a sequence
    /// number no other session of this process has drawn, and the wall clock.
    ///
    /// The sequence number is what makes two sessions of one process distinct.
    /// The process id and the clock alone were not: concurrent executions in
    /// one host process are released together from the same cached kernel
    /// probes, run the same instructions, and can read the same nanosecond, so
    /// the second `mkdir` met `EEXIST` and its execution was refused before
    /// spawning (card `EXECUTION-QUOTA-LOAD-INDEPENDENCE`). The process id
    /// separates live processes, and the clock separates this process from a
    /// dead one that reused its id and left a cgroup it could not release.
    pub(crate) fn session_name(pid: u32, sequence: u64, nanos: u128) -> String {
        format!("qcore-exec-{pid}-{sequence}-{nanos}")
    }

    impl QuotaSession {
        /// Creates the per-execution cgroup and installs `pids.max`.
        ///
        /// Every failure here happens before anything is spawned, so it is a
        /// refusal rather than an unbounded execution.
        pub(crate) fn create(quota: &ProcessQuota) -> Result<Self, String> {
            let parent = Path::new(&quota.cgroup_parent);
            let path = parent.join(session_name(
                std::process::id(),
                NEXT_SESSION.fetch_add(1, Ordering::Relaxed),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|since| since.as_nanos())
                    .unwrap_or_default(),
            ));
            std::fs::create_dir(&path)
                .map_err(|error| format!("could not create `{}`: {error}", path.display()))?;
            let installed = (|| -> std::io::Result<OwnedFd> {
                let mut max = std::fs::File::create(path.join("pids.max"))?;
                write!(max, "{}", quota.max_processes)?;
                max.sync_all()?;
                Ok(OwnedFd::from(
                    std::fs::OpenOptions::new()
                        .write(true)
                        .open(path.join("cgroup.procs"))?,
                ))
            })();
            match installed {
                Ok(procs) => Ok(Self {
                    path,
                    procs,
                    quota: quota.clone(),
                }),
                Err(error) => {
                    let _ = std::fs::remove_dir(&path);
                    Err(format!(
                        "could not install a `{}` quota of {} in `{}`: {error}",
                        quota.kind,
                        quota.max_processes,
                        path.display()
                    ))
                }
            }
        }

        /// A write descriptor on this cgroup's `cgroup.procs`, for the child's
        /// `pre_exec` closure. The descriptor is opened here, in the parent,
        /// with the parent's credentials; the child writes `0` to it, which is
        /// the kernel's own spelling for "the writing process".
        ///
        /// The duplicate can fail (`EMFILE`, `ENFILE`, `ENOMEM`). That is a
        /// refusal, never a spawn without the join: a child spawned without it
        /// would run outside this cgroup while the record named the quota
        /// (card `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`). `probe` forced
        /// unavailable makes the duplicate fail with `EMFILE` here, at the
        /// real call site, and the reason says so.
        pub(crate) fn procs_descriptor(&self, probe: MechanismProbe) -> Result<OwnedFd, String> {
            let forced = probe.forced_refusal("per-execution cgroup join descriptor");
            let duplicate = match forced {
                Some(_) => Err(std::io::Error::from_raw_os_error(
                    super::FORCED_DESCRIPTOR_ERRNO,
                )),
                None => self.procs.try_clone(),
            };
            duplicate.map_err(|error| {
                let reason = format!(
                    "could not duplicate the write descriptor on `{}` for the child's join: \
                     {error}",
                    self.path.join("cgroup.procs").display()
                );
                match forced {
                    Some(forced) => format!("{reason}; {forced}"),
                    None => reason,
                }
            })
        }

        fn read_number(&self, file: &str) -> Option<u64> {
            std::fs::read_to_string(self.path.join(file))
                .ok()?
                .trim()
                .parse()
                .ok()
        }

        fn rejections(&self) -> Option<u64> {
            let events = std::fs::read_to_string(self.path.join("pids.events")).ok()?;
            events.lines().find_map(|line| {
                line.strip_prefix("max ")
                    .and_then(|count| count.trim().parse().ok())
            })
        }

        /// Reads what the quota observed, then removes the per-execution
        /// cgroup. Observation happens first: removal is only possible once the
        /// cgroup is empty, and the counters would be gone with it.
        pub(crate) fn finish(self) -> QuotaObservation {
            let peak_processes = self.read_number("pids.peak");
            let quota_rejections = self.rejections();
            let observation = QuotaObservation {
                mechanism: self.quota.mechanism.clone(),
                cgroup_path: self.path.to_string_lossy().into_owned(),
                max_processes: self.quota.max_processes,
                peak_processes,
                quota_rejections,
                released: false,
            };
            let released = self.release();
            QuotaObservation {
                released,
                ..observation
            }
        }

        /// Kills whatever is left in the cgroup and removes it. `rmdir` only
        /// succeeds on an empty cgroup, so the return value is a fact about
        /// the tree, not a formality.
        fn release(&self) -> bool {
            // `cgroup.kill` (Linux 5.14+) terminates every member atomically.
            // Where it is absent the host's own process-group kill has already
            // run, and the retries below cover the reaping delay.
            let _ = std::fs::write(self.path.join("cgroup.kill"), b"1");
            for attempt in 0..50 {
                if std::fs::remove_dir(&self.path).is_ok() || !self.path.exists() {
                    return true;
                }
                std::thread::sleep(std::time::Duration::from_millis(if attempt < 10 {
                    1
                } else {
                    10
                }));
            }
            false
        }
    }

    impl Drop for QuotaSession {
        fn drop(&mut self) {
            // `finish` normally removes it; this is the path where the run
            // failed before observation.
            if self.path.exists() {
                self.release();
            }
        }
    }

    /// Joins the calling process to `procs`'s cgroup.
    ///
    /// # Safety
    ///
    /// Must be called in a forked child before `exec`, with `procs` a live
    /// write descriptor on a `cgroup.procs` file. It allocates nothing, takes
    /// no lock and makes one `write`.
    pub(crate) unsafe fn join(procs: std::os::fd::RawFd) -> std::io::Result<()> {
        // "0" is the kernel's spelling for the writing process itself, so the
        // child needs neither its own pid nor a formatter here.
        const SELF: &[u8; 2] = b"0\n";
        // SAFETY: `procs` is the live descriptor the parent opened and the
        // buffer is a static two-byte literal.
        let written = unsafe { libc::write(procs, SELF.as_ptr().cast(), SELF.len()) };
        join_outcome(written, SELF.len(), std::io::Error::last_os_error)
    }

    /// The join's result from what its one `write` returned. Only the whole
    /// buffer is a join: a negative count is the kernel's error, and a short
    /// count is refused as `EIO` rather than taken as a membership the kernel
    /// never confirmed. A `cgroup.procs` write on Linux returns the whole count
    /// or an error, so the short arm is not expected to be reached; it is
    /// checked so that no count is ignored (card
    /// `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`).
    ///
    /// Allocation-free, because it runs in the forked child before `exec`:
    /// both errors are raw OS errors.
    pub(crate) fn join_outcome(
        written: libc::ssize_t,
        expected: usize,
        os_error: impl FnOnce() -> std::io::Error,
    ) -> std::io::Result<()> {
        if written < 0 {
            return Err(os_error());
        }
        if written.unsigned_abs() != expected {
            return Err(std::io::Error::from_raw_os_error(libc::EIO));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RLIMIT_KINDS: [&str; 6] = [
        "memory_bytes",
        "cpu_seconds",
        "core_bytes",
        "max_file_bytes",
        "max_open_files",
        "max_processes",
    ];

    #[test]
    fn only_the_confining_profile_installs_a_per_execution_quota() {
        assert!(profile_installs_quota(
            ProcessExecutionProfile::LinuxConfinedV1
        ));
        assert!(!profile_installs_quota(
            ProcessExecutionProfile::LinuxRlimitV1
        ));
        assert!(!profile_installs_quota(
            ProcessExecutionProfile::UnenforcedLegacy
        ));
    }

    #[test]
    fn the_quota_kind_is_never_confused_with_the_per_uid_rlimit_kind() {
        assert_ne!(MAX_EXECUTION_PROCESSES, "max_processes");
        assert!(!RLIMIT_KINDS.contains(&MAX_EXECUTION_PROCESSES));
        assert!(!PROCESS_QUOTA_KINDS.contains(&"max_processes"));
        // Under the confining profile the two kinds take different roles, so a
        // manifest can never obtain one while declaring the other.
        if cfg!(target_os = "linux") {
            assert_eq!(
                declared_kind_role(
                    ProcessExecutionProfile::LinuxConfinedV1,
                    "max_processes",
                    &RLIMIT_KINDS
                ),
                Some(DeclaredKindRole::Rlimit)
            );
            assert_eq!(
                declared_kind_role(
                    ProcessExecutionProfile::LinuxConfinedV1,
                    MAX_EXECUTION_PROCESSES,
                    &RLIMIT_KINDS
                ),
                Some(DeclaredKindRole::ProcessQuota)
            );
        }
    }

    #[test]
    fn a_profile_that_installs_no_quota_has_no_role_for_the_quota_kind() {
        for profile in [
            ProcessExecutionProfile::UnenforcedLegacy,
            ProcessExecutionProfile::LinuxRlimitV1,
        ] {
            assert_eq!(
                declared_kind_role(profile, MAX_EXECUTION_PROCESSES, &RLIMIT_KINDS),
                None
            );
        }
    }

    #[test]
    fn an_unknown_kind_has_no_role_under_any_profile() {
        for profile in [
            ProcessExecutionProfile::UnenforcedLegacy,
            ProcessExecutionProfile::LinuxRlimitV1,
            ProcessExecutionProfile::LinuxConfinedV1,
        ] {
            assert_eq!(
                declared_kind_role(profile, "network_bandwidth_bps", &RLIMIT_KINDS),
                None
            );
        }
    }

    #[test]
    fn an_override_can_only_force_a_mechanism_unavailable() {
        assert_eq!(MechanismProbe::default(), MechanismProbe::AsProbed);
        assert_eq!(MechanismProbe::AsProbed.forced_refusal("landlock"), None);
        let forced = MechanismProbe::ForcedUnavailable
            .forced_refusal("landlock")
            .expect("a forced mechanism refuses");
        assert!(forced.contains("availability override"));
        assert!(forced.contains("not a statement about this kernel"));
        assert!(!MechanismAvailability::default().any_forced());
        assert!(
            MechanismAvailability {
                process_quota: MechanismProbe::ForcedUnavailable,
                ..MechanismAvailability::default()
            }
            .any_forced()
        );
    }

    /// Two sessions of one process that read the same nanosecond still get
    /// distinct cgroups, and the name decodes back to exactly the triple it was
    /// built from, so no two distinct triples share a name.
    #[test]
    #[cfg(target_os = "linux")]
    fn per_execution_cgroup_names_never_collide_within_one_process() {
        use linux::session_name;
        let (pid, nanos) = (4_242, 1_791_799_458_013_850_267_u128);
        let names: std::collections::BTreeSet<String> = (0..64)
            .map(|sequence| session_name(pid, sequence, nanos))
            .collect();
        assert_eq!(names.len(), 64);
        for (pid, sequence, nanos) in [(1, 23, 4), (12, 3, 4), (1, 2, 34), (0, 0, 0)] {
            let name = session_name(pid, sequence, nanos);
            let fields: Vec<&str> = name
                .strip_prefix("qcore-exec-")
                .expect("the prefix is fixed")
                .split('-')
                .collect();
            assert_eq!(
                fields,
                [pid.to_string(), sequence.to_string(), nanos.to_string()],
                "{name}"
            );
        }
    }

    /// Only a write of the whole buffer is a join. A short count is refused
    /// with `EIO` and a negative one with the kernel's own error, so no count
    /// the join's `write` returns is ignored; and the forced descriptor
    /// override counts as forced.
    #[test]
    #[cfg(target_os = "linux")]
    fn only_a_whole_join_write_is_a_join() {
        use linux::join_outcome;
        let unused = || panic!("only a negative count reads the OS error");
        assert!(join_outcome(2, 2, unused).is_ok());
        for short in [0, 1] {
            let refused = join_outcome(short, 2, unused).unwrap_err();
            assert_eq!(refused.raw_os_error(), Some(libc::EIO), "{short}");
        }
        assert_eq!(
            join_outcome(3, 2, unused).unwrap_err().raw_os_error(),
            Some(libc::EIO)
        );
        let failed = join_outcome(-1, 2, || std::io::Error::from_raw_os_error(libc::EBUSY));
        assert_eq!(failed.unwrap_err().raw_os_error(), Some(libc::EBUSY));
        assert!(
            MechanismAvailability {
                quota_join_descriptor: MechanismProbe::ForcedUnavailable,
                ..MechanismAvailability::default()
            }
            .any_forced()
        );
    }

    #[test]
    fn the_refusals_name_the_kind_and_the_profile_that_would_enforce_it() {
        let refusal = QuotaPlanRefusal::ProfileEnforcesNoQuota {
            profile: ProcessExecutionProfile::LinuxRlimitV1,
            kind: MAX_EXECUTION_PROCESSES.to_owned(),
        }
        .to_string();
        assert!(refusal.contains("linux-rlimit-v1"));
        assert!(refusal.contains(MAX_EXECUTION_PROCESSES));
        assert!(refusal.contains("linux-confined-v1"));
        let zero = QuotaPlanRefusal::ZeroQuota {
            kind: MAX_EXECUTION_PROCESSES.to_owned(),
        }
        .to_string();
        assert!(zero.contains("no process at all"));
    }
}
