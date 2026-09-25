# W44-TEST-SUITE-WALL-TIME: the standing gate's Rust suite, from 22 minutes to about 4½

Card W44-TEST-SUITE-WALL-TIME, wave 44, 2026-09-25. Base: local `main`
`94502987`. The card does not change what is tested. Every test of the base
runs with the same outcome. The only additions are two unit tests for the one
production change (§2.2).

## 1. Outcome

All runs start from `cargo clean --profile dev` and use `CARGO_INCREMENTAL=0`.
They ran on the 16-CPU shared host while other gates were running, so the load
column matters. It shows the 1-minute load average at start and at end, and my
own runs raise it. Seconds are wall time from `/usr/bin/time -v`. Build is the
time until the first test binary runs. Capture is the runner's launch-recording
pass (§3). Run is the time from the first test binary to the end.

| Run | Configuration | Wall | Build | Capture | Run | Peak RSS | `target/` | Load start → end |
|---|---|---:|---:|---:|---:|---:|---:|---|
| `base-1` | base: plain `cargo test --workspace --no-fail-fast` | **22:13** (1332.9 s) | 124.6 | — | 1208.2 | 2.16 GiB | 62.4 GB | 8.0 → 12.7 |
| `profile-plain-1` | plain, `debug = "line-tables-only"` | 23:21 (1401.4 s) | 55.9 | — | 1345.4 | 1.44 GiB | 21.0 GB | 23.5 → 17.0 |
| `runner-j8-a` | runner `-j 8`, base profile, no weights yet | 12:29 (748.9 s) | 193.4 | 22.4 | 532.8 | 2.11 GiB | 62.4 GB | 13.8 → 30.8 (5-min 40–44) |
| `final-1` | runner + line tables (before the Clang cache) | 8:22 (502.2 s) | 69.3 | 7.2 | 425.4 | 1.50 GiB | 21.0 GB | 17.0 → 36.5 |
| `profile-cache-plain-1` | plain, line tables + Clang cache | 9:13 (552.9 s) | 46.8 | — | 506.1 | 1.48 GiB | 21.0 GB | 26.2 → 8.8 |
| **`final-cache-1`** | **final**: runner + line tables + Clang cache | **4:28** (267.9 s) | 51.5 | 4.9 | 211.3 | 1.34 GiB | 21.0 GB | 8.3 → 33.3 |
| **`final-cache-2`** | final | **5:35** (334.7 s) | 117.1 | 8.1 | 209.3 | 1.50 GiB | 21.0 GB | 32.8 → 38.5 |
| **`final-cache-3`** | final | **3:59** (238.6 s) | 61.5 | 4.7 | 172.3 | 1.49 GiB | 21.0 GB | 38.5 → 26.2 |
| `jobs-4` | final with `-j 4` | 4:31 (270.9 s) | 47.1 | 5.0 | 218.6 | 1.38 GiB | 21.0 GB | 6.8 → 19.9 |
| `jobs-16` | final with `-j 16` | 3:56 (235.7 s) | 47.0 | 6.3 | 182.2 | 1.41 GiB | 21.0 GB | 19.2 → 26.3 |

`evidence/runs.json` holds every figure, together with free disk before each
run (at least 136G every time; see §8.2) and the runner's totals.

- **Final configuration:** 3:59–5:35 against the base's 22:13, a 4.0–5.6×
  cut in wall time.
- **Build:** 47–117 s, against 125 s at a lower load. The spread comes from
  load; the same build took 1:03–2:29 under different loads (§4).
- **`target/`:** 62.4 GB → 21.0 GB.
- **Peak RSS:** 2.16 → about 1.5 GiB.
- **Test outcomes:** every run above, the base included, is green. Each has
  the same executed test set and outcomes as the base (§6).

Each of the three changes pays off in a different phase:

