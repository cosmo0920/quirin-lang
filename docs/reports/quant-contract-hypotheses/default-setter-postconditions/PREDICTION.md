# Typed default setter postconditions — before implementation

Base 2eeec990f. Decompose the remaining histogram, summary and exponential-
histogram default setter sentence clauses in `typed_setters.qv`. Keep nullable
lookup and preparation failures distinct from successful scalar/array updates.
Histogram buckets use the defined u64 → double → u64 conversion, not exact
integer copying. Null summary quantile input does not suppress storage
preparation; only admitted slots receive quantile-set calls, and existing flag/
values are preserved under the appropriate guarded projection.

For exponential histograms, positive counts with null inputs fail before swap;
partial temporary copies are released on preparation failure. Successful swap
installs fresh metric-owned copies (or null for zero counts), and old arrays
are retired after unlock. Input arrays and enclosing owner/map/metric remain
protected under the existing caller and physical-frame prerequisites. Preserve
lookup effects on every failure; no general rollback or atomic-snapshot claim.

Use separate direct wrapper, helper-descendant, phase-return and allocation-
epoch projections. Exact fields, complete source traces, copy/allocator/backend
behavior, numeric/bit conversions and Rust/Lean correspondence remain open.
Match conditional bulk/control/retirement model properties with audited Lean;
refresh official provider inventories and the current manifest binding. Predict
zero live corpus/system/slice verdict moves and no new C call permission.
