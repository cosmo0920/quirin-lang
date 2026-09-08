# P03-F — Remaining confinement bounds, plus the P09-lock CI rider

Follow-on card `P03-F` (wave 7), 2026-09-09. Baseline `d91316b`.

P03 shipped `linux-confined-v1` and recorded three things it did **not** close.
This card closes them:

| P03 open item | This card |
|---|---|
| (a) A per-execution child quota. `RLIMIT_NPROC` counts every process of the real UID, not of one tree (`P02-FLAKE`). | Implemented as a cgroup v2 `pids.max` quota under a new declared kind, `max_execution_processes` (§2, §3). |
| (b) Default profile wiring. Selection was Rust-only, so every configured route was stuck on the historical profile. | `ExecutionPolicy`: profile selection is explicit configuration; `unenforced-legacy` is still the recorded default (§4). |
| (c) Refusal grounds 1 and 4 implemented but never executed on a host. | Both now execute — ground 1 on the Windows CI job, ground 4 against a **genuinely** denying kernel (§5, §6). |

Plus the rider `P09-lock`: nothing in CI built the provider packages, which is
how `providers/Cargo.lock` stayed broken from P09 until the wave-6 repair. CI
now builds and verifies all five (§6.3).

## 1. Non-goals — read this before quoting anything below

- **Still not an OS sandbox.** This card adds one resource class (task count)
  to P03's two (filesystem, network). There is still no seccomp filter, no
  mount/PID/IPC/UTS namespace, no CPU or memory cgroup, and no capability drop
  beyond `PR_SET_NO_NEW_PRIVS` and the unmapped user namespace. P03 §8's
  trust boundary stands, with one line removed: descendants are now bounded in
  *number* by a per-tree quota when one is declared.
- **The quota bounds task count, nothing else.** It does not bound memory, CPU
  or descendant lifetime. `RLIMIT_AS`/`RLIMIT_CPU` from the composed limit
  profile remain per-process, and the process-group kill remains what ends
  descendants.
- **The quota is opt-in and environment-dependent.** A manifest that declares
  no `max_execution_processes` installs no quota and its record says so. A
  host without a delegated cgroup subtree **refuses** the declaration before
  spawning; there is no downgrade to `RLIMIT_NPROC` and no unbounded run.
- **No default profile change.** `unenforced-legacy` is still the default, and
  an `execution` block absent from a configuration means exactly that profile.
  What changed is that a stronger selection can now be *written down*.
- **No verdict semantics change.** Every non-completed provider outcome remains
  a scoped `Unknown` under `QP006` with no certificate, never a program
  `Unsafe`. No raw Cycle 19 unit is promoted, CT coverage is unchanged at
  24/50 and the contract corpus is unchanged at 60 modules.
- **Ground 4's two halves are not equally strong.** The user-namespace half is
  executed against a kernel that genuinely denies the mechanism. The Landlock
  half is executed through a test-only availability override, which exercises
  the refusal path and its pre-spawn semantics but is **not** the kernel's own
  `ENOSYS`/`EOPNOTSUPP` classification. §5.2 says so explicitly.

## 2. The mechanism: why cgroup v2 `pids`, and what an unprivileged harness can enforce

### 2.1 The problem `RLIMIT_NPROC` cannot solve

P02's `max_processes` installs `RLIMIT_NPROC`, which the kernel counts **per
real UID**. That makes it a bound on the account, not on one execution:

- A value high enough to let the account keep working is far above anything one
  provider would reach, so it bounds no fork storm.
- A value low enough to bound a storm is below the account's existing usage, so
  *every* fork fails — including the package's first one. P02's own fixture
  relies on exactly this, and its report says so.

There is no value in between, because the quantity being compared is the wrong
one. A per-execution quota needs a mechanism that counts the tree.

### 2.2 The candidates, and what was measured