| Change | What it cuts | Evidence |
|---|---|---|
| `[profile.dev] debug = "line-tables-only"` | build time (about halved at equal load), `target/` 62→21 GB | §4 |
| Clang is resolved and probed once per environment | serial test time 1208 → 506 s (plain `cargo test`) | `profile-plain-1` vs `profile-cache-plain-1`, §5 |
| `scripts/run_test_binaries.py` | the serial sum becomes a parallel phase bounded by the longest binary | `final-cache-*` run 172–211 s vs 506 s serial |

`--jobs` barely matters. At 4, 8 and 16 the run phase is 219, 172–211 and
182 s. The phase ends when the heaviest binary ends (§5.3). The default stays
at half the CPUs (8), which is kinder to a shared host.

## 2. What changed

### 2.1 Build profile: `[profile.dev] debug = "line-tables-only"` (root `Cargo.toml`)

The root manifest had no profile section. `cargo test` therefore built every
unit with full debuginfo, which it then linked into each of the 281 test
executables. `[profile.test]` inherits from `dev`, and it governs every unit
`cargo test` builds, dependencies included. This was checked with `-v`; see
the note at the end of §10.

The setting goes in `[profile.dev]`, not only in `[profile.test]`. Otherwise
the gate's two `cargo run --example reevaluation_*` steps, which use `dev`,
would stop sharing artifacts with the test build and rebuild everything.

Why this setting is safe:

- `opt-level`, `debug-assertions` and `overflow-checks` keep their dev
  defaults. No test observes a different program: `debug_assert!`,
  overflow panics and `cfg(debug_assertions)` behave as before.
- Only DWARF variable and type information is dropped. Panic locations never
  depended on it, and backtraces keep `file:line`.
- No test reads debuginfo. The repository has no `backtrace` or `debuginfo`
  consumer; its two `-g` references are C frontend flags.

Developers who want full debugger information can set
`CARGO_PROFILE_DEV_DEBUG=true`.

Two options were measured and rejected:

- **`debug = false`** built at the same speed as line tables (62 s against
  66–69 s at similar load) and gave a 5.6 GB `target/`. It was rejected
  because backtraces would lose `file:line`.
- **`opt-level = 1`** doubled the build (199.5 s). It also barely moved the
  heaviest binary: 116.5 s at opt-level 1 against 120.0 s at opt-level 0,
  both at load about 5 (`evidence/opt-level-probe.txt`). The tests are not slow
  because analysis runs in debug mode.

### 2.2 The one production change: Clang is resolved and probed once per environment (`crates/qcore-c-gateway/src/clang.rs`, commit `585b4260`)

**Where the time went.** `clang::command()` ran discovery on every call.
Discovery launches `clang --driver-mode=gcc --version` and checks the banner,
and every gateway Clang run called it first. In the heaviest binary,
`unsupported_close_route_parity`, that probe accounted for **5,411 of 9,603 Clang
launches**. A launch costs about 14 ms of process start even for `--version`
(`evidence/clang-launches.json`).

**The change.** `resolved_once` keeps the last successful resolution in a
process-wide `Mutex`. The cache key is the `QCORE_CLANG` value and `PATH`.

- A changed override or search path is resolved afresh.
- A failed resolution is never cached, so an unavailable Clang is examined
  again on every call.
- An explicit `QCORE_CLANG` stays authoritative: it never falls back.

**Why no verdict can change.**

- The executable was already trusted to stay stable while in use. `ClangIdentity`'s
  documentation says so, and constraint 28 lists faithful Clang extraction as
  a premise.
- A later run still fails closed. It fails if the executable cannot be
  launched, if it exits nonzero, or if its output does not parse.
- The existing `resolve` tests are unchanged.
- The new tests `a_successful_resolution_is_reused_only_under_the_same_environment`
  and `a_failed_resolution_is_never_cached_or_masked` pin these properties: one
  probe per distinct environment, none for a repeat, and a failure that is
  neither cached nor masked by an earlier success.

No Lean model covers Clang discovery, which sits on the transport side of
constraint 28. This change therefore has no Lean counterpart to update.

**Measured effect.**

