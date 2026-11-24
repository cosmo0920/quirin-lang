# K1-CALL-ACTUAL-ACCESS: prediction

Frozen before the rule-75 commit and before any measurement of an
implementation build. The integrator measures with base `rev:3ca85c7a6` on
corpus (eight-home), twelve-home, system and slice. The forecast reads the
re-census of the user's 2026-11-24 `current` run ([census/recensus.json](census/recensus.json)),
the reproduced `-E -P` text of the 124 units holding a K1 row
([scripts/preprocess.py](scripts/preprocess.py), [scripts/pp_classify.py](scripts/pp_classify.py))
and unit probes of the gateway on a scratch harness (the generic `d_*`/`k_*`
fixture contracts). The base unit-scoped census run of this card had not
started when this file was committed (the `audit` slots were held).

## Mechanism and what it can move

Rule 75 changes one reading: the never-bound closure's reading of
`& ( p ) -> f`, the macro-expanded spelling of a member address
(`mtx_lock(&(rktp)->rktp_lock)`, `rd_atomic32_get(&(rk)->rk_terminate)`,
`rd_slice_read(&(rkbuf)->rkbuf_reader, …)`), no longer seeds `p` as addressed
whole. Probes on the base: with `p` a typedef-spelled record pointer (every
librdkafka handle), `d_len(&(t)->n)` in a statement, a condition and a return
is refused and `d_len(&t->n)` is `Safe`; with the rule both are `Safe`. Every
near-miss stays refused (a released, reassigned, incremented or
whole-addressed `p`; a pointer member `&(t)->name`; an owned local).

A row moves only when every leaf it carries is such a site and the rest of
each site is decided. In `current` the lexical class holds 2,245 K1 leaves
(1,691 leaf groups) on 441 Unknown rows; **2** rows carry nothing else:

* `rd_kafka_broker_destroy_error` (`return (rd_atomic32_get(&(rk)->rk_terminate) & 0x1) ? A : B;`,
  `rd_atomic32_get` declared `Borrowed`; the probe of this shape is `Safe` with the rule);
* `map_cpair_toppar_list_t_new` (`rd_map_init(&(map)->rmap, 0, cmp, hash, NULL, destroy)`:
  the callee has no contract in this tree, so the expected outcome is the same row
  re-attributed to ``call to uncontracted function `rd_map_init` `` (rule 12k), not `Safe`).

The other 439 rows keep another blocker; the rule removes their parenthesised
member-address leaves where the root is bound by no other event, exchanging
each for nothing (a decided read) or for the call's own named refusal (rule
12k's ``call to uncontracted function `F` `` where the callee has no contract).

## Predicted moves

| Route | Function | Today | Predicted |
|---|---|---|---|
| corpus (eight-home) | — | — | no status move (no librdkafka unit; FB core and cmetrics leaves of this spelling may fall) |
| twelve-home | — | — | no status move |
| system, slice | — | — | no move |
| `current` | `rd_kafka_broker_destroy_error` | Unknown | Safe |
| `current` | `map_cpair_toppar_list_t_new` | Unknown | Unknown (re-attributed to `rd_map_init`) |

No `Safe -> non-Safe` move and no new `Unsafe` row on any route.

## Predicted leaf changes

* `current`: between 300 and 2,245 `indirect memory access …` / condition /
  return leaves removed on at most 441 rows, the largest families
  `mtx_lock`/`mtx_unlock`/`rwlock_*` over `&(h)->h_lock` (the
  `rd_kafka_*_lock` macros), `rd_atomic32_get(&(rk)->rk_terminate)` (the
  `rd_kafka_terminating` macro) and `rd_slice_read(&(rkbuf)->rkbuf_reader, …)`
  (the `rd_kafka_buf_read_*` macros); some exchanged for
  ``call to uncontracted function `F` `` or `unknown_alias` at a read of a
  bound base (the read's obligation, never an accusation); zero change of
  `leak`, `double_release`, `use_after_release` or `summary_conflict` counts.
* corpus and twelve-home: 0 to 200 removed leaves of the same spelling.

## Against the census draft

The census draft (predictions/K1.md, base `7b726587b`) forecast 51 Safe
(34–68) from 135 functions freed alone. On the 11-24 report the bundle frees
110 functions alone (113 rows); the unit probes show that the base already
decides the generic shapes of all three tags (scalar, pointer and member
actuals in statements, conditions and returns), so the leaves that remain are
specific refusals. Of the 113 rows, 32 carry a custody-taking position
(`*_destroy(arr[i])`, `cfl_sds_destroy(ctx->x->name)`, `close(cf->fd)`: by
design), 8 are the mock API's nested `rd_kafka_op_err_destroy` return
(RK-OP-ERR-DESTROY-CONTRACT), and most of the rest pass a function pointer
(a callback) or call a run-defined callee with no contract (the census's 62–74%
UNC-RUN share), which reading the actual only re-attributes. This card's rule
is the one exact reading defect found; the remaining sub-shapes are
registered with counts in the report.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
