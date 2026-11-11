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

//! The Linux confinement adapter behind
//! [`ProcessExecutionProfile::LinuxConfinedV1`](crate::ProcessExecutionProfile::LinuxConfinedV1).
//!
//! Two kernel mechanisms, both enforced by the kernel rather than by
//! convention, and both installed in the forked child before `exec`:
//!
//! * **Filesystem — Landlock.** A ruleset is built in the *parent* (the child's
//!   `pre_exec` closure opens nothing and allocates nothing), handling every
//!   filesystem access right the running kernel's Landlock ABI knows, and
//!   granting rights only beneath the explicitly admitted paths. The child sets
//!   `PR_SET_NO_NEW_PRIVS` and calls `landlock_restrict_self`; the restriction
//!   is inherited across `exec` and cannot be lifted.
//! * **Network — namespaces.** The child calls
//!   `unshare(CLONE_NEWUSER | CLONE_NEWNET)`, so it runs in a fresh network
//!   namespace whose only interface is a down `lo`. No socket of any family
//!   routed by that namespace can reach a peer, and the abstract Unix socket
//!   namespace is replaced with an empty one.
//!
//! What this is **not** is recorded in [`Confinement::unconfined`] and repeated
//! in `docs/reports/remaining-p03/README.md`. It is confinement of two named
//! resource classes, not a general OS sandbox.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::fmt;

use serde::{Deserialize, Serialize};

/// The access class one admitted path is granted.
///
/// A class is a *grant*, never a declaration: a path that carries no class is
/// not reachable at all under the profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathAccess {
    /// Read and execute. Directory traversal, file reads, `execve`.
    ReadExecute,
    /// Read only. No `execve`, no write of any kind.
    ReadOnly,
    /// Read, write, create and remove beneath the path. Never `execve`.
    ReadWrite,
}

impl PathAccess {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ReadExecute => "read-execute",
            Self::ReadOnly => "read-only",
            Self::ReadWrite => "read-write",
        }
    }
}

impl fmt::Display for PathAccess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Whether an admitted path existed when the ruleset was built.
///
/// An absent path grants nothing, which narrows the child's reach rather than
/// widening it, so it is recorded rather than refused. It is recorded because a
/// report that quoted only the policy would otherwise overstate what the child
/// could reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathState {
    /// The path was opened and a Landlock rule was installed for it.
    Admitted,
    /// The path did not exist; no rule was installed and nothing was granted.
    Absent,
}

/// One path the policy admits, with the class it was granted and whether a rule
/// was actually installed for it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmittedPath {
    pub path: String,
    pub access: PathAccess,
    pub state: PathState,
}

/// The explicit filesystem reach a confined execution is given.
///
/// There is no implicit entry and no default policy: a caller that selects the
/// confined profile without supplying a policy is refused, because a policy
/// nobody wrote is a policy nobody reviewed. [`Self::linux_runtime_defaults`]
/// is an explicitly named starting point, not a fallback.
///
/// The package root is admitted `read-execute` by the profile itself — the
/// entrypoint could not be `exec`ed otherwise — and is recorded in the
/// evidence alongside the policy's own paths.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfinementPolicy {
    /// Absolute paths the child may read and execute from. These are the
    /// package's runtime dependencies: the interpreter, the dynamic loader and
    /// the shared libraries it resolves.
    #[serde(default)]
    pub read_execute: Vec<String>,
    /// Absolute paths the child may read but never execute or write.
    #[serde(default)]
    pub read_only: Vec<String>,
    /// Absolute paths the child may read, write, create and remove beneath.
    #[serde(default)]
    pub read_write: Vec<String>,
}