| Candidate | Verdict on this kernel |
|---|---|
| `RLIMIT_NPROC` | Rejected: per-UID by definition. Retained unchanged for the `max_processes` kind. |
| PID namespace | Rejected: `unshare(CLONE_NEWPID)` affects the caller's *children*, so the `exec`ed package would stay in the old namespace while its children moved. And a PID namespace is not a quota — it renumbers, it does not count. |
| cgroup v2 `pids` controller | **Accepted and measured.** Counts a whole subtree, needs no privilege given a delegated writable cgroup, and rejects the fork with `EAGAIN` rather than killing anything. |
| cgroup v1 | Not attempted: this host is unified-only, and a v1 hierarchy would need a different delegation story with no measured benefit. |

The unprivileged feasibility was measured before anything was implemented. On
this host (`/sys/fs/cgroup`, cgroup2, `nsdelegate`), the invoking user owns its
systemd session subtree and `pids` is enabled in the parent's
`cgroup.subtree_control`. A direct probe created a cgroup there, wrote
`pids.max = 4`, moved a process in and had it fork 50 times:

```
controllers: memory pids
pids.max written: 4
fork failed after 3 at 3 : [Errno 11] Resource temporarily unavailable
pids.peak: 4      pids.events: max 1
```

Four tasks total (the moved process plus three forks), the fourth fork refused,
and the kernel's own counters recording it. That is a real per-tree quota
obtained with no privileged setup, so the honest answer here is *not* "nothing
is enforceable" — the mechanism exists and is implemented.

### 2.3 What is enforced, and where

`crates/qcore-verifier-host/src/process_quota.rs`. For one execution:

1. **Pre-spawn, in the parent.** Probe the delegation (§2.4). Create a
   per-execution cgroup beside this process's own — a *sibling*, because
   cgroup v2 forbids a cgroup from holding processes and enabling controllers
   for children at once. Write `pids.max`. Open its `cgroup.procs` for
   writing, with the parent's credentials.
2. **In the forked child, before `exec`.** Write `0` to that descriptor —
   the kernel's own spelling for "the writing process". This is the **first**
   `pre_exec` closure, before the Landlock restriction, before the namespace
   unshare and before every `setrlimit`: the child must already be a member
   when its first descendant could exist, and joining after the user-namespace
   unshare would depend on how the kernel re-checks a migration's credentials.
   The closure allocates nothing, takes no lock and makes one `write`.
3. **After the run, in the parent.** Read `pids.peak` and the `max` counter of
   `pids.events` *before* removing the cgroup — `rmdir` only succeeds on an
   empty cgroup and the counters go with it. Then write `cgroup.kill` and
   `rmdir` with bounded retries. Whether the removal succeeded is recorded, not
   assumed: a cgroup that could not be removed still held a task.

### 2.4 The probe, and what it refuses

`probe()` is cached per process, and **a failure is cached as a failure**, so an
unavailable mechanism refuses every execution rather than being retried into a
different answer — the same rule P03 applies to its two probes. It reads the
cgroup2 mount point from `/proc/self/mountinfo`, this process's own cgroup from
`/proc/self/cgroup`, and requires:

- a cgroup2 (unified) mount to exist;
- this process to be in it, with a parent to create a sibling beside;
- `pids` already present in that parent's `cgroup.subtree_control`;
- an actual `mkdir` there, and an actual write to the new cgroup's `pids.max`,
  both immediately undone. A permission bit the kernel would still refuse is
  not evidence, so writability is measured by doing it.

Each failure has its own message naming what was missing.

**Recorded boundary, deliberately not repaired:** where the parent has `pids` in
its `cgroup.controllers` but not in its `cgroup.subtree_control`, this refuses
rather than writing `+pids` into the parent. Enabling a controller on a shared
cgroup changes accounting for every sibling in it, which is not this harness's
to do. The remedy — delegating a subtree with `pids` already enabled — is named
in the refusal text.

## 3. Measured results

`crates/qcore-verifier-host/tests/execution_quota.rs`, 13 tests. Every positive
case reads the kernel's own counters for a cgroup that existed for exactly one
execution.

