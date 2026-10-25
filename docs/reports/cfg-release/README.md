# CFG-RELEASE: release the projection result after its last full-record consumer

Base `710021bd713b90037e803d4a1f87d95eba6476c9` (= origin/main at launch).
Implementation `3d358f76e9ea02f5537961af2065441842e685de`, measured as TIP.
It changes no verdict. Parent: [AUDIT-IR-RETAINED-PRODUCTS](../audit-ir-retained-products/README.md),
which archived the patch as [proposed-private-cfg-release.patch](../audit-ir-retained-products/proposed-private-cfg-release.patch)
after automatic review refused to apply it. The user approved the patch; this
card applies it with its own gate and a report parity check.

## What changed

`git apply -p0` applied the archived patch cleanly on the base. The only
other edit is rustfmt's layout of the new `match` arms, so no hand port was
needed. The diff touches one function, `record_projection` in
`crates/qcore-audit/src/projection.rs`:

- A private enum, `ProjectionPublication` (`Safe(n)`, `Unsafe(n)`,
  `Unknown { reason, projected }`, `Error(LegacyCError)`), holds exactly what
  the tally and console still need: the projected-function count, the
  Unknown reason and the gateway error.
- It is computed by moving `result` immediately after the last full-record
  consumer, `contract_dependencies::annotate_unit`. Before that point
  `report.record`, `summary_of` and both annotators read `result` as before.
- `result` is consumed there, so the certificate or summary, with every
  function's CFG, is dropped before the reading-site binding, the
  callback-member record, the enumeration record and the tally. Before the
  card it lived to the end of the function.
- The tally and console arms read the precomputed counts. The published
  verdict still comes from `report.last_unit_verdict()`, and the Unsafe arm
  still tallies `Unsafe` without consulting it, as before.

The count expressions match the old ones exactly:
`certificate.checked_functions.len()`, `summary.functions.len()`, and
`partial.map_or(0, |s| s.functions.len())`. The console strings are
unchanged. Nothing between the new drop point and the tally read `result`;
the compiler enforces this, because `result` has been moved. No new test was
added. The change moves where an owned value is dropped, and the parity runs
below test the observable output on four routes.

No inference rule, verdict path, refusal, source identity or published field
changes. So there is no Lean counterpart, and Lean is not in the gate's scope.

## Fixture parity

[scripts/parity.py](scripts/parity.py) is the parent card's harness. Only the
recorded fixed build label differs. It runs four units with twelve function
rows: two applied refutations over two phases, held cross-unit callers, a Safe
certificate and a data-only unit. Receipt: [fixture-parity.json](fixture-parity.json).

- BASE runs at one worker; TIP at one and at two.
- Exit 0 on every run.
- `report.json` and `report.assumptions.json` are byte-identical at both job
  counts.
- `report.invocation.json` is byte-identical at one worker. At two workers it
  differs only in the harness's explicitly checked `jobs` record and its two
  phase observations.

Both binaries are the release builds used below. Private copy:
`/media/Data/GitHub/quirin-lang/private/cfg-release/fixture-parity/`.

## Identity run: corpus, slice and system, BASE against TIP

### Method

Drivers are in [scripts/](scripts/). They reuse AUDIT-CANONICAL-PATHS's
`pin_run.sh`, `measure.py`, `compare.py`, `verdicts.py` and `time_key.py`
unchanged.

- **Snapshots.** Each side has its own `git archive` snapshot, BASE
  `710021bd7` and TIP `3d358f76e`. The pinned libevent submodule
  (`d82464a2`) is cloned into each.
- **Builds.** Each snapshot is built and run in a private mount namespace,
  bound at one fixed path (`private/cfr/snap`), with its output root bound at
  `private/cfr/out`. Analyzer, profiles, contracts, inputs and outputs
  therefore have one spelling on both sides.
- **Build label.** Both release builds carry the fixed comparison label
  `QCORE_AUDIT_BUILD_COMMIT=710021bd7…`. Otherwise `build.rs` would record the
  HEAD of the enclosing worktree (MEASURE-SNAPSHOT-HAZARDS item 4). The actual
  source and binary digest of each side are recorded separately:
  BASE `947b81bc…b44b`, TIP `5cc9c97c…9889`.
