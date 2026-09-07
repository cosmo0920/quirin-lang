# P03 — Enforced filesystem and network confinement for one supported target

Work card P03 (W5, implementation), 2026-09-07. Baseline checkpoint `a4492da`
(the remaining-work wave consolidation). This card adds a second
**target-qualified Linux execution profile** to the verifier host's
process-provider path, `linux-confined-v1`, which composes with P02's
`linux-rlimit-v1` and additionally confines two named resource classes —
the filesystem and the network — with mechanisms the kernel enforces.

P02 recorded this as its first open item: "filesystem and network confinement
(`filesystem_inputs`, `network_access` enforcement), and any namespace/cgroup/
seccomp mechanism". Before this card the child ran with the invoking user's full
filesystem and network access; `env_clear()` and `current_dir(package_root)`
were the only environmental restrictions.

## 1. Non-goals — read this before quoting anything below

- **Not a general OS sandbox.** Two resource classes are confined. No seccomp
  filter is installed, no cgroup is created, no mount/PID/IPC/UTS namespace is
  unshared, and no capability set is dropped beyond what `PR_SET_NO_NEW_PRIVS`
  and the unmapped user namespace imply.
- **Not confinement of the admitted paths.** Everything beneath an admitted
  path is fully reachable at its granted class, including any Unix-domain
  socket bound there. A policy that admits too much confines nothing useful,
  and this card supplies no reviewer for a policy's *content*.
- **No claim that JSON fields enforce anything.** `filesystem_inputs` and
  `network_access` remain manifest *declarations*. What this profile enforces is
  the host-side [`ConfinementPolicy`](../../../crates/qcore-verifier-host/src/confinement.rs)
  the caller supplies, plus the package root. The one place the manifest and the
  enforcement meet is `network_access`: `PluginManifest::validate` already
  rejects `network_access: true`, so a confined execution enforces exactly what
  every admissible manifest declares, and the evidence records the declaration
  it enforced.
- **No cross-platform isolation guarantee.** Off Linux the profile refuses every
  execution. macOS/BSD and Windows have no path here.
- **No verdict semantics change.** The core still refuses every protocol-v1
  `Safe` proposal; an unchecked `Unsafe` still downgrades to `Unknown`
  (`QP010`); every non-completed provider outcome is a scoped `Unknown` under
  `QP006` with no certificate, never a program `Unsafe`. No raw Cycle 19 unit is
  promoted; CT coverage is unchanged at 11/50; the contract corpus is unchanged
  at 59 modules.
- **Not a defense against a kernel bug or a privileged escape.** The claim is
  exactly: on a kernel that passes the probes in §3, the two mechanisms in §2
  were installed before `exec`, and the fixtures in §5 measured what that
  denied.

## 2. Mechanisms and why

Both are installed in the forked child, in the `pre_exec` closure, before
`execve`. The closure allocates nothing, takes no lock and makes only syscalls;
every object it needs is built in the parent.

### 2.1 Filesystem — Landlock

`landlock_create_ruleset` / `landlock_add_rule` / `landlock_restrict_self`
(Linux 5.13+). Chosen over a mount namespace because it needs no privileged
setup, no bind-mount tree and no `pivot_root`, it is unprivileged by design, and
it is *irreversible and inherited across `exec`* — a restricted process cannot
lift it, and neither can the package it `exec`s.

The ruleset is created in the parent, `handled_access_fs` set to **every**
filesystem right the probed ABI knows (§3.2), so an operation gated by a handled
right is denied unless a rule grants it. Rules are `LANDLOCK_RULE_PATH_BENEATH`
on descriptors the parent opens `O_PATH|O_CLOEXEC`. The child runs
`prctl(PR_SET_NO_NEW_PRIVS, 1)` — required for unprivileged
`landlock_restrict_self`, and independently useful because it stops a set-user-ID
`exec` from re-acquiring privilege — then `landlock_restrict_self`.

Three grant classes, and no class grants write and execute together:

| Class | Rights granted (masked by what the ABI handles) |
|---|---|
| `read-execute` | `EXECUTE`, `READ_FILE`, `READ_DIR` |
| `read-only` | `READ_FILE`, `READ_DIR` |
| `read-write` | `READ_FILE`, `READ_DIR`, `WRITE_FILE`, `REMOVE_DIR`, `REMOVE_FILE`, `MAKE_{CHAR,DIR,REG,SOCK,FIFO,BLOCK,SYM}`, `TRUNCATE`, `IOCTL_DEV` |