| # | Case | Measured |
|---|---|---|
| 1 | A 40-deep fork storm under `max_execution_processes = 4` | **peak 4**, **1 refused fork**, cgroup released, package still answered (`Completed`) |
| 1c | *Control:* the identical fixture under a quota of 64 | **peak 42**, **0 refused forks**, released |
| 2 | The same fixture with **no** declared quota | no quota installed, `process_quota: none`, both optional keys absent from the serialized record |
| 3 | A quota of 6 with one backgrounded descendant | **peak 4** (entrypoint, subshell, `sleep`, `cat`), 0 refusals, released |
| 4 | A declared quota of **0** | refused pre-spawn: *"admits no process at all, not even the package's own entrypoint"* |
| 5 | The quota kind under `linux-rlimit-v1` | refused pre-spawn, naming `linux-confined-v1` as the profile that enforces it |
| 6 | The quota kind under `unenforced-legacy` | not refused — named in `unenforced_kinds`, which is that profile's honest answer |
| 7 | A quota mechanism forced unavailable | refused pre-spawn, `enforced: None`, marker absent, no observation |
| 8 | Landlock forced unavailable | refused pre-spawn; registry reports `Unknown` / `QP006` / no certificate |
| 9 | User namespace **genuinely** denied by the kernel | refused pre-spawn with the kernel's own errno text (§5.2) |
| 10 | All three mechanisms forced, under `linux-rlimit-v1` | decision **unchanged**; the same rlimits install and the run completes |
| 11 | Both enforcing profiles off Linux | refused, *"unavailable on this target"* (executed on the Windows job) |
| 12 | A declared quota off Linux | refused, same ground |
| 13 | A written `ExecutionPolicy` selection | reaches the provider; both mismatch directions refuse |

**Row 1 versus row 1c is the load-bearing comparison.** The same fixture, the
same account, the same moment: bounded at 4 with a refusal, and 42 unbounded.
The bound is the quota, not the fixture's size, and it did not depend on the
account's process count at any point — which is precisely what `RLIMIT_NPROC`
cannot say.

### 3.1 P02-FLAKE: what changed and what deliberately did not

P02's `a_declared_process_limit_stops_the_fixture_from_forking` is load-flaky:
it asserts `ProcessOutcome::Terminated`, but under concurrent load the host can
observe `Io` (a broken pipe on the stdin write that lost the race) instead.
Both are faithful observations of "the fixture could not fork".

**That assertion is unchanged.** Weakening it to accept `Io` would be a real
loss of precision in P02's evidence, and this card does not make that call.

What this card adds is a **deterministic companion** under the new mechanism:
`the_quota_binds_without_depending_on_the_accounts_process_count`. It measures
the same intent — this execution's tree is bounded — without depending on the
account's process count, without needing the bound to be below existing usage,
and therefore without the race that makes P02's fixture flaky. The relationship
is: P02's test pins `RLIMIT_NPROC` behaviour and stays as it is; this one pins
per-tree quota behaviour and is deterministic. Neither supersedes the other,
because they enforce different bounds.

One assertion in that companion is **deliberately absent**, and the test says
so: it does not assert that the descendant never wrote its marker. The
descendant holds the inherited stdout/stderr pipes, so the host's read loop —
and therefore its process-group kill — only runs after the descendant exits, by
which time it has written. P02's fixture can assert marker absence precisely
because its descendant never forks at all. Asserting it here would pass only by
accident of timing. What *is* asserted instead is that the per-execution cgroup
was empty and removable afterwards, which is a measured fact about the whole
tree: nothing this execution started survived it.

## 4. Configured profile selection

`crates/qcore-verifier-host/src/execution_policy.rs`. `ExecutionPolicy` is a
serde record — a version, a profile, and an optional confinement block:

```json
{
  "version": 1,
  "profile": "linux-confined-v1",
  "confinement": {
    "base": "linux-runtime-defaults",
    "read_only": ["/etc/qcore/policy.d"],
    "read_write": ["/var/tmp/qcore-scratch"]
  }
}
```

**The default does not move.** `ExecutionPolicy::default()` is
`unenforced-legacy` with no policy; an absent `execution` key and an empty
object `{}` both deserialize to exactly that, and the key is omitted from
serialization when it holds the default — so `qcorec plugins enable` writes the
same bytes it wrote before this field existed, pinned by a test.

