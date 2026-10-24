# GATE-ELASTIC

Seam E of the [pluggable-cards plan](../../plans/pluggable-cards/README.md),
under the user's decisions 4 (a scoped gate per card, the full standing gate
once per landing) and 5 (audit slots by memory budget, 7 GiB by default, 5 or
4 GiB selectable) of 2026-10-24. Base `fb5412930`. No analyzer, contract or
Lean change, so no identity run and no Lean parity obligation.

## What changed

- `scripts/gate_scope.py --base <rev>`: the scoped card gate. It maps the diff
  (both sides of a rename, plus uncommitted and untracked paths, because the
  commands test the working tree) to the crates of both workspaces from
  `cargo metadata --no-deps`, closes over reverse dependencies across the two
  workspaces, prints the affected set with every member's reason, and runs the
  steps, writing argv, exit, seconds, counts and log of each into `gate.json`.
  `--plan` previews, `--full` forces the whole suite, `--no-slots` runs inside a
  caller's lock, `--readers` prints the cross-crate reader census.
- `scripts/run_test_binaries.py --packages a,b`: every phase restricted to those
  members (`-p` instead of `--workspace` for build and capture; the doc-test
  unit over the selected members with a library target, none if there is
  none). The `EXCLUSIVE` phase is unchanged and still fails closed on a stale
  entry of a selected member; a non-member name is refused before building;
  `summary.json` records `packages` (`null` = whole workspace) and the run
  prints the selection. Without `--packages` the runner is byte-for-byte the
  old command line.
