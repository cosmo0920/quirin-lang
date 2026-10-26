# CMetrics low-level setter postconditions — before implementation

Base 23a1835b9. Decompose all sentence-shaped normal clauses in model-only
`cmetrics/metric_setters.qv` across its nine functions. Describe own-call stores,
successful CAS and timestamp order as explicit complete-trace conditions;
keep scalar conversion functions, double bit patterns, target fields, indices
and caller/source/ABI interpretations explicit. The guarded quantile no-op
must not dereference a null metric. Preserve call exclusion prerequisites.

Atomic stores in a sequence do not become one atomic record snapshot. CAS
loops have no new termination or wait-free guarantee. Express these limitations
as comments, not negations of state guarantees. Ownership/release/escape facts
are scoped to the selected call and nested helper trace. Keep guarded null and
out-of-range behavior, defined-C-conversion bounds, and normal-only selection.

Match new trace properties with audited Lean; exact source-to-trace, field,
backend, conversion and Rust/Lean bindings remain open. No interpreted C call
summary is added. Refresh official provider inventories and current CMetrics
manifest binding. Predict zero live corpus/system/slice verdict moves.