- Clang launches of the heaviest binary: 9,603 → **4,803**.
- `--version` launches: 5,411 → 611. Of the 611, 608 come from
  `ClangIdentity::capture`, which is a follow-up (§9).
- The heaviest test, run alone and alternating builds: 160.4 s and 124.1 s
  before, 82.8 s and 74.2 s after (`evidence/ab-probe-cache.txt`).
- The whole serial suite: run phase 1345 → 506 s. Every heavy binary is 2–4×
  faster (§5.1).

This commit is self-contained. If the integrator prefers to land the runner
and profile without it, dropping it leaves the rest intact. The run phase then
reverts to about `final-1`'s 425 s.

### 2.3 The runner: `scripts/run_test_binaries.py` (std-only, with `scripts/tests/test_run_test_binaries.py`)

See §3. `scripts/test_binary_weights.json` holds each unit's serial seconds
from `base-1`. It is a scheduling hint only, so the runner can start the
longest binaries first.

### 2.4 Gate documentation

`docs/agents/validation.md` is the authority for the gate:

- The standing block now runs `python3 scripts/run_test_binaries.py`, and its
  comment names `cargo test --workspace --no-fail-fast` as the equivalent
  reference command.
- A new paragraph sets the runner's contract and makes the reference command
  the arbiter when the two disagree.
- `scripts/task_context.py` derives its required-command list from that block,
  so the list follows it. Its test still finds the reference command there.

No other copy of the gate was generated. `docs/development/build-and-test.md`'s
setup example keeps the plain command, which remains valid.

## 3. The runner

`cargo test` runs its test executables one after another. The runner produces
the same result in five steps:

1. **Build.** It runs `cargo test --workspace --no-run
   --message-format=json-render-diagnostics`, which builds exactly what the
   plain command builds, examples included. It reads every `profile.test`
   artifact with an executable.
2. **Capture.** It runs `cargo test --workspace --no-fail-fast --tests` with
   `--config target.<host>.runner=[…]`. That runner is a five-line inline
   Python (`-I -S`, about 28 ms per launch). For each launch it records the
   argv, the working directory and the **complete environment cargo built**,
   then exits 0 without running the test. The capture pass rebuilds nothing:
   runner configuration is not part of a unit's fingerprint. It took 4.7–8.1 s
   in the final runs.
3. **Plan.** It checks the plan and fails closed before running anything if:
   - a binary was built but not launched, or launched but not built;
   - a binary was launched twice;
   - two binaries share a unit id;
   - an `EXCLUSIVE` entry names a package of this workspace but matches no
     binary, which is how a renamed load-sensitive binary would otherwise slip
     into the parallel phase.
4. **Parallel phase.** Units run under `--jobs`, longest recorded first. One
   extra unit, `cargo test --workspace --doc --no-fail-fast`, runs the
   doc-tests. Each unit replays its captured argv, cwd and environment
   **verbatim**, with nothing merged in. stdin is `/dev/null`. stdout and stderr
   go to per-unit logs.
5. **Exclusive phase.** It starts only after the pool has shut down, and its
   units run one at a time (§7).

**Verdict and output.** A unit passes only if it exits 0 and prints at least
one libtest summary, and every summary is `ok` with zero failures. Every other
outcome fails the run with exit 101: a signal, a crash after a clean summary,
a spawn error, a missing executable, an exit of 0 without a summary, or a
clean summary with a nonzero exit. The runner then prints:

- a totals line, which sums the same summaries `cargo test` prints;
- the failing units, with their log paths;
- `summary.json`, which holds per-unit outcomes, durations, sorted
  `test … …` lines and every unit's start and end, so exclusivity can be
  audited after the fact.

**Scope.** The runner refuses any workspace but the root (exit 2) before
building anything. The provider workspace's conformance binaries launch
process providers throughout, so they keep plain `cargo test` (§7).

**Environment fidelity, verified independently.** `harness/verify_env.py`
compares two views of the same launch:

- what the kernel saw when plain `cargo test -p P --test T` launched the test,
  with no runner configured: `strace -f -v -e trace=execve,chdir` gives the
  execve argv and envp, and the last `chdir` of that pid gives the cwd;
- the capture record for the same target.

For `qcore-schemas::reason_codes` and `qcore-compiler::chunk_lifecycle_coverage`,
argv, cwd (the package root) and all **114 environment variables** are
identical (`evidence/verify-env.json`). That includes `CARGO_MANIFEST_DIR`,
`CARGO_MANIFEST_PATH`, the `CARGO_PKG_*` set, `CARGO` and `LD_LIBRARY_PATH`.

**Unit tests (16, in `scripts/tests/test_run_test_binaries.py`).** They cover:

- aggregation of several summaries per unit, including doc-tests;
- a failing binary; a binary crashing with SIGSEGV after a clean summary; an
  aborting one; a missing executable;
- exit 0 without a summary, and an `ok` summary with a nonzero exit;
- **exclusivity**, shown from the recorded intervals: no exclusive unit
  overlaps any other, the parallel phase really overlaps, and the job limit
  holds;
- longest-first ordering;
- verbatim replay of a captured cwd and environment, with the runner's own
  `HOME` not leaking in;
- the capture command recording without running, and failing when it cannot
  record;
- the built/launched cross-checks and stale `EXCLUSIVE` entries;
- the non-root-workspace refusal.

**A real fail-closed case.** The first full attempt (`attempt-1-relative-logdir`)
passed a relative `--log-dir`, which the capture runner could not write from
each package root. Every capture failed. The run reported 281 unlaunched
binaries and five stale `EXCLUSIVE` entries and exited 101. That attempt was
kept as evidence and not used for timing. The log directory is now resolved to
an absolute path, and a plan problem now stops the run before anything is
executed.

## 4. Build-only profile options

Each option was built with `cargo test --workspace --no-run` from an empty
`target/` and cleaned afterwards (`harness/build_options.sh`). There were two
rounds; the second ran in a different order to expose load effects.

| Option | Round 1 | Round 2 | Peak RSS | `target/` |
|---|---:|---:|---:|---:|
| base (full debuginfo) | 112.6 s (load 17.5) | 148.7 s (37.8) | 2.13 GiB | 62.4 GB |
| `debug = "line-tables-only"` (**chosen**) | 102.9 s (41.6) | 69.4 s (32.1) | 1.44 GiB | 21.0 GB |
| line tables + dependencies `debug = false` | — | 66.5 s (28.7) | 1.49 GiB | 20.8 GB |
| `debug = false` | 63.2 s (46.8) | 62.3 s (23.4) | 1.13 GiB | 5.6 GB |
| line tables + `opt-level = 1` | 199.5 s (35.0) | — | 1.46 GiB | 22.1 GB |

Dropping dependency debuginfo on top of line tables changes nothing, because
the workspace's own crates dominate the size. No faster linker is installed:
there is no `mold`, `lld` or `ld.lld`. Only the toolchain's bundled `rust-lld`
exists, and switching to it was not measured.

## 5. Where the heavy binaries' time goes

### 5.1 Per binary, before and after

This table lists the base's 14 longest binaries. The serial columns come from
plain `cargo test`; the runner columns are the same binaries inside the
parallel phase, where they share the machine. All figures are in seconds.