`LANDLOCK_ACCESS_FS_REFER` is *handled and granted to nothing*, so a rename or
hard link that crosses two admitted hierarchies is denied. A rule whose parent is
not a directory is narrowed to the file rights — the kernel rejects a
directory-only right on a file rule with `EINVAL` — and the directory test is an
`fstat` on the descriptor already opened, not a second path lookup, so it cannot
be raced.

The package root is admitted `read-execute` by the profile itself: the entrypoint
could not be `exec`ed otherwise. It is deliberately **not** writable, so a package
cannot rewrite its own executable or drop files beside it (§5 case 5).

### 2.2 Network — namespaces

`unshare(CLONE_NEWUSER | CLONE_NEWNET)`. A fresh network namespace has exactly
one interface, `lo`, and it is down, so no socket routed by that namespace
reaches a peer — this covers TCP, UDP, raw sockets and anything else the child
opens, not only the TCP that Landlock ABI 4 can gate. The abstract Unix-domain
socket namespace is scoped to the network namespace, so it is replaced with an
empty one too.

`CLONE_NEWUSER` is what makes the network unshare available to an unprivileged
process. **No UID map is written.** That is deliberate: without a map the
child's kernel-internal UID is unchanged, so every filesystem permission check
answers exactly as it did before, while `getuid()` reports the overflow UID.
Writing `0 <uid> 1` would instead make the child namespace-root, which is more
privilege than a provider needs.

From Landlock ABI 4 the ruleset *also* handles `LANDLOCK_ACCESS_NET_BIND_TCP`
and `LANDLOCK_ACCESS_NET_CONNECT_TCP` and grants neither. That is redundant with
the namespace and costs nothing; on this host both are handled (`0x3`).

Ordering in the child is `prctl` → `landlock_restrict_self` → `unshare` →
`setrlimit`. The Landlock restriction is inherited into the new user namespace,
so the unshare cannot be used to escape it.

### 2.3 Composition, not replacement

`linux-confined-v1` computes the *same* `setrlimit` plan as `linux-rlimit-v1`
from the same manifest, with the same derivation labels and the same refusal for
a declared kind it cannot enforce, then adds the confinement.
`enforceable_limit_kinds` returns the same six kinds for both, and a test asserts
the two profiles' `rlimits` vectors are equal for the same manifest — against
both a shell fixture and all five shipped packages. The host-side bounds (wall
clock, stdout/stderr caps, process-group `SIGKILL`) are unchanged.

## 3. Fail-closed: what is refused, before anything is spawned

Five independent grounds, none of which has a weaker fallback:

1. **Non-Linux target.** `linux-confined-v1` refuses with a message naming the
   two Linux-only mechanisms.
2. **No policy.** The profile has no default policy. A caller that selects it
   without `with_confinement_policy` is refused: *"requires an explicit
   confinement policy; it admits no filesystem path beyond the package root and
   has no default."* A policy nobody wrote is a policy nobody reviewed.
3. **A policy under a non-confining profile.** Handing a policy to
   `unenforced-legacy` or `linux-rlimit-v1` is refused in the other direction,
   so a caller can never hold a reviewed policy and an unconfined execution at
   the same time.
4. **The kernel lacks a mechanism.** `landlock_create_ruleset(NULL, 0,
   LANDLOCK_CREATE_RULESET_VERSION)` returning `ENOSYS` (no Landlock),
   `EOPNOTSUPP` (compiled in but absent from the enabled LSM list) or an ABI
   below the minimum; or the forked `unshare(CLONE_NEWUSER|CLONE_NEWNET)` probe
   failing. Each has its own message.
5. **The policy cannot be installed.** An admitted path the kernel refuses to
   open, or a rule it refuses to add.

Grounds 1–3 and 5 surface from `ProcessProvider::enforced_limits()` or from the
pre-spawn ruleset build; ground 4 surfaces from the cached probe. All of them
record `ProcessOutcome::RefusedByProfile` with `"enforced": null` — nothing ran,
so nothing was enforced — and reach the registry as `Unknown` under `QP006` with
no certificate.

### 3.1 The probes, and why one of them forks

