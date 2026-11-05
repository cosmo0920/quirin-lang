# MEASURE-QUOTA-AND-INCREMENTAL

Round 19, card C (2026-11-05), base `25508ccaf`. User request (verbatim,
2026-11-05): "We need to record the requirements of saving gating time
tasks.", then "Then, we launch the cards which are planned by another model
who has huge sight of this repo's code base. Launch three cards of gate-time
reductions." Scope: requirements R7 and R8 of
[gate-time reduction](../../plans/gate-time-reduction/README.md), tracked by
`MEASURE-SLOT-WAIT-OUTSIDE-QUOTA` and `MEASURE-INCREMENTAL-BASELINE-COMPOSITION`.

Outcome: **R7 is done** (the quota counts analysis time; slot waits are
recorded apart). **R8 stops with options** (§2): with what baselines record
today, fail-closed reuse re-audits every profile of every contract card, so it
saves nothing; a saving needs a decision on what may count as evidence that a
unit did not consult a changed declaration.

No analyzer, gateway, contract or Lean semantics change, and nothing an audit
reads changes, so there is no prediction and no declared measurement.

## 1. R7: the quota counts analysis time

### Where the quota lives

`--timeout-ms` is the subcommand host's quota (`crates/qcore-audit/src/subcommand_plugins.rs`,
`supervise`), not the measure plugin's: the host kills the plugin's process
group at the deadline. So R7 needed the host to learn about waits, which is
outside the brief's file list (`crates/quirin-audit-plugin-measure/**`,
`scripts/gate_slot.py`). No other card of the round claims the host file.

### Design

| Part | What it does |
|---|---|
| `scripts/gate_slot.py --wait-record PATH` | A live record (`quirin-gate-wait/1`): `waiting` from the moment the script asks for its slot, `admitted` with `waited_seconds` once the slot is held and before the command starts; each replaced whole by a rename. A nested job covered by its parent is `admitted` at once with 0. |
| Measure plugin (`src/waits.rs`, `sides::run_route`) | Every route job is begun before any is spawned; each route polls its record every 100 ms. The measurement is **idle** while every route job in flight waits; only then does it keep the host's marker `.quirin-audit-slot-wait` in its output. One route waiting while another runs is progress, so it counts. A marker write or removal that fails fails the measurement. |
| Host (`supervise`, `SlotWaits`) | Polls the marker with its existing 5 ms loop; time between two polls that both saw it accrues, up to `--slot-wait-limit-ms` (default the quota itself; `0` counts every wait, the behaviour before this card). Deadline = start + quota + accrued. A marker left in a reused output is removed before the spawn. |
| Records | Host report `run`: `wall_time_ms`, `slot_wait_ms`, `analysis_time_ms`; invocation record: `slot_wait_limit_ms`; timeout reason names the excluded wait and the limit. Comparison record `timing`: the measurement's wall, declared wait and analysis time, and per side and route (with its profile) the route's own wait and run time, also in each route's `run.json` entry. |
| SDK | `quirin_audit_plugin::SLOT_WAIT_MARKER`; no protocol change (the request and its `Quotas` are unchanged). |
| Slot variables | `QUIRIN_GATE_DIR`, `QUIRIN_GATE_LEGACY_LOCK`, `QUIRIN_GATE_MEMINFO` reach the routes only when the host passes them with `--plugin-env`, so a test's scratch gate never touches `/tmp/quirin-gate`. |

**Trust boundary.** The wait is the plugin's declaration: the host does not
verify that the plugin's descendants really wait. The limit is what bounds it:
a run never exceeds quota + limit (by default twice the quota). The quota is a
resource bound; a breach is `Unknown`, and no verdict path reads the wait.
Recorded as follow-up `SLOT-WAIT-DECLARED-BY-PLUGIN`.

### Tests (all pass in the card gate at `cffb3e4e1`)

- `crates/quirin-audit-plugin-measure/tests/slot_wait.rs`: a real `flock`,
  held by `gate_slot.py rust-suite -- sleep 6` in a scratch gate directory,
  and a measurement through the real host with a 4,000 ms quota. With
  `--slot-wait-limit-ms 0` (the old accounting) the host kills it,
  `plugin-timeout`, `Unknown`; with the waits excluded it completes, the
  host records the wait apart from the analysis time, and the record's
  `timing` gives the tip side's `slice` route (profile `fixture-profile`) its
  own wait. Records: [r7-held-slot.json](r7-held-slot.json)
  (written by [scripts/evidence.py](scripts/evidence.py)).
