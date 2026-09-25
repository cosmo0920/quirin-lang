# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-25, on merge commit `6ae03dd0`. It
is merged onto main `eca27447`, after W44-LIBC-RESIDUALS,
W44-AGENT-GUIDANCE-OPUS55 and W44-AUDIT-PARALLEL-UNITS. Only records
conflicted:
* the ledger was merged per entry: 378 + 1 = **379**;
* the increment paragraph was prepended;
* the report index was regenerated.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,535 passed / 0 failed / 11 ignored** over 295 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 186 audits (`AuditGenericSelection` added) |
| Python before matrix regeneration | 1 | only the derived-facts check, on the Lean count |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 package Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w44-libc-residuals/integration/artifacts` | 1 | by design; this card moves the corpus |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 379 follow-ups; 333 root Python tests |

## Composition

**Card delta.** Against `w44-libc-residuals/integration/artifacts` (the card's
own base), the card removes 37 leaves and adds 34. It moves 1 function
(`mk_string_char_search`, Unknown → Safe) and no units.

**Overlap: 0 leaves.** None of the card's removals were already gone on the
combined tree; the card's base and the combined baseline (`PREV`) are the same
artifacts, so there is nothing for an earlier integration to have already
retired.

**Complementary moves: none.** No function's last blocker was cleared only in
combination with another card.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | combined before (`w44-libc-residuals`) | combined after |
|---|---|---|
| functions | 300 / 9 / 4,525 | **301 / 9 / 4,524** |
| units | 3 / 8 / 170 / 4 | 3 / 8 / 170 / 4 |
| leak | 19 | **34** |
| unknown_alias | 3,357 | 3,370 |
| unsupported | 43,885 | **43,854** |
| summary_conflict | 1 | 1 |

* **Unknown → Safe:** `mk_string_char_search`. Its only leaf was glibc's C23
  `_Generic` selection around `memchr`; the selection is selection-invariant
  (every association is the same expression up to pointer casts), so it
  lowers to that expression, decided relative to the declared `memchr`
  premise.
* **Leak 19 → 34:** lowering the `_Generic`-wrapped `strchr` selections in
  `ut_sasl_oauthbearer_oidc_assertion_create`'s `RD_UT_ASSERT` conditions let
  the desugaring model them; the 15 added leaves are the same two
  `rd_strndup` objects reported again at later failure-path exits of that
  already-provisional-Unsafe unit test. All 15 read as class (a) by the
  card's reading (the same class the leak audit gave the function's 9 earlier
  leaves), never an upstream defect claim.
* No violation candidates were added beyond the above.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
