# P02 — Enforced process resource limits for one supported target

Work card P02 (W5, implementation), 2026-09-07. Baseline checkpoint `7c8e93f`
(P01 merged). This card adds a **target-qualified Linux execution profile** to
the verifier host's process-provider path that actually installs the limits a
provider manifest declares, and refuses — rather than silently downgrades —
whenever it cannot.

Before this card, `PluginLimits` was a pure declaration. P01 recorded that
explicitly: "the manifest's `limits`, `capabilities`, `filesystem_inputs` and
`network_access` fields are *declarations* consumed by the host, not enforcement
mechanisms", with containment limited to the Unix process group, the wall-clock
deadline and the output caps. `memory_bytes` in particular was never applied to
anything.

## 1. Non-goals (read this before quoting anything below)

- **Not an OS sandbox.** No namespace, cgroup, seccomp, capability-drop or
  chroot mechanism is used or claimed.
- **No filesystem or network confinement.** That is card **P03**, explicitly out
  of scope here. The child still runs with the invoking user's full filesystem
  and network access; `env_clear()` and `current_dir(package_root)` are the only
  environmental restrictions, and both predate this card.
- **Not a defense against a deliberately escaping process.** A child that calls
  `setsid`, re-`fork`s out of the group, or raises a limit it can raise is not
  contained. `setrlimit` hard limits *are* irreversible downward for an
  unprivileged process, which is what makes the memory/CPU bounds hold, but the
  process-group kill remains a cooperative-descendant mechanism.
- **No verdict semantics change.** The core still refuses every protocol-v1
  `Safe` proposal; unchecked `Unsafe` still downgrades to `Unknown` (`QP010`);
  no raw Cycle 19 unit is promoted. Enforcement bounds *how a provider runs*, not
  what its output is allowed to mean.
- **Non-Linux is unavailable, not weakened.** Off Linux the profile refuses
  every execution rather than silently applying a partial bound.

## 2. Profile design

### 2.1 Additive selection

`ProcessExecutionProfile` (in `crates/qcore-verifier-host/src/lib.rs`) has two
values and defaults to the historical one:

| Profile | Name | Behavior |
|---|---|---|
| `UnenforcedLegacy` *(default)* | `unenforced-legacy` | Exactly the pre-existing path: wall-clock deadline, bounded stdout/stderr reads, process-group kill. Installs **no** `setrlimit`. |
| `LinuxRlimitV1` | `linux-rlimit-v1` | Linux only. All of the above **plus** every declared limit kind installed with `setrlimit` in the child before `exec`. Refuses a declared kind it cannot enforce. |

Selection is by an explicit builder call:

```rust
let provider = ProcessProvider::from_package(&package)?
    .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1);
```

Nothing selects the enforced profile by default — no registry, CLI flag, env
var or first-party discovery path changed. Every pre-existing caller keeps the
legacy behavior it had, which is why all 821 baseline tests pass unmodified.

### 2.2 Declared limit kinds

`PluginLimits` gains one optional field, `additional: BTreeMap<String, u64>`,
carrying limit kinds beyond the three mandatory fields. It is
`#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`, so manifests
that declare nothing serialize byte-identically to before and manifests written
before the field existed still load. `PluginManifest::validate` rejects a kind
name that is not 1–64 bytes of `[a-z0-9_]`, and rejects re-declaring
`timeout_ms` / `memory_bytes` / `output_bytes` there (that would be two
declarations of one kind, resolved silently in favor of one).

`additional` is still only a *declaration*. Which kinds are enforced is a
property of the selected profile, and that is the whole point of §2.4.

### 2.3 Enforced kinds — `linux-rlimit-v1`

| Kind | Mechanism | Source when absent |
|---|---|---|
| `memory_bytes` *(mandatory manifest field)* | `RLIMIT_AS` | — always declared |
| `cpu_seconds` | `RLIMIT_CPU` | derived as `ceil(timeout_ms/1000) + 1` |
| `core_bytes` | `RLIMIT_CORE` | profile default `0` |
| `max_file_bytes` | `RLIMIT_FSIZE` | not installed |
| `max_open_files` | `RLIMIT_NOFILE` | not installed |
| `max_processes` | `RLIMIT_NPROC` | not installed |