- `crates/qcore-audit/tests/subcommand_plugins.rs`,
  `declared_slot_waits_are_outside_the_wall_time_quota_up_to_the_limit`: the
  marker excludes a 2 s wait from a 1 s quota; the default limit is the
  quota; limit 0 times out; a 300 ms limit times out with the reason naming
  it; a stale marker excuses nothing.
- `scripts/tests/test_gate_slot.py`: the wait record says `waiting` under a
  held admission lock and `admitted` with the measured wait before the command
  runs; a nested job is admitted at once.
- `waits::tests::idle_only_while_every_job_in_flight_waits` (unit).

### Wall split, before and after

On the held-slot test (this host, [r7-held-slot.json](r7-held-slot.json)):

| Accounting | Outcome | Wall | Slot wait excluded | Analysis (counted) |
|---|---|---:|---:|---:|
| waits counted (`--slot-wait-limit-ms 0`) | killed, `plugin-timeout` | 4,001 ms | 0 ms | 4,001 ms |
| waits excluded (this card) | completed | 6,143 ms | 6,010 ms | 133 ms |

On the two recorded measurements of §1 of the plan, from their own
`gate_slot` receipts ([scripts/split.py](scripts/split.py);
[split-libc-gap.txt](split-libc-gap.txt),
[split-cfl-mpack-r2.txt](split-cfl-mpack-r2.txt)). These are derived from the
records, not re-measured: the host's quota counted the whole wall then.

| Measurement | Host wall (counted then) | Every route waiting (excluded now) | Counted under R7 | Tip routes: wait / run |
|---|---:|---:|---:|---|
| LIBC-GAP `libc-gap-20261105T034105Z` | 383.0 min | 218.8 min | 164.2 min | corpus 218.8 / 117.7; system 354.4 / 20.4; slice 354.4 / 18.3 |
| CFL-MPACK r2 `cfl-mpack-r2-20261105T023115Z` | 238.1 min | 89.4 min | 148.7 min | corpus 89.4 / 145.8; system 89.4 / 16.0; slice 89.4 / 14.0 |

On LIBC-GAP, system and slice waited 135.6 min more while the corpus ran;
that stays counted (the measurement was working).

## 2. R8: incremental baseline composition (stopped with options)

### What a unit's verdict depends on, and what a baseline records

Read against `25508ccaf`:

1. **Contracts are one run-wide environment.** Every profile's baseline
   records the same twelve bundle digests (`contract` and eleven provider
   contract trees; `private/baselines/c8c7edf21…/<profile>/baseline.json`),
   and every unit is analysed against the whole environment. A per-unit
   contract digest exists only through what the unit consulted.
2. **The published consultation record omits misses.** The gateway notes
   every name looked up, `found` and `missed`
   (`qcore_schemas::contract_consultation::Consulted`), but a row publishes
   only declarations (`declarations`, `held`, `withdrawn`, plus `summaries`,
   `null_premises`, `hypothesis_dependencies`;
   [depkeys-system-c8c7edf21.txt](depkeys-system-c8c7edf21.txt)). A card that
   **adds** a declaration (both sample cards do: LIBC-GAP adds 23 libc
   declarations, CFL-MPACK adds `sds_lifetime.qv` and `mpack/writer.qv`)
   changes units whose baseline rows list nothing about it. The record is also
   declared "evidence only: no verdict reads this", and its completeness
   rests on every whole-map reader calling `note`; callback candidates
   (`relied`) need not be spelled in the body.
3. **Run-wide phases carry cross-unit facts.** The refutation table
   (`refuted_contracts.rs`, phase two) holds every call site of a declaration
   its own body refutes, in every unit; the cross-unit link homes
   (`QCORE_CROSS_UNIT_LINK_MAP`, set by every measure route) install home
   units' W4 summaries, computed under the contracts, into every unit's
   environment; the pin evaluation reads the whole contract set once.
4. **The corpus route has no unit seam.** `eight-home` runs the pinned replay,
   whose ChunkIO prelude/postlude share typed certificates across a library's
   units (`W45-CHECKPOINT-PINNED-REPLAY-ROUTE`). The `current` route's `.qo`
   checkpoints refuse the null selection (`QCORE_NULL_PROPERTY`) and the
   runtime projection (`QCORE_OUTER_PROJECTION`) that every measure route
   sets, and their identity names the analyzer build commit and binary
   digest, so they never reuse across revisions.
