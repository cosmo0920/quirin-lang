# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-27, on merge commit `80133806`
(`integrate/w46-external`, branch merge `25453cb7` + `e104ff92`), onto main
`25453cb7` after W45-SR5-SOUNDNESS and every wave-45 card. No records
conflicted in the merge itself: the 59 changed files are the card's own
(report, PREDICTION, arbiter artifacts, `w46_external_callee_contracts.rs`,
the nine `.qcore` contract additions and their `.lock` files, the ledger,
`STATUS.md`, `docs/agents/gateway.md` rule 10j, and the new
`docs/development/increments/wave-46.md`, whose paragraph the merge moves out
of `wave-45.md`). This gate helper additionally:
* archived the combined run as the new baseline under [artifacts/](artifacts/);
* re-derived the `STATUS.md` corpus rows and header provenance sentence from
  this arbiter;
* regenerated the report index and follow-up matrix (509 registered
  follow-ups, unchanged by this card).

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **6,021 passed / 0 failed / 11 ignored** over 336 summaries (runner totals: `test binaries: 322 units, 334 summaries; 6021 passed; 0 failed; 11 ignored`) |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | **210** audits |
| Python evidence-validator tests | 0 | 364 root tests |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 129 / 0 / 1 over 43 summaries; 15 package Python tests |
| release corpus run, arbiter | 0 | — |
| identity step 32 against `docs/reports/w46-external-callee-contracts/artifacts/arbiter` (the card's own AFTER) | 0 | `verdict_bearing_identical: true`; `diagnostics` artifact `byte_identical: true` |
| composition check ([composition-check.txt](composition-check.txt)) | 0 | **COMPOSES** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 509 follow-ups; `python rc=0`, 364 tests OK |

No load-sensitive row needed a rerun; every step passed on the first run.

## Composition

**Card delta** (against `docs/reports/w45-container-root-reach/integration/artifacts`,
which reproduces the card's own BEFORE exactly): the card removes 76 leaves
and adds 2, moving 7 functions and 0 units.

**Overlap: 0 leaves.** No earlier integration had already removed any of the
card's leaves.

**Explained: 0 leaves.** No transitive callers of a combined-only Safe
function lost an uncontracted-call leaf; nothing needed integrator
explanation.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | combined baseline before (`w45-container-root-reach`) | combined (this integration) |
|---|---|---|
| functions | 398 / 9 / 4,427 | **405 / 9 / 4,420** |
| units | 5 / 8 / 168 / 4 | 5 / 8 / 168 / 4 |
| leak | 34 | 34 |
| unknown_alias | 3,571 | **3,572** |
| unsupported | 41,314 | **41,239** |
| summary_conflict | 1 | 1 |

* **Unknown → Safe (7):** `pack_basic_header_finish`, `pack_bool`,
  `pack_bytes`, `pack_string`, `mk_sched_send_signal`, `mk_socket_create`,
  `rd_kafka_thread_cnt` — the nine new external-callee contracts
  (`fcntl` at `F_SETFD`, `mk_utils_libc_error`, `rd_atomic32_get`,
  `cfl_sds_len`, five mpack writer primitives) clear the last uncontracted-call
  blocker on each. No violation candidates were added; no corrections were
  made to any prior verdict.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
