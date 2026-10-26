# Typed CMetrics default setter postconditions

The three remaining default setters replace 23 sentence-shaped postconditions
with 56 `quant condition normal` formulas. Their 51 existing entry roots are
preserved: histogram 17 entry/10 normal, summary 16/20, exponential histogram
18/26. All six declarations in `typed_setters.qv` now use typed entry/normal
conditions; together with the nine low-level setters, the existing regression
requires complete typed condition inspection for all fifteen APIs and retains
strict resource/bounds permission refusal.

Histogram values use the explicit u64 → double → u64 conversion function.
Summary storage preparation precedes the null-input check; new storage is
zeroed, existing storage and null-input values/flag are preserved, and updates
require both parent and metric bounds. Exponential preparation failures keep
post-lookup arrays and release non-null temporary copies; success installs
copies (null at zero count), swaps before unlock and retires old epochs after
unlock. No lookup rollback or joint atomic snapshot is asserted.

`QCore.CmetricsDefaultSetterConditions` adds sixteen audited conditional model
properties: converted bucket counts/values, summary allocation and slot guards,
zero/null/copy bounds, temporary cleanup/exact-once scheduling under distinct
allocation epochs, and successful retirement/order. The targeted audit exited 0
in `private/quant-contract-hypotheses/default-setter-models-6/`; the preceding
fifteen-theorem audit also passed. Attempt 4 refused the audit argument without
its required `.lean` suffix; it performed no successful audit.

The migration record binds the exact before/after source bytes. Caller-readable
array footprints, exact direct/nested events, lookup-phase state, fields,
floating conversions, allocation epochs, helper/backend semantics and Rust/Lean
correspondence remain open bindings. The model proofs confer no C call
permission. Input custody applies only to actual live/nonzero input storage;
ignored zero-count pointers cannot protect retired metric storage by aliasing.

Provider inventory regeneration exited 0 in default-setter-records-1. The
reviewed lock changes typed_setters.qv from 139 to 172 clauses, with only the
three default setters changing. The current CMetrics manifest binding is
refreshed. All-target Clippy passed with the integration repairs; final
standing/provider validation is pending.
This increment does not close the all-provider migration. The preimplementation
prediction remains unchanged: zero live verdict moves and no new permission.