The configured route is `qcorec`'s explicit provider selection
(`crates/qcorec/src/plugin_config.rs`): `registry()` validates the written
selection **before opening any package**, then applies it to each provider
after identity verification and before registration, so every provider in that
registry runs under exactly the profile the configuration names. A new
`qcorec plugins profile [--config FILE]` prints the selection, so a run can
state what it enforced rather than leaving it implicit.

Every check this module makes is a check the provider already makes; it only
makes them *earlier*, where the configuration author can see them. Four
refusals, all fail-closed, none with a weaker fallback:

| Written selection | Refusal |
|---|---|
| A confining profile with no `confinement` block, or an empty one | *"requires an explicit confinement policy"* — a policy nobody wrote is a policy nobody reviewed |
| A `confinement` block under `unenforced-legacy` or `linux-rlimit-v1` | *"enforces no confinement policy"* — the other direction of P03's ground 3 |
| A relative admitted path | *"is relative"* — what it admits would depend on the reader's working directory rather than on the configuration |
| An unknown `version`, or an unknown key inside the block | refused rather than guessed; a misspelled profile key cannot be silently ignored into the default |

## 5. The availability override, and the executed refusal grounds

### 5.1 The override's design, and why it is safe to ship

`MechanismProbe` has exactly two variants: `AsProbed` (the default, and the
production answer) and `ForcedUnavailable`. **There is deliberately no variant
that forces a mechanism available.** That is the whole of its safety argument:
an override can only cause additional pre-spawn refusals, so no use of it — in
a test, in CI, or anywhere else — can produce an execution that ran with weaker
enforcement than the kernel offered. It is therefore exposed as an ordinary
API (`ProcessProvider::with_mechanism_availability`) rather than hidden behind
a build flag, where a `cfg` mistake could change production behaviour silently.

Three things pin the direction: a Rust test that a forced override leaves a
non-confining profile's decision byte-identical while refusing the confining
one; the refusal text, which names itself as an override and says *"not a
statement about this kernel"*, so no report can read it as a kernel fact; and
two Lean theorems (§7).

### 5.2 Ground 4, executed against a genuinely denying kernel

P03 recorded ground 4 (the kernel lacks a mechanism) as implemented but never
executed, because the development host has Landlock at ABI 8 and permits
unprivileged user namespaces.

For the **user-namespace half** that is now fixed without any override. An
unprivileged process can construct the denial: inside a user namespace it is
root *of that namespace*, and `/proc/sys/user/max_user_namespaces` is
namespaced and writable there, so writing `0` makes every further
`unshare(CLONE_NEWUSER)` fail with a real kernel errno:

```
$ unshare -Ur sh -c 'echo 0 > /proc/sys/user/max_user_namespaces; unshare -U -n true'
unshare: unshare failed: No space left on device
```

`a_kernel_that_denies_user_namespaces_refuses_the_confined_profile` re-executes
itself in exactly that environment and asserts the profile refuses before
spawning, with the kernel's own message, with no override set, with
`enforced: None`, and with the package demonstrably never run. Measured here:

```
P03F-GROUND4: executed against a genuinely denying kernel
```

Where the environment cannot be constructed (no `unshare(1)`, or a host that
denies unprivileged user namespaces outright), the test says so in its output
and asserts the same refusal through the override instead. CI greps for the
marker, so a silent regression to the weaker path is visible in the log.

For the **Landlock half** there is no unprivileged way to make a kernel that
has Landlock behave as one that does not — nesting a restriction does not make
the syscall absent. That half is executed through the override, which
exercises the refusal path, the pre-spawn semantics and the `QP006` registry
outcome, but not the kernel's `ENOSYS`/`EOPNOTSUPP` classification. Executing
that needs a runner whose kernel lacks Landlock, which remains open (§9).

### 5.3 Ground 1, executed on Windows

P03's confinement suite is `#![cfg(target_os = "linux")]`, so no job had ever
executed the *confining* profile's non-Linux refusal — it was argued by
construction from the limit profile's. The new suite's portable cases decide
before anything is spawned, so they compile and run on Windows, where there is
no Landlock, no user namespace and no cgroup v2 and the refusal is the real
one. They are named as their own CI step (§6.2).