| Binary | Tests | base serial | line tables, serial | line tables + cache, serial | final runner 1 / 2 / 3 |
|---|---:|---:|---:|---:|---|
| `unsupported_close_route_parity` | 3 | 193.2 | 137.9 | 53.5 | 187.4 / 183.2 / 153.0 |
| `composition_mutations` | 7 | 124.4 | 75.0 | 34.6 | 102.4 / 104.5 / 85.3 |
| `file_close_composition_mutations` | 7 | 112.8 | 108.1 | 43.2 | 144.9 / 142.4 / 117.4 |
| `file_close_route_parity` | 3 | 87.2 | 133.4 | 38.7 | 128.7 / 129.9 / 108.7 |
| `unsupported_close_composition_mutations` | 10 | 75.3 | 68.1 | 30.6 | 71.5 / 80.5 / 64.6 |
| `filesystem_chunk_close_composition_mutations` | 8 | 70.3 | 76.8 | 32.5 | 80.9 / 84.6 / 70.2 |
| `filesystem_chunk_close_route_parity` | 3 | 67.8 | 99.1 | 32.0 | 97.5 / 100.6 / 81.7 |
| `cleanup_replay_route_parity` | 4 | 40.4 | 114.5 | 19.1 | 55.7 / 64.2 / 52.7 |
| `memory_close_composition_mutations` | 8 | 37.3 | 46.5 | 16.0 | 57.6 / 51.3 / 41.3 |
| `unsupported_close_composition_source` | 15 | 35.5 | 43.7 | 16.6 | 55.5 / 48.4 / 40.4 |
| `filesystem_chunk_close_composition_source` | 13 | 31.2 | 38.4 | 14.4 | 53.4 / 46.4 / 39.6 |
| `qcore_compiler` (lib unit tests) | 179 | 29.5 | 52.2 | 15.6 | 38.4 / 34.6 / 30.1 |
| `file_close_composition_source` | 13 | 27.3 | 26.9 | 11.1 | 37.4 / 33.8 / 29.0 |
| `cleanup_replay_mutations` | 9 | 21.9 | 21.2 | 10.3 | 30.9 / 27.9 / 24.6 |
| all 290 sections (sum) | 5,514 | 1208.2 | 1345.4 | 506.1 | 1525.8 / 1503.7 / 1244.3 |

`evidence/per-binary.json` lists every binary in every run.

The line-tables column ran at a higher load, and within that noise it shows
no runtime gain. The whole serial gain is the Clang cache's. The runner's sum
exceeds the serial sum because each heavy binary runs 2.5–3.5× slower while
sharing the CPUs. The phase is still 2.4–2.9× shorter than serial.

### 5.2 Why they are slow, and why no test was rewritten

These nine binaries are ChunkIO route-parity and composition-mutation suites.
By design, each forged row makes the core **re-establish** its composition from
the pinned C sources: preprocessing, syntax validation and one typed-AST run
per function, each a Clang launch. After the cache, the heaviest binary still
makes these launches:

| Launch | Count |
|---|---:|
| `-ast-dump=json` runs | 2,048 |
| `-fsyntax-only` runs | 768 |
| `-E -P` runs | 768 |
| `ClangIdentity::capture` pairs (`--version` + `-dumpmachine`) | 608 |

Two things were ruled out as causes:

- **Debug-mode analysis.** See §2.1: opt-level 1 barely changes the time.
- **Fixtures recomputed per test.** The reusable part is small. In
  `unsupported_close_route_parity`, for example, only the retained route's
  file-backend evidence over the unmutated `backend::SOURCE` repeats across
  rows. The moved route and every forged row's core evaluation must stay
  per-row, because that re-evaluation is the property under test.

A `LazyLock` around the one shareable value would have meant editing nine test
files, each needing its own equivalence argument, to trim part of one route.
The Clang cache removed the dominant repeated cost at its source, with no test
edits. So no test file was changed: no test was weakened, skipped, ignored or
had an assertion reduced. What remains is measured and registered in §9.

`evidence/per-test-base-parallel6.json` has the per-test times of the nine
binaries' 53 tests, all of which passed. Each ran alone, six processes at a time, at load up to
36, so it gives the ranking, not clean durations. The longest single tests are
`both_routes_refuse_the_same_body_mutations` in the filesystem-chunk-close and
unsupported-close route-parity files (12 mutations × 2 routes each), which
took 190–198 s under that load.

### 5.3 The critical path