- `scripts/gate_slot.py <class> [--memory-budget 7G|5G|4G] -- <cmd>`: numbered
  `flock` slots under `/tmp/quirin-gate/<class>/<n>`; classes, budget passing
  and the legacy interlock are specified once in
  [validation](../../agents/validation.md#gate-slots).
- `scripts/gate_lean.py`: the scoped Lean step, built from
  `check_lean_audits.py`'s own toolchain guard, bounded build step and
  parallel audit runner (imported, not copied or edited).
- `docs/agents/validation.md` amended on the user's authority: the standing
  per-change gate is now "scoped card gate per card, full gate per landing, by
  the integrator"; new sections [Scoped card gate](../../agents/validation.md#scoped-card-gate)
  and [Gate slots](../../agents/validation.md#gate-slots). The full gate's
  command block and text are unchanged; the one sentence that said who runs it
  is replaced. `operations.md`'s working rule and `AGENTS.md`'s gate pointer now
  defer to that section instead of saying "run the full gate".

## Scope rules and why

**Compile-time readers are found, not listed.** Every tracked `.rs` file a
crate compiles is scanned, including files outside its directory that it
reaches through `#[path]` (and their plain `mod` children), for
`include_str!`/`include_bytes!`/`include!` with a literal or
`concat!(env!("CARGO_MANIFEST_DIR"), …)` argument and for `#[path]`. The census
(`gate_scope.py --readers`, [readers.json](readers.json)) finds 130
cross-crate (directory, owner, reader, kind) rows; the one include it cannot resolve
is `ct_corpus.rs:8069`, which is text inside a string literal, and is listed
in every `gate.json` as unresolved. **Directory granularity** is the default,
as the brief states it (a directory read by `include_str!`/`#[path]` belongs to
the reading crate), so a sibling of a read file counts. `--reader-granularity
file` claims only the read file, which is exactly what the macro compiles in;
the selection table shows what the choice costs: under the default every
`proof/QCore/*.lean` change reaches `qcore-c-gateway` (it includes more than fifty
files there) and so runs the full suite, under `file` such a change selects
`qcore-compiler` and `qcorec` unless the gateway includes that very file. The
default is the integrator's to change; I kept the brief's.

**Run-time readers are over-approximated.** A string literal in code
`cargo test` runs (examples and benches excluded, except the two reevaluation
examples the gate runs) that names a tracked path or one of its parents,
resolved against the crate root, the file and the repository root, makes the
crate a reader of everything under it; a bare `.join("x")` counts too. This is
why most crate changes select `qcore-compiler` (`workspace_layering.rs` joins
`crates`), `qcore-verification` (`composition_bounds.rs` walks `crates/`) and
their dependents, and why any `providers/` path reaches `qcore-contract`
(`production_symbolic_adoption.rs` walks `providers/`) and so the full suite.
It cannot see a path computed from data or a whole-repository walk filtered
afterwards (`qcore-contract`'s `bounds_contract.rs`, `compiler/tests/tracked_inputs.rs`).

**Full suite** when the diff reaches one of the four named crates, a workspace
manifest, lockfile or toolchain file, `run_test_binaries.py` or
`gate_scope.py` itself, or a path no crate owns or reads that is not prose
(`docs/`, top-level `*.md`, `editors/`, `.github/`), Python, Lean or provider
tooling: fail-closed for unclassified inputs.

**Provider gate** when `providers/` changes, exactly the five commands of
`providers/README.md` (a test keeps them equal) with the package output in the
run's directory, plus a verify of every built package as CI does. Provider
members reached only through their dependencies on core crates (the
conformance crate dev-depends on six) are listed in `gate.json`, not gated: the
full gate does not gate them either (follow-up below).

[selection.json](selection.json) (from [scripts/selection_table.py](scripts/selection_table.py))
records the selection for 18 representative paths under both granularities.
Docs, scripts and editor paths select no Rust crate; an example, a CT fixture
or a `proof/` change under file granularity selects `qcore-compiler` and
`qcorec`; a non-core crate selects five or six of the thirteen root members;
the four core crates, `providers/`, `tests/support/` and the lockfile select
the full suite.

## Slots

`rust-suite` takes its slot, then closes audit admission (a shared-locked
`audit/admission` file each audit passes through), then every audit slot up to
the host ceiling (`MemTotal` at the smallest budget, so every possible audit
slot), so it waits for running audits and no new audit starts. An audit asks
for a slot among `max(1, floor(MemAvailable / budget))` at each attempt and
receives the first free one. The budget's arithmetic mirrors
`crates/qcore-audit/src/parallel.rs` `budgeted_jobs` ((budget - 1024) / 2048
workers: 3 at 7 GiB, 2 at 5 GiB, 1 at 4 GiB), and a test pins that line of
`parallel.rs`. Every slot job holds the legacy lock shared; a nested
invocation inside a slot that covers it runs in the parent's slot, any other
nesting is refused. Lock files carry the holder's pid and argv while held.

## Evidence

**Scoped gate on this card's own diff** (`gate_scope.py --base fb5412930`):
mode `full`, because `run_test_binaries.py` and `gate_scope.py` changed; Lean
`all`, because `gate_lean.py` changed; no provider gate; no crate owns or reads
any other changed path (the docs and report paths are prose with no reader).
That is the selection expected, and its steps are the standing gate's commands,
so the one run is both the scoped-gate demonstration and the full standing
gate.

**Full standing gate** at measured tip `4d8f568e7`, run as
`flock /tmp/qcore-c-card-validation-20261022.lock python3 scripts/gate_scope.py --base fb5412930 --no-slots`
(2026-10-24 13:17:58Z to 13:30:56Z, after `git submodule update --init
--recursive`, exit 0), record in [gate.json](gate.json), logs under the
worktree's `private/ge/fullgate/`:

| Step | Command | Exit | Counts |
|---|---|---|---|
| submodules | `git submodule status --recursive` | 0 | 1 submodule, none missing or moved |
| fmt | `cargo fmt --all -- --check` | 0 | |
| Clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite | `python3 scripts/run_test_binaries.py` (whole workspace) | 0 | 549 units, 562 summaries; 8,639 passed; 0 failed; 15 ignored |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | 370 audits, Lean 4.35.0 |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | 428 tests |

Clang present (`clang-version.txt`, exit 0). The provider gate is not due
(`providers/` unchanged); `python3 -m unittest discover -s providers/tests`
was run once anyway and passes 25 tests (81.9 s).

**After the measured tip.** While the slot demonstration waited, a `flock`
legacy job blocked in the kernel queue took the lock ahead of the two slot
jobs, which polled every 2 s: a poller loses those races every time. Commit
`db0e4d1c2` therefore makes an untimed wait for the legacy lock block in the
kernel queue like `flock(1)` (a timed wait still polls), with a test. Only
`scripts/gate_slot.py`, its test, three `gate_lean.py` refusal tests and this
report changed after `4d8f568e7`. The scoped gate over that delta
(`gate_scope.py --base 4d8f568e7`, [delta-gate.json](delta-gate.json)) selects
no crate and no Lean step, as a scripts-and-prose change should, and runs the
submodule check, fmt and the Python suite; its exits are in that file.

**Incomplete supplemental attempts.** A live slot demonstration
(`gate_slot.py rust-suite -- run_test_binaries.py --packages
qcorec,quirin-audit-plugin-libevent`, and `gate_slot.py lean -- gate_lean.py`
on `QCore.ListLookup` with two audits) was started twice, first polling and then
blocking, and never got the legacy lock: it waited behind three other cards'
legacy jobs and behind this card's own legacy full gate for about four hours in
all, and I stopped both jobs (task handles, not patterns) at 2026-10-25 00:17
JST. They are not evidence of a scoped run; they do show the interlock from the
slot side (no slot job ran while any legacy job held the lock), and their logs
remain in `private/ge/demo{A,L}-{polling,blocking}-attempt/`. Slot accounting
is covered by real-`flock` tests instead (`test_gate_slot.py`: fixed counts,
memory-budget counts, suite-excludes-audits, both interlock directions, the
untimed kernel-queue wait, receipts, refusals, nesting, signals), and the
restricted runner by `test_run_test_binaries.py`'s `PackageSelectionTests`;
`gate_lean.py` has run only through its refusal tests, not against Lake.

## Boundaries

- The scoped gate is a card's gate, not the arbiter: the full gate per landing
  is, and the run-time scan's blind spots above are why.
- Under `-p`, cargo unifies features over the selected members only, so a
  scoped build can differ from the `--workspace` one; the landing gate covers
  the difference.
- fmt, Clippy and builds take no slot, so another card's build can load the host
  beside a `rust-suite` job's `EXCLUSIVE` phase; with the legacy lock gone this
  is the remaining load path into constraint 16's rows.
- `MemAvailable` excludes running audits' resident memory, so audit admission
  counts them twice and is conservative; a `flock` exclusive waiter (a legacy
  job) can wait indefinitely while slot jobs keep the legacy lock shared.
- `run.json` of an audit records the budget only on the `current` route (the
  flag); on other routes it records `QCORE_AUDIT_JOBS`, and the slot receipt
  records both. Teaching the pinned route the flag is an audit change, outside
  this card.

## Follow-ups

Registered in [integration-amendments.json](integration-amendments.json):
`GATE-SCOPE-RUNTIME-READS-HEURISTIC`, `GATE-SCOPED-RUN-UNEXERCISED`,
`PROVIDER-CONFORMANCE-CORE-DEPS-UNGATED`,
`GATE-SLOT-BUILDS-UNSLOTTED`, `PROVIDER-PYTHON-TESTS-UNGATED`,
`TASK-CONTEXT-GATE-ROLES`, `LEGACY-GATE-LOCK-INTERLOCK`; an append to
`CT-FIXTURE-EXTERNAL-READERS`.
