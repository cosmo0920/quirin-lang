# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-26, on merge commit `5b7942bb`. It
is merged onto main `9d3ab13c`, after all of wave 44 (through
W44-CONTRACTS-ONLY-NO-BUILD). Records were merged, not conflicted:
* the acceptance matrix (ledger) was merged three-way: follow-ups now 395
  (+2 W45 return follow-ups, −2 retired W43 ones), and rule 5j's corpus row
  updated to the combined figures;
* rule 5j was added to `docs/agents/gateway.md` and the rule router
  (`docs/agents/README.md`);
* the increment paragraph was appended to `docs/development/increments/wave-45.md`;
* the report index (`docs/reports/INDEX.md`) was regenerated;
* the `STATUS.md` corpus rows and header provenance sentence were re-derived
  from this arbiter.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,676 passed / 0 failed / 11 ignored** over 303 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 191 audits |
| Python before matrix regeneration | 1 | only the derived-facts check, on the Lean count (190 committed vs 191 produced) |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 15 package Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` (32-identity) against `w44-collection-allocation/integration/artifacts` | 1 | by design; this card moves the corpus |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 351 root Python tests |

## Composition

**Card delta.** Against `w44-collection-allocation/integration/artifacts` (the
card's own BEFORE, reproduced byte for byte), the card removes 405 leaves and
adds 40. It moves 76 functions and no units.

**Overlap: 0 leaves.** No removal was already gone on main; nothing else moved
the corpus since the baseline.

**Explained / transitive: 0.** No combined-only leaves needed hand
attribution, and no caller gained a transitive "call to uncontracted
function" clearing from a newly summarized callee.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main `9d3ab13c` | combined |
|---|---|---|
| functions | 306 / 12 / 4,516 | **382 / 12 / 4,440** |
| units | 3 / 10 / 168 / 4 | 3 / 10 / 168 / 4 |
| leak | 37 | 37 |
| unknown_alias | 3,507 | 3,517 |
| unsupported | 43,590 | 43,215 |
| summary_conflict | 1 | 1 |

* **Unknown → Safe: 76 functions**, matching the card's own 306→382 move
  exactly. Per `STATUS.md`'s "Projected functions" row: 72 blocked only by
  the return refusal (69 scalar-result reads, such as the `cJSON_Is*`,
  `rd_kafka_AclBinding_*` and `rd_kafka_error_*` accessors, and 3 casts of a
  whole call, including `XXH32_createState` and `XXH64_createState` escaping
  their fresh site) and 4 callers of a newly summarized accessor. None moved
  to provisional Unsafe; leak stays 37.
* No unit changed status.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