- **Inputs.** Both sides read the ORIGINAL pinned inputs (`measure.py
  --inputs original`). Since AUDIT-CANONICAL-PATHS, BASE selects canonical
  paths too, so no mirror and no prefix substitution is needed.
- **Slots and workers.** Each route of each side runs in a `gate_slot.py
  audit` slot (7 GiB budget, `--jobs 3`). The two sides of a route run
  concurrently.
- **Clock.** `SOURCE_DATE_EPOCH=1761004800` is set on both sides. Clang
  honours it for `__DATE__`/`__TIME__`: checked directly with
  `/usr/lib/llvm-21/bin/clang -E`. The audit's Clang invocations inherit the
  environment; no `env_clear` exists in the gateway, audit or
  source-analysis crates. So Monkey's `MONKEY_BUILT` no longer depends on the
  clock, which is the PREPASS-CLOCK-DEPENDENT-KEY residue. This is a
  measurement control, set identically on both sides. It is not an analyzer
  change.

### Results

All six routes exit 0 in the audit. **Every published artifact is raw
byte-identical, BASE against TIP, on all three routes:** ten files per route,
each the same before and after normalisation. The files are the report, every
sidecar (`contracts`, `chunk-contracts`, `source-effects`, `invocation` where
the route writes it), the null projection and its summary, the outer
projection, `monkey-source.json` on the corpus, and the audit/console logs.
Comparisons are in [identity/](identity/) (`*-bytes.json`).

Verdicts match row for row (`*-verdicts.json`): every unit status and reason
code, every row status, and every null row's custody, combined and null
verdict, scope and check outcomes.

| Route | Units | Rows (Safe / Pending / Unknown / Unsafe) | Null rows (Safe / Unknown / Unsafe) |
|---|---|---|---|
| corpus | 185 | 1,013 / 0 / 3,812 / 9 | 2,549 / 2,355 / 17 |
| slice | 16 | 57 / 4 / 342 / 1 | 190 / 216 / 0 |
| system | 15 | 101 / 0 / 343 / 4 | 211 / 243 / 0 |

These counts are the base's own verdicts, reproduced. They are not new
coverage. The corpus Unsafe rows remain provisional.

The pre-pass summary keys are all shared, with equal values
(`*-time-key.json`): 1,058 on the corpus, 1,039 on the slice, 1,037 on the
system, and none unshared. So the clock-dependent key that AUDIT-CANONICAL-PATHS
had to explain does not appear under the fixed epoch.

**Environment-digest substitution: none.** Every artifact is raw
byte-identical. No path, environment digest or build label had to be
rewritten, so the one-to-one substitution the brief allows was not used.
`compare.py`'s only normalisation drops the `preparation:` timing line from
logs (docs/agents/validation.md).

**Driver exits.** Slice and system drivers exit 0. Both corpus drivers exit
101 with audit exit 0. During the route, the audit's own `git` rewrote
`third_party/libevent/.git/index`, the stat cache of the freshly cloned
submodule. It did so on both sides, even though `git update-index --refresh`
had run on each clone beforehand. That index is the only snapshot file that
changed; externals and binaries are stable (`drivers` in
`corpus-bytes.json`). This is MEASURE-SNAPSHOT-HAZARDS item 3, unchanged.

## Memory (corpus)

This is the card's purpose. **Result: no material memory reduction.** Receipt:
[memory.json](memory.json), from [scripts/memory.py](scripts/memory.py).

Each side ran in a systemd user scope with memory accounting and no memory
limit ([scripts/cgmeasure.sh](scripts/cgmeasure.sh)). Two figures are
recorded:

- `/usr/bin/time -v`'s maximum resident set size. This is the largest single
  descendant, the `quirin-audit` process.
- The scope's `memory.peak`, `memory.events` and `memory.pressure`, read just
  before the scope ended.

