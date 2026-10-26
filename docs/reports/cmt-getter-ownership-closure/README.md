# CMetrics getter ownership closure

This card closes the ownership question rooted at `cmt_map_metric_get` with a
validated reachable counterexample on the pinned Fluent Bit source. It does not
turn the unresolved helper bodies into summaries. The published `Unsafe` is a
provisional result for `resource-custody.v1` under the recorded source and
profile; it is neither whole-Fluent-Bit safety coverage nor a claim that an
upstream fix exists.

## Ordered evidence

Semantic BASE is `3e3d707bf7d8d12fb29d6cf6b0e690561ef80d64`. Its immutable
binary and 314-file input snapshot produced the fresh 4,834-row corpus and the
2,189-body bottom-up plan before prediction commit
`7ba66bd86f4c2351d35eff83c17098388fc04701`. Semantic implementation commit
`9e42dc021317d24c13cb60cfaf2d6d8ec1cc1300` follows that prediction. The
[BASE receipt](baseline-receipt.json), [prediction](PREDICTION.md), and private
`private/cmt-getter-closure/semantic-freeze.json` record those identities. The
frozen TIP binary is SHA-256
`24c4ae9c32892b2eda9afbe75668c781256e7556301b43e19e3187849a5b6807`.

BASE corpus is 1,013 Safe / 3,812 Unknown / 9 provisional Unsafe. The planner
records baseline same/cross closure 6/8, conditional ceiling 384, and 12 body
wait classes. BASE system is 101 Safe / 343 Unknown / 4 provisional Unsafe;
BASE slice is 57 Safe / 4 Pending / 342 Unknown / 1 provisional Unsafe. These
are separate selections, not a promise that a CMetrics row occurs in each one.

## Reachable hash-zero storage loss

The NUL-free byte sequence `42 d5 68 e1 38 d7 27 ff` hashes to zero under the
pinned XXH3 default secret and short-input mixer. Both the one-shot and CFL
streaming APIs return `0000000000000000`. Empty namespace and subsystem make
that sequence a valid metric name; adding one empty label preserves the exact
stream while selecting the labeled lookup path.

The public histogram observe call then reaches these source states:

1. `metric_hash_lookup` selects `map->metric` because the digest is zero.
2. `metric_prepare_storage` allocates embedded histogram storage.
3. The dynamic primary metric list stays empty and `metric_static_set` stays
   zero, because that flag is set only by the zero-label constructor path.
4. `cmt_map_destroy` guards `metric_release_storage(&map->metric)` with
   `metric_static_set`, so it does not free the allocation.

The [sanitized witness](native-witness.json) records compile/run exits, pinned
source and archive hashes, both digest results, the public API return, the flag
and list state, and intercepted allocation/free behavior. Runtime addresses are
omitted from the published file. The complete command/output record is retained
at `private/cmt-getter-closure/hash-zero-witness.json`.

## Audited mechanism

`legacy_conditional_embedded_storage` separates a generic ownership graph from
the CMetrics source adapter. The graph represents the embedded child, primary
owned children, secondary index aliases, the cached pointer, committed optional
storage, issued borrow epochs, and destructor coverage. The adapter emits the
leak only after the complete translation unit establishes the exact XXH3 short
model and secret prefix, hash-zero embedded selection, fqname/label stream,
allocation, list/index/cache writes, flag-setting condition, public lock/get/
unlock wrapper, and guarded embedded cleanup. Missing functions, changed fields,
changed constants, changed calls, changed stream lengths, or an unsupported
function scan decline to the old analysis. Nine source mutations exercise that
fail-closed path. The new diagnostic is appended after ordinary function
analysis, so the pre-existing uncontracted-helper diagnostic remains present
and the verdict join selects provisional Unsafe.

`QCore.CMetricsGetterOwnership` is the matching Lean model. Its audited theorems
establish the NUL-free witness, exact 64-bit zero result, embedded selection,
primary/secondary/cache preservation, borrow invalidation, and the reachable
destruction leak. `modelPermission` remains false even when a model parses;
there is no project axiom or placeholder and no `.qv` text grants executable
call permission.

The canonical CFL [hash model](../../../providers/quirin-provider-cfl/contracts/cfl/hash_state.qv)
states reset/update/digest mutation and the concrete possibility of a nonempty
NUL-free zero digest. The CMetrics [map model](../../../providers/qcore-provider-cmetrics/contracts/cmetrics/map_lifetime.qv)
uses a concrete nonzero-digest safety precondition instead of the former
abstract exclusion. Both remain declared models outside interpreted resource,
bounds, and outer views. Their inventories, load path, corpus pins, contract
counts, and provider documentation are deliberately updated.
