# Second merged gate: 508407913

The full gate ran outside the sandbox through `scripts/gate_scope.py --base
f774f54dee6c64c6449a4fca4d83dcb0e6b30d39 --full` on
508407913f1e8f928808ae67b5e962c32478c8b3. Its private evidence is
`private/quant-merge-20261027/full-gate-2/run/gate.json`; commands, exits,
counts, slot receipts and per-step logs are retained there. Clang was recorded
in this checkout beside this gate. This attempt failed and is not replaced by
later evidence.

Root Rust: exit 101, 571 units / 590 summaries, 8831 passed, 1 failed,
15 ignored, 24 filtered. The pin comparison helper unwrapped an omitted empty
section. Python: exit 1, 546 tests, one failure: the overall 144-module pin was
correct but the quirin-owned subtotal still expected 55 instead of 56 after
CFL atomic was added. Provider tests: exit 101, 6 passed / 1 failed before
Cargo stopped; a broad negative-control replacement also changed the typed
old-field entry domain, so derivation correctly refused the unsupported entry.

Submodules, format, Clippy, both reevaluation probes, all 388 Lean audits,
provider package build and both package verifications, provider format and
provider Clippy exited 0. Provider tests were incomplete, not seven total tests.

The repair preserves exact omitted-section comparison, targets the complete
normal condition in the provider negative control and asserts that it changed,
and deliberately updates the per-provider corpus pin. No production semantics
or source bindings change. A fresh full gate follows on the repair commit.