`landlock_abi()` is a pure syscall. The namespace probe is not: a sysctl read
cannot answer it, because a distribution can permit unprivileged user namespaces
globally and still deny them to a particular executable through an LSM policy
(Ubuntu's `kernel.apparmor_restrict_unprivileged_userns` is exactly this, and it
is `1` on the host below). The only faithful probe is the syscall itself, in a
process whose credentials are the ones the child will have, so the probe forks a
child that calls `unshare` and `_exit`s with the errno. Both probes are cached
per process in a `OnceLock`, and **a failure is cached as a failure** — an
unavailable mechanism refuses every execution rather than being retried into a
different answer.

### 3.2 ABI handling is derived, not fixed

Handling a right the kernel does not know is `EINVAL`; *not* handling one it does
know silently leaves the operations it gates unrestricted. So the handled mask is
computed from the probed ABI: ABI 1 rights, `+REFER` at 2, `+TRUNCATE` at 3,
`+IOCTL_DEV` at 5, and network rights at 4. A kernel above the highest ABI this
adapter knows (5) is used at that level, and the shortfall is recorded in the
evidence rather than hidden — see the last `unconfined` entry in §4, which the
ABI 8 host below actually emits.

## 4. Evidence format

`EnforcedLimits` gains one optional field, `confinement: Option<Confinement>`,
`#[serde(default, skip_serializing_if = "Option::is_none")]`. A profile that
confines nothing omits the key entirely, so records written before this field
existed are byte-identical; a test pins both directions.

Every field is measured, not declared: the ABI comes from the running kernel's
version probe, the handled masks are what the ruleset was created with, and each
path's state is the result of opening it. An **absent** admitted path grants
nothing — which narrows the child's reach rather than widening it, so it is
recorded rather than refused, because a report quoting only the policy would
otherwise overstate what the child could reach.

Real output from a completed confined run on the host in §6 (path list elided to
its shape; the full 20-entry list is what the run emitted):

```json
{
  "profile": "linux-confined-v1",
  "wall_clock_ms": 30000,
  "stdout_bytes": 4194304,
  "stderr_bytes": 65536,
  "process_group_kill": true,
  "rlimits": [
    { "kind": "core_bytes",   "resource": "RLIMIT_CORE", "soft": 0, "hard": 0, "source": "profile-default" },
    { "kind": "cpu_seconds",  "resource": "RLIMIT_CPU",  "soft": 31, "hard": 31, "source": "derived-from-wall-clock" },
    { "kind": "memory_bytes", "resource": "RLIMIT_AS",   "soft": 4294967296, "hard": 4294967296, "source": "declared" }
  ],
  "unenforced_kinds": [],
  "confinement": {
    "filesystem_mechanism": "landlock",
    "network_mechanism": "unshare(CLONE_NEWUSER|CLONE_NEWNET)",
    "kernel_release": "7.0.0-30-generic",
    "landlock_abi": 8,
    "landlock_min_abi": 1,
    "handled_access_fs": "0xffff",
    "handled_access_net": "0x3",
    "network_access_declared": false,
    "paths": [
      { "path": "<package root>",     "access": "read-execute", "state": "admitted" },
      { "path": "/bin",               "access": "read-execute", "state": "admitted" },
      { "path": "/usr/lib",           "access": "read-execute", "state": "admitted" },
      { "path": "/etc/ld.so.cache",   "access": "read-only",    "state": "admitted" },
      { "path": "/proc/self/maps",    "access": "read-only",    "state": "admitted" },
      { "path": "/dev/null",          "access": "read-write",   "state": "admitted" },
      { "path": "<writable dir>",     "access": "read-write",   "state": "admitted" }
    ],
    "unconfined": [
      "filesystem access through a path admitted by the policy, including any Unix-domain socket bound beneath one",
      "the child's own address space, CPU and descendants beyond what the composed `linux-rlimit-v1` limits and the process-group kill bound",
      "System V IPC, POSIX message queues, and every namespace other than the user and network namespaces this profile unshares",
      "the system-call surface itself: no seccomp filter is installed, so any syscall the kernel permits under these credentials is reachable",
      "loopback traffic *within* the child's own network namespace, once the child brings `lo` up inside it",
      "filesystem access rights introduced after Landlock ABI 5, which this adapter does not handle on this ABI 8 kernel"
    ]
  }
}
```

The `unconfined` inventory is part of the record on purpose: a report cannot
quote the mechanisms without also carrying the boundary. A refusal instead
carries `"enforced": null` and the refusal text in `detail`, exactly as P02's
refusals do.

