# Rule 36b prediction: tracked/interior self-initializer effects

Semantic BASE `bb574e826490f403c84bd0d0e3fb2177c11f530d`.
The fresh corpus completed with exit 0 and stable input identities before this
prediction: 4,834 rows = 1,009 Safe / 3,816 Unknown / 9 provisional Unsafe.
The fresh bottom-up planner also exited 0: 2,192 candidate bodies, conditional
ceiling 384, and baseline same/cross closure 6/6. The getter's conditional
11-caller ceiling remains a planning quantity, not a promised gain.
[Receipt](baseline-receipt.json), [complete planner](base-plan.json.gz).
No Rust or Lean changes precede this prediction commit.

The feature will transport an initializer leaf's actual self-write set into
one exact live owned object or its exact embedded record member. It will reuse
the existing complete leaf/source/CG11 validation, retain explicit writes in
the caller CFG, invalidate stale member facts through aliases, and preserve
resource obligations. It will export no ordinary neutral custody summary.
The full boundary and controls are in [DESIGN.md](DESIGN.md).

Ex ante forecast:

- Generic complete callers allocating, checking, initializing and releasing or
  returning an object may become custody Safe. No fixed production gain is
  promised; every new native movement must be attributed.
- `map_metric_create` has 18 own leaves, including two initializer calls.
  Those calls may lose their unsupported-call leaves on paths where ownership
  and frame checks succeed. Existing unresolved effects can keep the old
  fallback. `cmt_map_create` also has earlier unsupported stores; no diagnostic
  reduction is promised for it.
- `metric_prepare_storage`, `metric_index_add`, `map_metric_create`,
  `map_metric_get_unlocked`, `cmt_map_create` and `cmt_map_metric_get` are
  expected to remain Unknown. Predicted whole-getter closure gain: **0**.
- No prior Safe result should regress. Older modeled/declared calls and rule
  36a keep precedence; structurally unreachable new markers regain their old
  fallback. Borrowed/name-only, released, escaped, nullable, mixed, derived or
  ambiguous roots and unresolved ownership frames refuse. Child, containment,
  membership, collection, retention, borrow, reference, callback or constructed
  relations are never erased to admit initialization.
- No provider, contract or source pin changes are planned. The independent
  NULL property should retain its semantic results; complete dictionaries and
  runtime/projection records will still be compared. Dependency, reading,
  source-version and time metadata changes require exact attribution.

Acceptance requires matching registered placeholder-free Lean and shared
Rust/Lean vectors; actual source, flow, alias/lifetime, deep-reader,
summary/callback/export and certificate controls; the standing gate; fresh TIP
corpus/system/slice measurements on the frozen BASE inputs; complete raw
resource/NULL/runtime differences and their attribution. Unsafe results remain
provisional. Full list operations, nested label/SDS ownership, primary versus
secondary membership, caches, map-returned borrows and hashing stay under the
parent chain. The unrelated exact-return witness precision residual stays Open.
