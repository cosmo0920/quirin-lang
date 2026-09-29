# W47-FB-DOUBLE-STOP-OBSERVATION: native joins of the engine thread

Card W47-FB-DOUBLE-STOP-OBSERVATION (wave 47, native observation only).
Base: local `main` at `f269c231` (this worktree's HEAD, no reset). No
analyzer, contract, provider, gateway-rule or Lean change; no arbiter run,
because nothing an arbiter reads changed.

**Every result here is a native observation at the pinned build under the
stated inducer; not a proof; no Quirin verdict moves; not an upstream defect
claim.** The candidates of `W47-ENGINE-THREAD-JOIN-CANDIDATES` stay
candidates.

## 1. What was asked

The candidates come from the rule 10q census
([W47-CONDVAR-JOIN-STOP-PATHS](../w47-condvar-join-stop-paths/README.md)) and
the lifecycle check ([W47-FLB-LIB-LIFECYCLE](../w47-flb-lib-lifecycle/README.md) §3:
L3-L5 refuted because every feasible `flb_stop` path joins
`ctx->config->worker` and nothing clears it):

* (i) after a `flb_start` whose engine failed, `flb_stop` joins the engine
  thread a second time (`do_start` already joined it at `src/flb_lib.c:984`/`:999`);
* (iii) after a `flb_start` that failed before the spawn, `flb_stop` joins
  `ctx->config->worker`, which was never written;
* (v) a second `flb_stop` joins `ctx->config->worker` again.

No `PREDICTION.md` was frozen: the card takes no arbiter measurement, and the
expected outcome of each scenario is the candidate text already committed in
the acceptance matrix before this card (two joins for (i) and (v), a join of
an unwritten identifier for (iii), one join for the control).

## 2. Build

[`scripts/build.sh`](scripts/build.sh) `TREE VARIANT` builds only
`libfluent-bit.so` under `private/w47-dstop/attempt-1/` (outside every user
checkout; nothing installed): Clang 21.1.8, CMake/Ninja, `Debug`,
`-O0 -g -fno-omit-frame-pointer`, `FLB_SHARED_LIB=On`, binary, examples and
tests off; the `tsan` variant adds `-DSANITIZE_THREAD=On`.

| Tree | Source copy | Library SHA-256 |
|---|---|---|
| `90978ac6` | `export.tar` of W47-TAIL-PAUSE-BINDINGS (SHA-256 `04a0fbee…` checked before extraction); 12,188 files, manifest `f031a05a…` | debug `1f90b4f5…`, tsan `45c3c616…` |
| `046accf6` | the `quirin-audit prepare` clone of the system slice, working tree copied without `.git`; 12,183 files, manifest `1de77600…` | debug `6290d41e…` |

The two trees are byte-identical on every file the scenarios run through
(`flb_lib.c`, `flb_engine.c`, `flb_fips.c`, `flb_input.c`, `flb_config.c`,
`mk_utils.c`, `flb_lib.h`, `flb_config.h`, `in_dummy`, `out_null`;
[compare.txt](artifacts/identity/compare.txt)). The `046accf6` copy was not
re-verified against the commit's tree object. Every configure, build and
harness exit is 0 ([build-exits.txt](artifacts/identity/build-exits.txt));
full hashes and toolchain in [evidence.json](evidence.json) and
[artifacts/identity/](artifacts/identity/). The library's banner prints
`commit=f269c23141`, which is this repository's HEAD: Fluent Bit's CMake
reads `FLB_GIT_HASH` with `git rev-parse` in an unversioned copy under this
repository's ignored `private/`. The value is only a version string.

## 3. Harness and observation

[`scripts/harness.c`](scripts/harness.c) uses only the public `flb_*` API
(`dummy` → `null`, Flush 1, Grace 1). It prints `ctx->status` and
`ctx->config->worker` (the identifier `flb_stop` reads at `:1047`) before each
call. The inducers come from the paths the census names:

| Scenario | Sequence | Inducer |
|---|---|---|
| control | start, stop, destroy | none |
| (v) | start, stop, **stop**, destroy | none |
| (i) | start (fails after the spawn), stop, destroy | `dummy` given a property its config map does not declare: `flb_input_init_all` fails on the engine thread, which writes `FLB_ENGINE_FAILED`; `do_start` joins it at `:999` and returns -1 |
| (iii) | start (fails before the spawn), stop, destroy | `security.fips_mode=on`; this host has no OpenSSL FIPS provider, so `flb_fips_init` fails at `:963` |

[`scripts/joinlog.c`](scripts/joinlog.c) is an `LD_PRELOAD` interposer.
It logs every `pthread_create` (the new identifier and the creating stack)
and every `pthread_join` (the identifier before the call and the returned
value after it), then calls the real function unchanged.
[`scripts/summarize.py`](scripts/summarize.py) resolves frames with
`addr2line` against the library each run used. The engine thread is the one
created through `do_start` (`:969`). A later `create` that reused its
identifier would make "the same thread" ambiguous; none did in any run.
[`scripts/run_gdb.sh`](scripts/run_gdb.sh) is an independent cross-check with
no interposer: a `pthread_join` breakpoint prints the argument and the caller.
Every run is its own process under `timeout -k 5 60` with `ulimit -c 0`
([`scripts/run.sh`](scripts/run.sh), [`scripts/campaign.sh`](scripts/campaign.sh)).
The watchdog fired in none of the 240 recorded runs.

## 4. Results

The debug variant ran 20 runs per scenario at each tree; the outcomes are
identical at `90978ac6` and `046accf6`
([90978ac6](artifacts/runs-90978ac6-debug.summary.json),
[046accf6](artifacts/runs-046accf6-debug.summary.json)):

| Scenario | `flb_start` | Joins of the engine identifier (site, returned) | Exit |
|---|---|---|---|
| control | 0 | **1**: `flb_stop` :1077 → 0 | 0 (40/40) |
| (v) | 0 | **2**: `flb_stop` :1077 → 0, then `flb_stop` :1058 → 0 | 0 (40/40) |
| (i) | -1 | **2**: `do_start` :999 → 0, then `flb_stop` :1058 → 0 | 0 (40/40) |
| (iii) | -1, no thread created | **1 join of identifier 0** (`worker` read 0): `flb_stop` :1058, no return | SIGSEGV, 139 (40/40) |

The gdb pass shows the same join sequence at both trees
([gdb-90978ac6](artifacts/gdb-90978ac6.summary.json),
[gdb-046accf6](artifacts/gdb-046accf6.summary.json)). In (iii) the fault is
inside `__pthread_clockjoin_ex` (`nptl/pthread_join_common.c:47`) called from
`flb_stop` at `flb_lib.c:1058`. The other joins in the gdb pass are of other
identifiers, each joined once: the output thread pool (`flb_output_thread.c:646`,
where outputs were started) and the log worker (`flb_log.c:1347`, reached
from `flb_destroy`).

ThreadSanitizer at `90978ac6`, 20 runs per scenario, no interposer
([summary](artifacts/runs-90978ac6-tsan.summary.json)): the control ran clean
(exit 0, no report) 20/20. In (v), (i) and (iii), TSan's runtime aborted at
the `pthread_join` called from `flb_lib.c:1058`, 20/20 each (exit 66,
`CHECK failed: sanitizer_thread_registry.cpp:363 "((t)) != (0)"` in
`ThreadRegistry::ConsumeThreadUserId`): it tracks no live thread under that
identifier.

**Per candidate:**

* **(i) observed** at both trees: after the failed start, `flb_stop` joined
  the identifier `do_start` had already joined.
* **(iii) observed for the `flb_fips_init` arm** at both trees: `flb_stop`
  joined the never-written identifier 0 and the process died in
  `pthread_join`. The `mk_utils_worker_spawn` failure arm (`pthread_create`
  or `malloc` failing) cannot be reached from the public API without fault
  injection and was not exercised.
* **(v) observed** at both trees: the second `flb_stop` joined the joined
  engine thread again.
* (ii) and (iv) were **not exercised**. The TSan control never runs
  `flb_stop` against an engine that is exiting on its own, so its clean
  result says nothing about (ii).

## 5. What is and is not shown

Shown: on this host (glibc 2.43, Linux 7.0, Clang 21 `-O0`), under these
inducers, `flb_stop` performs the joins the census read from its body, and
the control joins exactly once. glibc returned 0 for the second join of a
joined thread in every run: the undefined behaviour did not surface there,
only under TSan. The join of identifier 0 crashed every time.

Not shown: that another libc, build or schedule behaves the same (POSIX
leaves both joins undefined); anything about the fixed `in_tail` test's own
run, whose C5 status is unchanged; any Quirin verdict. Corpus, system and
slice cannot move, since no input of theirs changed. No upstream path is
called a defect: each item stays a candidate. A defect claim would still need
the user's decision to report upstream and, for (ii), a racing reproduction.

## 6. Records

* `W47-ENGINE-THREAD-JOIN-CANDIDATES` amended with the outcomes above
  ([followups.json](followups.json), applied by
  [`scripts/update_ledger.py`](scripts/update_ledger.py), idempotent). It is
  not retired, because (ii) and (iv) are open. Nothing is registered;
  follow-ups stay at 566.
* [evidence.json](evidence.json): hashes, commands, watchdog outcomes,
  summary hashes and the superseded development runs (`trial-1`,
  `trial-tsan-1`: same outcomes, not counted). The first run of each scenario
  and every gdb log are in [artifacts/logs/](artifacts/logs/). The other run
  logs stay under `private/w47-dstop/attempt-1/`, each with its SHA-256 in
  its campaign summary.
* STATUS: one measurement row with no verdict. `wave-47.md`: one paragraph.
  No gateway rule.

## 7. Gate

[`scripts/gate.sh`](scripts/gate.sh) runs the standing gate after the records
are regenerated. Exits are in [gate/](gate/), figures in [gate.json](gate.json)
and logs under `private/dstop-gate/` of this worktree. No native build or run
took place while the gate ran.
