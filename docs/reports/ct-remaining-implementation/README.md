# Remaining CT implementation integration

Integration record, 2026-09-19. **Validation in progress; not a completed gate.**

## Result and boundaries

Six GPT-5.6 Sol medium agents worked in two waves of three: implementation
checkpoints followed by independent completion reviews in the same isolated
card worktrees. The integration branch starts from `b3ec447b`, incorporates
`7d2c1fa6`, and preserves that main snapshot while another task updates main.

| Card | Implemented question | Evidence boundary |
|---|---|---|
| [CT24](../ct24-cleanup-task-composition/README.md) | Does cleanup release an owner while its child loan remains live? | The actual task ledger refuses; after-join and independent-owner controls admit. Success-arm credit evidence is conditional. |
| [CT10](../ct10-source-layout/README.md) | Are simultaneous field writes independent under a declared layout? | Shared bit-field units refuse Q661. Property-only successful spawn/join window; no task-lifetime proof or C emission. |
| [CT41](../ct41-sync-model/README.md) | Does a successful release/acquire observation order a payload write/read? | Dedicated bounded C API with declared invocation counts. The original CT41 case remains unmapped; plain and volatile both lack synchronization. |

Model proofs, case-bound proof stages and native observations remain distinct.
These changes establish no Fluent Bit production verdict. The
[production queue](../../plans/fluent-bit-system-coverage/README.md) now names
candidate transfers and their missing obligations; its source census stays first.

## Review corrections

CT24 preserves the original after-all-joins check for direct task-scope regions;
only the exact parsed spawn-success arm gains the new placement. Cleanup is
bound to its real resource and source position, and its Lean theorem now binds
the cleanup trace to the same singleton action.

CT10's certificate and CLI explicitly carry the successful-window assumption.
Its case credits footprint lowering/checking, not the scoped-task ledger. The
existing footprint checker and shared-memory-location refusal are unchanged.
The first full gate exposed new footprint-stage metadata marked
`not_applicable` on still-unmapped cases; those rows must remain `unassessed`.
The card records that failure and the subsequent correction separately.

CT41 rejects narrowing integer conversions, semantic attributes and assembler
accesses that evade reference counts, and redeclarations that could introduce
another declaration identity for a selected object or function. Static AST
access counts do not prove dynamic invocation counts: those remain declared
bounds in the certificate alongside source/configuration identity and review
premises for extraction and Rust/Lean correspondence.

## Integration and validation

Pending final branch merges and the combined standing gate. Per-card gate
reports retain failed or incomplete attempts and their actual outcomes. The
combined gate will own its Clang recording, counts, exits and private logs.
No tests, diagnostic refusals or historical evidence are weakened to obtain a
passing result.

## Reconciliation

Pending final tracker reconciliation and generated matrix/status update.
