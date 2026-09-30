# W48-QR-SCALAR-TYPES-R6: integration onto main

The card was gated at `8a378bf3`. While it ran, main moved to `3e69322c`: CG06-VALUE-CONNECTIVES-AND-CONDITIONAL landed, touching the C gateway only. It was merged as `integrate/w48-r6` `171d363e`; the only code both sides touched was `proof/QCore.lean`, which auto-merged. The ledger was merged structurally to **625**.

Under the user's landing-race policy, this merge got a **targeted check**:

| Check | Result |
|---|---|
| fmt, clippy | 0 |
| Gateway tests | 2,029 passed, 0 failed |
| The card's and earlier scalar test binaries | 194 passed |
| Lean | 243 audits |
| Provider tests | 129 / 0 / 1 |
| Provider clippy, packages | 0 |

The full test run is the card's own gate: 6,745 passed, 0 failed. Exits are in `exits/`.
