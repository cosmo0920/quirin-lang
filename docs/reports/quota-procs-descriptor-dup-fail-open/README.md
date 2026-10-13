# QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN: the quota join is fail-closed

Card of wave 48, base `e0b92ece6`, prediction frozen at `b5da3bb4b`
([PREDICTION.md](PREDICTION.md)). Authority: the ledger entry of the same
name (recorded by EXECUTION-QUOTA-LOAD-INDEPENDENCE, found by reading) and
hard invariant 1. The card changes `crates/qcore-verifier-host`
(`src/lib.rs`, `src/process_quota.rs`, `tests/execution_quota.rs`),
`QCore.ProcessQuotaPlan` (six theorems) and constraint
[26](../../agents/providers.md#constraint-26). No analyser, checker, contract
or verdict code changes.

## 1. The defect, reproduced

`quota_descriptor` mapped `QuotaSession::procs_descriptor()`, a `dup` of the
per-execution cgroup's `cgroup.procs` write descriptor, through `.ok()`. If
the `dup` failed (`EMFILE`, `ENFILE`, `ENOMEM`), `quota_procs` was None, so:

* `run_process` installed no join `pre_exec`, and the child ran outside its
  cgroup;
* `verify_process` still called `finish_quota`, which recorded an observation
  of the empty cgroup, under `EnforcedLimits` whose `process_quota` named the
  quota.

The ledger's claim reproduces. Mutation `m1` (§4) restores the `.ok()` mapping
with the new test seam in place. The forced execution then **completed**:
`verify` returned the provider's response, so the package ran without joining
its quota.

## 2. Audit of the quota path

These are every `.ok()`, `unwrap_or*`, `let _ =` and unchecked count between
`QuotaSession::create` and the child's join, read at the base:

| # | Site | Child outside its quota while the record names it? | Action |
|---|---|---|---|
| 1 | `quota_descriptor`: `procs_descriptor().ok()` | **yes**, the defect | **fixed**: refuse before spawning |
| 2 | `verify_process`: `finish_quota` for every execution that created a cgroup, including an identity refusal, a pre-spawn `Protocol` error and a failed spawn (a failed join fails the spawn) | no child outside it, but an **observation of a cgroup no child was shown to join** | **fixed**: observe only when `spawn` returned a child |
| 3 | `process_quota::linux::join`: only `written < 0` checked | not on Linux: a kernfs `cgroup.procs` write returns the whole count or an error | **hardened**: any count other than 2 is a failed join (`EIO`, allocation-free) |
| 4 | `QuotaSession::create`: clock `unwrap_or_default()` | no: it feeds names only, the sequence number keeps them distinct, and `EEXIST` refuses | none |
| 5 | `QuotaSession::create`: `let _ = remove_dir` on its failure path | no: cleanup after an error that is returned as a refusal | none |
| 6 | `finish` / `release`: counter `.ok()`, `let _ =` on `cgroup.kill` | no: after the run, `Option` measurement fields | none |

`prepare_confinement` and `prepare_quota` propagate with `?` and already
refuse before spawning. `std::process::Command` runs `pre_exec` closures in
registration order, and a failing closure fails the spawn. The join is the
first closure registered, so a child returned by `spawn` has joined.

## 3. The repair

* `quota_descriptor` returns `Result<QuotaProcs, String>`. It is `Ok(None)`
  only when no quota was declared. `verify_process` chains
  `prepare_quota_join` after `prepare_quota`, so a failed duplicate takes the
  existing pre-spawn refusal arm:
  - `RefusedByProfile`, with `enforced` None and `quota_observed` None;
  - the detail names the profile, the kind, the `cgroup.procs` path and the
    errno;
  - dropping the session removes the cgroup.
* `SpawnObservation::spawned` is set after `spawn` returns `Ok`.
  `quota_observed` is `finish_quota(quota)` only then. Otherwise the session is
  dropped (its `Drop` kills and removes the cgroup), and no observation is
  recorded.
* `join_outcome(written, expected, os_error)` refuses a negative count with
  the kernel's error and any other short or long count with `EIO`.

**Test seam.** `RLIMIT_NOFILE` cannot isolate the duplicate. `create` opens
`pids.max`, then `cgroup.procs`, then closes `pids.max`, so the `dup` takes
the slot `pids.max` freed, and any limit that admits the cgroup's creation
admits the duplicate. `MechanismAvailability` gains a fourth field,
`quota_join_descriptor`, with the same one-directional `MechanismProbe`.
Forced unavailable, it is **not** consulted by the plan. The cgroup is
created, and `procs_descriptor` then fails with `EMFILE` at its real call
site. The detail appends the override's own text ("not a statement about this
kernel"), so a forced refusal never reads as a kernel fact. `any_forced`
counts the new field. Like the other overrides, it can only add a refusal.
The one in-repository struct literal that names every field
(`a_forced_override_never_weakens_an_execution`) gains the field, forced.

**Record shape left as it was.** A `SpawnFailed` record still carries the
planned `enforced` limits, now with no observation. Three pre-spawn
`Protocol` errors inside `run_process` also keep that shape. Neither runs a
package; this is registered as a follow-up (§8).

## 4. Tests, and what they catch

`execution_quota` 13 → **16**; `qcore-verifier-host` unit tests 31 → **32**.

| Test | Asserts |
|---|---|
| `a_failed_join_descriptor_refuses_before_spawning` | On a verified, installed package with a quota of 4, the forced duplicate refuses: `RefusedByProfile`; detail names the kind, `cgroup.procs`, `os error 24` and the override; `enforced`, `quota_observed`, `identity_window_us`, exit code and signal all None. The marker in the confinement's writable directory is absent. Through a registry the verdict is `Unknown` / `QP006` with no certificate. Control: the same package without the override completes, writes the marker, has an identity window and an observation with peak ≥ 1, and is released. |
| `an_identity_refusal_after_the_cgroup_exists_records_no_observation` | The verified script is rewritten in place after verification. The outcome is `RefusedByIdentity`, with no observation and no marker. |
| `a_failed_spawn_records_no_observation` | A mode-600 entrypoint fails `execve` after the join. The outcome is `SpawnFailed` (`ProviderFailure::Spawn`), with no observation and no marker. |
| `process_quota::tests::only_a_whole_join_write_is_a_join` | Counts 2 → Ok; 0, 1 and 3 → `EIO`; −1 → the kernel's error; the new field counts as forced. |

Each Linux row asserts a refusal and says so if the host delegates no cgroup
subtree. On this host every leg ran: the "no delegated subtree" line was not
printed.

**Mutations.** [`scripts/mutations.py`](scripts/mutations.py) reverts each
repair alone, runs the target that must catch it, restores the bytes and
asserts the round trip. It ran at the code commit `6167e17a9`; logs are in
[artifacts/mutations/](artifacts/mutations/).

| Mutation | Reverts | Caught by | Result |
|---|---|---|---|
| `m1` | the `.ok()` mapping (base behaviour) | `a_failed_join_descriptor_refuses_before_spawning` (verify returned Ok: the package ran) | caught |
| `m2` | observation of every created cgroup | the identity-refusal and failed-spawn rows | caught |
| `m3` | the short-count check | `only_a_whole_join_write_is_a_join` | caught |

**3 of 3 caught.**

## 5. Lean

`QCore.ProcessQuotaPlan` gains a model of one execution's steps
(`ExecutionSteps`: quota declared, descriptor obtained, join written, rest
succeeded) and its record (`QuotaRecord`: outcome, `namesQuota`, `observed`),
with `recordOf` for the host after the card and `legacyRecordOf` for the
host before it. The theorems are audited in `AuditProcessQuotaPlan.lean`:

* `a_failed_join_descriptor_refuses_before_spawning`: the record is the
  pre-spawn refusal, naming nothing and observing nothing.
* `an_observation_implies_the_join`.
* `a_spawned_record_naming_a_quota_implies_the_join`.
* `a_failed_join_never_spawns`: also leaves no observation.
* `the_repair_changes_only_failed_quota_executions`: with no quota, or with a
  successful duplicate and a spawned child, the record equals the legacy one.
* `the_legacy_host_observed_a_quota_its_child_never_joined`: a witness that
  the defect is real in the model.

The model is the step function. That the Rust follows it is pinned by the
tests and mutations of §4, not proved; the kernel's `pids` bound stays the
measured boundary of `remaining-p03-f`.

## 6. Arbiter identity

Only the spawn path of a process provider under a declared quota changed, and
no arbiter route spawns one. Every route was nevertheless measured with the
null property selected (`QCORE_NULL_PROPERTY`), BEFORE with the base binary
(`ee90db45…`, built at `b5da3bb4b`, whose code is `e0b92ece6`'s) and AFTER
with the tip binary (`01d72517…`, built at `6167e17a9`). Each corpus
`run.json` `binary_sha256` matches its binary.

| Route | BEFORE | AFTER | Wall time (s) before / after |
|---|---|---|---|
| corpus | 935 / 9 / 3,890, `bdf5f1c8…` (the brief's) | **byte-identical** | 530.5 / 519.0 |
| system | 93 / 4 / 351, `be06d139…` | **byte-identical** | 76.3 / 76.2 |
| slice | 52 / 1 / 347, 4 Pending, `ca40870b…` | **byte-identical** | 38.1 / 38.3 |
| null property, corpus | 1,113 / 1,435 / 2,355 / 18 Unsafe | **byte-identical** rows and summary | |
| null property, system | 84 / 125 / 245 | **byte-identical** | |
| null property, slice | 71 / 103 / 232 | **byte-identical** | |

Every report, sidecar, null stream and null summary is byte-identical. The
`audit.log`, `run.json` and invocation records are identical once the run
name, binary digest and times are removed. The typed-path parity streams
(689,660 / 142,456 / 51,389 rows) hold the same rows as a multiset, with Clang
node addresses normalised. See [artifacts/identity.txt](artifacts/identity.txt)
and [`scripts/identity.py`](scripts/identity.py). The system and slice digests
differ from the brief's with equal totals, because those reports embed the
measuring worktree's paths. No verdict moved, so no correction is named.

## 7. Gate

Every code step ran at `1e47c8d50` (the code of `6167e17a9` plus records)
with [`scripts/gate.sh`](scripts/gate.sh) `g1`, each step under the shared
heavy lock, so no arbiter shared the host with the suite. Exits are in
[gate/](gate/), logs in `private/qpdf/gate-g1/`, and the summary in
[gate.json](gate.json).

| Step | Exit | Count |
|---|---:|---|
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 487 units, 500 summaries; **7,943 passed, 0 failed**, 14 ignored; `private/qpdf/tests-g1/` |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **316** audits |
| `python3 -m unittest discover -s scripts/tests`, `build_docs_index.py --check` (08, 09) | gate/ | |
| `build_matrix.py . --gate gate.json`, the Python suite and the index again (`finish-gate.sh`, 09a–10c) | gate/ | |

The test step passed on its first run; nothing was rerun in isolation. The
suite is main's 7,939 plus the card's four rows. `execution_quota` ran 16 of 16
in the runner's `EXCLUSIVE` phase.

There is no provider gate, because `providers/` is unchanged. Its conformance
crate depends on `qcore-verifier-host` by path, so
[`scripts/providers-check.sh`](scripts/providers-check.sh) ran
`cargo check --manifest-path providers/Cargo.toml --workspace --all-targets
--locked --offline` once: exit 0. Clang: `clang --version` was recorded in this
worktree (`gate/clang-version.txt`).

## 8. Prediction against outcome

| Predicted | Outcome |
|---|---|
| audit finds sites 1–3 needing action, 4–6 none | held |
| test seam through a fourth `MechanismAvailability` field; `RLIMIT_NOFILE` cannot isolate the dup | held |
| `execution_quota` 13 → 16, unit 31 → 32, suite +4 | held for the crate; suite total in §7 |
| Lean +6-style theorems, audit count stays 316 | held (six theorems, 316 audits) |
| every route and the null property byte-identical | held |
| ledger 763 → 762 | **763**: the entry retires, and the audit registered one residual the prediction did not foresee (below) |
| constraint 26 amended in place, `wave-48-part7.md` opened | held |

## 9. Follow-ups

* **Retired:** `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`. Its DISCHARGE holds: a
  dup failure refuses before spawning (`RefusedByProfile`, `quota_observed`
  None), and a test forces it, with a mutation showing the test catches the
  base behaviour.
* **Registered:** `HOST-PRE-SPAWN-PROTOCOL-RECORD-NAMES-LIMITS`. `run_process`
  returns `Protocol` before `spawn` in three places:
  - a non-executable entrypoint (unreachable);
  - an unmappable rlimit kind (unreachable);
  - "provider timeout exceeds clock range" (reachable with `timeout_ms` near
    `u64::MAX`).

  Each is recorded as outcome `Protocol` with the planned `enforced` limits,
  the shape of an execution that ran. This is evidence shape, not fail-open:
  no child exists, the call is `Unknown` (`QP006`), and since this card no
  quota observation is recorded for it.