In every final run, `unsupported_close_route_parity` starts first and is the
last heavy binary to finish: at 187.4, 183.2 and 153.0 s. The parallel phase
ends at 188.6, 185.8 and 153.8 s. The next-longest binary finishes 36–42 s
earlier. The exclusive phase then adds 18–23 s. So further cuts must shorten that binary's longest
test. More jobs will not help (§1). This is registered in §9.

## 6. Equivalence evidence

`harness/compare.py` compares each run with the base, binary by binary. A
binary is identified by its executable stem with the hash stripped, and all
doc-test sections form one unit. For each binary it compares:

- the sorted `test <name> ... <outcome>` lines;
- the summed summaries: passed, failed, ignored, measured, filtered out, and
  the number of summaries;
- the totals across all binaries.

Six stems are shared by two packages, for example `module_imports` and
`source_tasks`. Their lines are compared as the merged multiset on both sides.

The base has **276 keys**: 281 test executables plus doc-tests. It has
**5,514 test lines** and totals of **5,503 passed, 0 failed, 11 ignored,
0 measured, 0 filtered out over 292 summaries**.

| Run | Result |
|---|---|
| `profile-plain-1`, `runner-j8-a`, `final-1` | **IDENTICAL** |
| `profile-cache-plain-1`, `final-cache-1/2/3`, `jobs-4`, `jobs-16` | **IDENTICAL apart from 2 declared, passing additions**: the two `clang::tests` unit tests of §2.2 in `qcore_c_gateway` (503 instead of 501 passed there, 5,505 in total) |

`harness/equivalence.sh` declares the two additions with `--expect-added`.
Each declared line must be absent from the base, present and `ok` in the other
run, and nothing else may differ. `evidence/equivalence.txt` holds the output
for every run. `evidence/tests/<run>.json.gz` holds every run's sorted test
lines and summaries per binary, so the comparison can be redone without the
private logs.

## 7. Load-sensitive handling

`EXCLUSIVE` in the runner lists the binaries that run alone, after the parallel
phase. Membership is conservative. It includes constraint 16's load-sensitive
rows, plus every other root-workspace binary that launches a process provider
under a declared wall-clock timeout, because such a timeout is a wall-clock
budget. The candidates were found by grepping the tests for
`UnverifiedProvider` / `ProcessProvider` / `timeout_ms` / `elapsed()`.

| Binary | Reason | Serial seconds |
|---|---|---:|
| `qcore-compiler::test::cycle20_verifier_plugins` | `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` (constraint 16) | 1.4 |
| `qcore-verifier-host::test::standalone_host` | `closed_input_does_not_mask_…` one-second bound (constraint 16, sixth row) | 2.1 |
| `qcore-verifier-host::test::enforced_process_limits` | `memory_growth_…` and the `RLIMIT_NPROC` row (constraint 16) | 1.8 |
| `qcore-verifier-host::test::confined_process_execution` | confined-fixture write test (constraint 16) | 0.3 |
| `qcore-verifier-host::test::execution_quota` | `RLIMIT_NPROC` rows, process quotas | 6.0 |
| `qcore-verifier-host::test::package_identity` | process providers under a 5 s timeout | 0.0 |
| `qcore-compiler::test::w3_evidence_v2` | an out-of-process provider under a declared timeout (it also runs `cargo build` of its fixture) | 11.8 |
| `qcorec::test::plugin_packages` | process providers through the CLI | 0.0 |

Each runs exactly as under `cargo test`, with its own internal test threads,
and with nothing else of the runner's running. In all three final runs, and
in `final-1`, `jobs-4` and `jobs-16`, **all eight passed, and no exclusive
interval overlapped any other unit**. The check reads the intervals recorded
in `summary.json` (`harness/timeline.py`; `exclusive_overlaps` in
`evidence/runs.json`). Constraint 16's instruction is unchanged: a failing row
is verified in isolation, never weakened.