## 6. The CI matrix

### 6.1 Linux job — executed refusal grounds and measured quota

```yaml
- name: Execute the confinement refusal grounds and the per-execution quota
  run: |
    cargo test -p qcore-verifier-host --test execution_quota -- \
      --nocapture --test-threads=1 2>&1 | tee quota.log
    grep -q 'P03F-GROUND4:' quota.log
```

`cargo test --workspace` already runs these; naming them separately with
`--nocapture` puts the executed refusal ground and the measured quota counters
in the CI log, where a regression to the override path would otherwise be
invisible. Validated locally: 13 passed, marker present, grep exit 0.

### 6.2 Windows job — the non-Linux refusal

```yaml
- name: Execute the non-Linux execution-profile refusals
  shell: pwsh
  run: cargo test -p qcore-verifier-host --test execution_quota -- --nocapture
```

Runs on both existing Windows matrix legs. Not runnable on this host; the
target-conditional code it exercises is the `#[cfg(not(target_os = "linux"))]`
arm of `linux_confinement`, `linux_process_quota` and `linux_rlimit_v1_limits`,
and the test's Linux-only helpers and imports are `cfg`-gated so the file
carries no dead code off Linux.

### 6.3 P09-lock rider

```yaml
- name: Build and verify the five provider packages against the pinned lockfile
  run: |
    python3 providers/package.py build --output "$RUNNER_TEMP/provider-packages"
    for package in qcore.chunkio qcore.cmetrics qcore.ctraces qcore.libevent qcore.librdkafka; do
      python3 providers/package.py verify "$RUNNER_TEMP/provider-packages/$package" > /dev/null
    done

- name: Run the provider workspace tests
  run: cargo test --manifest-path providers/Cargo.toml --workspace
```

`providers/package.py build` runs cargo with `--locked --offline`, so a
`providers/Cargo.lock` left stale by a version bump fails here. Nothing in CI
built the packages before, which is exactly how that lockfile stayed broken
from P09 until the wave-6 repair. The provider workspace tests were also never
run by CI; they are added in the same step group.

Validated locally: all five packages build and verify. **No manifest field was
added by this card** — `max_execution_processes` is an ordinary
`PluginLimits::additional` key, which is already a free-form map — so
`qcore-schemas` stays **6.0.0**, no wire format moves and no package needed
rebuilding. The five were rebuilt and verified anyway as part of the gate.

## 7. Lean

`proof/QCore/ProcessQuotaPlan.lean`, audited by
`proof/AuditProcessQuotaPlan.lean`: **29 theorems**, no new axiom, no
placeholder. The Lean gate is now **39 audits**.

The checkable relation is the *plan*, not the kernel: given a target, a profile
and the kinds a manifest declares, which mechanism installs each kind and what
happens to a kind no mechanism installs.