Host-side (both profiles, unchanged): the wall-clock deadline from
`timeout_ms`, the stdout payload cap from `output_bytes`, a separate 64 KiB
stderr cap, and the process-group `SIGKILL` on every exit path.

Soft and hard limits are set equal, so the child cannot raise one back. The
values are computed in the parent and installed by a `pre_exec` closure that
allocates nothing, takes no lock and calls only `setrlimit`; a failed
`setrlimit` fails the spawn rather than running the package with a limit
missing.

Two honest notes on the mechanisms:

- **`RLIMIT_CPU` is derived, not omitted, when undeclared.** A wall-clock
  deadline cannot by itself bound CPU burned by a spinning child during the
  reader loop's sleeps, so the profile always installs a CPU ceiling. The record
  labels it `derived-from-wall-clock`, never `declared`.
- **`RLIMIT_NPROC` counts processes per *real UID*, not per process tree.** A
  value large enough to be safe for the account is therefore not a meaningful
  per-execution bound, and a value small enough to bind (as in §4 case 4b)
  binds by being below the account's *existing* usage. It is enforced because
  it is declarable, but the real containment for descendants is the
  process-group kill, not `RLIMIT_NPROC`. Reports must not present it as a
  per-execution child-process quota.

### 2.4 Refusal — what the profile will not run

Fail-closed, checked before anything is spawned:

1. **Declared kind the profile cannot enforce.** Any `additional` key outside
   the table above.
2. **Profile unavailable on this target.** Any non-Linux build.

Both surface as `ProviderFailure::Protocol` with an explicit message naming the
kind and the enforceable inventory, which the registry reports as `Unknown`
under `QP006`. `ProcessProvider::enforced_limits()` exposes the same check as a
pure precheck, so a caller can refuse at configuration time instead of at
dispatch. There is no path from a refusal to a weaker profile.

The legacy profile does **not** refuse an unrepresented kind — it never enforced
anything, including `memory_bytes` — but it now *names* what it did not enforce
(§3), so a report cannot silently present a legacy run as bounded.

## 3. Evidence format

`ProcessProvider::last_execution()` returns a `ProcessExecutionRecord` for the
most recent execution. Achieved limits are separated into three groups: what the
OS applied (`rlimits`), what the host applied in its own read loop
(`wall_clock_ms`, `stdout_bytes`, `stderr_bytes`, `process_group_kill`), and
what nothing applied (`unenforced_kinds`).

Real output for a completed enforced run (`serde_json`):

```json
{
  "profile": "linux-rlimit-v1",
  "enforced": {
    "profile": "linux-rlimit-v1",
    "wall_clock_ms": 5000,
    "stdout_bytes": 4194304,
    "stderr_bytes": 65536,
    "process_group_kill": true,
    "rlimits": [
      { "kind": "core_bytes",     "resource": "RLIMIT_CORE",   "soft": 0,         "hard": 0,         "source": "profile-default" },
      { "kind": "cpu_seconds",    "resource": "RLIMIT_CPU",    "soft": 6,         "hard": 6,         "source": "derived-from-wall-clock" },
      { "kind": "max_open_files", "resource": "RLIMIT_NOFILE", "soft": 256,       "hard": 256,       "source": "declared" },
      { "kind": "memory_bytes",   "resource": "RLIMIT_AS",     "soft": 268435456, "hard": 268435456, "source": "declared" }
    ],
    "unenforced_kinds": []
  },
  "outcome": "completed",
  "wall_time_ms": 5,
  "stdout_bytes": 132,
  "stderr_bytes": 0,
  "exit_code": 0,
  "signal": null,
  "detail": ""
}
```

A refusal carries `"enforced": null` — nothing ran, so nothing was enforced:

```json
{
  "profile": "linux-rlimit-v1",
  "enforced": null,
  "outcome": "refused-by-profile",
  "wall_time_ms": 0, "stdout_bytes": 0, "stderr_bytes": 0,
  "exit_code": null, "signal": null,
  "detail": "execution profile `linux-rlimit-v1` cannot enforce declared limit kind `network_bandwidth_bps`; it enforces [core_bytes, cpu_seconds, max_file_bytes, max_open_files, max_processes, memory_bytes]"
}
```

A legacy run states its own boundary rather than implying one:

```
profile=unenforced-legacy; wall_clock_ms=5000; stdout_bytes=4194304;
stderr_bytes=65536; process_group_kill=true; rlimits=[];
unenforced=[max_open_files, memory_bytes]; outcome=completed; ...
```

`ProcessOutcome` values: `completed`, `refused-by-profile`, `spawn-failed`,
`wall-clock-timeout`, `output-limit`, `terminated`, `io`, `protocol`. Every
non-`completed` value is an environment fact about the provider run. All of them
already reach the registry as `ProviderFailure`, which becomes
`VerificationOutcome::unknown("QP006", …)` with no certificate — a **scoped
`Unknown`**, never a program `Unsafe`, and never dropped.

## 4. Adversarial test outcomes

Fixtures are shell providers written by the test into a private temp directory
(`crates/qcore-verifier-host/tests/enforced_process_limits.rs`, 12 tests). Values
below are from an instrumented run of the same fixtures on this host.

| # | Case | Observed result | Bound |
|---|---|---|---|
| 1 | Memory growth to 16 MiB under `memory_bytes = 8 MiB` | `Terminated`, exit 2, stderr `Out of space`, **4.7 ms** | `RLIMIT_AS` |
| 1c | *Control:* same fixture, legacy profile | `Completed`, exit 0, 36 ms — the allocation succeeds | none installed |
| 2 | 20 KiB of stdout under `output_bytes = 4096` | `OutputLimit`, **1.6 ms**, ≤ 12 KiB buffered | host read cap + group kill |
| 3a | `/bin/sleep 30` under `timeout_ms = 300`, `cpu_seconds = 60` | `WallClockTimeout`, **300.6 ms** | wall clock |
| 3b | `while :; do :; done` under `timeout_ms = 60000`, `cpu_seconds = 1` | `Terminated`, **signal 9**, **1.004 s** | `RLIMIT_CPU` |
| 4a | Fixture backgrounds `sleep 1` then exits 0 | `WallClockTimeout` at 300 ms; marker file **absent** after 1.5 s | process-group kill |
| 4b | Same fixture under `max_processes = 1` | `Terminated`, exit 2, stderr `Cannot fork`, **2.7 ms**; marker **absent** | `RLIMIT_NPROC` |
| 5 | Manifest declares `network_bandwidth_bps` | `RefusedByProfile` in **51 µs**; fixture's marker file **absent** — never spawned | pre-spawn refusal |
| 6 | Well-behaved provider, generous limits | `Completed`, exit 0, valid response decoded | — |

Case 1 is the non-vacuousness control for the whole set: the identical fixture
completes under the legacy profile and dies under the enforced one, so the bound
demonstrably comes from the enforcement.

Case 3b distinguishes the two time bounds: the wall clock was 60 s and the run
ended at 1.0 s by `RLIMIT_CPU`.

Case 5 is checked three ways — the pure precheck `enforced_limits()`, the
`verify` refusal, and the registry outcome (`Unknown`, `QP006`, message contains
the kind, no certificate) — plus a legacy control showing the same manifest
still runs there with `unenforced_kinds = [memory_bytes,
network_bandwidth_bps]`. That is the "labeled legacy, never a silent downgrade"
property.

Additional tests in the same file pin the derived-CPU labeling, the published
enforceable-kind inventory, the target qualification, and that the default
profile is still `unenforced-legacy` with no `setrlimit`.

### Real packages under the enforced profile

