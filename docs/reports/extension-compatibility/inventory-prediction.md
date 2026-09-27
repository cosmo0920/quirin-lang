# EXT-01 inventory prediction (before implementation)

Base: `64ddac3f60fe9b848dd789e895d67b809cce93e2`.

The tracked suffix census should contain 621 inputs: 616 `.qcore` and 5 `.qcorelib`. Of the `.qcore` inputs, 182 are frozen under `docs/reports/` and remain at their historical paths. Parsed `module contract;` files should propose `.qv`; parsed executable and library files should propose `.qr`; `.qcorelib` should propose `.qrlib`. Library-mode sources are expected in archived multi-file probes, and their consumer context must be recorded. Full parse failures are expected for deliberately malformed fixtures and remain unresolved until their intended mode is established. No source bytes, verdicts, or corpus counts should change in this inventory-only card.

This is a predicted direction, not a claim of parsed-mode counts. The measured mapping and exceptions will be added after the Rust classifier runs.
