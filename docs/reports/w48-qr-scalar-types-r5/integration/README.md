# W48-QR-SCALAR-TYPES-R5: integration onto main

The card was gated at `6883623c`. While it ran, main moved to `c0ccc66a`: CG04-CONDITION-ACQUISITION landed, touching the C gateway only. It was merged as `integrate/w48-r5` `4a785289`; the only code both sides touched was `proof/QCore.lean`, which auto-merged. The ledger was merged structurally to **630**.

Under the user's landing-race policy (2026-09-30), this merge got a **targeted check, not a full gate**:

| Check | Result |
|---|---|
| fmt, workspace clippy `--locked` | 0 |
| Gateway tests | 2,003 passed, 0 failed |
| Scalar, loop and buffer test binaries | 167 passed |
| Lean | 237 audits |
| Provider tests `--locked` | 129 / 0 / 1 |
| Provider clippy, package build and 5 verifies | 0 |

The full test run is the card's own gate at its base: 6,681 passed, 0 failed. Exits are in `exits/`.