`EnforcedLimits::summary()` appends `confinement=…` for a confining profile and
`confinement=none` otherwise, so a legacy row states its own boundary rather than
leaving the question open.

### 4.1 Package runtime dependencies are explicit

`ConfinementPolicy::linux_runtime_defaults()` is a **named** policy a caller opts
into, never a fallback that applies when a caller supplies nothing. It names the
runtime files an execution needs on a conventional Linux layout — `/bin`,
`/sbin`, `/lib`, `/lib64`, `/usr/{bin,sbin,lib,lib64,libexec,share}` as
`read-execute` for the interpreter, the dynamic loader and the shared libraries
it resolves; `/etc/ld.so.cache`, `/etc/ld.so.conf`, `/etc/ld.so.conf.d`,
`/etc/localtime`, `/dev/urandom`, `/dev/zero`, `/proc/self/maps` as `read-only`;
`/dev/null` as the only `read-write` entry. Anything else, including the
package's own writable scratch space, is the caller's explicit addition. All
five shipped packages run under exactly this policy plus their package root.

## 5. Adversarial fixture results

`crates/qcore-verifier-host/tests/confined_process_execution.rs`, 12 tests. Each
fixture is a shell provider the test writes into a private temp tree with three
disjoint areas — the package the host executes, a directory the policy admits
for writing, and a directory the policy never names. Values below are from an
instrumented run of the same fixtures on the host in §6.

| # | Case | Confined | Control (`unenforced-legacy`) |
|---|---|---|---|
| 1 | Connect to a listening `127.0.0.1` server | exit **3**, 86.6 ms; server accepted **0** connections | `Completed` exit 0, 27.8 ms; server accepted **1** |
| 2 | `cat` a file outside the admitted paths | exit **5**, 3.2 ms | `Completed` exit 0, 4.4 ms |
| 3 | Write a file outside the admitted paths | exit **6**, 3.0 ms, *"Permission denied"*; target **absent** | `Completed` exit 0; target **present** |
| 4 | Write **and** read back inside the admitted writable directory, then answer | `Completed` exit 0, 6.2 ms; file contains `written` | — |
| 5 | Write into the package root | write denied, fixture answers normally, marker **absent** | — |
| 6 | Grow to 16 MiB under `memory_bytes = 8 MiB` | `Terminated` exit 2, *"Out of space"*, 5.3 ms | — |
| 7 | Confining profile, **no policy** | `RefusedByProfile` in **59 µs**; marker **absent** | — |
| 8 | Policy under `unenforced-legacy` / `linux-rlimit-v1` | `RefusedByProfile` both times; marker **absent** | — |
| 9 | Policy naming a path the kernel cannot open | `RefusedByProfile` in **239 µs**; marker **absent** | — |
| 10 | Policy naming a path that does not exist | recorded `absent`; grants nothing | — |
| 11 | Evidence fields, `unconfined` inventory, serde omission | — | — |
| 12 | Default profile unchanged, confines nothing | — | — |

Notes that matter for reading the table:

- **Case 1 is not vacuous.** The fixture runs `python3 -c 'pass'` first and
  exits **4** if the interpreter cannot start; it exits **3** only when the
  interpreter started and the *connect* failed. The measured exit code is 3.
  The count of reachable attempts is taken by draining the listener afterwards —
  a connect to a listening socket completes through the backlog whether or not
  the test accepts it — so 0 versus 1 is the kernel's answer, not a timing
  artifact. Incidentally, the confined child's stderr carried
  `Unable to read /etc/apt/apt.conf.d/ - opendir (13: Permission denied)` from
  the interpreter's own start-up: Landlock denying a path outside the policy,
  observed from inside the child.
- **Cases 1–3 each have a control.** The identical fixture, unconfined, reaches
  the server, reads the file and writes the file. Without those controls the
  confined rows would prove nothing about the enforcement.
- **The "marker absent" rows are the pre-spawn evidence.** The marker is written
  into the package root, which the confined profile makes read-only — so had a
  refused execution actually spawned, it would have spawned *unconfined*, and
  the marker would exist. Its absence is what distinguishes "refused before
  spawning" from "spawned and denied".
- **Case 9** is the reachable instance of "the mechanism could not be
  installed": a dangling symlink is a path the policy check sees as existing and
  the kernel refuses to open when the rule is added. See §8 for what this does
  *not* stand in for.
