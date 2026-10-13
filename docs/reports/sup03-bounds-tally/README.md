# SUP03-BOUNDS-TALLY-ON-INTEGRATED-MAIN: the bounds tally on the integrated main

Question ([clang-gap queue](../../plans/clang-gap-cards/README.md)): what is
the bounds property's tally on the integrated main? This is a measurement and
records card. No analyzer, gateway, contract, Lean or provider file changed.

**Answer, main `e0b92ece6`:** bounds **162 Safe / 0 Pending / 0 Unsafe / 4,759
Unknown** over 4,921 definitions, unchanged. Combined custody + bounds is
**113 Safe / 4,799 Unknown / 9 Unsafe**, against the recorded 111 / 4,801 / 9.
The +2 are `cmt_initialize` and `unittest_telemetry_decode`. Both were already
bounds Safe, and W45-CALLEE-CASCADE-RESIDUAL round 3 made them custody Safe.

## 1. Runs

Each run is the pinned corpus arbiter (`66910c10`, `--jobs 4`) with
`QCORE_BOUNDS_PROPERTY` set, run under the shared heavy lock. The binary's
digest is checked against `run.json`'s `binary_sha256`. Scripts are in
[`scripts/`](scripts/); exits, wall times and digests are in
[artifacts/digests.txt](artifacts/digests.txt).

| Run | Tree | Binary | Exit | Wall | Rows | Custody report |
|---|---|---|---|---|---|---|
| `a1` (the measurement) | this checkout, built at `e0b92ece6`, run at `f60cb6a2d` (the prediction commit; docs only) | `2b338d53…` | 0 | 412.2 s | `775de9be…` | `bdf5f1c8…` |
| `base2` (attribution) | main `43732604c` (CG16 transport landed), exported with `git archive` | `db28e0fc…` | 0 | 405.6 s | `79a90176…` | `ab8f5f14…` |
| `base` (failed attempt, kept) | as `base2` | `b21ca758…` | **1** | 405.7 s | none | none |

The custody report digests equal the arbiter identities on record: main's
`bdf5f1c8…` (QR-PRINT-ROUTE-MATCHES-DECLARATION) and CG16 transport's
`ab8f5f14…`. `base2`'s rows equal CG16 transport's published AFTER
`79a90176…` byte for byte. That is the comparison the prediction expected to
be unavailable: the old rows are gone, so they were regenerated.
`base` failed with `Pin(SourceRevisionMismatch { pin: "libevent", … })`.
`git archive` leaves the submodule empty, so the libevent check read the
enclosing checkout's HEAD ([artifacts/base-attempt-failure.txt](artifacts/base-attempt-failure.txt)).
For `base2`, [`measure-base.sh`](scripts/measure-base.sh) links this
checkout's submodule, after checking that both revisions pin
`d82464a2…`. The `base` attempt is not used as evidence.

Commands (from the worktree root; each writes its exit to a file):

```sh
bash docs/reports/sup03-bounds-tally/scripts/build.sh                     # submodule 0, build 0
bash docs/reports/sup03-bounds-tally/scripts/measure-bounds.sh a1         # 0
bash docs/reports/sup03-bounds-tally/scripts/measure-base.sh 43732604c base   # export 0 0, build 0, arbiter 1 (discarded)
bash docs/reports/sup03-bounds-tally/scripts/measure-base.sh 43732604c base2  # export 0 0, build 0, arbiter 0
python3 docs/reports/sup03-bounds-tally/scripts/summarize.py a1 docs/reports/sup03-bounds-tally/artifacts/bounds-summary.json   # 0
python3 docs/reports/sup03-bounds-tally/scripts/bounds_compare.py base2 a1 docs/reports/sup03-bounds-tally/artifacts/bounds-compare.json   # 0
```

Private logs: `private/sup03/` in this worktree (`build.log`, `submodule.log`,
`a1-bounds.{log,stderr,jsonl,summary.json,contracts.json}`, `a1-bounds/`,
`base-*` and `base2-*`, and the exported trees `src-base/` and `src-base2/`).
Both stderr files are empty.

## 2. Prediction against outcome

Prediction: [PREDICTION.md](PREDICTION.md), committed in `f60cb6a2d` before
any run. `a1-bounds.head` records `f60cb6a2d`.

| Figure | STATUS / recorded | Predicted | Measured | Held? |
|---|---|---|---|---|
| Bounds | 162 / 0 / 0 / 4,759 of 4,921 | same | **162 / 0 / 0 / 4,759** of 4,921 | held; equals STATUS |
| Per-function reason codes | 4,650 / 817 / 214 / 87 / 8 | same | same | held |
| Bounds Unsafe, refutation table | none, empty | same | none, empty | held |
| Combined | 111 / 4,801 / 9 | 111 / 4,801 / 9 (low confidence; any move upward) | **113 / 4,799 / 9** | **not held**: +2, upward as stated |
| Custody column | — | 935 / 9 / 3,890 + 87 absent | 935 / 9 / 3,890 + 87 absent | held |
| Obligations by kind and outcome | QUANT-INDEX-CHECKS' counts | same except reason text | call-precondition / missing-extent 91,680 → **91,678**, all else equal | not held as worded; see below |

