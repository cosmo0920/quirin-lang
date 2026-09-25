# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-26, on merge commit `7de5edf6`. It
is merged onto main `400063c5`, after W45-RETURN-EXPRESSION-FAMILIES and
W45-CALLEE-SUMMARY-CASCADE. 58 files changed, 4,692 insertions(+), 12
deletions(-). Conflicts, per the merge commit message:
* hooks from all three wave-45 cards are kept in `legacy_c.rs`;
* the census sink is listed once;
* the card's paragraph closes constraint 5i before 5j/5k;
* `STATUS.md` is left at main's rows for the finisher;
* the acceptance matrix is merged three-way (follow-ups 405).

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,720 passed / 0 failed / 11 ignored**, 304 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | **193 Lean audits** |
| Python before matrix regeneration | 1 | only `test_the_derived_facts_are_recomputed_rather_than_carried` (stale derived facts, expected before regeneration) |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | **124 / 0 / 0**, 39 summaries; provider Python clean |
| release corpus run, arbiter | 0 | — |
| byte-identity against `w45-callee-summary-cascade/integration/artifacts` | 1 | by design; this card moves the corpus |
| composition check ([compose_check.py](compose_check.py), [composition-check.txt](composition-check.txt)) | 0 | **COMPOSES** |
| `build_matrix.py --gate`, then Python (after regeneration) | 0 / 0 | **351** root Python tests; 15 package Python tests |

## Composition

**Card delta.** Against `w44-collection-allocation/integration/artifacts` (the
card's own BEFORE), the card removes 230 leaves and adds 0. It moves 1
function (`destroy_id`, Unknown → Safe) and no units.

**Overlap: 0 leaves.** None of the card's removed leaves were already absent
from the preceding combined baseline
(`w45-callee-summary-cascade/integration/artifacts`).

**Explained: 0 leaves.** No leaves needed integrator explanation; the card's
delta composed onto the combined tree exactly, function for function.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | preceding combined baseline | combined (this integration) |
|---|---|---|
| functions | 389 / 12 / 4,433 | **390 / 12 / 4,432** |
| units | 4 / 10 / 167 / 4 | 4 / 10 / 167 / 4 |
| leak | 37 | 37 |
| unknown_alias | 3,568 | 3,568 |
| unsupported | 42,924 | **42,694** |
| summary_conflict | 1 | 1 |

* **Unknown → Safe:** `destroy_id` (stores into a by-value record parameter,
  relative to the card's sub-object store rule).
* No unit, leak, or violation changed.

The combined run is archived as the new baseline under [artifacts/](artifacts/).

## Corrections

No function is Safe in the preceding combined baseline
(`w45-callee-summary-cascade/integration/artifacts`) and not Safe in this
combined arbiter.