5. **Where the time is.** Tip run time (§1 table): corpus 117.7 and
   145.8 min, system 20.4 and 16.0, slice 18.3 and 14.0. The corpus is
   80-83 % of the analysis.

So, fail-closed with today's records: any contract change re-audits every unit
of every profile. A reuse rule built only from published records would
re-audit everything on both sample tips, and the equality check would compare
a full run with itself. That is not evidence for R8, so I did not build it.

### Options

- **A. Unit reuse on the configured routes behind a published consultation
  closure (recommended, with D for the corpus).** The gateway publishes per
  unit every name looked up (found and missed), the `relied` callback
  candidates, and a flag for any whole-map read without a `note`; a new
  baseline publication carries it. The measure tool reuses a baseline unit
  only when (a) the diff between the two revisions touches only contract data
  (`providers/*/contracts/**`, their locks, `contract/**`), (b) no changed,
  added or removed declaration name (with every declaration whose
  include/inherit resolution changes) meets the unit's closure, (c) the tip's
  refutation table, link-home evidence and pin template equal the baseline's,
  iterated to a fixpoint over the re-audited units, else full, and (d) the
  unit's compile configuration and preprocessed digest match. It elevates an
  evidence-only record to a reuse input, so it needs the user's approval, a
  Lean locality statement (a unit's result is a function of the environment
  restricted to its closure), and a gateway card. Saving: system and slice
  only, about 30-35 of 150-165 analysis minutes per contract card.
- **B. Profile reuse only.** Reuse a profile's baseline whole when no
  analyzer path and no bundle changed. Sound by determinism, measure-plugin
  only, but every contract card changes a bundle every profile reads: no
  saving on R8's target.
- **C. Advisory incremental by spelled names.** Re-audit units whose
  preprocessed text spells a changed name, reuse the rest, never for a
  landing. Unsound as a claim (callback candidates and minted names need not
  be spelled; refutations and link homes move other units), so the landing's
  full measurement would stay the arbiter and a wrong card measurement would
  surface only there.
- **D. Corpus first.** Per-group checkpoints of the pinned route
  (`W45-CHECKPOINT-PINNED-REPLAY-ROUTE`), then A extended to it. The large
  saving (the corpus is four fifths of the analysis), the most work.

Recommendation: decide A's evidence rule, implement it as a gateway card (the
closure and its Lean statement) followed by the measure-side card, with D
scheduled for the corpus; keep incremental opt-in until the equality check on
sampled landings holds, as the brief says. R7 already takes the idle waits
(218.8 of LIBC-GAP's 383.0 minutes) out of the quota.

## 3. Gate

`python3 scripts/gate_scope.py --base 25508ccaf --out <worktree>/private/gate`
on `cffb3e4e1` ([gate.json](gate.json)): scoped mode, eight packages
(qcore-audit, qcore-compiler, qcore-verification, qcorec, quirin-audit-plugin,
quirin-audit-plugin-libevent, quirin-audit-plugin-measure,
quirin-source-analysis); no Lean, no provider gate.

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| Clippy (eight packages) | 0 | |
| Rust suite (`rust-suite` slot) | 0 | 264 units, 270 summaries, 3,358 passed, 0 failed, 5 ignored; 6,826.7 s slot wait behind another card, then 7.5 min |
| reevaluation probe / follow-up | 0 / 0 | |
| Python suite | 1 | 579 tests, 2 failures, both outside this diff and present at the base: `test_build_matrix_sections` (the stale matrix Lean-audit count, 408 committed against 410) and `test_gateway_registry` (`crates/qcore-c-gateway/src/gateway_rule_registry.rs` stale against its generator); this card touches neither the matrix, `crates/gateway-rules` nor the gateway |

A first attempt on `6a7a80c85` had a third Python failure of this card's own:
the wait-record test listed a temporary directory with `glob`, which rule 29's
tripwire refuses; `cffb3e4e1` checks the one path instead.

## 4. Follow-ups

- `MEASURE-INCREMENTAL-BASELINE-COMPOSITION` (amended): the findings and
  options above; open on a user decision.
- `SLOT-WAIT-DECLARED-BY-PLUGIN` (new): the host trusts the plugin's
  declared waits up to the limit; discharge by a wait the host verifies
  itself (the slot script polls with non-blocking `flock`, so a kernel waiter
  list does not show it) or a recorded decision that the limit is the bound.
