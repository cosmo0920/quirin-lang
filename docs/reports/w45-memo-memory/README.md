# W45-MEMO-MEMORY: the member-release memo is removed

Card W45-MEMO-MEMORY, 2026-09-26, on base `675e7289` (local `main`, "Record the
w45-audit-checkpoints integration gate").

## Headline

W45-GATEWAY-MEMO-KEY made the per-thread memo of `member_releasing_functions`
(`crates/qcore-c-gateway/src/legacy_member_release.rs`) sound: an entry was
served only when every token text was equal, so the entry kept a copy of all
of them. W45-AUDIT-CHECKPOINTS then measured that this raised the 16-job corpus
peak by about 240 MiB (§13 of [its report](../w45-audit-checkpoints/README.md)).
This card takes the brief's option (a) and **removes the memo**. The set is
computed on every call.

On the corpus arbiter at 16 jobs the peak falls from **1,091 to 848 MiB**
(−243 MiB, −22 %). A checkpointed `quirin-audit current` at 16 jobs falls from
1,108 to 879 MiB. At 4 jobs the arbiter falls from 426 to 349 MiB. At 1 job the
change is 154 → 153 MiB. CPU time moves by less than its own scatter at every
job count. All 20 runs write the same five artifacts, byte for byte, for main
and for the change, and the diagnostics are byte-identical to the
W45-FB-BACKLOG-CUSTODY integration baseline.

## 1. Why option (a)

The brief ranks the options (a) remove, (b) scope to one unit and (c) share
the unit's tokens, and asks for the choice to rest on measurement.

* **Soundness is exact without a key.** There is no memo state, so no call can
  be answered from another stream. `QCore.MemoKey.run_eq_map` already proved
  the sound memo observationally equal to the uncached function. The new
  corollary `removing_the_memo_changes_no_answer` states the removal directly
  (§4).
* **The memo saved no time.** W45-GATEWAY-MEMO-KEY measured user CPU medians of
  1,063 s with the memo and 969 s without it at 8 jobs. This card's series
  (§3) shows the same: the change is within main's own scatter at 1, 4 and
  16 jobs.
* **So (b) and (c) were not built.** Option (a) comes first in the brief's
  order, and it meets the brief's condition: cache-off is as fast. Two
  observations remain. They were not measured. First, the stream this function
  reads is not always the unit's. `build_function_cfg_with` (`legacy_c.rs`)
  passes a vector rewritten for one function whenever `_Generic` resolution,
  the literal premises or the out-alias normalisation change the body. So a
  per-unit hoist of the answer would change its input, and a unit-scoped memo
  would still need the full-text key. Second, no caller holds an
  `Rc<[Token]>`, so (c) would change ownership along the CFG builders, which
  is not local to the memo.

## 2. The change

* `member_releasing_functions` is the computation itself: it returns a
  `BTreeSet<String>`, and `guard_unit_callees` still reads it lazily, once per
  CFG.
* Removed: the `thread_local!` `RELEASING`, `ReleasingEntry`,
  `releasing_digest`, the test hook `inject_releasing_entry` and the Cargo
  feature `member-release-memo-off`. No workspace member or script enabled the
  feature.
* Tests (`tests/unit/legacy_member_release.rs`): the two tests that drove the
  hook are removed with it, the forced collision and the control that the hook
  reached the memo. `interleaved_streams_always_get_their_own_answer` stays.
  It now also pins each stream's answer by content, as a guard against state
  reintroduced across streams. The module has 20 tests, and all pass.
* No rule text changes. No gateway constraint described the memo.

The frozen W45-GATEWAY-MEMO-KEY report keeps its claims.
Its mutation harness (`mutate_memo_key.py`) targets code that no longer
exists; it is evidence for that card's checkpoint only.

## 3. Memory and time

**Harness.** [`scripts/measure_variants.py`](scripts/measure_variants.py) runs
saved binaries, interleaved: round 1 of main, round 1 of the change, and so on.
It runs the arbiter's exact command and environment (`run_audit.py`), or the
checkpointed `quirin-audit current` of `run_current.py`. For each run it
records the following:

* the wall time;
* the child's user and system CPU time from `wait4`, Clang included;
* the tree peak: `ru_maxrss` from `wait4`, the value `/usr/bin/time -v`
  prints, which is the larger of the analyser's own peak and its largest
  child's;
* the analyser's own `VmHWM`, sampled every 50 ms.

The two peaks agree within 1.2 MiB in every run, so the analyser, not Clang,
sets the peak. [`scripts/series.sh`](scripts/series.sh) is the series.
[`scripts/summarise.py`](scripts/summarise.py) tabulates it and compares the
artifacts. The binaries were saved from this worktree, `main` at `675e7289`
(`87403308…`) and the change at `448ef784` (`3e442a59…`), and run from
worktree-private `measure/`. Per-run records are in [`evidence/`](evidence/).

| Route, jobs | Runs | Peak RSS main (MiB) | Peak RSS change (MiB) | Change | User CPU main (s) | User CPU change (s) | Wall main (s) | Wall change (s) |
|---|---|---|---|---|---|---|---|---|
| arbiter, 16 | 3 + 3 | **1,091** (1,075–1,095) | **848** (821–855) | **−243 (−22 %)** | 1,176 (1,145–1,197) | 1,147 (1,141–1,156) | 188 (142–241) | 195 (167–195) |
| checkpointed `current`, 16 | 2 + 2 | 1,108 (1,092, 1,124) | 879 (896, 862) | −229 (−21 %) | 1,084 (1,047, 1,121) | 1,137 (1,153, 1,121) | 242 (297, 187) | 208 (199, 216) |
| arbiter, 4 | 3 + 3 | 425.5 (416.5–425.9) | 349 (346–358) | −77 (−18 %) | 814 (717–929) | 880 (752–1,082) | 255 (187–295) | 232 (195–429) |
| arbiter, 1 | 2 + 2 | 154.3 (154.3, 154.3) | 153.1 (153.1, 153.0) | −1.2 | 618 (582, 655) | 632 (630, 633) | 656 (602, 711) | 661 (652, 669) |

Values are medians, with the range or the individual runs in parentheses. The
peak is the analyser's own `VmHWM`.

* **Memory.** The saving is consistent: every change run is below every main
  run at 4 and 16 jobs, in both routes. The saving grows with the number of
  worker threads: 1.2 MiB at 1 job, 77 MiB at 4 and 243 MiB at 16. That is
  about 15–19 MiB per worker at 4 and 16 jobs, against 1.2 MiB for one worker,
  so the copied texts alone do not account for it. What makes up the rest (for
  example allocator arenas held by long-lived entries in each thread) is not
  measured here. The 16-job figure matches W45-AUDIT-CHECKPOINTS's attribution
  (+238 MiB for the whole run, 934 → 1,172 MiB, measured once on another day).
  This card's absolute main figure, 1,091 MiB, is lower than that 1,172.
* **Time.** No consistent difference. The medians differ by −2.5 % (16 jobs),
  +5 % (checkpointed), +8 % (4 jobs) and +2 % (1 job). Each is smaller than
  main's own spread at that job count: 4 %, 7 %, 26 % and 12 %. The sign also
  changes between job counts.
* **Host load.** The host was shared with two other Opus cards. The load
  average after the runs ranged from 2 to 56. This card's own standing gate ran
  from 21:38 to 21:52, which overlapped arbiter change round 3 at 16 jobs and
  the first three checkpointed runs. The 1-job pairs ran at the lowest load
  (2–10). The CPU time includes Clang and so is not insensitive to load either.

## 4. Lean

`QCore.MemoKey` ([`proof/QCore/MemoKey.lean`](../../../proof/QCore/MemoKey.lean),
audited in `proof/AuditMemoKey.lean`, 8 → 9 theorems) gains
**`removing_the_memo_changes_no_answer`**:

```lean
theorem removing_the_memo_changes_no_answer (f : K → V) (digest : K → D)
    (ks : List K) (m : Option (Entry K V D)) (h : Filled f m) :
    ks.map f = run f digest ks m
```

Calling the uncached function on every key answers exactly what the memo
answered, from the empty memo or from any state the memo filled on earlier
keys (a worker thread's earlier units), for every digest. It depends on no
axioms. The module's text now records that its definitions model the replaced
code, and that the key premise of trust boundary 28 no longer arises because
there is no key. The other eight theorems are unchanged.

## 5. Identity

* **Main against the change.** In each of the four series, every run's five
  artifacts are byte-identical to the series' first main run, and so is every
  `audit.log`. For `quirin-audit current` the log's Clang preparation
  durations (`N us`) are masked. The result is in
  [`evidence/summary.json`](evidence/summary.json):
  `all_identical_and_exit_0: true`, and all 20 runs exit 0.
* **Against the named baseline.** `baseline_identity.py … --root-alias
  <worktree>=…/hungry-thompson-1740da` against
  `docs/reports/w45-fb-backlog-custody/integration/artifacts/` exits 0 for the
  first main and change run at 16, 4 and 1 jobs. Diagnostics, chunk-contracts
  and the Monkey source report are byte-identical in all six. The contracts
  and source-effects sidecars differ equally for main and the change
  (identical digests), as W45-AUDIT-CHECKPOINTS §13 recorded for `main`. The
  files are `evidence/baseline-identity-*.json`.
* **The checkpointed route.** Against W45-AUDIT-CHECKPOINTS's
  `artifacts/current-whole-merged/`, the diagnostics differ in exactly 558
  leaves, the same for main and the change:
  `compile_configuration.inventory_sha256` and the two
  `resource_contract_sources` digests of each of the 186 units
  (`evidence/ck-j16-*-leaf-classes.json`,
  [`scripts/leaf_classes.py`](scripts/leaf_classes.py)). Main and the change
  are byte-identical to each other. The difference is not this card's, and it
  is not attributed further.

## 6. Gate

The gate ran at `78e3f2b0` (code and Lean) with `CARGO_INCREMENTAL=0`. Every
exit was captured to a file ([`gate.json`](gate.json); logs in the worktree's
`private/memo-memory/gate1/`).

* Submodule 0.
* **Rust 0: 309 units, 319 summaries, 5,919 passed / 0 failed / 11 ignored**
  (`private/test-binaries/20260926T123837Z-3287785/`).
* `cargo clean --profile dev` 0.
* fmt 0, clippy 0.
* Both reevaluation examples 0.
* **Lean 0: 204 audits.**
* Python 0 (358 tests).
* Docs index `--check` 0.

The records commit that follows changes only documentation, evidence and the
matrix's follow-up text. At that tip, fmt, the Python suite and the docs index
check were rerun (`gate.json`, `records_tip`). The provider gate did not run,
because `providers/` is untouched.

## 7. Ledger

* `W45-CHECKPOINT-JOBS-MEMORY` stays open. Its text now gives this card's
  16-job figures, 848 MiB for the whole run and 879 MiB checkpointed. The
  scaling with `--jobs` that it tracks is unchanged in kind.
* No follow-up is registered. What makes up the per-worker saving beyond the
  copied texts is disclosed in §3, not tracked, because no rule depends on it.
