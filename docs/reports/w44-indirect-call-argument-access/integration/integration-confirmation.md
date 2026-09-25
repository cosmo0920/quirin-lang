# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-25, on merge commit `66ce7c7f`. It
is merged onto main `64e71955`, after W44-C11-GENERIC-SELECTION and
W44-TEST-SUITE-WALL-TIME. The card merged main `70ce6b44` itself (re-pinned
the suffix-hole test to Unknown; its three exposed-leak fixtures are now
Unknown with no leak under the refused hand-on rule). Only records
conflicted at the merge:
* the ledger was merged per entry: 382 + 8 − 1 = **389**;
* the increment paragraph was prepended;
* the report index was regenerated;
* the `STATUS.md` corpus rows were re-derived from this arbiter.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,572 passed / 0 failed / 11 ignored** over 296 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 187 audits |
| Python before matrix regeneration | 1 | only `test_the_derived_facts_are_recomputed_rather_than_carried`, on the Lean count (186 vs 187) |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` (`32-identity`) against `w44-c11-generic-selection/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 389 follow-ups; 349 tests |

## Composition

**Card delta.** Against `w44-libc-residuals/integration/artifacts` (the card's
own BEFORE), the card removes 195 leaves and adds 38. It moves 2 functions and
1 unit.

**Overlap: none.** No removed leaf was already gone on the combined tree.

**Explained: none.** No leaf needed integrator explanation; the combined
run's delta equals the card's own delta exactly.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | combined baseline before (`w44-c11-generic-selection`) | combined after (this card) |
|---|---|---|
| functions | 301 / 9 / 4,524 | **302 / 10 / 4,522** |
| units | 3 / 8 / 170 / 4 | **3 / 9 / 169 / 4** |
| leak | 34 | **35** |
| unknown_alias | 3,370 | **3,392** |
| unsupported | 43,854 | **43,674** |
| summary_conflict | 1 | 1 |

* **Unknown → Safe:** `cio_file_hash_print`, as on the card's own base.
* **Unknown → provisional Unsafe:** `mk_rconf_read` (unit
  `mk_core/mk_rconf.c`). Its one leak is class (c), a premise gap already
  registered as `MK-LIST-ADD-BORROWED-PUBLICATION`: the rule now models
  `mk_list_add(&file->_head, &conf->includes)`, so the refused hand-on rule
  no longer withdraws it. This is a provisional finding, not an upstream
  defect claim.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