- **Refusal completeness** (the card's named target). Under an enforcing
  profile a declaration is either fully placed or refused **by the name of a
  kind it actually declared**; an accepted plan covers every declared kind;
  each kind sits in its own mechanism's list; and no accepted plan carries a
  zero quota. There is no outcome in which a declared kind is silently
  dropped.
- **The historical profile is not a third behaviour.** It never refuses and
  never enforces: it names every declared kind as unenforced, which is its own
  honest answer.
- **The two kinds are never confused.** `max_processes` (per UID) and
  `max_execution_processes` (per tree) are distinct, the two mechanism tables
  are disjoint, and their spellings differ — so a manifest can never obtain one
  while declaring the other.
- **The deliberate pin update, proved.** The two profiles' `setrlimit` tables
  stay *equal*; the total enforceable sets differ by *exactly* the quota kind.
- **Target qualification.** Off Linux no profile places any kind, the
  enforceable set is empty, and any nonempty declaration refuses.
- **The override is one-directional.** The default probe leaves the decision
  unchanged; a forced probe that yields an enforced plan implies the unforced
  decision was that same plan; and a forced probe grants no profile a mechanism
  it does not install.

**Not proved here, and stated as such in the module header:** that cgroup v2's
`pids` controller actually bounds a process tree (that is §3's measurement),
that Landlock and the namespace unshare confine what they claim, and that the
Lean tables are the Rust tables — the enumeration is a transcription, pinned on
the Rust side by `execution_quota.rs`.

## 8. Gate results

| Gate | Result |
|---|---|
| `git submodule update --init --recursive` | libevent at its pinned revision |
| `cargo test --workspace` | **1679 passed**, 0 failed, 1 ignored (baseline 1651) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **39 Lean audits passed** (baseline 38) |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **58 passed**, 0 failed |
| `python3 providers/package.py build` + `verify` x5 | all five build and verify |
| `python3 -B -m unittest discover -s scripts/tests` | **108 ran, OK** |

Test delta **+28**: 13 in `execution_quota.rs`, 6 unit tests in
`process_quota.rs`, 7 in `execution_policy.rs`, 1 CLI test in
`plugin_packages.rs`, and 1 assertion group folded into the existing
`confined_process_execution.rs` case.

### 8.1 One deliberate pin update

`confined_process_execution.rs::the_confined_profile_composes_with_the_limit_profile`
asserted `enforceable_limit_kinds(LinuxConfinedV1) == enforceable_limit_kinds(LinuxRlimitV1)`.
That is now false by design: the confining profile enforces one kind more.

The pin was updated **deliberately, keeping both load-bearing halves**:

- the composition claim is unchanged and still asserted twice — the composed
  `rlimits` vector still equals the limit profile's, and the new
  `enforceable_rlimit_kinds` are still exactly equal between the two profiles;
- the negative half is kept and strengthened — the limit profile must **not**
  contain the quota kind, and the difference between the two enforceable sets
  must be *exactly* `["max_execution_processes"]`, so a future kind cannot
  slip into the confining profile unnoticed.

`enforceable_limit_kinds` keeps its name and now returns every enforceable
kind; the `setrlimit` table is available separately as
`enforceable_rlimit_kinds`. Both are proved in Lean (§7).

### 8.2 Audit-artifact parity

No analysis code changed. The diff is the new host modules, two additive
optional evidence fields, the CLI configuration key, tests, CI and this report;
`compiler/`, `crates/qcore-c-gateway`, `crates/qcore-audit` and `contract/` are
untouched. The two new evidence fields (`EnforcedLimits::process_quota`,
`ProcessExecutionRecord::quota_observed`) are `Option` with
`skip_serializing_if`, so a record that installs no quota serializes
byte-identically to one written before they existed — pinned by a test in both
directions. No verdict, contract-corpus count or CT mapping moves. The pinned
185-unit Fluent Bit artifacts are unchanged **by construction, not
re-measured**: the pinned corpus is absent from this worktree and the audit's
own pin gate refuses to run. Cycle 19 raw units remain unpromoted at 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units; CT coverage
remains 24/50 mapped; the contract corpus remains 60 modules.

## 9. Open items this card does not close

- **A kernel without Landlock in the CI matrix.** Ground 4's Landlock half is
  still executed through the override, not against a kernel that lacks the
  syscall (§5.2). This needs a runner image, not code.
- **`pids` enablement on a parent that offers but does not enable it.** Refused
  rather than repaired, because enabling a controller on a shared cgroup
  changes accounting for its siblings (§2.4).
- **Memory and CPU cgroups.** A `memory.max` would give a per-tree memory bound
  that `RLIMIT_AS` cannot, and `cpu.max` a per-tree CPU bound. Neither is
  attempted here; only the `pids` controller is used.
- **seccomp**, and every namespace other than user and network. Unchanged from
  P03.
- **A manifest-declared confinement policy.** `filesystem_inputs` is still a
  declaration, not an enforced grant; the enforced policy is host-side
  configuration. Binding the two remains the natural next increment.
- **P02's `max_processes` fixture flake.** Deliberately not repaired (§3.1).
  Its assertion is unchanged; it now has a deterministic companion.
- **Non-Linux confinement.** macOS sandbox profiles and Windows job objects
  would each be a separate target-qualified profile with its own evidence.
- **Configured selection beyond `qcorec`.** `qcore-audit`'s own configuration
  does not yet carry an `execution` block; the host-side type is ready for it.