| Route | Max RSS BASE | Max RSS TIP | TIP − BASE | Scope peak BASE / TIP | Wall s BASE / TIP |
|---|---:|---:|---:|---:|---:|
| corpus | 10,129,997,824 | 10,042,564,608 | −87,433,216 (−0.86 %) | 10,647,760,896 / 10,870,808,576 | 7,654 / 7,645 |
| slice | 6,815,649,792 | 6,774,689,792 | −40,960,000 (−0.60 %) | 7,766,237,184 / 7,685,464,064 | 813 / 811 |
| system | 6,770,163,712 | 6,784,974,848 | +14,811,136 (+0.22 %) | 7,588,831,232 / 7,164,059,648 | 755 / 750 |

On every route, `memory.events` has `max`, `oom` and `oom_kill` at 0, and peak
swap is 0. Memory pressure is negligible. The PSI `some` totals are
329 µs / 515 µs (corpus BASE / TIP), 251 / 69 (slice) and 8,983 / 4,352
(system); every avg10/60/300 is 0.00.

Reading:

- **Corpus.** The process peak falls by 87 MB of 10.1 GB (0.86 %) on the
  route this card targets. The slice falls by 0.6 %, and the system rises by
  0.2 %.
- **Noise.** Each figure is one observation per side. The two sides of a
  route ran concurrently on a shared host, and other cards' jobs ran beside
  them. The differences are within what scheduling of three workers can
  plausibly move, so this card claims **no demonstrated reduction**. It
  claims only that the release did not raise the peak materially.
- **Scope peak.** The scope figure also charges page cache. With both sides
  reading the same inputs at once, cache is charged to whichever side touches
  a page first, so the scope figures are not comparable as working-set
  measures. Here they move in both directions.
- **Why the effect is small.** The released object is one unit's projection
  result, now dropped a little earlier within `record_projection`. Before the
  card, it was dropped at the end of that function anyway, before the next
  unit's result is recorded. So the release shortens a per-unit lifetime by
  the reading-site, callback-member and tally steps. It does not remove
  anything that accumulates across units. The parent report already places
  the whole-job peak in run-wide final rows, the member pre-pass and the
  final folds, and this observation is consistent with that. It does not
  attribute the peak to an owner.
- **Peak against slot budget.** The corpus process peaks at about 10.1 GB
  under `--jobs 3`, above the 7 GiB slot budget that capped its workers. The
  pinned corpus route receives the budget only as `QCORE_AUDIT_JOBS`, not as
  a memory plan (docs/agents/validation.md, gate slots). This is an
  observation for the gate-slot owners; this card does not change it.

## Gate

GATE_SECTION

## Ledger

- No acceptance-matrix entry tracked the patch. `named_followups` has no
  CFG, retained-products or projection-result entry; the open item lived only
  in the parent report and the halt record
  (`docs/reports/w48-impl-round1-halt/`). Nothing is retired.
- `PREPASS-CLOCK-DEPENDENT-KEY` gets an appended observation. Callers can
  already make the key reproducible with `SOURCE_DATE_EPOCH`, and this card's
  routes are byte-identical under it. The entry stays open: the audit does
  not fix or record the epoch itself.
- `MEASURE-SNAPSHOT-HAZARDS` gets an appended observation. Item 3 (the
  libevent `.git/index` refresh) recurs even after `git update-index
  --refresh` on the fresh clone before the run.
- `CORPUS-PEAK-ABOVE-AUDIT-SLOT-BUDGET` is registered. It records the
  observed corpus process peak above the slot budget.

## Boundaries and follow-ons

- The release is applied, and its parity is byte-exact on four routes. No
  memory gain is demonstrated: one paired observation per route, with a
  0.86 % corpus process-peak decrease that is not distinguishable from noise.
  The parent memory objective is not advanced by a measurable amount.
- The corpus process peaks above its 7 GiB slot budget (about 10.1 GB at
  `--jobs 3`), registered as `CORPUS-PEAK-ABOVE-AUDIT-SLOT-BUDGET`.
- The parent card's other residuals remain open: run-wide pre-pass
  ownership, final-row streaming, bounded encoding and page-cache accounting,
  and the 6 GB and cold/warm targets.
- `PREPASS-CLOCK-DEPENDENT-KEY`: the audit could set and record a fixed
  `SOURCE_DATE_EPOCH` for its own Clang invocations. That is the entry's
  discharge, not this card's.
