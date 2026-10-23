# Rule 36: exact borrowed-formal return

The human narrowed this card on 2026-10-23 to one audited feature. This is a design, not an implementation or a verdict result. The CMetrics getter chain remains open.

A body that returns its borrowed pointer formal itself should be able to publish an exact derived-result summary. The existing derived-result route already handles returning a declared exact alias of the formal, but refuses the bare formal. The new exit records the same non-owning relation. It creates, discharges and transfers no resource obligation.

## Source and flow obligations

1. The written return expression is one bare named formal. The derived CFG alone selects the new exit. Ordinary CFGs and the earlier live-entry and removed-result summary routes retain precedence.
2. The declaration reader positively and unambiguously reads the formal and result as matching depth-one object pointers. Unknown declarations, incompatible qualifiers, scalar and function-pointer results decline. The AST must answer the exact, unique declaration; shadowing declines. The new source route also requires that the formal is never rebound. This keeps existing summaries for a formal assigned a declared derived result on their existing route, rather than replacing that route with an exact-identity exit.
3. At that exit, the flow binding is either definitely NULL or one known identity over the original formal's name-only site. An owned or derived target, unresolved identity, mixed target, touched site, or another formal's site declines.
4. The accepted identity is observed as an exact derived result. NULL is observed as NULL. The transfer preserves the complete state. A binding rejected at transfer or observation cannot publish a summary.
5. The existing full-body, all-reachable-exit and fresh reported-premise validation gates still run. The base remains borrowed and name-only. Callers use the existing exact-derived semantics, including base-relative nullability and invalidation after release.

## Required controls

Source tests cover plain and NULL-guarded returns, a later formal, const pointers, scalar/function-pointer/unknown declarations, shadowing, rebinding to another formal or a fresh allocation, mixed exits, release/consumption/store of the base, callers releasing the alias, use after the base ends, leaks remaining owed, and certificate replay. A shared Rust/Lean table exercises each flow premise's refusing direction. Lean proves the exact observation denotes the original concrete pointer, including NULL, and that admission changes no custody. Rust extraction and Rust–Lean correspondence remain trust-boundary-28 premises.

## CMetrics remainder

`metric_prepare_storage` also allocates directly into a caller-supplied metric's members. `map_metric_create` returns a nullable metric owning a list of labels and SDS values. The getter publishes the metric in a primary owned list, adds a secondary hash index membership, caches it, and returns a borrow from the map. Exact returns alone establish none of those storage, collection, indexing or borrow obligations. Hash helper declarations and CFL list effects likewise remain independently required. They must be tracked with discharge conditions in the final integration amendments; this card will not count their conditional planner ceiling as a measured gain.
