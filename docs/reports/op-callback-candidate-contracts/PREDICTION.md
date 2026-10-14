# OP-CALLBACK-CANDIDATE-CONTRACTS: census and prediction

Frozen before any contract is written (the contracts are this card's
implementation; `scripts/write_contracts.py` is committed with this file but
not yet run). Base: local `main` at `2afce9973`. Rule placement: amend 12n and
13u in place; a new id, if one is truly needed, is `13l`. No new surface word
is needed: every clause below is existing vocabulary.

## 1. The op callback protocol

Read at `66910c10` and `046accf6` (byte-identical): `rdkafka_op.h:249-264`
(the result enum and its remarks), `rd_kafka_op_call` (`rdkafka_op.c:933-943`),
`rd_kafka_op_handle_std` (`:1056-1081`), `rd_kafka_op_handle` (`:1096-1126`),
`rd_kafka_op_destroy`'s head (`:345-354`), `rd_kafka_q_pop_serve0`
(`rdkafka_queue.c:398-505`) and `rd_kafka_q_serve0` (`:558-650`).

| result | `rko_op_cb` (through `rd_kafka_op_call`) | `rko_serve` (after `PASS`) |
| --- | --- | --- |
| `PASS` | not handled: the op stays with `rd_kafka_op_handle`, which hands it to the serve callback or the pop's caller; the callback borrowed it | not handled: `rd_kafka_q_serve0` asserts it never happens; `rd_kafka_q_pop_serve0` returns the op to its caller: **kept by the callee** |
| `HANDLED` | `rd_kafka_op_handle` destroys the op (`rd_kafka_op_call` cleared `rko_op_cb`, so the destroy does not call it again): **borrowed** | the callback disposed of it (destroy, reply, hand-off): **taken** |
| `KEEP` | the callback **took** the op; forbidden under `RD_KAFKA_RESP_ERR__DESTROY` (`rdkafka_op.h:258-262`) | taken |
| `YIELD` | `rd_kafka_op_handle` returns without touching the op, which the callback must have re-enqueued or destroyed (`rdkafka_op.h:250-253`): **taken**; `rd_kafka_op_call` also turns any result into `YIELD` when `rd_kafka_yield_thread` is set (only `rd_kafka_yield`, from an application callback) | taken |

`rd_kafka_op_destroy` calls `rko_op_cb(rko->rko_rk, NULL, rko)` with
`rko_err = RD_KAFKA_RESP_ERR__DESTROY` and asserts the result is neither
`YIELD` nor `KEEP`: there every candidate borrows the op.

The declared invocations (rule 12p, card OP-CONSTRUCTOR-DECLARATIONS) are:
`rd_kafka_op_destroy`'s `rko_op_cb(_, _, rko)` (the op at a parameter
position), the serve and request families' `rko_op_cb(_, _, _)` and
`rko_serve(_, _, supplied_owned, cb_type, _)`. So:

* an `rko_op_cb` candidate is admitted (rule 13u's identity) when its
  contract borrows all three formals with `resource_return_none()`: it must
  return only `HANDLED` or `PASS` and never release, keep or move out the op;
* an `rko_serve` candidate is admitted (rule 12m's taking candidate) when it
  takes the op (`resource_consumed` / `resource_released`) on every return,
  i.e. never returns `PASS`;
* a candidate that takes the op on `KEEP` only takes it on some return codes:
  rule 12m refuses that (user decision 2026-10-14) and C04.13 cannot even
  state it (a disposition needs a signed result; `rd_kafka_op_res_t` is `u32`
  by rule 12u). It stays undeclared, never weakened into a borrow.

## 2. Census (`scripts/candidate_census.py`, `artifacts/candidate-census.json`)

The candidate sets are Clang's run closure (rule 12n, the
OP-CONSTRUCTOR-DECLARATIONS sidecar): `rko_op_cb` 27, `rko_serve` 4
(`rd_kafka_poll_cb` is the app-rk card's). The census is a textual reading
aid (results returned, an early `__DESTROY` test, rk_conf callback reach over
an over-approximate call graph); every row below was read by hand.

"Destroy" says what the body does when `rd_kafka_op_destroy` calls it
(`rko_err = __DESTROY`). "App" is an application callback the body reaches
(Fluent Bit registers three: `log_cb` = `cb_kafka_logger` and `dr_msg_cb` in
`out_kafka`, `oauthbearer token_refresh_cb` in `flb_aws_msk_iam.c`). Every
candidate but the two noted reaches `rd_kafka_log_buf`, hence `log_cb`.

### 2.1 `rko_op_cb` (27)

| candidate | results | op custody | destroy | other effects on objects the op holds | app | predicted |
| --- | --- | --- | --- | --- | --- | --- |
| `rd_kafka_broker_monitor_op_cb` | `HANDLED` | borrowed | returns at once | none; calls `rko_u.broker_monitor.cb(rkb)` (only `rd_kafka_coord_rkb_monitor_cb`, `rd_kafka_txn_coord_monitor_cb`) | via the monitor callbacks' logging | **declared** (premises `broker_monitor_callback`, `rk_log_callback`) |
| `rd_kafka_cgrp_assignor_handle_Metadata_op` | `HANDLED` | borrowed | returns at once | reads `rko_u.metadata.mdi` (assignor run) | log | **declared** (premises `assignor_callback`, `rk_log_callback`) |
| `rd_kafka_cgrp_consumer_handle_Metadata_op` | `HANDLED` (forwarded) | borrowed | returns at once | none | log | **declared** |
| `rd_kafka_cgrp_handle_Metadata_op` | `HANDLED` | borrowed | returns at once | none | log | **declared** |
| `rd_kafka_offset_reset_op_cb` | `HANDLED` | borrowed | **runs a whole offset reset** (`rd_kafka_offset_reset` with `err = __DESTROY`: re-enqueued from another thread, or `auto.offset.reset` applied, fetch state moved, an error op or an offset request) | none (its own new ops hold their own partition references) | log | **declared** (the reset described in the comment) |
| `rd_kafka_offset_validate_op_cb` | `HANDLED` | borrowed | **runs an offset validation** | none | log | **declared** |
| `rd_kafka_share_ack_commit_cb_register_op` | `HANDLED` | borrowed | sets the handle's registration flag | none | log | **declared** |
| `rd_kafka_share_fetch_reply_op_cb` | `HANDLED` (forwarded) | borrowed | **runs the share-fetch reply** (ack callback ops, counters, new fetch ops) | none; reads `ack_details`, `target_broker` (released after by the `SHARE_FETCH` arm) | log | **declared** |
| `rd_kafka_txn_op_init_transactions` … `_abort_transaction_ack` (9) | `HANDLED` | borrowed | return at once | none | log | **declared** (9) |
| `rd_kafka_txn_op_send_offsets_to_transaction` | `HANDLED`, **`KEEP`** | taken on `KEEP` (the op is the `AddOffsetsToTxn` request's opaque) | returns at once | — | log | **residual: KEEP** |
| `rd_kafka_admin_worker` | `HANDLED`, **`KEEP`** | taken on `KEEP` (waiting for a broker, controller, broker list or response) | fails the request, `worker_destroy(…, rd_false)` | timers, eonce | log | **residual: KEEP** |
| `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` | `HANDLED`, **`KEEP`** | taken on `KEEP` (the eonce waits for metadata) | replies: **moves `rko_u.leaders.partitions` into a new reply op** | moves a member out | log | **residual: KEEP** |
| `rd_kafka_admin_fanout_worker` | `HANDLED` | borrowed | **runs in full** | **destroys the fanout parent** (`rko_u.admin_result.fanout_parent`) when the last result arrives; `partial_response` merges results | log | **residual: interior release** |
| `rd_kafka_DeleteRecords_leaders_queried_cb` | `HANDLED` | borrowed | goes to its error path | **destroys the fanout op** in `rko_u.leaders.opaque` on any error | log | **residual: interior release** |
| `rd_kafka_ListOffsets_leaders_queried_cb` | `HANDLED` | borrowed | goes to its error path | the same | log | **residual: interior release** |
| `rd_kafka_share_commit_async_fanout_op` | `HANDLED` | borrowed | **runs in full** | **hands `ack_batches` on and clears it** | log | **residual: interior move** |
| `rd_kafka_share_fetch_fanout_op` | `HANDLED` | borrowed | **runs in full** | the same | log | **residual: interior move** |
| `rd_kafka_share_commit_sync_fanout_op` | `HANDLED` | borrowed | **runs in full** | **moves `ack_batches`, `results` and the op's reply-queue reference into the group** | log | **residual: interior move** |
| `rd_kafka_oauthbearer_refresh_op` | `HANDLED` | borrowed | returns at once | none | **`token_refresh_cb`** (registered by Fluent Bit) | **residual: application callback** (app-rk card) |

An effect on an object the op's members hold (a release, or a move out of
the op) is a custody effect a plain declaration cannot state (C04.13 "Not in
this clause"; P-B08-SHALLOW reads such an object as untouched): those six
candidates stay undeclared rather than be read as borrowing.

### 2.2 `rko_serve`, the three non-poll candidates

| candidate | results | op custody | other | app | predicted |
| --- | --- | --- | --- | --- | --- |
| `rd_kafka_cgrp_op_serve` | `HANDLED` | **taken on every return** (destroyed, replied, handed to a request, the commit, the assign handler or terminate) | `opaque` is the group | log | **declared**: `resource_consumed(rko, nonnull)` |
| `rd_kafka_mock_cluster_op_serve` | `HANDLED` | **taken on every return** (`rd_kafka_op_reply`) | `opaque` is the mock cluster; closing mock connections reaches `rk_conf.closesocket_cb` (never registered: NULL) | log | **declared** (premises `rk_closesocket_callback`, `rk_log_callback`) |
| `rd_kafka_toppar_op_serve` | `HANDLED` | taken on every return | — | **`rko_u.offset_commit.cb`** (holds `rk_conf.offset_commit_cb` or `rd_kafka_commit_queue`'s callback), `open_cb` through the offset file | **residual: application callback** (app-rk card) |

### 2.3 Totals

Declared: **19** (17 `rko_op_cb`, 2 `rko_serve`), each a declared assumption
with its body read at both revisions, its standing published by rules
13i–13k (a premise is never covered, so at best `partly-validated`).
Residual: **11** of the 30 (3 `KEEP`, 6 interior release or move, 2
application callbacks), plus `rd_kafka_poll_cb` (app-rk card).

## 3. Predicted measurement

* **Corpus, system, slice: no status change** (936 / 9 / 3,889; 93 / 4 /
  351; 52 / 1 / 347 / 4 Pending). No Safe gain, no Safe → non-Safe.
* **`rd_kafka_op_destroy`'s callers** (132 / 83 / 2 leaves / functions /
  alone at the ledger's base; 131 leaves after OP-CONSTRUCTOR-DECLARATIONS:
  118 `callback-member-open`, 13 `callback-uncontracted`): the 118 stay (the
  unit stores nothing into `rko_op_cb`, so its own set is empty, rule 12n). The
  13 sit in `rdkafka.c` (unit set `{rd_kafka_share_fetch_reply_op_cb}`) and
  `rdkafka_offset.c` (`{rd_kafka_offset_reset_op_cb,
  rd_kafka_offset_validate_op_cb}`); all three are declared, so those calls
  decide as the identity and the rows move to their next leaf. A row that
  became `Safe` in its unit would be demoted by the run fold
  (`callback-member-open`: the run's 27 candidates are not contained in the
  unit's set), so **no Safe gain** on them; the fold's `demoted` list may grow.
* **The serve family:** the member `rko_op_cb` refuses first in every unit
  whose own set is empty or holds an undeclared candidate, so no serve call
  decides; some leaves may change name (`callback-uncontracted` naming
  `rd_kafka_cgrp_op_serve` or `rd_kafka_mock_cluster_op_serve` → the
  member's refusal).
* **The candidates' own rows:** unchanged in status (each declaration is
  consulted only where the function is called through the members).
* **Null property:** byte-identical rows on all three routes.
* **C04.18 on `rd_kafka_q_purge0`: kept.** Its premise is about a reference
  on the served queue reachable through `rk` or the op (the callbacks receive
  `rkq = NULL`), which no candidate contract states (they are shallow), 11
  candidates stay undeclared, and a re-declaration with the member clause
  would be demoted by the fold anyway. `rd_kafka_toppar_purge_and_disable_queues`
  does not move.
