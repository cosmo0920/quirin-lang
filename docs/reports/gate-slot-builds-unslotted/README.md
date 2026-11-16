# GATE-SLOT-BUILDS-UNSLOTTED: a build class the EXCLUSIVE phase closes

Card `GATE-SLOT-BUILDS-UNSLOTTED` (ledger entry of the same id, registered by
`GATE-ELASTIC`). Base main `0f6e17118`. Branch
`worktree-agent-a751a7886bfed1a17`.

## The gap

`scripts/gate_slot.py` serialised a `rust-suite` job against audits and, since
`GATE-SLOT-SHORT-JOB-LANE`, against lane jobs, but fmt, Clippy and cargo builds
took no slot. Another card's Clippy or release build could therefore load the
host while a suite's `EXCLUSIVE` phase ran constraint 16's load-sensitive rows.
Under the legacy lock every heavy step was serialised, so the path opened as
cards moved to slots. A measurement's side builds (`quirin-audit measure`,
`cargo build --release -p qcore-audit` per `rev:` or checkout side) took no slot
either.

## Decision: the first discharge option

The ledger offered two discharges: a build class the `rust-suite` class
excludes during its `EXCLUSIVE` phase only, or a measurement of the
load-sensitive rows under a concurrent build. This card takes the first.

- The phase is short. In the integrate/r9 landing suite
  (`private/integrate-r9/gate-merge-d0777/rust-suite/summary.json`, main
  checkout) the 12 exclusive units ran 62.7 s of a 740 s run phase, so a build
  that arrives then waits about a minute.
- A measurement would hold for the build, the host and the rows it was taken
  with. Rows join `EXCLUSIVE` over time (12 today), builds change size, and a
  pass under one concurrent build is not evidence for a larger one. A class is
  structural: it holds for any build that takes it.
- It keeps the suite's drain, build and parallel phases open to builds, so
  cards keep building while a suite compiles or runs its parallel units.

## Mechanism

`gate_slot.py build -- <cmd>`:

- A build holds `<gate dir>/build/running` shared for its run and is admitted
  whenever `build/admission` can be taken shared (for a few system calls). No
  slot count, no queue, no memory reading, no legacy lock (builds never took
  it, and it would hold them for a legacy job's whole run). A build waiting for
  admission holds `build/waiting` shared so `status` lists it.
- `run_test_binaries.py`, when it runs under a `rust-suite` slot
  (`QUIRIN_GATE_HELD` names `rust-suite/0`), takes `build/admission` exclusive
  before its first exclusive unit (new builds wait), then `build/running`
  exclusive (the builds in flight end first), runs the phase and lets go of
  both. Batched runs close per batch. `summary.json` gains `build_exclusion`:
  `null` outside a suite slot, else one record per phase (`closed_at`,
  `builds_in_flight_at_close`, `waited_seconds`, `drained`, `units`,
  `reopened_at`).
- `gate_slot.py rust-suite --close-builds` makes the whole command the phase,
  for a suite without one of its own: the provider workspace's `cargo test`,
  whose conformance binaries launch process providers under declared timeouts
  throughout. The receipt gains `build_exclusion` and `limits.close_builds`.
- The wait for builds in flight is bounded by `QUIRIN_GATE_BUILD_DRAIN`
  (default 30 min). Past it the phase runs beside them, new builds still kept
  out, and the record says `drained: false` with `overlapping_builds`. No job
  stops another, and an overlap is never recorded as drained. A closure that
  cannot take its locks records the error and closes nothing.
- Nesting: a build inside a `rust-suite`, `audit` (drained before any suite
  runs), `short` (closed for a suite's whole run) or `build` job is covered; one
  inside a `lean` job, which runs beside a suite, is refused (exit 2). The
  child's `QUIRIN_GATE_HELD` now lists every held lock but the legacy one; it
  used to drop the first label, which was always the legacy lock until this
  class, which takes none.
- `status` lists build holders, waiters and the closing phase (roles
  `EXCLUSIVE phase (builds closed)`, `draining builds (...)`, `waiting for the
  builds to reopen (...)`; `builds` in `--json`).

Users: `scripts/gate_scope.py` runs fmt, every Clippy step, both reevaluation
examples and the provider package build, fmt and Clippy under `build`, and the
provider suite under `rust-suite --close-builds` (`Step.slot_args`, recorded in
`planned_steps`). The measure plugin wraps each side's release build in
`gate_slot.py build --receipt sides/<side>/build.slot.json` when slots are on,
records `build_slot` in the side record (`build_command` stays the cargo argv),
and passes the slot script's location variables as it does for routes. No
analysis input or output changes: this card moves no corpus figure, so no
prediction block; the side build waits at most for one phase (or, behind a
wedged build, the bound) and that wait counts against the measurement's quota
like the build itself.

Older clients (not guaranteed): main's script before this card has no `build`
class (exit 2), takes no build lock and its runner closes nothing, so a build
may run beside an older suite's phase until every worktree is rebased. Neither
can deadlock the other: the build locks are files no older script opens, and an
older `status` reads a gate with build files. The pre-card script is kept byte
for byte as `scripts/tests/fixtures/gate_slot_lane_only.py.txt` (sha256
`ef00cd2a...`) to test that.

Not covered by any class, as before: the Python suite, the provider package
verification, Lean (its two slots run beside a suite), and anything run outside
`gate_slot.py` (a runner without a suite slot, `gate_scope.py --no-slots`).

## The 2026-11-14 starvation-test failure

On 2026-11-14 the integrate/r10 landing gate's Python suite failed once on
`test_gate_slot_starvation` (the spinning holder measured 0.19 CPU-seconds
against a 0.2 floor while a parallel card's measurement loaded the host, load
average about 15); it failed 1 of 3 isolated reruns and passed on rerun. The
same assertion failed twice in BUILTIN-EXPECT-NULL-TEST's gate (0.18 and 0.2).

The slot design should not cover it. The Python suite runs outside every slot,
and its slot tests use scratch gate directories, so no slot promises them an
idle host; putting the whole suite in an exclusive class would queue it behind
audits for hours to protect three assertions. The assertions were the
load-sensitive part: each required a fixed CPU share of a spinner (0.2 s in a
0.5 s window: 40 % of a core; 0.3 s in at least 1 s) although the accounting
under test was right.

They now bound the reported figure by the spinner's own CPU time, read from
`/proc/<pid>/stat` by the test, independent of `gate_slot`:

- `Status.test_holders_a_draining_suite_and_a_waiter` and
  `Drain.test_a_drain_gives_up_reopens_admission_releases_and_names_the_holder`:
  an outer bracket read before and after the measuring call (the report can be
  no more: this rejects a cumulative or inflated figure, because the spinner
  first spins 0.1 s), and an inner one read at the start and end of the
  measurer's own sleeps, which lie inside its measurement window (the report
  can be no less: this rejects a missing figure). A window in which the spinner
  got under three ticks is inconclusive: the interval or drain bound is doubled
  (up to 8 s), the bounds never relaxed, and a run that stays inconclusive
  fails.
- `Deadlines.test_a_tree_without_io_is_stopped_...`: the stall report shows CPU
  for the spinning tree (`> 0`), and a new sleeper control in
  `test_an_unreaped_zombie_does_not_hide_a_stall` shows none (at most two
  ticks). That zombie test asserted `samples_unmeasured == 0`, which a starved
  core broke once (a sample caught the child between exit and zombie); it now
  allows unreadable I/O only for that child, never the live sleeper, and still
  requires the stall to fire.

Evidence, with harnesses in [scripts/](scripts/):

- [starved_core.sh](scripts/starved_core.sh) runs the four tests pinned to one
  CPU shared with 15 spinners (a share near 1/17; only that core is loaded).
  The base tests failed 3 of 4 ([log](starved-base.log): 0.04, 0.04 and 0.06
  against floors of 0.2, 0.2 and 0.3); this card's passed 4 of 4 in four runs
  ([2](starved-card-2.log), [3](starved-card-3.log), [4](starved-card-4.log), [5](starved-card-5.log)).
  The first run of the card's tests (before the inner bracket and the zombie
  fix) failed 2 of 4 and is kept only in the worktree's private log: its lower
  bound subtracted the measurer's wall time outside the interval, which a
  starved measurer stretched to 0.53 s.
