# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-25, on merge commit `ed36b895`. It is
merged onto main `94502987`, after W44-BRANCH-CONDITION-CALLS,
W44-LEAK-PIN-AUDIT and W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE. Only records
conflicted:
* the ledger was merged per entry: 379 + 3 − 5 = **377**;
* the increment paragraph was prepended;
* the report index was regenerated;
* the `STATUS.md` corpus rows were re-derived from this arbiter.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | see [gate.json](gate.json) |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 185 audits (`AuditGnuBuiltinReader`, `AuditLiteralTailActual` added) |
| Python before matrix regeneration | 1 | only the derived-facts check, on the Lean count |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w44-assignment-destination-suffix-place/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check ([composition-check.txt](composition-check.txt), [explained-leaves.json](explained-leaves.json)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 377 follow-ups; 330 tests |

## Composition

**Card delta.** Against `w44-corpus-callee-summaries/integration/artifacts`, the
card removes 1,629 leaves and adds 6. It moves 3 functions and no units.

**Overlap: 169 leaves.** 169 of the card's removals were already gone on main.
They are mostly `call to uncontracted function `__bswap_*`` leaves on
`rd_kafka_buf_read_*` macro lines, whose sub-object stores
W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE refuses at the lexical gate before any
call is classified.

**Explained: 4 leaves.** On three such lines (in
`rd_kafka_mock_handle_ConsumerGroupHeartbeat`,
`rd_kafka_mock_handle_ShareGroupHeartbeat` and `rd_kafka_buf_read_NodeEndpoints`),
the card replaced the `__bswap_16` leaf with the call classifier's "resource
call with a formal embedded in an unmodeled expression". On the combined tree
the gate refuses the statement first, so that replacement never arises.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main `94502987` | combined |
|---|---|---|
| functions | 297 / 9 / 4,528 | **300 / 9 / 4,525** |
| units | 3 / 8 / 170 / 4 | 3 / 8 / 170 / 4 |
| leak | 19 | 19 |
| unknown_alias | 3,357 | 3,357 |
| unsupported | 45,343 | **43,885** |
| uncontracted-call leaves on Unknown functions (`__bswap_*`) | 9,703 (428) | 9,221 (0) |

* **Unknown → Safe:** `ut_put_be16`, `ut_put_be32` and `ut_put_be64`.
  * Their `__bswap_*` callees are now validated W4 summaries of glibc's static
    header bodies, admitted by linkage rather than a declared premise.
    `libc/byteswap.qcore` is dropped.
  * No violation candidates were added.

The combined run is archived as the new baseline under [artifacts/](artifacts/).
