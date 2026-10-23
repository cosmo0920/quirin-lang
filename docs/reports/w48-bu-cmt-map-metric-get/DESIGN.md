# Rule 36: exact borrowed-formal return

The human narrowed this card on 2026-10-23 to one audited feature. The bounded rule is implemented; validation and native attribution are still in progress. The CMetrics getter chain remains open.

A body that returns its borrowed pointer formal itself should be able to publish an exact derived-result summary. The existing derived-result route already handles returning a declared exact alias of the formal, but refuses the bare formal. The new exit records the same non-owning relation. It creates, discharges and transfers no resource obligation.

## Source and flow obligations

1. The written return expression is one bare named formal. The derived CFG alone selects the new exit. Ordinary CFGs are unchanged. Earlier non-neutral results retain precedence. A live-entry `None` result may be refined only by a freshly validated derived summary with the identical parameter custody vector. Failed refinements retain the earlier result.
2. The declaration reader positively and unambiguously reads the formal and result as matching depth-one object pointers. Unknown declarations, incompatible qualifiers, scalar and function-pointer results decline. The AST must answer a unique typed formal at the exact parameter index used by the summary; a local, wrong index or shadowing declines. The new source route also requires that the formal is never rebound. This keeps existing summaries for a formal assigned a declared derived result on their existing route, rather than replacing that route with an exact-identity exit.
3. At that exit, the flow binding is either definitely NULL or one known identity over the original formal's name-only site. An owned or derived target, unresolved identity, mixed target, touched site, or another formal's site declines.
4. The accepted identity is observed as an exact derived result. NULL is observed as NULL. The admitted transfer preserves the complete state. A refusal records `UnknownAlias` and uses the existing unresolved-effect marking for possibly escaping targets; it changes no resource status and discharges nothing. A binding rejected at transfer or observation cannot publish a summary.
5. The new identity read participates in the existing volatile-settlement pass: volatile pointer objects cannot supply a stable identity fact. The existing full-body, all-reachable-exit and fresh reported-premise validation gates still run. The base remains borrowed and name-only. Callers use the existing exact-derived semantics, including base-relative nullability and invalidation after release.

The new source guard compares each bare return with its actual Clang `ParmVarDecl` reference, including the formal index and declaration identity within that AST. It requires complete source spans and matching lexical/AST occurrence counts, rejects writes or address-taking of the formal, and rejects conflicting value declarations or references with the same spelling anywhere in the function. These checks are local to the new route; the existing declaration-reader queries keep their behavior. Missing source or AST facts decline.

The new instruction is appended to the serialized instruction enum, preserving existing variant tags.

## Required controls

Source tests cover plain and NULL-guarded returns, a later formal, const pointers, scalar/function-pointer/unknown declarations, shadowing, rebinding to another formal or a fresh allocation, mixed exits, release/consumption/store of the base, callers releasing the alias, use after the base ends, leaks remaining owed, and certificate replay. A shared Rust/Lean table exercises each flow premise's refusing direction. Lean proves the exact observation denotes the original concrete pointer, including NULL, and that admission changes no custody. Rust extraction and Rust–Lean correspondence remain trust-boundary-28 premises.

## CMetrics remainder

`metric_prepare_storage` also allocates directly into a caller-supplied metric's members. `map_metric_create` returns a nullable metric owning a list of labels and SDS values. The getter publishes the metric in a primary owned list, adds a secondary hash index membership, caches it, and returns a borrow from the map. Exact returns alone establish none of those storage, collection, indexing or borrow obligations. Hash helper declarations and CFL list effects likewise remain independently required. They must be tracked with discharge conditions in the final integration amendments; this card will not count their conditional planner ceiling as a measured gain.

## Implementation finding after the prediction

Focused tests showed that the existing live-entry route publishes a neutral `None` result for plain `return p;`. Leaving that result first makes the new route unreachable and can lose caller alias lifetime facts. Rule 36 therefore permits the bounded neutral-result refinement above. This can expose caller lifetime diagnostics behind formerly neutral results; any such native movement must be retained and attributed against the committed forecast, whose preservation expectation is not rewritten. No non-neutral result or parameter custody mode is replaced.

The independent NULL control also exposed a path-search bug: a non-nullable exact-derived call discarded a known NULL base and could produce `path-search:infeasible`. The bounded repair preserves known NULL for an exact alias, retains known non-null, and cuts unresolved bases as unresolved. This prevents a false Safe result when applying the newly published summary. It records the consulted resource contract and is modeled in the same Lean module.

Rule 13q's existing deep-transport control now exercises the new source walks. The deepest admitted index for both controlled shapes (filtered depth 253) used 328 KiB of stack, below the unchanged 1 MiB reader budget; beyond-bound inputs retained no index. No transport bound or importer access changed. The focused measurement is retained in `private/w48-cmt-map/steps/focused-depth-1.log`.