impl ConfinementPolicy {
    /// An explicitly named set of runtime roots sufficient to `exec` a
    /// dynamically linked executable or a `#!` script on a conventional Linux
    /// filesystem layout.
    ///
    /// This is a *named* policy a caller opts into, not a default that applies
    /// when a caller supplies nothing. It grants no write access anywhere
    /// except `/dev/null`, and it does not admit the package's own inputs —
    /// the profile admits the package root, and anything else is the caller's
    /// explicit addition.
    #[must_use]
    pub fn linux_runtime_defaults() -> Self {
        Self {
            read_execute: [
                "/bin",
                "/sbin",
                "/lib",
                "/lib64",
                "/usr/bin",
                "/usr/sbin",
                "/usr/lib",
                "/usr/lib64",
                "/usr/libexec",
                "/usr/share",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            read_only: [
                "/etc/ld.so.cache",
                "/etc/ld.so.conf",
                "/etc/ld.so.conf.d",
                "/etc/localtime",
                "/dev/urandom",
                "/dev/zero",
                "/proc/self/maps",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            read_write: vec!["/dev/null".to_owned()],
        }
    }

    /// Adds one read-and-execute root.
    #[must_use]
    pub fn admitting_read_execute(mut self, path: impl Into<String>) -> Self {
        self.read_execute.push(path.into());
        self
    }

    /// Adds one read-only path.
    #[must_use]
    pub fn admitting_read_only(mut self, path: impl Into<String>) -> Self {
        self.read_only.push(path.into());
        self
    }

    /// Adds one read-write path.
    #[must_use]
    pub fn admitting_read_write(mut self, path: impl Into<String>) -> Self {
        self.read_write.push(path.into());
        self
    }

    /// Every admitted path with its class, in evidence order: the profile's own
    /// package-root grant first, then read-execute, read-only, read-write.
    pub(crate) fn entries(&self, package_root: &str) -> Vec<(String, PathAccess)> {
        let mut entries = vec![(package_root.to_owned(), PathAccess::ReadExecute)];
        for path in &self.read_execute {
            entries.push((path.clone(), PathAccess::ReadExecute));
        }
        for path in &self.read_only {
            entries.push((path.clone(), PathAccess::ReadOnly));
        }
        for path in &self.read_write {
            entries.push((path.clone(), PathAccess::ReadWrite));
        }
        entries
    }
}

/// What one confined execution actually confined, with the kernel facts it was
/// derived from.
///
/// Every field is measured, not declared: the ABI number comes from the running
/// kernel's Landlock version probe, the handled-access masks are what the
/// ruleset was created with, and each path's [`PathState`] is the result of
/// opening it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Confinement {
    /// The kernel mechanism confining the filesystem.
    pub filesystem_mechanism: String,
    /// The kernel mechanism confining the network.
    pub network_mechanism: String,
    /// `uname -r` of the kernel that answered the probes.
    pub kernel_release: String,
    /// The Landlock ABI the running kernel reports.
    pub landlock_abi: u32,
    /// The lowest ABI this profile accepts.
    pub landlock_min_abi: u32,
    /// The filesystem access rights the ruleset *handles*: an operation gated
    /// by a handled right is denied unless a rule grants it. Rights the running
    /// ABI does not know are not handled, and the operations they gate are
    /// therefore not restricted.
    pub handled_access_fs: String,
    /// The network access rights the ruleset handles, granted to nothing.
    /// Empty below Landlock ABI 4, where the network namespace is the only
    /// network mechanism.
    pub handled_access_net: String,
    /// The manifest's `network_access` declaration. Manifest validation already
    /// rejects `true`, so a confined execution enforces exactly what every
    /// admissible manifest declares.
    pub network_access_declared: bool,
    /// Every admitted path, with its granted class and whether a rule was
    /// installed.
    pub paths: Vec<AdmittedPath>,
    /// What this profile does **not** confine. Stated in the evidence so a
    /// report cannot quote the mechanisms without the boundary.
    pub unconfined: Vec<String>,
}

impl Confinement {
    /// A single-line summary suitable for a diagnostic or a report row.
    pub fn summary(&self) -> String {
        let admitted = self
            .paths
            .iter()
            .filter(|path| path.state == PathState::Admitted)
            .count();
        format!(
            "filesystem={}(abi={}, handled_fs={}); network={}(handled_net={}); \
             kernel={}; admitted_paths={admitted}/{}",
            self.filesystem_mechanism,
            self.landlock_abi,
            self.handled_access_fs,
            self.network_mechanism,
            self.handled_access_net,
            self.kernel_release,
            self.paths.len(),
        )
    }
}

/// The residual reach a confined execution keeps. Enumerated here rather than
/// left to prose so the evidence record carries its own boundary.
pub(crate) fn unconfined_inventory(landlock_abi: u32) -> Vec<String> {
    let mut inventory = vec![
        "filesystem access through a path admitted by the policy, including any \
         Unix-domain socket bound beneath one"
            .to_owned(),
        "the child's own address space, CPU and descendants beyond what the \
         composed `linux-rlimit-v1` limits and the process-group kill bound"
            .to_owned(),
        "System V IPC, POSIX message queues, and every namespace other than the \
         user and network namespaces this profile unshares"
            .to_owned(),
        "the system-call surface itself: no seccomp filter is installed, so any \
         syscall the kernel permits under these credentials is reachable"
            .to_owned(),
        "loopback traffic *within* the child's own network namespace, once the \
         child brings `lo` up inside it"
            .to_owned(),
    ];
    if landlock_abi < LANDLOCK_ABI_REFER {
        inventory.push(
            "renaming and linking across separate admitted hierarchies \
             (Landlock ABI 1 does not gate `LANDLOCK_ACCESS_FS_REFER`)"
                .to_owned(),
        );
    }
    if landlock_abi < LANDLOCK_ABI_TRUNCATE {
        inventory.push(
            "truncation through an already-open descriptor (Landlock ABI below 3 \
             does not gate `LANDLOCK_ACCESS_FS_TRUNCATE`)"
                .to_owned(),
        );
    }
    if landlock_abi < LANDLOCK_ABI_IOCTL_DEV {
        inventory.push(
            "`ioctl` on device files (Landlock ABI below 5 does not gate \
             `LANDLOCK_ACCESS_FS_IOCTL_DEV`)"
                .to_owned(),
        );
    }
    if landlock_abi > LANDLOCK_ABI_KNOWN {
        inventory.push(format!(
            "filesystem access rights introduced after Landlock ABI \
             {LANDLOCK_ABI_KNOWN}, which this adapter does not handle on this \
             ABI {landlock_abi} kernel"
        ));
    }
    inventory
}

/// The lowest Landlock ABI this profile runs under. ABI 1 already gates
/// `execve`, file reads and writes, and directory creation and removal, which
/// is the whole of what the policy grants.
pub(crate) const LANDLOCK_MIN_ABI: u32 = 1;
/// The highest Landlock ABI whose filesystem rights this adapter knows. A
/// higher-ABI kernel is used at this level, and the shortfall is recorded.
pub(crate) const LANDLOCK_ABI_KNOWN: u32 = 5;
const LANDLOCK_ABI_REFER: u32 = 2;
const LANDLOCK_ABI_TRUNCATE: u32 = 3;
const LANDLOCK_ABI_NET: u32 = 4;
const LANDLOCK_ABI_IOCTL_DEV: u32 = 5;

pub(crate) const FS_EXECUTE: u64 = 1 << 0;
pub(crate) const FS_WRITE_FILE: u64 = 1 << 1;
pub(crate) const FS_READ_FILE: u64 = 1 << 2;
pub(crate) const FS_READ_DIR: u64 = 1 << 3;
const FS_REMOVE_DIR: u64 = 1 << 4;
const FS_REMOVE_FILE: u64 = 1 << 5;
const FS_MAKE_CHAR: u64 = 1 << 6;
const FS_MAKE_DIR: u64 = 1 << 7;
const FS_MAKE_REG: u64 = 1 << 8;
const FS_MAKE_SOCK: u64 = 1 << 9;
const FS_MAKE_FIFO: u64 = 1 << 10;
const FS_MAKE_BLOCK: u64 = 1 << 11;
const FS_MAKE_SYM: u64 = 1 << 12;
const FS_REFER: u64 = 1 << 13;
const FS_TRUNCATE: u64 = 1 << 14;
const FS_IOCTL_DEV: u64 = 1 << 15;

const NET_BIND_TCP: u64 = 1 << 0;
const NET_CONNECT_TCP: u64 = 1 << 1;

/// The rights that exist only on a directory. Granting one of them on a rule
/// whose parent is not a directory is `EINVAL`, so a non-directory rule is
/// narrowed to the file rights rather than dropped.
pub(crate) const FS_DIRECTORY_ONLY: u64 = FS_READ_DIR
    | FS_REMOVE_DIR
    | FS_REMOVE_FILE
    | FS_MAKE_CHAR
    | FS_MAKE_DIR
    | FS_MAKE_REG
    | FS_MAKE_SOCK
    | FS_MAKE_FIFO
    | FS_MAKE_BLOCK
    | FS_MAKE_SYM
    | FS_REFER;

/// Every filesystem right the running ABI knows. Handling a right the kernel
/// does not know is `EINVAL`, and *not* handling one it does know silently
/// leaves the operations it gates unrestricted, so the mask is derived from the
/// probed ABI rather than fixed.
pub(crate) const fn handled_access_fs(landlock_abi: u32) -> u64 {
    let mut handled = FS_EXECUTE
        | FS_WRITE_FILE
        | FS_READ_FILE
        | FS_READ_DIR
        | FS_REMOVE_DIR
        | FS_REMOVE_FILE
        | FS_MAKE_CHAR
        | FS_MAKE_DIR
        | FS_MAKE_REG
        | FS_MAKE_SOCK
        | FS_MAKE_FIFO
        | FS_MAKE_BLOCK
        | FS_MAKE_SYM;
    if landlock_abi >= LANDLOCK_ABI_REFER {
        handled |= FS_REFER;
    }
    if landlock_abi >= LANDLOCK_ABI_TRUNCATE {
        handled |= FS_TRUNCATE;
    }
    if landlock_abi >= LANDLOCK_ABI_IOCTL_DEV {
        handled |= FS_IOCTL_DEV;
    }
    handled
}

/// TCP bind and connect are handled and granted to nothing from Landlock ABI 4,
/// which is redundant with the network namespace and costs nothing.
pub(crate) const fn handled_access_net(landlock_abi: u32) -> u64 {
    if landlock_abi >= LANDLOCK_ABI_NET {
        NET_BIND_TCP | NET_CONNECT_TCP
    } else {
        0
    }
}

/// The rights one access class grants, masked to what the ruleset handles.
///
/// `LANDLOCK_ACCESS_FS_REFER` is handled but granted to nothing, so a rename or
/// hard link that crosses two admitted hierarchies is denied.
pub(crate) const fn granted_access_fs(access: PathAccess, handled: u64) -> u64 {
    let granted = match access {
        PathAccess::ReadExecute => FS_EXECUTE | FS_READ_FILE | FS_READ_DIR,
        PathAccess::ReadOnly => FS_READ_FILE | FS_READ_DIR,
        PathAccess::ReadWrite => {
            FS_READ_FILE
                | FS_READ_DIR
                | FS_WRITE_FILE
                | FS_REMOVE_DIR
                | FS_REMOVE_FILE
                | FS_MAKE_CHAR
                | FS_MAKE_DIR
                | FS_MAKE_REG
                | FS_MAKE_SOCK
                | FS_MAKE_FIFO
                | FS_MAKE_BLOCK
                | FS_MAKE_SYM
                | FS_TRUNCATE
                | FS_IOCTL_DEV
        }
    };
    granted & handled
}

#[cfg(target_os = "linux")]
pub(crate) mod linux {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::sync::OnceLock;