* **Combined.** The prediction assumed that none of the 51 functions that
  were custody Unknown and bounds Safe had become custody Safe. Two had. The
  rows show `cmt_initialize` (`cmetrics.c`) and `unittest_telemetry_decode`
  (`rdkafka_telemetry_decode.c`) moving custody Unknown → Safe between
  `base2` and `a1`, with bounds Safe in both runs.
  W45-CALLEE-CASCADE-RESIDUAL round 3 attributes both moves:
  `cmt_initialize` through `cmt_atomic_initialize`'s declaration, and
  `unittest_telemetry_decode` through the validated `unit_test_telemetry`
  summaries. That card's report counts only custody moves, so it did not
  record their effect on the combined row.
* **The prediction's stated reason was also wrong in one part.** It said
  custody had no Safe → non-Safe move since CG16 transport. The rows show
  two: `rd_kafka_topic_partition_list_sort_by_topic` and `_by_topic_id`.
  CG22-CALLBACK-REGISTRY-CONTRACT named these as corrections in its
  report. Both are bounds Unknown, so the combined row does not move.
  Custody's +54 is therefore +56 / −2.
* **Obligations.** The −2 call obligations come from CG16 transport itself,
  not from anything after it. `cio_file_st_get_hash`'s declaration states no
  precondition, so its 2 calls are no longer obligations; that report records
  them. `base2` → `a1` shows no difference in any judgement.

## 3. What changed since the last bounds measurement (`base2` → `a1`)

[bounds-compare.json](artifacts/bounds-compare.json):

* **0** rows differ in anything other than their custody and combined
  columns. Verdicts, reasons, judgements, contract dependencies and identity
  are the same, and every row has `contracts_sha256` `bd174836…`, CG16's
  bounds environment.
* **58** rows move only in the custody column: 56 Unknown → Safe and 2 Safe →
  Unknown (the CG22 corrections above). 2 of the 56 also move combined
  Unknown → Safe.
* The selection record now lists **98 declarations from 6 modules** (93
  null-property declarations pinned by the NULL-* cards), against 5 from 2 at
  `base2` (audit log line 6:
  [a1](artifacts/a1-audit-bounds-lines.txt),
  [base2](artifacts/base2-audit-bounds-lines.txt)). The bounds consumer still
  reads 5. Body evidence is unchanged: 5 declarations, 3 validated and 2 partly
  validated. All 162 Safe rows rest on no declaration. This matches the
  prediction's reason that `for_bounds()` removes the null clauses.

## 4. Unknown bounds rows by cause

From the run's own `.summary.json` and its rows
([a1-bounds.summary.json](artifacts/a1-bounds.summary.json),
[bounds-summary.json](artifacts/bounds-summary.json)). A function counts once
for each code it carries, so the codes sum to more than 4,759.

| Reason code | Unknown functions |
|---|---|
| `bounds-missing-extent` | 4,650 |
| `bounds-unsupported-form` | 817 |
| `bounds-unsupported-range` | 214 |
| `bounds-import-failure` | 87 |
| `bounds-range-not-contained` | 8 |

By exact code set: `bounds-missing-extent` alone 3,715; with
`unsupported-form` 730; with `unsupported-range` 124; with both 73; with
`range-not-contained` 3, with it and `unsupported-range` 2, and with all four
3; `bounds-import-failure` alone 87; `unsupported-form` alone 10,
`unsupported-range` alone 11, and those two together 1. In 4,650 of the 4,759
Unknown rows, a missing extent is at least one cause. 109 rows fail on another
cause only.

Obligations by kind and outcome (`bounds-summary.json`): call-precondition
91,678 missing-extent and 100 excluded; member-access 87,469 missing-extent and
2,930 in-bounds; dereference 2,903 missing-extent and 555 in-bounds;
element-access 1,958 missing-extent, 606 unsupported-range and 1,796 in-bounds;
formation 1,673 missing-extent and 355 in-bounds; same-object 372
missing-extent. The file has the full table.

Custody × bounds: Safe × Safe 113, Safe × Unknown 822, Unknown × Safe 49,
Unknown × Unknown 3,841, Unsafe × Unknown 9, absent × Unknown 87.

## 5. Follow-ups

None registered. SUP03 is retired (`named_followups` 763 → 762): its
discharge, a re-measure on the current main with attribution and STATUS
updated, holds. The combined row's +2 is a correct consequence of custody
moves already attributed, not a defect. One process note, recorded here and
not registered: a custody card's prediction does not cover the combined row,
so combined moves only surface when a bounds measurement is taken.

## 6. Gate

Records-only card: the standing gate does not apply to doc and plan changes.
The measurement's own checks are listed in §1. After the ledger edit,
`python3 -m unittest discover -s scripts/tests` was run because it reads the
committed matrix: exit 0, 365 tests OK (`private/sup03/unittest.log`).
`python3 scripts/build_docs_index.py` exited 0 and its `--check` exited 0.