- Case 7 also asserts the registry path: `Unknown`, first diagnostic `QP006`, no
  certificate.

Four further unit tests in `confinement.rs` pin the ABI-derived handled masks
(including that a right is not handled below the ABI that introduced it, and
that an ABI above 5 is used at 5), that no class grants write and execute
together and none grants `REFER`, that a lower ABI records the rights it cannot
gate, and that the policy adds nothing implicitly beyond the package root.

### 5.1 Real packages

`providers/conformance/tests/packages.rs::real_packages_answer_identically_under_the_confined_profile`
builds all five shipped provider packages and, for each of their **11** declared
capabilities, asserts the confined-profile response is **equal** to the
legacy-profile response, for both protocol v1 and evidence v2, with the run
recorded as `completed` / exit 0 under `linux-confined-v1`. It also asserts the
composed `rlimits` vector equals `linux-rlimit-v1`'s for the same manifest, that
the package root is admitted `read-execute`, that `/dev/null` is the only
writable entry, and that no shipped manifest declares network access. The
packages are dynamically linked Rust binaries and need nothing beyond
`linux_runtime_defaults()` plus their package root.

## 6. Host and kernel

| | |
|---|---|
| Kernel | `7.0.0-30-generic` (Ubuntu, x86_64) |
| Enabled LSMs | `lockdown,capability,landlock,yama,apparmor,ima,evm` |
| Landlock ABI | **8** (this adapter handles rights up to ABI 5; `handled_access_fs = 0xffff`, `handled_access_net = 0x3`) |
| `kernel.apparmor_restrict_unprivileged_userns` | `1` — and the fork probe still succeeds for this executable, which is exactly why the probe is a syscall and not a sysctl read |

## 7. Schema version

**No `qcore-schemas` change.** The confinement policy is host-side configuration,
not a manifest declaration, and `EnforcedLimits`/`Confinement` live in
`qcore-verifier-host`, which publishes no versioned schema. `qcore-schemas`
stays **3.0.0**; manifests, protocol v1, evidence v1/v2, program schema/profile
v2 and every provider package version are unchanged, and no manifest's
serialization moves by a byte. P02's schema-versioning policy is followed by not
needing it.

## 8. Trust boundary — plainly, what is NOT confined

This is the same list the evidence record carries, stated once more because a
reader of this report should not have to fetch a JSON blob for it.

- **Everything beneath an admitted path**, at its granted class. Including a
  Unix-domain socket bound beneath one: filesystem sockets are gated by Landlock
  path rights, not by the network namespace, so an admitted writable directory
  is an IPC channel.
- **The child's own memory, CPU and descendants**, beyond the composed
  `setrlimit` bounds and the process-group kill. P02's boundary is inherited
  whole, including that `RLIMIT_NPROC` is per-UID and not a per-execution quota.
- **The system-call surface.** No seccomp filter. Any syscall the kernel permits
  under these credentials is reachable.
- **Every namespace except user and network.** No mount, PID, IPC, UTS, cgroup
  or time namespace. System V IPC and POSIX message queues are shared with the
  host.
- **Loopback inside the child's own network namespace**, once the child brings
  `lo` up there. It reaches nothing outside that namespace, but two processes the
  child spawns can talk to each other.
- **Filesystem rights introduced after Landlock ABI 5**, on a kernel that has
  them. Recorded per-execution rather than assumed away.
- **The policy's content.** Nothing here reviews whether a policy is tight. A
  policy admitting `/` read-write is installed exactly as written and recorded
  exactly as written.
- **The manifest-to-executable binding.** P01's finding **F2** is unchanged and
  now matters slightly more: a non-digest manifest edit that passes `qcorec
  plugins verify` changes the limits the composed profile installs. It still
  cannot admit a `Safe` verdict, and `package.py verify` still detects the edit.
- **Refusal grounds not reproduced on this host.** Grounds 1 (non-Linux) and 4
  (kernel lacks Landlock, or denies the namespace) are implemented and reachable
  but were **not** executed here: this host is Linux, its Landlock is enabled at
  ABI 8, and its namespace probe succeeds. They are argued by construction —
  they are the same `Protocol` refusal shape as grounds 2, 3 and 5, which *are*
  measured above with the marker evidence — and by the ABI-mask unit tests that
  exercise the low-ABI branches directly. Reproducing them needs a kernel
  without Landlock and one with unprivileged user namespaces disabled; that is a
  CI-matrix item, recorded in §10.