    use super::{
        AdmittedPath, Confinement, ConfinementPolicy, LANDLOCK_MIN_ABI, PathState,
        granted_access_fs, handled_access_fs, handled_access_net, unconfined_inventory,
    };

    const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1 << 0;
    const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;

    #[repr(C)]
    struct RulesetAttr {
        handled_access_fs: u64,
        handled_access_net: u64,
    }

    // The kernel declares `landlock_path_beneath_attr` packed; a padded layout
    // would be rejected as an unknown attribute size.
    #[repr(C, packed)]
    struct PathBeneathAttr {
        allowed_access: u64,
        parent_fd: i32,
    }

    /// What the running kernel actually offers. Probed once per process; a
    /// failure is cached as a failure, so an unavailable mechanism refuses every
    /// execution instead of being retried into a different answer.
    #[derive(Clone, Debug)]
    pub(crate) struct KernelProbe {
        pub(crate) landlock_abi: u32,
        pub(crate) kernel_release: String,
    }

    fn kernel_release() -> String {
        // SAFETY: `uname` fills a caller-owned `utsname`; the buffer is
        // zero-initialized so the release field is NUL-terminated either way.
        unsafe {
            let mut info = std::mem::zeroed::<libc::utsname>();
            if libc::uname(&raw mut info) != 0 {
                return "unknown".to_owned();
            }
            let release = std::ffi::CStr::from_ptr(info.release.as_ptr());
            release.to_string_lossy().into_owned()
        }
    }

