# AUDIT-IR-CLANG-TRANSPORT

Base: `31c5ed500d9d4a1448a623b839e1644adb19efd1`.

Prediction before implementation: Clang stdout currently remains as one owned
byte vector throughout decoding and consumer construction. Replace transport
storage/readers with bounded reading where the existing full-document and
retry semantics permit it. Preserve subprocess status/stderr and input failure
precedence, complete JSON validation, default versus 256-level admission,
root checks, node budgets, source identity, cross-declaration facts and every
published record. Malformed suffixes must never publish a partial result.
Existing declaration visitors already reduce tree retention; do not duplicate
or change the other cards' member closure and CFG/report work.

The intended outcome is a measured reduction in transient single-unit memory,
including Clang and any charged file cache, with exact source-reading parity.
No new semantic admission, verdict movement, selected-configuration omission,
cache authority or default `.qc` capture is intended. Temporary persisted AST
transport uses `.qc` and is disposable, never trusted evidence. Retain cleanup
on errors and unwinding. Existing matching Lean depth/transport definitions
remain applicable if admission is unchanged; any semantic edit requires matching
audited Lean. Run malformed/depth/large-input controls and the full standing
gate. Serialize heavy gates and measurements with
`/tmp/quirin-audit-ir-next-heavy.lock`. The parent's single full-run receipt is
context only, not before/after evidence for this change, and the parent memory
target remains open.
