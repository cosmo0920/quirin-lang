# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-26, on merge commit `a6deb69f`. That
merge puts this card onto main `b4430a14`, after W44-INDIRECT-CHAIN-ACCESS. The run
used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang 21.1.8.
Figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | **101** | 5,432 passed / **1 failed** / 11 ignored, 290 summaries (see below) |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 179 audits |
| Python before regeneration | 1 | only the derived-facts check, on the Lean count |
| provider tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w44-indirect-chain-access/integration/artifacts` | 1 | by design, because this card moves the corpus |
| composition check v2 ([composition-check.txt](composition-check.txt)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 358 follow-ups; 330 tests |

**The one test failure.** `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
in `compiler/tests/cycle20_verifier_plugins.rs` is a process-budget row that
fails under load; the ledger tracks such rows as load-sensitive. This card touches
nothing on its path. It passed 3 of 3 isolated reruns
([vp04-isolated-reruns.txt](vp04-isolated-reruns.txt)). The exit code of 101 is
recorded as measured, and the reruns sit beside it rather than replacing it.

## Composition

The card's delta (measured against `w43-indexed-resource-collections/integration/artifacts`)
is exact on the combined tree:

* 55 leaves removed and 15 added;
* `mk_event_init` and `mk_event_backend` move Unknown → Safe;
* no overlap, no transitive effects, no unit moves.

Two numbers appear in the record:

* **Card baseline paths.** A first run of the integrator's finishing script named
  the collections card's own branch archive as the card baseline. This record
  uses the integration archive the card actually measured against, and both
  compose.
* **Uncontracted-call refusals.** The STATUS row is re-derived from this run.

## Hand-off

Validated reserved-name `static` header bodies (`__bswap_*`) are handed to
W44-LIBC-RESIDUALS (`W44-CALLEE-SCOPE-RESERVED-BODIES`), which resumes on this
main.

The combined run is archived as the new baseline in [artifacts/](artifacts/),
captured in the integrator worktree.