`providers/conformance/tests/packages.rs::real_packages_answer_identically_under_the_enforced_profile`
builds all five shipped provider packages and, for each of their 11 declared
capabilities, asserts the enforced-profile response is **equal** to the
legacy-profile response, for both protocol v1 and evidence v2, with the run
recorded as `completed` / exit 0 and `rlimits = [core_bytes, cpu_seconds,
memory_bytes]` matching the manifest's declared `memory_bytes`. No shipped
manifest declares a kind the profile cannot enforce, so none is refused.

## 5. Schema version

`PluginLimits` gained a public field, which the documented policy in
`docs/reports/schema-1-compatibility/README.md` ("a major version when public
fields or serde shapes change") makes a breaking change. `qcore-schemas` is
therefore **3.0.0**, and all seven workspace consumers, the provider workspace
and the isolated W3 process fixture now require `3.0.0`; all three lockfiles
were regenerated offline with only that version changed. Protocol v1, evidence
v1/v2, program schema/profile v2 and every provider package version are
unchanged.

Serde parity is pinned by
`crates/qcore-schemas/tests/plugin_limit_kinds.rs` (4 tests): a manifest with no
declared kinds emits no `additional` key, a pre-existing manifest still loads,
and `deny_unknown_fields` still rejects a typo.

## 6. Gate results

| Gate | Result |
|---|---|
| `git -c protocol.file.allow=always submodule update --init --recursive` | libevent at pinned `d82464a2` |
| `cargo test --workspace` | **837 passed**, 0 failed (66 suites; baseline 821 / 64) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | **47 passed**, 0 failed (16 suites; baseline 46) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `python3 -m unittest discover -s scripts/tests` | **43 ran, OK** |
| `cd proof && lake build` | 62 jobs, success |
| `python3 -B scripts/check_lean_audits.py` | **20 Lean audits passed** |

Test deltas: +12 (`enforced_process_limits.rs`), +4 (`plugin_limit_kinds.rs`) in
the core workspace; +1 (`real_packages_answer_identically_under_the_enforced_profile`)
in the provider workspace. No existing test was modified.

### Audit-artifact parity

The configured CMetrics process audit — the production route that actually
dispatches to a process provider — was re-run against the prepared persistent
build and is unchanged:

```
cargo run -p qcore-audit -- \
  build/cycle19/cmetrics-clang-reproduction/fluent-bit-clang-production/cmetrics-audit.json.config
→ source contracts: 1 checked function contract(s) / 6 Unknown; no whole-unit promotion
  1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

The full 185-unit Fluent Bit audit was **not** re-run in this worktree: as B01
recorded, `cycle19_fluent_bit_libraries_audit` is rejected at its pin gate here
(fail-closed, exit 1) because the pinned corpus checkout is not present. No
production analysis code changed — the diff is the schema field, the additive
host profile, docs and tests — so no verdict, contract-corpus count or CT
mapping moves. Cycle 19 raw units remain unpromoted at 2 Safe / 42 provisional
Unsafe / 137 Unknown plus four data-only units; CT coverage remains 11/50.

## 7. Open items this card does not close

- **P03**: filesystem and network confinement (`filesystem_inputs`,
  `network_access` enforcement), and any namespace/cgroup/seccomp mechanism.
- A per-*execution* child-process quota. `RLIMIT_NPROC` is per-UID; a real
  per-tree bound needs a cgroup, which is P03 territory.
- Wiring the enforced profile into a default selection path (registry policy,
  `qcorec plugins` configuration, or the first-party discovery route). Selection
  stays API-level and explicit here, deliberately.
- Enforcement of `capabilities`. Those remain routing declarations.
- P01's finding **F2** — non-digest manifest declarations are not bound to the
  executable's embedded template at `enable` time — is unchanged and now matters
  slightly more: a widened `limits` edit that passes `qcorec plugins verify`
  changes what the enforced profile installs. It still cannot admit a `Safe`
  verdict, and `package.py verify` still detects the edit.
- Non-Linux enforcement. macOS/BSD `setrlimit` would be a separate profile with
  its own evidence; Windows has no equivalent path here.
