# W48-QR-TASK-RECORD-CAPTURES: integration onto main

The card was gated at `bcdc13b9`. While it ran, main moved to `5d33dbd2`: CG08-CASTS-AND-TYPE-QUERIES and its merges landed, touching the C gateway and Lean only. It was merged as `integrate/w48-c3` `6625b88b`.

- **Code conflict:** one import pair in `proof/QCore.lean`; both imports are kept.
- **Ledger:** merged structurally to **638**.

Under the user's landing-race policy, this merge got a **targeted check**:

| Check | Result |
|---|---|
| fmt, clippy | 0 |
| Gateway tests | 2,080 passed |
| Scalar, record and task test binaries | 284 passed |
| `qcore-frontend` | 288 passed |
| `qcore-core` | 1,133 passed |
| Lean | 248 audits |
| Provider tests | 129 / 0 / 1 |
| Packages | 0 |

The full test run is the card's own gate: 6,833 passed, 0 failed. Exits are in `exits/`.
