# CG28-WEAK-MEMORY-EVENTS: prediction

Frozen before any AFTER measurement (corpus arbiter, system route or
tail-pause slice) of this card's code.

**Disclosure.** As with CG19, the core model, the Lean modules and the route
code were written before this file (while the BEFORE measurement of base
`b6a3dc42` was running). Unit and source-level tests had run; no AFTER
corpus, system or slice measurement had. The prediction below is derived from
CG19's measured census (its `after-census.json`: 1 atomic-builtin leaf on 1
function) and from reading the route's hooks, and is checked against this
card's own BEFORE census once that run finishes.

## Scope

* Core: `qcore_core::weak_memory` gains `AtomicReadModifyWrite`,
  `AtomicCompareExchange` and `AtomicFence` events, RMW atomicity, C11
  (N1570) release sequences with RMWs, the 7.17.4 fence synchronisation rules
  and the 7.17.3 seq_cst fence rules. `memory_order_consume` stays out of
  scope (refused at the source route).
* Route: `atomic_source_route` routes the read-modify-write, compare-exchange,
  thread-fence and signal-fence rows and the `_Atomic` compound assignment and
  increment; route G lowers them to their custody effect (a fence: none; an
  RMW: the object's dereference with a write; a compare-exchange: the object's
  dereference with a write, the `expected` operand's write, the desired
  value's read). `__sync_*` (GCC full barriers), the generic by-pointer
  `__atomic_exchange`, a consume order on an RMW or compare-exchange, a
  failure order that is release/acq_rel or stronger than the success order and
  an unevaluated weak flag are refused by name.

## Corpus arbiter (`66910c10`)

BEFORE expected: the brief's baseline, **629 / 10 / 4,195**,
`resource-diagnostics.json` `3d53a5d0…`.

| Function | BEFORE | Predicted AFTER | Why |
|---|---|---|---|
| `cmt_atomic_compare_exchange` (`lib/cmetrics/src/cmt_atomic_gcc.c`) | Unknown, one leaf ``atomic compare-exchange `__atomic_compare_exchange` is not represented by the weak-memory model (CG28)`` | **Safe** | its only leaf; the generic compare-exchange of `storage` with `&old_value` (a scalar formal, forgotten on failure) and `&new_value` (a scalar formal, read) is routed and lowered to the custody effect `cmt_atomic_store` already has, and its `_Bool` result is a returned scalar |

Nothing else moves: **630 / 10 / 4,194**. No Safe → non-Safe move, no added
leak or violation. Few corpus moves are expected because the corpus spells no
`_Atomic`, no fence and no other read-modify-write or compare-exchange
(CG19's census). The digest of `resource-diagnostics.json` changes by that one
function's record only.

## System route and tail-pause slice

Unchanged, byte-identical digests: system **44 / 2 / 188** (`79404fed…`),
tail-pause **29 / 1 / 239 / 1 Pending** (`93cc660c…`). Neither carries a leaf
naming an atomic builtin, and every hook of this card fires only on an atomic
builtin's refusal or an `_Atomic` statement.

## What a move means

A route-G `Safe` is custody under the declared contracts. It is not an
ordering, race-freedom or liveness claim: the core model checks one supplied
candidate execution (`SINGLE_GRAPH_ASSUMPTION`, W07/CG29). The weak-memory
property this card publishes is the core checker's: C11 (N1570) execution
validity and data-race freedom **of one supplied graph**, now including RMW,
compare-exchange and fence events. It is reported separately from custody
coverage.