## 9. Gate results

| Gate | Result |
|---|---|
| `git -c protocol.file.allow=always submodule update --init --recursive` | libevent at pinned `d82464a2` |
| `cargo test --workspace` | **1013 passed**, 0 failed (76 suites; baseline 997 / 74) — see the flake note below |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cd proof && lake build` | 64 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | **21 Lean audits passed** |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **48 passed**, 0 failed (16 suites; baseline 47) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean |
| `python3 -B -m unittest discover -s scripts/tests` | **43 ran, OK** |

Test deltas: **+16** in the core workspace (12 in
`crates/qcore-verifier-host/tests/confined_process_execution.rs`, 4 unit tests in
`crates/qcore-verifier-host/src/confinement.rs`) and **+1** in the provider
workspace. No existing test was modified.

No Lean definition or theorem changed. This card adds a host execution profile,
not a semantic rule, so there is no Lean parity obligation to discharge; the 21
audits are re-run unchanged.

### 9.1 One observed flake, not repaired here

On the **first** `cargo test --workspace` run, P02's
`enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
failed:

```
assertion `left == right` failed: Broken pipe (os error 32)
  left: Io
 right: Terminated
```

It passes in isolation and passed on the immediately following full-workspace
run (the 1013/0 row above). The cause is a race this card makes more likely
without introducing: that fixture declares `max_processes = 1`, which — as P02's
own report records — binds only by being *below the account's existing usage*, so
every `fork` fails and the child dies immediately. Whether the host observes
`Terminated` (the child's exit status) or `Io` (a broken pipe on the stdin write
that lost the race) depends on scheduling, and this card's suite adds concurrent
load, including two `python3` starts in the network fixture.

Both outcomes are faithful observations of "the fixture could not fork", and both
are scoped `Unknown` at the registry. **The assertion was not weakened**: making
it accept `Io` would be a real loss of precision in P02's evidence, and that is
not this card's call to make. Recorded for the integrator as a pre-existing
fragility in P02's fixture, surfaced by added load; a repair belongs with P02's
test, not here.

### Audit-artifact parity

No analysis code changed: the whole diff is the new host module, the additive
profile variant and its evidence field, two test files and documentation. No
production Rust outside `crates/qcore-verifier-host` is touched, no Lean model
changes, and no `contract/` file changes, so no verdict, contract-corpus count or
CT mapping moves. The pinned 185-unit Fluent Bit artifacts are unchanged **by
construction, not re-measured** — as B01, P02 and C02 all recorded, the pinned
corpus checkout is absent from this worktree and the audit's own pin gate
refuses to run. Cycle 19 raw units remain unpromoted at 2 Safe / 42 provisional
Unsafe / 137 Unknown plus four data-only units; CT coverage remains 11/50 mapped;
the contract corpus remains 59 modules.

## 10. Open items this card does not close

- **Default profile wiring.** The default is still `unenforced-legacy`.
  Selection of `linux-confined-v1` stays API-level and explicit, deliberately:
  a registry policy, a `qcorec plugins` configuration key or a first-party
  discovery default would change what existing evidence means, and needs its own
  parity record.
- **A policy source of truth.** No configuration file, manifest field or CLI
  flag expresses a `ConfinementPolicy` yet; it is a Rust value a caller builds.
  Binding it to a package (so `filesystem_inputs` becomes an enforced grant
  rather than a declaration) is the natural next increment.
- **A per-execution process quota** and any cgroup mechanism. Still open from
  P02; a cgroup would also give a per-tree memory bound that `RLIMIT_AS` cannot.
- **seccomp.** Named as unconfined above, not attempted here.
- **A negative-kernel CI leg.** §8's last bullet: a runner without Landlock, and
  one with `kernel.unprivileged_userns_clone = 0`, to execute refusal grounds 1
  and 4 rather than argue them.
- **Non-Linux confinement.** macOS sandbox profiles and Windows job
  objects/AppContainer would each be a separate target-qualified profile with its
  own evidence.
- **P02's `max_processes` fixture flake** (§9.1). Not repaired here because
  repairing it means editing P02's assertion.
- **Mount-namespace confinement of `/proc`.** `/proc/self/maps` is admitted
  read-only by the named default policy because the Rust runtime reads it;
  admitting a `/proc` path at all is a wider grant than a `procfs` remount inside
  a mount namespace would be.
