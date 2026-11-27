# CG19-ATOMIC-SOURCE-ROUTE: frozen prediction

Frozen before the AFTER measurement, on base `c374de96`. CG00 wrote no
`predictions/CG19.md` (its predictions stop at CG15), so this prediction is
re-derived from CG00's attribution record
(`docs/reports/cg00-clang-gap-baseline/artifacts/attribution-functions.json`,
card CG19: 2 functions freed alone, 2 with no alias leaf, no partner card)
and from this card's own BEFORE census
(`scripts/census.py`, run on the BEFORE arbiter). **Disclosure:** the route
code was written before this file; no AFTER run of it existed when this file
was committed, and the figures below come from the BEFORE census alone.

## BEFORE (reproduced)

Corpus arbiter on `c374de96`, `resource-diagnostics.json`
`3a25c4e968e8f646f85a61378fb50fd41287534a7d6a2e91725ef47b018a19ca`, equal to
the brief's baseline digest: **608 Safe / 10 Unsafe / 4,216 Unknown**.

Every corpus leaf that names an atomic builtin (3 leaves, 3 functions, all in
`lib/cmetrics/src/cmt_atomic_clang.c`, each function's only leaf):

| Function | BEFORE leaf |
|---|---|
| `cmt_atomic_load` | ``call to uncontracted function `__atomic_load_n` `` |
| `cmt_atomic_store` | ``call to uncontracted function `__atomic_store_n` `` |
| `cmt_atomic_compare_exchange` | ``call to uncontracted function `__atomic_compare_exchange` `` |

## Prediction

- `cmt_atomic_load` (`return __atomic_load_n(storage, 5)`) and
  `cmt_atomic_store` (`__atomic_store_n(storage, new_value, 5)`): **Unknown →
  Safe**. Both are plain sequentially consistent accesses of a `uint64_t`
  through an untracked parameter; the route lowers each to the typed
  reader's dereference of `storage`, whose obligation the unbound parameter
  meets.
- `cmt_atomic_compare_exchange`: **stays Unknown**, its one leaf renamed to
  ``atomic compare-exchange `__atomic_compare_exchange` is not represented by
  the weak-memory model (CG28)``.
- Nothing else moves: no other corpus leaf names an atomic builtin, and every
  hook fires only on one. **AFTER: 610 / 10 / 4,214**, no Safe → non-Safe move,
  no added leak or violation.
- The system slice and the tail-pause slice carry no atomic-builtin leaf
  (CG15's BEFORE system run `f9deddc3…` and the shared slice run `w48cc-after`
  each contain none), so the change cannot reach them; they are not re-run.

These are upper bounds: a function moves only if the typed reader admits the
dereference.

## Amendment (before the AFTER run, with reasons)

After the file above was committed, and still before any AFTER run, the
route gained its `_Atomic` statement hook: a statement any route refused
whose top Clang node is an `_Atomic` simple assignment through a pointer is
admitted as the plain store, and an `_Atomic` compound assignment or
increment has its refusal renamed to the route's read-modify-write refusal.
Predicted corpus effect: **none**. The six source files of the pinned tree that
spell `_Atomic` or an `atomic_*` type (`munit.c`, jemalloc's `atomic_c11.h`,
WAMR's `refcount.h` and `posix/main.c`, two `vmlinux.h`) are none of the
arbiter's 185 units, and the arbiter analyses each unit's own definitions.
The figures above stand: **610 / 10 / 4,214**.
