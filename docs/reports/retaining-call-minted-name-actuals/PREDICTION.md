# RETAINING-CALL-MINTED-NAME-ACTUALS prediction

Committed before the implementation commit, on base
`a9f7bebbe506ab9d1b8657b990660e99c69f69ea`. The rows were relisted at
`baseline:a9f7bebbe506ab9d1b8657b990660e99c69f69ea` (the main checkout's
`private/baselines/`).

## The rule this predicts

Rule 12t classifies a call of a withheld declaration a second time, under its
minted name `qcore_retained__<callee>`. Before this card, the access gate's
W44 probe matched its admitted actuals under the minted name only at the four
cell-bearing declarations (rule 38a). At every other withheld declaration an
admitted actual left the gate in the outer classification and was refused
again in the inner one. The card matches the minted name at every withheld
declaration, for the call's own callee only, for an actual the probe
admitted at a declared `Borrowed` position. A rule-1 release actual, a
custody-taking position, an uncontracted callee, a function-map callee and
anything the probe refused keep the refusal. An actual admitted under the
minted name must also still sit at a `Borrowed` position of the classified
call: rule 12t reads a callback's requirement as `resource_owned` after the
inner pass, and such a call keeps the refusal.

**What can move.** Only a statement that the gate already refused, whose
first call is a withheld declaration without cells, with an actual that
reads memory at a `Borrowed` position. W44's probe applies only when the
re-classification is a contracted call. So where rule 12t's route itself
refuses the call (an undecided retention spelling, an unnamed registry
holder, an open callback set, a callback formal), the probe restores the
gate's refusal byte for byte and the row is unchanged.

## How the rows were relisted

1. **The withheld set.** The baseline contract sidecars' sources were loaded
   with `qcore_contract`'s loader, and `ResourceContracts::retaining()` was
   listed. It has 108 declarations, the same on all three routes, 4 of them
   cell-bearing.
2. **The leaves.** Every `indirect memory access is outside the resource
   projection` leaf of every function row was read at its origin line in the
   analysed sources: corpus `build/cycle19/c-gateway-wave-pin` (66910c10),
   system `private/w45-fb/w46-external-callee-contracts/attempt-1` (046accf6),
   slice `private/w47-tail/attempt-1/prep` (34155955). The leaves kept are
   those whose statement's first call is a withheld declaration without
   cells, after expanding librdkafka's `rd_kafka_q_new`, `rd_kafka_q_destroy`,
   `rd_kafka_op_new`, `rd_kafka_toppar_keep`, `rd_kafka_topic_destroy` and
   `rd_kafka_toppar_destroy` macros. They must also have an actual that reads
   memory at a declared `Borrowed` position.
3. **The rows.** The rows below carry at least one such leaf.

| Route | Rows | Rows whose every other leaf is an unresolved identity or none |
|---|---|---|
| corpus | 332 | 54 |
| system | 4 | 0 |
| slice | 10 | 2 (`flb_input_collector_destroy`, both units) |

The leaves by callee (corpus):

| Callee | Leaves |
|---|---|
| `rd_list_add` | 145 |
| `rd_list_init` | 111 |
| `rd_list_destroy` | 91 |
| `cfl_sds_cat_safe` | 32 |
| `rd_kafka_q_destroy_owner` | 16 |
| `rd_kafka_q_new0` | 14 |
| `rd_kafka_interceptor_method_add` | 14 |
| `mk_event_add` | 14 |
| `rd_kafka_confval_set_type` | 13 |
| `rd_list_new` | 12 |
| `rd_kafka_q_serve` | 9 |
| `qsort` | 8 |
| `rd_kafka_op_new_cb` | 7 |
| 21 others | 46 |

System has 4 leaves, all `msgpack_packer_init`. Slice has 17:
`flb_input_set_collector_time` 6, `mk_event_del` 4,
`flb_input_set_collector_event` 3, `mk_event_add` 2 and
`msgpack_packer_init` 2.

## Status

**No row moves on any route.**

Each row in the 332 / 4 / 10 above is `Unknown` at the baseline, except
Monkey's `mk_net_conn_create`, which is `Unsafe`. Of these, 278 / 4 / 8 rows
keep a leaf on another statement that this card does not touch: an
uncontracted callee, a branch, loop or return condition, a gate refusal at a
function-map or nested call, a registry or reference refusal, a nested call
or an unmodelled assignment. Those rows stay `Unknown`.

