# CMetrics scalar wrapper postconditions — before implementation

Base 8549dd130. Migrate the counter, gauge and untyped setter normal clauses in
`typed_setters.qv` to typed conditions. Keep write-op lookup, nullable lookup
failure, counter reset/decrease refusal, successful scalar delegation/stores and
custody conditions explicit. Preserve lookup side effects; failure is not a
rollback claim. Direct wrapper calls and complete nested store/custody traces
are distinct event domains, so logging callbacks cannot be mistaken for the
wrapper's own scalar-set invocation. Counter decrease uses the exact observed
C floating comparison, including NaN behavior, not mathematical integer order.

Reuse the audited low-level store projection and add a conditional control-flow
model; actual source/trace, observed floating comparison, field, conversion,
backend and Rust/Lean bindings stay open. Other typed default setters remain
for the next increment. Refresh official inventory/manifest bindings. Predict
zero live corpus/system/slice verdict moves and no C permission promotion.
