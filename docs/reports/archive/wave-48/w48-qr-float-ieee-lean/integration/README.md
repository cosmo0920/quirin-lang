# W48-QR-FLOAT-IEEE-LEAN-MODEL: integration onto main

The card was gated at `d11179c9`. It was merged onto main `f7a4c4bb` (card 5 and W48-QR-FLOAT-BUFFER-PAYLOADS) as `6ed02022`.

Composition:
- **Code:** the card changes only Lean and one Rust test, and the only shared code file is the `proof/QCore.lean` import list, where both sides' imports are kept.
- **Spec §L14.7:** the card's narrowed item 13, then main's items 14–19 and §L14.8.
- **Ledger:** merged structurally to **657**.

This merge got a **targeted check**:

| Check | Result |
|---|---|
| fmt, clippy | 0 |
| `float_ieee_vectors` | 14 passed |
| `float_types` | 3 passed |
| `float_buffer_payloads` | 13 passed |
| Lean | 253 audits |

The workspace and provider figures are carried from the previous full gate. Exits are in `exits/`.
