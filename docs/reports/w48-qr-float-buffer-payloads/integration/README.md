# W48-QR-FLOAT-BUFFER-PAYLOADS: integration onto main

The card was gated at `d11179c9`. Main moved to `93789344` (W48-QRLIB-LINKAGE), and the two sides overlap in `crates/qcore-verification/src/driver.rs`, so the merge (`94f65627`) got the **full gate**.

Composition:
- `proof/QCore.lean`: an import pair; both kept.
- The grammar's float position table: the card's buffer and payload rows, and card 5's `.qrlib` spelling row.
- The ledger: merged structurally to **656**.

Gate results:

| Check | Result |
|---|---|
| Tests | 6,984 passed, 1 failed |
| The one failure, rerun alone | 13 / 13 |
| Lean | 252 audits |
| Provider tests | 129 / 0 / 1 |
| Every other step | 0 |

The one failure was the load-sensitive `execution_quota` fork-storm test.

The Python suite failed first on the stale Lean count; it passed after the matrix was regenerated. Exits are in `exits/`.
