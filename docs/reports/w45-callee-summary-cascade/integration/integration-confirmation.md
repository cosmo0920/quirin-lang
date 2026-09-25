# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-26, on merge commit `7546ff88`. It is
merged onto main `9a380045`, after W45-RETURN-EXPRESSION-FAMILIES. The merge was
reworked before landing: the first version of gateway constraint 5k certified a
double member release Safe; the integrator re-ran the counterexamples on this
combined tree and all are Unknown or Unsafe, while the destructor shape stays
Safe. Records conflicted:
* both cards' `legacy_c.rs` hooks were kept; the census sink is listed once;
  5k follows 5j;
* `STATUS.md` rows were left at main's figures for the integration finisher
  (this report);
* the acceptance matrix was merged three-way (399 named follow-ups);
* the report index was regenerated.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,695 passed / 0 failed / 11 ignored** |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 192 audits |
| Python before matrix regeneration | 1 | only the derived-facts check, on the Lean count |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0; 15 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w45-return-expression-families/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 399 follow-ups; 351 tests |

## Composition

**Card delta.** Against `w44-collection-allocation/integration/artifacts` (the
card's own base), the card removes 295 leaves and adds 55. It moves 7 functions
and 1 unit.

**Overlap: none.** All of the card's removals were still present on the
combined tree; none were already withdrawn by an earlier integration.

**Transitive: none.** No caller lost an uncontracted-call leaf through a
callee this card alone made Safe on the combined tree beyond its own moves.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main `9a380045` | combined |
|---|---|---|
| functions | 382 / 12 / 4,440 | **389 / 12 / 4,433** |
| units | 3 / 10 / 168 / 4 | **4 / 10 / 167 / 4** |
| leak | 37 | 37 |
| unknown_alias | 3,517 | **3,568** |
| unsupported | 43,215 | **42,924** |
| uncontracted-call leaves on Unknown functions (sole blocker) | 9,155 (419) | 9,149 (418) |

* **Unknown → Safe (functions):** `cio_error_set`, `cmt_histogram_buckets_destroy`,
  `mk_config_sanity_check`, `rd_kafka_ConsumerGroupListing_destroy`,
  `rd_kafka_ConsumerGroupListing_free`, `rd_kafka_consumer_group_metadata_destroy`,
  `rd_kafka_q_consumer_propagate`.
  * Each validates under gateway constraint 5k: a pointer member is admitted
    at a nullable release only under lexical premises P1-P4 (an uncopied
    formal, one release of the place, no enclosing loop or goto, and a later
    release of the formal on every path), relative to the declared release
    premises the destructor bodies call.
  * The aggregate null-test repair (putting the typed base read before every
    `BranchOnNull` that dereferences) moved no function out of `Safe` on this
    corpus.
* **Unknown → Safe (units):** `chunkio/cio_error.c`, whose last Unknown
  function's only remaining leaf was a pointer member null test now decided
  by the same rule.
* No violation candidates were added.

**Corrections.** No function is `Safe` in the PREV baseline
(`w45-return-expression-families/integration/artifacts`) and not `Safe` in
the combined arbiter. The aggregate-null-test fix and the tightened member
release therefore withdrew nothing that PREV had certified; there are no
corrections to report.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