    fn landlock_abi() -> Result<u32, String> {
        // SAFETY: the version query passes a null attribute pointer with a zero
        // size, which is the documented way to ask for the ABI number.
        let abi = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                std::ptr::null::<RulesetAttr>(),
                0_usize,
                LANDLOCK_CREATE_RULESET_VERSION,
            )
        };
        if abi < 0 {
            let error = std::io::Error::last_os_error();
            return Err(match error.raw_os_error() {
                Some(libc::ENOSYS) => {
                    "this kernel has no `landlock_create_ruleset` syscall".to_owned()
                }
                Some(libc::EOPNOTSUPP) => {
                    "Landlock is compiled in but disabled; it is absent from the kernel's \
                     enabled LSM list"
                        .to_owned()
                }
                _ => format!("Landlock version probe failed: {error}"),
            });
        }
        u32::try_from(abi).map_err(|_| format!("implausible Landlock ABI {abi}"))
    }

    /// Forks once and asks the kernel, in the child, whether an unprivileged
    /// user and network namespace can be created at all.
    ///
    /// A sysctl read would not answer this: a Linux distribution can permit
    /// unprivileged user namespaces globally and still deny them to this
    /// executable through an LSM policy. The only faithful probe is the
    /// syscall, in a process whose credentials are the ones the child will
    /// have.
    fn network_namespace_probe() -> Result<(), String> {
        // SAFETY: the forked child touches nothing but two async-signal-safe
        // calls before `_exit`, so it cannot observe a lock the parent's other
        // threads held across the fork.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(format!(
                "could not fork to probe namespace support: {}",
                std::io::Error::last_os_error()
            ));
        }
        if pid == 0 {
            let code = unsafe {
                if libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNET) == 0 {
                    0
                } else {
                    std::io::Error::last_os_error()
                        .raw_os_error()
                        .unwrap_or(libc::EPERM)
                        .clamp(1, 120)
                }
            };
            // SAFETY: `_exit` is async-signal-safe and runs no atexit handler,
            // so the forked child cannot flush the parent's buffers.
            unsafe { libc::_exit(code) };
        }
        let mut status = 0;
        // SAFETY: `pid` is this process's direct child and `status` is a live
        // local.
        if unsafe { libc::waitpid(pid, &raw mut status, 0) } < 0 {
            return Err(format!(
                "could not wait for the namespace probe: {}",
                std::io::Error::last_os_error()
            ));
        }
        let exited = libc::WIFEXITED(status).then(|| libc::WEXITSTATUS(status));
        match exited {
            Some(0) => Ok(()),
            Some(code) => Err(format!(
                "this kernel refuses `unshare(CLONE_NEWUSER | CLONE_NEWNET)` for this \
                 executable ({}); unprivileged user namespaces may be disabled by sysctl or \
                 by an LSM policy",
                std::io::Error::from_raw_os_error(code)
            )),
            None => Err("the namespace probe was killed by a signal".to_owned()),
        }
    }

    fn probe_once() -> Result<KernelProbe, String> {
        let landlock_abi = landlock_abi()?;
        if landlock_abi < LANDLOCK_MIN_ABI {
            return Err(format!(
                "Landlock ABI {landlock_abi} is below the minimum {LANDLOCK_MIN_ABI} this \
                 profile requires"
            ));
        }
        network_namespace_probe()?;
        Ok(KernelProbe {
            landlock_abi,
            kernel_release: kernel_release(),
        })
    }

    /// The cached result of probing this kernel.
    pub(crate) fn probe() -> Result<KernelProbe, String> {
        static PROBE: OnceLock<Result<KernelProbe, String>> = OnceLock::new();
        PROBE.get_or_init(probe_once).clone()
    }

    /// The confinement this policy would achieve on this kernel, without
    /// creating anything. This is the pre-spawn answer: an unavailable
    /// mechanism is an error here, never a weaker execution.
    pub(crate) fn describe(
        policy: &ConfinementPolicy,
        package_root: &str,
        network_access_declared: bool,
        availability: crate::process_quota::MechanismAvailability,
    ) -> Result<Confinement, String> {
        // A forced-unavailable mechanism refuses here, before the kernel is
        // asked, and names itself as an override so no report can read it as a
        // statement about the running kernel. The override can only remove a
        // mechanism, never add one.
        if let Some(forced) = availability.landlock.forced_refusal("Landlock") {
            return Err(forced);
        }
        if let Some(forced) = availability
            .user_namespace
            .forced_refusal("user and network namespace")
        {
            return Err(forced);
        }
        let probe = probe()?;
        let paths = policy
            .entries(package_root)
            .into_iter()
            .map(|(path, access)| AdmittedPath {
                state: if std::fs::symlink_metadata(&path).is_ok() {
                    PathState::Admitted
                } else {
                    PathState::Absent
                },
                path,
                access,
            })
            .collect();
        Ok(Confinement {
            filesystem_mechanism: "landlock".to_owned(),
            network_mechanism: "unshare(CLONE_NEWUSER|CLONE_NEWNET)".to_owned(),
            kernel_release: probe.kernel_release,
            landlock_abi: probe.landlock_abi,
            landlock_min_abi: LANDLOCK_MIN_ABI,
            handled_access_fs: format!("{:#x}", handled_access_fs(probe.landlock_abi)),
            handled_access_net: format!("{:#x}", handled_access_net(probe.landlock_abi)),
            network_access_declared,
            paths,
            unconfined: unconfined_inventory(probe.landlock_abi),
        })
    }

    /// Builds the Landlock ruleset in the parent. The returned descriptor is
    /// close-on-exec, so the child uses it in `pre_exec` and the `exec`ed
    /// package never sees it.
    ///
    /// Every failure here happens before anything is spawned.
    pub(crate) fn build_ruleset(confinement: &Confinement) -> Result<OwnedFd, String> {
        let handled_fs = handled_access_fs(confinement.landlock_abi);
        let handled_net = handled_access_net(confinement.landlock_abi);
        let attr = RulesetAttr {
            handled_access_fs: handled_fs,
            handled_access_net: handled_net,
        };
        // A pre-ABI-4 kernel does not know the network field. Sending only the
        // prefix it knows keeps the refusal honest instead of relying on the
        // kernel accepting a zeroed tail.
        let attr_size = if handled_net == 0 {
            size_of::<u64>()
        } else {
            size_of::<RulesetAttr>()
        };
        // SAFETY: `attr` is a live local of at least `attr_size` bytes.
        let ruleset = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                &raw const attr,
                attr_size,
                0_u32,
            )
        };
        if ruleset < 0 {
            return Err(format!(
                "could not create the Landlock ruleset: {}",
                std::io::Error::last_os_error()
            ));
        }
        let ruleset = RawFd::try_from(ruleset)
            .map_err(|_| format!("implausible Landlock ruleset descriptor {ruleset}"))?;
        // SAFETY: the syscall returned a fresh owned descriptor.
        let ruleset = unsafe { OwnedFd::from_raw_fd(ruleset) };
        for admitted in &confinement.paths {
            if admitted.state == PathState::Absent {
                continue;
            }
            let path = CString::new(admitted.path.as_bytes())
                .map_err(|_| format!("admitted path `{}` contains a NUL byte", admitted.path))?;
            // SAFETY: `path` is a live NUL-terminated buffer.
            let parent = unsafe { libc::open(path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
            if parent < 0 {
                return Err(format!(
                    "could not open admitted path `{}`: {}",
                    admitted.path,
                    std::io::Error::last_os_error()
                ));
            }
            // The kernel rejects a directory-only right on a rule whose parent
            // is a file, so the class is narrowed to the file rights. The check
            // is on the opened descriptor, not on the path, so it cannot be
            // raced.
            let directory = {
                // SAFETY: `stat` is a live local and `parent` is the descriptor
                // just opened; `fstat` is permitted on an `O_PATH` descriptor.
                let mut stat = unsafe { std::mem::zeroed::<libc::stat>() };
                if unsafe { libc::fstat(parent, &raw mut stat) } != 0 {
                    let error = std::io::Error::last_os_error();
                    // SAFETY: `parent` is the descriptor opened above.
                    unsafe { libc::close(parent) };
                    return Err(format!(
                        "could not inspect admitted path `{}`: {error}",
                        admitted.path
                    ));
                }
                stat.st_mode & libc::S_IFMT == libc::S_IFDIR
            };
            let mut allowed = granted_access_fs(admitted.access, handled_fs);
            if !directory {
                allowed &= !super::FS_DIRECTORY_ONLY;
            }
            let rule = PathBeneathAttr {
                allowed_access: allowed,
                parent_fd: parent,
            };
            // SAFETY: `rule` is a live local matching the packed layout the
            // kernel expects for `LANDLOCK_RULE_PATH_BENEATH`.
            let added = unsafe {
                libc::syscall(
                    libc::SYS_landlock_add_rule,
                    ruleset.as_raw_fd(),
                    LANDLOCK_RULE_PATH_BENEATH,
                    &raw const rule,
                    0_u32,
                )
            };
            // SAFETY: `parent` is the descriptor just opened above and is not
            // used again.
            unsafe { libc::close(parent) };
            if added < 0 {
                return Err(format!(
                    "could not admit path `{}` ({}): {}",
                    admitted.path,
                    admitted.access,
                    std::io::Error::last_os_error()
                ));
            }
        }
        Ok(ruleset)
    }

    /// Installs the confinement in the forked child, before `exec`.
    ///
    /// Called from a `pre_exec` closure: it allocates nothing, takes no lock and
    /// makes only syscalls. Any failure fails the spawn rather than running the
    /// package unconfined.
    ///
    /// # Safety
    ///
    /// Must be called in a forked child before `exec`, with `ruleset` a live
    /// Landlock ruleset descriptor.
    pub(crate) unsafe fn apply(ruleset: RawFd) -> std::io::Result<()> {
        unsafe {
            // Landlock without `CAP_SYS_ADMIN` requires `no_new_privs`, which
            // also stops a set-user-ID `exec` from re-acquiring privilege.
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            // Irreversible and inherited across `exec`.
            if libc::syscall(libc::SYS_landlock_restrict_self, ruleset, 0_u32) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            // A fresh network namespace has one interface, `lo`, and it is
            // down. The user namespace is what makes the network unshare
            // available to an unprivileged process; it installs no UID map, so
            // the child's kernel-internal UID — and therefore every filesystem
            // permission check — is unchanged.
            if libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNET) != 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handled_rights_track_the_probed_abi() {
        // ABI 1 knows neither REFER nor TRUNCATE nor IOCTL_DEV.
        assert_eq!(handled_access_fs(1), (1 << 13) - 1);
        assert_eq!(handled_access_fs(2) & FS_REFER, FS_REFER);
        assert_eq!(handled_access_fs(2) & FS_TRUNCATE, 0);
        assert_eq!(handled_access_fs(3) & FS_TRUNCATE, FS_TRUNCATE);
        assert_eq!(handled_access_fs(4) & FS_IOCTL_DEV, 0);
        assert_eq!(handled_access_fs(5) & FS_IOCTL_DEV, FS_IOCTL_DEV);
        // A higher ABI is used at the level this adapter knows.
        assert_eq!(handled_access_fs(9), handled_access_fs(LANDLOCK_ABI_KNOWN));
        assert_eq!(handled_access_net(3), 0);
        assert_eq!(handled_access_net(4), NET_BIND_TCP | NET_CONNECT_TCP);
    }

    #[test]
    fn no_class_grants_write_and_execute_together_and_none_grants_refer() {
        let handled = handled_access_fs(LANDLOCK_ABI_KNOWN);
        for access in [
            PathAccess::ReadExecute,
            PathAccess::ReadOnly,
            PathAccess::ReadWrite,
        ] {
            let granted = granted_access_fs(access, handled);
            assert_eq!(granted & !handled, 0, "{access} grants an unhandled right");
            assert_eq!(granted & FS_REFER, 0, "{access} grants REFER");
            assert!(
                granted & FS_EXECUTE == 0 || granted & FS_WRITE_FILE == 0,
                "{access} grants write and execute together"
            );
        }
        assert_eq!(
            granted_access_fs(PathAccess::ReadOnly, handled) & FS_WRITE_FILE,
            0
        );
        assert_eq!(
            granted_access_fs(PathAccess::ReadOnly, handled) & FS_EXECUTE,
            0
        );
        assert_eq!(
            granted_access_fs(PathAccess::ReadExecute, handled) & FS_WRITE_FILE,
            0
        );
    }

    #[test]
    fn a_lower_abi_records_the_rights_it_cannot_gate() {
        let low = unconfined_inventory(1).join("\n");
        assert!(low.contains("REFER"));
        assert!(low.contains("TRUNCATE"));
        assert!(low.contains("IOCTL_DEV"));
        let high = unconfined_inventory(LANDLOCK_ABI_KNOWN).join("\n");
        assert!(!high.contains("REFER"));
        assert!(!high.contains("TRUNCATE"));
        assert!(!high.contains("IOCTL_DEV"));
        // Every inventory names the seccomp and admitted-path residuals.
        for abi in [1, LANDLOCK_ABI_KNOWN, 9] {
            let inventory = unconfined_inventory(abi).join("\n");
            assert!(inventory.contains("seccomp"));
            assert!(inventory.contains("Unix-domain socket"));
        }
        assert!(
            unconfined_inventory(9)
                .join("\n")
                .contains("introduced after")
        );
    }

    #[test]
    fn the_package_root_is_the_first_admitted_entry_and_the_policy_adds_nothing_implicitly() {
        let policy = ConfinementPolicy::default();
        assert_eq!(
            policy.entries("/package"),
            vec![("/package".to_owned(), PathAccess::ReadExecute)]
        );
        let named = ConfinementPolicy::linux_runtime_defaults();
        assert!(named.read_write.iter().all(|path| path == "/dev/null"));
        assert!(
            named
                .entries("/package")
                .iter()
                .all(|(path, _)| path.starts_with('/'))
        );
    }
}