`mk_net_conn_create` stays `Unsafe`. Its leak is acquired at `mk_net.c:247`
and reported at `:320`. The path to it (`mk_net_fd_connect` succeeds, then
`return NULL`) does not pass the `mk_event_add` call at `:276`.

The 56 rows without such a leaf were read one by one, and each family was
also run through the implementation on fixtures under the baseline route's
own contract sources. That run is not a measurement.

| Family (rows) | What happens at the call | Status |
|---|---|---|
| `rd_kafka_q_serve_maybe_consume`, `rd_kafka_q_pop_serve_maybe_consume`, `rd_kafka_q_serve0`, `rd_kafka_q_pop_serve0`: `rd_kafka_poll`, `rd_kafka_queue_poll`, `rd_kafka_queue_poll_callback`, `rd_kafka_q_serve_maybe_consume`, `rd_kafka_q_pop_serve_maybe_consume` (5) | The sidecar records `rd_kafka_op_s.rko_serve` and `rko_op_cb` as open (36 units unread), or the callback is a formal. Rule 12t refuses, so the gate's refusal is kept | `Unknown` |
| `rd_kafka_confval_set_type`: `rd_kafka_AdminOptions_set_{broker,opaque,operation_timeout,request_timeout,validate_only}` (5) | The retained pointer is `&local` or `&formal`, and the holder is `&options->F`. Rule 12t refuses, so the gate's refusal is kept (fixture) | `Unknown` |
| `rd_list_destroy`, `rd_list_init`, `rd_list_add`, `rd_list_clear`, with a holder `&X->f…` or `X->f…` (22) | The registry holder is not one the body names (callback-registry-undecided). Rule 12t refuses, so the gate's refusal is kept (fixture; the same refusal is a baseline leaf in `rd_kafka_interceptors_init`) | `Unknown` |
| `rd_kafka_interceptor_method_add` (14) | `(void *)on_X` is not a retained spelling (retention-undecided). Rule 12t refuses, so the gate's refusal is kept (fixture) | `Unknown` |
| `rd_kafka_queue_new0`: `rd_kafka_queue_get_{main,consumer,partition}`, `rd_kafka_queue_get_sasl` (4) | The call is admitted. `rk` is retained in the returned queue, and the exit refuses: "a holder retaining `rk` may outlive this function's frame" (fixture) | `Unknown` |
| `rd_kafka_q_destroy_owner` / `rd_kafka_q_destroy0` on `rkqu->rkqu_q`: `rd_kafka_queue_destroy` (1) | The call is admitted and refuses `reference-drop-unheld`. The `rkqu` identity leaf stays (fixture) | `Unknown` |
| `bsearch`: `rd_kafka_broker_request_supported` (1) | The call is admitted. The comparator's requirement makes `key` (`&skel`) an `Owned` position, and `&skel` is not a name: "arguments do not match resource contract for `bsearch`". `ret` stays unresolved. This is a near-verbatim fixture of the body under glibc's `<stdlib.h>`; with a hand-written prototype the fixture was `Safe`, so this is the least certain row | `Unknown` |
| `qsort`: `rd_list_sort` (1) | `rd_list_cmp_curr = cmp` at `rdlist.c:267`, before the call, keeps its leaf | `Unknown` |
| `rd_kafka_op_new0` and `rd_kafka_toppar_keep0`: `rd_kafka_op_new_reply` (1) | The store `rko->rko_rktp = rd_kafka_toppar_keep(…)` at `:776` is refused at the gate as a member store, before any call is classified | `Unknown` |
| `mk_event_del(config->evl, &coll->event)`: `flb_input_collector_destroy` (slice, ×2) | The call is admitted. The unresolved `coll` at `flb_free(coll)` (`:3153`) stays (fixture) | `Unknown` |

- **No `Unsafe` move.** No newly admitted read's base is released on a path
  that reaches it. The flagged release-before-read candidates were read: each
  sits in an error arm that returns, or in another `switch` case or loop
  arm. Rule 12t refuses a release while held and never accuses.
- **No move away from `Safe` or `Pending`.** Only a statement the gate
  already refused is affected.
- **Leaves.** Leaves trade on rows where the call is now admitted:
  - the gate leaf becomes rule 12t's answer (`retention-undecided`,
    `reference-drop-unheld`, a removal), or no leaf;
  - unresolved-identity leaves caused by the refused statement's havoc may
    disappear;
  - a path that used to stop being precise there can reach later refusals.

The contract sidecar, provider locks, digests and bounds identity are
unchanged, because no contract is touched.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