- [mutate_cpu_report.sh](scripts/mutate_cpu_report.sh) mutates
  `sample_delta`'s CPU figure in a scratch copy: reported as 0, and reported
  cumulative instead of over the window. Both bracketed tests fail on both
  mutants and pass unmutated ([log](mutate_cpu_report.log)): 2 of 2 mutants
  killed by each test.

## Tests

`scripts/tests/test_gate_slot_builds.py` (16): unlimited builds without queue
or legacy lock, a legacy holder not blocking builds, a closure keeping new
builds out until it reopens (listed by `status`), waiting for builds in flight,
running past its bound beside a build with the overlap recorded, a suite slot
outside its phase leaving builds open, `--close-builds` and its receipt,
refusals and nesting, the child's held-lock variable, the runner closing builds
around exclusive units only (parallel units saw them open) with
`build_exclusion` in its summary, the scoped gate's step classes and argv, and
the older script sharing a gate. `test_gate_slot.py` names an unknown class
other than `build`. `quirin-audit-plugin-measure`'s
`sides::tests::a_side_build_takes_the_build_class_under_slots`.

## Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 0f6e17118
--out <dir>` (main checkout's toolchain env). The runner and scoper changed,
so the whole Rust suite ran.

- Attempt 1 (`private/gate-1`, tree at `b0b8e922b` plus the report): exit 1.
  submodules, fmt, Clippy (both under `build`), Rust suite (638 units, 681
  summaries; 9,510 passed, 0 failed, 17 ignored) and both reevaluation
  examples exit 0; Python 777 tests, 1 failed
  ([log](gate-1-python.log)): rule 29's walk tripwire
  (`test_tracked_files`) caught this card's `Path("/proc").iterdir()` in the
  starvation test's child lookup. Repaired in `6ac9a54e7` (read
  `/proc/<pid>/task/<pid>/children`, no walk); the starved-core run was
  repeated after it ([5](starved-card-5.log), 4 of 4).
- Attempt 2 (`private/gate-2`, `39994b407`): **exit 0**
  ([gate.json](gate.json)). submodules, fmt, Clippy exit 0; Rust suite 638
  units, 681 summaries, 9,510 passed, 0 failed, 17 ignored, 148 filtered;
  both reevaluation examples exit 0; Python 777 tests OK. The suite's
  `summary.json` records the first live build closure: the EXCLUSIVE phase
  (12 units) closed builds at 23:55:35Z, found none in flight (other
  worktrees run main's script), waited 0.019 s, reopened at 23:56:44Z,
  `drained: true`.

## Follow-ons

Registered in [integration-amendments.json](integration-amendments.json):
`GATE-SLOT-LEAN-BESIDE-EXCLUSIVE` (Lean and the other unslotted steps still
run beside the phase) and `GATE-SLOT-BUILD-DRAIN-REVIEW` (the 30 min bound, and
a measurement's build wait counted against its quota). Amended:
`GATE-SLOT-QUEUE-OLD-CLIENT-RETIREMENT` (the new fixture) and
`GATE-SLOT-LANE-DURING-SUITE-RUN` (the runner now marks its phase, which an
EXCLUSIVE-only lane closure could reuse).