The runner cannot keep other processes off the host, such as other worktrees'
gates. That is the same condition plain `cargo test` runs under, and why the
load column is published. The seventh, unnamed row
(`LOAD-SENSITIVE-ROW-UNNAMED`) is still unidentified, so the table is known to
be incomplete, as constraint 16 says. The provider workspace's own
load-sensitive rows are not in the table. Among them are the `packages` rows
and `configured_audit::pinned_second_codebase_runs_configured_process_checks_without_a_rust_import_adapter`,
whose 5 s process quota failed closed under the concurrent corpus arbiter.
That workspace keeps plain `cargo test`, and the runner refuses to run it.

## 8. Validation

### 8.1 Gate

GATE-PLACEHOLDER

### 8.2 Measurement hygiene

- Every measured build started from `cargo clean --profile dev`, and at most
  one profile build was on disk at a time.
- Free disk before each timed run was at least 136G (`evidence/runs.json`).
- Mid-card the coordinator reported /home at 37G free. My `target/` had reached
  120G at that point. The cause was a hand-run capture probe, launched without
  `CARGO_INCREMENTAL=0`, which rebuilt the workspace incrementally on top of
  the base build. That probe was not a timing and is not used anywhere here.
  After a full `cargo clean`, free space was 144G, and every timing in §1 and
  §4 started with at least 136G free.
- Logs are in the worktree-private `measure/`.

## 9. Follow-ups (measured residuals)

Registered in `followups.json` and in the acceptance matrix's `named_followups`:

- **`W44-CLANG-IDENTITY-RECAPTURE`.** After §2.2, 1,216 of the heaviest
  binary's 4,803 Clang launches (25%) are `ClangIdentity::capture`'s
  `--version` and `-dumpmachine` pairs: 608 captures in one process.
  Discharge: capture the identity once per resolution under the same stability
  premise, with tests for a changed environment, and measure the launch count
  again.
- **`W44-TEST-CRITICAL-PATH-ROUTE-PARITY`.** In the final configuration,
  `unsupported_close_route_parity` alone sets the end of the parallel phase:
  153–187 s of a 172–211 s run phase, against 53.5 s serially. Its
  `both_routes_refuse_the_same_body_mutations` repeats the unmutated file
  backend's full Clang pipeline for each of 12 rows on the retained route.
  Discharge: a measured cut of that binary's longest test that keeps every
  row's own evaluation. A shared retained-backend fixture with an equivalence
  argument is one way. Report the new critical path.

## 10. Files

| Path | Role |
|---|---|
| `Cargo.toml` | `[profile.dev] debug = "line-tables-only"` |
| `crates/qcore-c-gateway/src/clang.rs` | resolution cache and its two unit tests |
| `scripts/run_test_binaries.py` | the runner |
| `scripts/test_binary_weights.json` | longest-first hint |
| `scripts/tests/test_run_test_binaries.py` | runner unit tests |
| `docs/agents/validation.md` | gate authority: runner step, reference command, contract |
| `harness/measure_cargo.sh`, `harness/sequence.sh`, `harness/ts.py` | timed runs from a clean dev profile |
| `harness/build_options.sh` | build-only profile options |
| `harness/profile_probe.sh`, `harness/per_test_times.py`, `harness/ab_test.py` | per-binary and per-test probes |
| `harness/clang_timing_wrapper.sh`, `harness/clang_share.sh` | Clang launch counting |
| `harness/verify_env.py` | strace-based check of the captured environment |
| `harness/cargo_log.py`, `harness/compare.py`, `harness/equivalence.sh`, `harness/timeline.py` | per-binary split, equivalence, exclusivity |
| `harness/make_weights.py`, `harness/collect.py` | weights and evidence collection |
| `scripts/run_gate.py`, `scripts/write_gate.py`, `scripts/update_ledger.py` | gate and ledger |
| `evidence/` | the collected figures described above |

Measured on the probe note in §2.1: `cargo test -p qcore-schemas --no-run -v
--config 'profile.test.debug="line-tables-only"'` passes
`-C debuginfo=line-tables-only` to `serde_json`, the `qcore_schemas` library
and its test harness alike. So the test profile governs every unit `cargo test`
builds.
