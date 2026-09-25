# W44-AUDIT-PARALLEL-UNITS: the corpus audit, unit-parallel and byte-identical

Card W44-AUDIT-PARALLEL-UNITS, 2026-09-25, on base `1ab56306` ("Record the
branch-condition-calls integration and archive the combined baseline"). The
base profile and the prediction were committed first
([PREDICTION.md](PREDICTION.md), `c1947d80`), before any parallel run was
timed; the implementation is `56ed4995`, with formatting, clippy allowances, a
configured-route determinism test and documentation in `d7eb514a`. Later
commits change the report, the ledger and evidence only.

## Headline

* **What changed.** Both routes of
  `cycle19_fluent_bit_libraries_audit` — the pinned arbiter
  (`--pinned-evidence`, `compatibility::run`) and the configured route
  (`compatibility::run_current` through `qcore_audit::current`) — analyse
  translation units on `--jobs N` / `QCORE_AUDIT_JOBS=N` threads (default: the
  host's available parallelism, capped at 16). Each unit records into its own
  report fragment, tally, result lists and console buffer; one consumer folds
  them **in the sequential unit order**. `run_audit.py` gains `--jobs N` and
  records `QCORE_AUDIT_JOBS` in `run.json`'s environment block.
* **Identity.** At jobs 1, 4, 8 and 16 (16 four times) every archived artifact
  of the pinned arbiter is byte-identical to the base binary's own run in this
  worktree and passes `baseline_identity.py` against the W44-BRANCH-CONDITION-CALLS
  integration baseline, with only the known checkout-path digests differing;
  `audit.log` is `cmp`-identical between every pair of branch runs written
  through the shared output path (all but the first 16-job run, whose log
  differs only in the line naming its own directory). The configured route is byte-identical at jobs
  1, 4, 8 and 16 to the base binary's sequential run in every artifact and in
  `audit.log`, except the three wall-clock figures of its `preparation:` line,
  which differ between any two runs (§4.3).
* **Timing (pinned arbiter).** 424–590 s sequential → **129 s at 4 jobs, 96 s
  at 8, 77–87 s at 16** (4.9–5.5× the branch's own sequential run on the same
  loaded host, 6.8–7.7× the base run). Peak RSS 0.17 → 0.76 GiB.
* **Critical path.** `librdkafka/rdkafka_admin.c` (46.4 s sequential) takes
  83.9 s of the 86.0 s unit phase at 16 jobs: dispatched first, finished last.
  Registered as `W44-AUDIT-CRITICAL-UNIT` (§6).
* **Prediction.** 4 jobs hit (129 s against 135–150 s, slightly better); 8 and
  16 jobs **missed** on the slow side (96 s against 75–90 s; 77–87 s against
  60–75 s), because every unit runs 2.35× slower at 16 jobs than alone, not
  the 1.3–1.5× assumed; memory came in **below** the predicted range at every
  job count (§3).
* **Fail-closed.** No verdict, status, diagnostic or count depends on
  scheduling; the first error in unit order is the one reported, after exactly
  the output a sequential run would have printed. Any active census sink forces
  one job; process-provider verifications stay one at a time. No analyzer rule
  changed, so no Lean change was made.

## 1. Design

### 1.1 Ordered execution

`qcore_audit::parallel::run_ordered(tasks, jobs, dispatch, work, consume)`
computes `work` on up to `jobs` scoped threads and hands each result to
`consume` on the calling thread **in task order**, waiting for a slot when it
is not yet done. `consume` stopping with an error stops the run: no new task is
started, running tasks are finished and discarded, and the error returned is
the first in task order. A panic in `work` is re-raised when the consumer
reaches that task. `jobs == 1` starts no thread and computes each task just
before consuming it — the sequential loop, lazily, including its error and
panic points. `dispatch` is a start order only (a permutation, checked); the
audit starts the largest source files first (`heaviest_first`, weights =
source bytes; unit time ranks with source size at Spearman 0.88 on the base
profile, and the largest file is the slowest unit).

### 1.2 Pinned route (`compatibility::run`)

Everything before the library loop (inputs, three summary profiles, the
contracts sidecar) is unchanged and sequential. The loop becomes an ordered
list of steps in exactly the order the nested loops performed them:

| Step | Where it runs | What it is |
|---|---|---|
| `Prelude(library)` | worker | what `audit_library` did before its unit loop: frame-helper import; ChunkIO's memory backend, filesystem dependencies and close composition. Publishes a `LibraryContext` through a `OnceLock` (or `None` on failure or panic, so no unit waits forever). |
| `Unit(library, entry)` | worker | the old loop body, recording into its own `AuditReport` fragment, `AuditCounts`, native/teardown result lists, ChunkIO captures and console buffer. It waits on its library's context. |
| `Finish(library)` | consumer | the ChunkIO postlude over the captured unit evidence, then the library's summary line. |
| `MonkeyGate` | consumer | the Monkey/Libevent contract check, the Monkey source-copy check and the callback source audit, which writes `monkey-source.json` — at the same point as before, so an earlier failure still publishes nothing. |
| `PlanningFailed(error)` | consumer | a library selection failure, at its sequential position; planning stops there, as the loop did. |

Library preludes are dispatched before every unit, so a unit's worker never
waits long on its context; units are then dispatched largest first.

### 1.3 Configured route (`compatibility::run_current`)

`CurrentAudit::audit_entry` is split, unchanged in behaviour, into
`evaluate_entry(&self, …) -> EntryOutcome` (console lines plus either the
unit's source path, report fragment and tally, or the error) and
`absorb_entry(&mut self, …)`. `audit_entry` remains as their sequential
composition. `run_current` makes one task per selected configuration, in the
order the two loops visited them, prints each group's header just before its
first configuration's lines, and absorbs the products in order before
`finish`.

### 1.4 Folding

`AuditReport::absorb` appends a fragment's rows and sums its per-category
diagnostic counts; a fragment of another revision, or one carrying
source-contract evidence, is refused. Every attachment (`record_enumeration`,
`record_configuration`) targets the row its own unit just recorded, which lives
in the same fragment. `AuditCounts::absorb` sums every field (exhaustive
destructuring, so a new field must decide how it folds); the three preparation
timings keep the original saturating addition. ChunkIO captures keep the
loop's rule that a later unit's evidence replaces an earlier one's, applied in
unit order. `project_unit_into` / `project_prepared_unit_into` write the lines
their printing counterparts printed; the printing functions remain, as
wrappers, for every other caller.

## 2. Shared state audited

| State | Thread safety | Order dependence | Handling |
|---|---|---|---|
| Census sinks: `indirect_census` (`SINK`, `RECORDS`, `RETURNS`, `STATEMENTS`, `DERIVED`, counter `UNIT_SEQ`), `traversal_census`, `indirect_chain_census`, `condition_call_census`, `legacy_callee_scope::census`, `provenance_rounds` (`SINK`, counter `SEQ`) | `Mutex` / atomics | records stream in analysis order; `UNIT_SEQ` and `SEQ` number them | New `qcore_c_gateway::active_census_sinks()` lists the compiled-in sinks whose variable is set; `qcore_audit::parallel::effective_jobs` forces **one job** when any is. Nothing is printed about it, so a census run's `audit.log` does not depend on the requested count. With the variables unset every record path returns before touching a sink; the unconditional `UNIT_SEQ` increment is atomic and read only by census records. (`legacy_leak_witness.rs`, named in the brief, does not exist in this tree.) |
| `OnceLock`s | `OnceLock` | none | Only Clang-availability test support, the verifier host's kernel and quota capability probes (idempotent) and two constant `LazyLock` strings in `qcore-verification`. |
| Environment | read-only | none | `QCORE_CLANG` (read per Clang command), `QCORE_PROFILES`, census and probe-cap variables; nothing in the process writes the environment. |
| Caches | — | — | None in the gateway, contract, audit or compiler analysis path (no `static` other than the above, no `thread_local!`). |
| Process provider (`VerifierRegistry` → `ProcessProvider`) | `Mutex` `last_execution` | last-writer record (read only by host tests); 5 s wall-clock limit per execution; per-execution cgroup named by pid and nanoseconds under a quota profile | Verification is serialized by a mutex in the adapter (three executions per run), so executions never overlap as before. The wall-clock limit is the one scheduling-sensitive input left: under heavy load an execution could time out and the run would **fail** (fail-closed, an error, never a verdict); it did not happen in any of the runs here. |
| stdout / stderr | — | the log interleaves both | The only library writers are `projection.rs` and `current.rs` (now buffered); everything else in the analysis path prints nothing. A worker panic's hook message would reach stderr at the time of the panic, before the earlier units' lines; the panic itself is re-raised at its sequential position. |
| Temporary files | — | — | None in the analysis path (Clang runs over pipes). |
| Cross-unit data | — | — | Precomputed and ordered: the ChunkIO prelude (four ChunkIO units and the chunk-source audits consume it), the ChunkIO postlude (consumes `cio_chunk.c`, `cio_stream.c` and `chunkio.c` evidence), the Monkey gate. No unit reads another unit's result inside the loop. |

The compiler checks the rest: every value a worker shares (`ResourceContracts`,
`SourceContractConfiguration` including the verifier registry, the library
plans and contexts, `CurrentAudit`) must be `Sync` for the closures to compile,
and they are, without any change.

## 3. Profile, prediction and timing

### 3.1 Base profile

`scripts/profile_run.py` with the temporary `scripts/profile-instrumentation.patch`
([summary](profile/base-pinned-summary.json)); the numbers are in
[PREDICTION.md](PREDICTION.md). In one sentence: 185 units are 99.3 % of the
run; 87 % of unit time is in-process analysis and 13 % Clang; everything
outside the loop is 3.5 s (ChunkIO prelude 1.9 s, Monkey callback source
1.3 s); the critical unit is `rdkafka_admin.c` at 46.4 s; peak RSS 173 MiB.

### 3.2 Pinned arbiter timing

`/usr/bin/time -v python3 run_audit.py … --jobs N` on the branch binary
(`0140f50d…`), the base row with the base binary (`e933caba…`). The host is
shared: other agents kept the one-minute load average between 5 and 22, and a
row marked * was taken straight after the previous run of its series, whose
load the figure includes. Sequential figures vary by 40 % between runs for
this reason (user time 402 s against 554 s for the same work).
[Rows](timing/pinned.json).

| Run | Jobs | Wall s | User s | System s | CPU | Peak RSS KiB | Load before |
|---|---:|---:|---:|---:|---:|---:|---:|
| base binary | 1 | 590.5 | 554.5 | 24.5 | 98 % | 173,152 | — |
| branch | 1 | 424.2 | 402.2 | 22.3 | 100 % | 174,800 | 11.4 |
| branch | 4 | 128.9 | 485.9 | 25.6 | 396 % | 362,732 | 16.2* |
| branch | 8 | 96.2 | 719.3 | 35.9 | 785 % | 496,968 | 21.8* |
| branch | 16 | 87.0 | 876.2 | 43.7 | 1,057 % | 763,576 | 7.2 |
| branch | 16 | 86.9 | 868.6 | 44.2 | 1,050 % | 768,016 | 4.7* |
| branch | 16 | 77.0 | 870.2 | 44.0 | 1,187 % | 758,152 | 17.7* |
| branch | 16 | 76.7 | 869.2 | 44.3 | 1,190 % | 760,988 | 19.9* |

### 3.3 Configured route timing

The user's command (`cargo run --quiet -p qcore-compiler --example
cycle19_fluent_bit_libraries_audit`, defaulting to `../fluent-bit`) is not
runnable as such here: `../fluent-bit/build/compile_commands.json` and the
current Monkey inputs were compiled with `/usr/bin/cc`, which the route
refuses before auditing ("audit requires the configured Clang executable"),
exactly as on the base. The route was therefore run with `--compile-database`
over the prepared current-revision Clang build (`build/cycle19/current-revision`,
source `d1ab8abf`: 185 source files, 186 configurations, 558 Clang
invocations, the same counts as the user's data point) and the pinned Monkey
Clang database (29 Monkey configurations, whose source copy the route checks
against the checkout as usual). Same measurement as §3.2 ([rows](timing/configured.json); the
confirmation run on the final binary took 165 s at load 27+ and is identity
evidence only):

| Run | Jobs | Wall s | User s | Peak RSS KiB |
|---|---:|---:|---:|---:|
| base binary | 1 | 466.3 | 439.4 | 179,928 |
| branch | 1 | 545.8 | 506.6 | 181,440 |
| branch | 4 | 163.0 | 600.3 | 375,204 |
| branch | 8 | 106.8 | 760.7 | 546,764 |
| branch | 16 | 79.1 | 832.0 | 850,420 |
| branch | 16 | 79.5 | 833.7 | 828,668 |

### 3.4 Where 16 jobs spend their time

A second temporary patch (`scripts/critical-path-instrumentation.patch`)
timed every step of one 16-job pinned run
([summary](profile/branch-j16-critical-path.json)). The unit phase took 86.0 s;
`rdkafka_admin.c` ran from 0.0 to 83.9 s, the last unit to finish. The sum of
unit wall times was 1,218 s against 518.6 s sequentially: each unit ran on
average 2.35× slower with 16 neighbours (SMT on 8 cores, four of them the
compact variant, and a laptop power limit), which is also why user time more
than doubles. At 16 jobs the run is therefore bounded both by that one unit and
by throughput; 16 jobs gain only 10–20 % over 8.

### 3.5 Prediction against measurement

| Jobs | Predicted wall | Measured | | Predicted RSS | Measured (audit process) | |
|---|---|---|---|---|---|---|
| 4 | 135–150 s | 128.9 s | hit, slightly faster | 0.4–0.7 GiB | 0.35 GiB | below |
| 8 | 75–90 s | 96.2 s | **missed** (slower) | 0.6–1.2 GiB | 0.47 GiB | below |
| 16 | 60–75 s | 76.7–87.0 s | **missed** (slower) | 1–2.5 GiB | 0.73 GiB (1.39 GiB with every Clang child, strace-sampled) | below |

The miss is the per-unit slowdown: the prediction assumed the critical unit
would run 1.3–1.5× slower beside 15 others and it ran 1.8× slower (all units:
2.35×). User time was predicted to rise 20–50 % at 16 jobs and rose 57 %
against the base run, 116 % against the branch's sequential run. The default
cap of 16 is kept: memory is far from a constraint on this 60 GiB host, and 16
is the host's own thread count.

## 4. Identity

### 4.1 Pinned arbiter

`baseline_identity.py RUN docs/reports/w44-branch-condition-calls/integration/artifacts
--root-alias <this worktree>=<hungry-thompson-1740da>` exits 0 for base, j1,
j4, j8 and all four j16 runs ([results](identity/)). In every run the
diagnostics, chunk-contracts and monkey-source artifacts are raw-identical to
the baseline, the contracts sidecar is identical after the root alias, and the
source-effects sidecar differs only in the checkout-path-bound digests
(`resource_contracts:<digest>` keys and the leaves under them, `build_digest`,
`coverage[0]`, `task_digest`, `obligation_id`, …) — exactly as the base
binary's own run in this worktree does.

Run against run (`scripts/compare_runs.py`): all five artifacts of j1, j4, j8
and the four j16 runs are byte-identical to the base binary's run and to each
other. A confirmation pair on the final binary (`441a3f0f…`, built from
`d7eb514a` after formatting; the runs above used `0140f50d…` from `56ed4995`,
which differs only in layout) repeats every result at jobs 16 and 1; it ran
while other work held the load average at 27–33, so its times (155 s and
1,029 s) are identity evidence only. `audit.log` names its output directory once; every branch run was
written through the same directory and renamed afterwards (all but the first
16-job run, `j16a`, taken before that harness change), so
`cmp j1/audit.log jN/audit.log` exits 0, raw, for N = 4, 8, the three later
16-job runs and both confirmation runs. Against the base run and `j16a`
(other directories) the only differing line is that path line.
Repeatability: the four 16-job runs are identical to one another in every
artifact, and in `audit.log` after that one path line.

### 4.2 Census

A census build (`--features qcore-c-gateway/indirect-census`,
`run_audit.py … --census`, which sets `QCORE_RETURN_CENSUS`) at `--jobs 1`
and `--jobs 16`: the five artifacts, `audit.log` (`cmp`, raw) and
`return-census.jsonl` (2,261 records) are byte-identical between the two, both
pass `baseline_identity.py`, and the census build's five artifacts and
`audit.log` are byte-identical to the non-census branch's `--jobs 1` run. The
`--jobs 16` census run took 661 s at 100 % CPU against 625 s at `--jobs 1`:
the requested count was overridden to one job, silently, as designed
([rows](timing/pinned.json)). Census identity is therefore by construction
rather than by buffering; per-unit census buffering was not built, because two
streams number their records from process-global counters that would have to
be renumbered in unit order.

### 4.3 Configured route

Base binary sequential against branch jobs 1, 4, 8, 16 and a second 16:
`resource-diagnostics.json`, its `.contracts.json`, the three Unknown
sidecars and `monkey-source.json` are byte-identical in every run, and
`audit.log` is identical line for line except line 1,075:

```text
preparation: 186 unit(s), 558 Clang invocation(s); -E -P 10617609 us / syntax 15045858 us / line markers 9850428 us
```

Its unit and invocation counts are identical in every run; its three
microsecond figures are summed per-unit wall-clock measurements and differ
between the base binary's run and the branch's own sequential run (10.6 s
against 10.9 s of `-E -P`) as much as they do at 16 jobs, where contention
triples them (29.9 s). They were never reproducible and are compared masked
(`compare_runs.py --mask-preparation`), which is disclosed beside every
comparison; nothing else is masked. The line's bytes and meaning are
unchanged. The configured route had no reproducible identity baseline before
this card; the extended regression test
(`compiler/tests/cycle19_current_command.rs`) now also runs the route on four
threads and asserts the report and assumptions bytes equal the sequential
run's.

### 4.4 Tests

* `qcore_audit::parallel` — consumption in task order whatever the completion
  and dispatch order (later tasks finish first; jobs 1–64); the first error in
  task order wins, and nothing after it is consumed; no task starts after the
  consumer stops; a worker panic is raised at its position; a repeated
  dispatch index is refused; job-count resolution and malformed counts.
* `qcore_audit::report` — twelve units recorded into fragments on 1, 4 and 12
  threads, completed in reverse, absorbed in unit order, write the sequential
  report byte for byte; a fragment of another revision, or one carrying
  source-contract evidence, is refused.
* `cycle19_current_command` — the configured route at jobs 4 equals jobs 1.

## 5. Gates

Measured on `d7eb514a` in this worktree with `CARGO_INCREMENTAL=0`,
`CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=2`, every exit captured to a file,
never piped ([gate.json](gate.json); logs in the worktree-private
`measure/gate` and `measure/provider-gate`). Other agents kept the host's load
average between 14 and 50 during it, and no arbiter ran at the same time.

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init --recursive` | 0 | |
| clang | `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) |
| test | `cargo test --workspace --no-fail-fast` | 0 | 5,464 passed / 0 failed / 11 ignored, 289 summaries; the load-sensitive `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` passed first time |
| clean | `cargo clean --profile dev` (after the test step) | 0 | 6.8 GiB |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` and `cargo clippy --workspace --all-targets -- -D warnings` | 0, 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| followup | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| lean | `python3 scripts/check_lean_audits.py --jobs 4` | 0 | 180 audits |
| census | `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings`; `cargo test -p qcore-c-gateway --features indirect-census --no-fail-fast` | 0, 0 | 1,330 passed / 0 failed / 7 ignored |
| offline | `cargo build --offline --workspace` | 0 | no dependency added; `Cargo.lock` unchanged |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w44-audit-parallel-units/gate.json` | 0 | named follow-ups 368 |
| python | `python3 -m unittest discover -s scripts/tests` | 0 | 330 before regeneration (exit 0) and 330 after it |
| docs index | `python3 scripts/build_docs_index.py --check` | 0 | after regenerating `docs/reports/INDEX.md` |
| provider test | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | 124 passed / 0 failed / 0 ignored, 39 summaries |
| provider fmt / clippy | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check`; `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0, 0 | |
| contracts | `python3 providers/contracts.py check` | 0 | |

No provider source changed, so the package build and verification of
constraint 18 were not rerun.

## 6. Ledger

One measured residual is registered (`followups.json`,
`scripts/update_ledger.py`): **`W44-AUDIT-CRITICAL-UNIT`** — at 16 jobs one
unit, `librdkafka/rdkafka_admin.c`, spans 83.9 of the 86.0 s unit phase; unit
parallelism cannot shorten it. Discharge: parallelism inside a unit (or a
cheaper analysis of that unit) with byte-identical outputs, remeasured with
this card's scripts, or a recorded decision that the bound is acceptable.
The per-unit slowdown under SMT and the provider's wall-clock limit under load
are recorded above as properties of the host and of an existing limit, not
registered: the first is hardware, and the second was never observed.

## 7. Files

* Code: `crates/qcore-audit/src/{parallel.rs (new), lib.rs, report.rs,
  projection.rs, current.rs}`, `crates/qcore-c-gateway/src/lib.rs`
  (`active_census_sinks`) and a crate-visibility widening of two census
  modules (`legacy_c.rs`, `legacy_joined_provenance.rs`),
  `compiler/examples/support/cycle19_compatibility.rs`,
  `compiler/examples/cycle19_fluent_bit_libraries_audit.rs`,
  `compiler/tests/cycle19_current_command.rs`.
* Scripts: `docs/reports/w43-return-expression/scripts/run_audit.py`
  (`--jobs`); this directory's `scripts/` — `profile_run.py`,
  `profile_summary.py`, `critical_path_summary.py`, `compare_runs.py`,
  `timing_table.py`, `update_ledger.py`, the two temporary instrumentation
  patches, and `harness/`, the shell wrappers that produced every run here
  (they carry this worktree's absolute paths).
* Evidence: `profile/`, `timing/`, `identity/` (every `baseline_identity.py`
  and `compare_runs.py` result, and the leaf diff of the source-effects
  sidecar against the baseline), `runs/` (every arbiter `run.json`).
* Docs: `docs/agents/validation.md` (corpus arbiter section),
  `docs/CYCLE19_BUILD_INPUTS.md`, the wave-44 increment record.
