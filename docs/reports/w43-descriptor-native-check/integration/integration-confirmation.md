# Integration confirmation (combined tree)

The integrator re-ran the gates on 2026-09-26, on merge commit `e94f1607`. That
merge puts this card onto main `08e9a76c`, after W43-INDEXED-RESOURCE-COLLECTIONS.
The card had already merged main `d2dd61c1` for Phase B. The run used the
integrator's own worktree with `CARGO_INCREMENTAL=0` and Clang 21.1.8. Figures are
in [gate.json](gate.json).

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,390 passed / 0 failed / 11 ignored**, 289 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | **177** audits |
| Python before regeneration | 1 | only the derived-facts check, on the Lean count |
| provider tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 Python tests |
| release corpus run; `baseline_identity.py` against `w43-indexed-resource-collections/integration/artifacts` | 0 / 0 | **all five artifacts byte-identical** |
| composition check (zero expected delta) | 0 | composes |
| `build_matrix.py --gate`, then Python | 0 / 0 | 345 follow-ups; 330 tests |

## What changed

**Ledger.** The ledger was merged entry by entry: 336 on main plus this card's 9,
giving **345**.

**Contract modules.** `corpus.contract_modules` is 111. This card adds
`library/descriptor_native_observation.qcore`, which is not in the resource
selection. That is why the contracts artifact is unchanged.

**Corpus.** The card is verdict-neutral, and the corpus baseline stays
`w43-indexed-resource-collections/integration/artifacts/`. No new archive was
written.

## Findings carried

The card's Phase A and Phase B findings are native observations, each scoped as
the card's report states. The integrator did not validate them as upstream
defects, and nothing was sent upstream. Whether to report them is the user's
decision. The findings are:

* `W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE`
* `W43-DESC-FLB-SCHED-TIMER-DOUBLE-CLOSE`
* `W43-DESC-FLB-STORAGE-SCAN-DOUBLE-FREE`
* `W43-DESC-FLB-CONFIG-DATA-PIPE-ZERO-GUARD`
* `W43-DESC-MONKEY-BIND-FAILURE-LEAK`
* `W43-DESC-FLB-PROXY-HANDLE-UNRELEASED`
* `W43-DESC-CTR-RANDOM-ZERO-DESCRIPTOR`, the native witness for the static
  provisional Unsafe `ctr_random_get`

Phase B bound 3,149 executions of 18 declared callees to their loaded contracts
with **0 contradictions**.
