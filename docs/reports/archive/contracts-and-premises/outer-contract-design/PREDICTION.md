# OUTER-CONTRACT-DESIGN: prediction

Frozen before any implementation (the card pauses for the user's choice;
no product code exists). Base: local `main` `3f8f1a617`, binary `2d42ddb4…`,
measured on all three routes with the null property
([README](README.md) section 2.2). Computed by
[project.py](scripts/project.py) ([projection.json](artifacts/projection.json))
from the measured rows, under the envelopes drafted in README section 4.

## 1. Static case (nothing loaded): every alternative

**No verdict moves**, on any route, in custody, the null property or the
combined row. The expectation is the user's ("Still seated as Safe without
the loading plugin") and holds by construction: the static projection is the
identity, and the outer contract only adds dependency fields
(`outer:NAME`). Measured identity check for the implementation: every row
equal to BEFORE once the outer fields are removed (as the premise cards
checked); the custody report digest moves only by those fields; the static
headline stays corpus 939 / 9 / 3,886, system 95 / 4 / 349, slice
52 / 1 / 347 / 4 Pending, null corpus 1,114 / 1,435 / 2,354 / 18, system
86 / 125 / 243, slice 69 / 121 / 216.

In Alternative B the 163 `resource_premise(rk_no_plugin_library, …)` clauses
are replaced by one envelope; in A and C by citations. Either way the 339
corpus null rows and 483 corpus custody rows that list the premise today list
`outer:rk_plugin_library` instead, with no verdict change.

## 2. Runtime case (published beside the static case)

Rows that move, by route, from the static verdict (`Safe` rows only: nothing
else can move, since the join never lowers `Unknown` or `Unsafe`).

Each cell is `Pending / Unknown` (the Safe rows that move to each); the runtime columns are `Safe / Pending / Unknown / Unsafe` after the move.

| Route | Scenario | Custody Safe → Pending / Unknown | Null Safe → Pending / Unknown | Combined Safe → Pending / Unknown | Runtime custody | Runtime null | Runtime combined |
| --- | --- | ---: | ---: | ---: | --- | --- | --- |
| corpus | A | 21 / 0 | 76 / 0 | 20 / 0 | 918 / 21 / 3,886 / 9 | 2,473 / 76 / 2,354 / 18 | 806 / 20 / 4,068 / 27 |
| corpus | B-fb | 0 / 30 | 1 / 75 | 1 / 28 | 909 / 0 / 3,916 / 9 | 2,473 / 1 / 2,429 / 18 | 797 / 1 / 4,096 / 27 |
| corpus | B-rk | 21 / 0 | 75 / 1 | 19 / 1 | 918 / 21 / 3,886 / 9 | 2,473 / 75 / 2,355 / 18 | 806 / 19 / 4,069 / 27 |
| corpus | B-both | 0 / 30 | 0 / 76 | 0 / 29 | 909 / 0 / 3,916 / 9 | 2,473 / 0 / 2,430 / 18 | 797 / 0 / 4,097 / 27 |
| corpus | B-both-cross | 21 / 9 | 76 / 0 | 20 / 9 | 909 / 21 / 3,895 / 9 | 2,473 / 76 / 2,354 / 18 | 797 / 20 / 4,077 / 27 |
| system | A | 0 / 0 | 2 / 0 | 2 / 0 | 95 / 0 / 349 / 4 | 209 / 2 / 243 / 0 | 63 / 2 / 385 / 4 |
| system | B-fb | 0 / 0 | 2 / 0 | 2 / 0 | 95 / 0 / 349 / 4 | 209 / 2 / 243 / 0 | 63 / 2 / 385 / 4 |
| system | B-rk | 0 / 0 | 0 / 2 | 0 / 2 | 95 / 0 / 349 / 4 | 209 / 0 / 245 / 0 | 63 / 0 / 387 / 4 |
| system | B-both | 0 / 0 | 0 / 2 | 0 / 2 | 95 / 0 / 349 / 4 | 209 / 0 / 245 / 0 | 63 / 0 / 387 / 4 |
| system | B-both-cross | 0 / 0 | 2 / 0 | 2 / 0 | 95 / 0 / 349 / 4 | 209 / 2 / 243 / 0 | 63 / 2 / 385 / 4 |
| slice | A | 0 / 0 | 16 / 0 | 0 / 0 | 52 / 4 / 347 / 1 | 174 / 16 / 216 / 0 | 31 / 2 / 372 / 1 |
| slice | B-fb | 0 / 0 | 16 / 0 | 0 / 0 | 52 / 4 / 347 / 1 | 174 / 16 / 216 / 0 | 31 / 2 / 372 / 1 |
| slice | B-rk | 0 / 0 | 0 / 16 | 0 / 0 | 52 / 4 / 347 / 1 | 174 / 0 / 232 / 0 | 31 / 2 / 372 / 1 |
| slice | B-both | 0 / 0 | 0 / 16 | 0 / 0 | 52 / 4 / 347 / 1 | 174 / 0 / 232 / 0 | 31 / 2 / 372 / 1 |
| slice | B-both-cross | 0 / 0 | 16 / 0 | 0 / 0 | 52 / 4 / 347 / 1 | 174 / 16 / 216 / 0 | 31 / 2 / 372 / 1 |

Scenarios:

* **A (and C)**: both outer contracts loaded; the four `p` declarations cite
  `flb_external_plugin`, the 163 premise clauses cite `rk_plugin_library`.
* **B-fb**: only Fluent Bit plugins loaded, B with the drafted envelope (the
  four `p` lines): librdkafka's closed-world facts are reachable from
  `config` through `void *` contexts and are not covered, so they read
  `Unknown`.
* **B-rk**: only an rk plugin library loaded: Fluent Bit's `p` facts are
  reachable through the conf's opaque and are not covered (`Unknown`);
  `cjson_allocator_hooks` is not reachable through the public API.
* **B-both**: both loaded, each envelope as drafted (the join of the two).
* **B-both-cross**: both loaded, each envelope also vouching for the other's
  facts: everything `Pending` except the 9 `cJSON_*` custody rows
  (`cjson_allocator_hooks`, a global a Fluent Bit plugin reaches through an
  exported symbol, kept `Unknown`: the cautious reading).

## 3. What the prediction says about the choice

* A and C differ from B only where a closed-world fact is not cited: in
  this measurement the 9 `cJSON_*` custody `Safe` rows (A: unaffected in the
  runtime case, which is the omission risk; B: `Unknown`).
* B makes the cross-library question visible: a Fluent Bit plugin can reach
  librdkafka's objects and an rk plugin can reach Fluent Bit's, so a runtime
  `Pending` needs each envelope to vouch for both, or the rows read
  `Unknown`.
* No row in any scenario becomes `Safe`; no static row changes.

## 4. Named corrections

None: no static verdict moves, and the runtime scope is new.
