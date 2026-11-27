# SUPPLIED-OWNED-RETURN-CODE-HANDOFF: census and prediction

Frozen before any implementation code. Base: local `main` at `10eb7692d`
(RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION landed). Inputs: the pinned corpus
`66910c10` (`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source`,
librdkafka 2.15.0) and the prepared system tree `046accf6`. Every body below
was read by hand with [`body.py`](scripts/body.py) and
[`window.py`](scripts/window.py); every text quoted is byte-identical at the
two revisions.

## 1. The protocol values

`rd_kafka_op_res_t` (`rdkafka_op.h:255-264`) is an anonymous `typedef enum`:
`RD_KAFKA_OP_RES_PASS` 0, `HANDLED` 1, `KEEP` 2, `YIELD` 3. Its C02 projection
is `u32` (rule 12u). The header states the protocol: `YIELD` means the handler
re-enqueued or destroyed the op ("the caller will not touch the op anymore"),
`KEEP` that the op "must not be destroyed by op_handle()", and `KEEP` is "NOT
PERMITTED" under `RD_KAFKA_RESP_ERR__DESTROY`.

## 2. Per-candidate return codes

| Candidate | Returns | The op after each result | Interior effects | Clause (user decisions 2026-10-15) | Prediction |
| --- | --- | --- | --- | --- | --- |
| `rd_kafka_poll_cb` (`rdkafka.c:5813-6096`) | `PASS` on seven paths (event set up, `FETCH` without `consume_cb` or on a return call type, `OFFSET_COMMIT` without its callback, `SHARE_ACK_COMMIT_CB_EXECUTE` without one, `CONSUMER_ERR` and `ADMIN_RESULT` on a return call type, `SHARE_FETCH_RESPONSE`); `HANDLED` (destroyed at the end); `YIELD` (`TERMINATE`: destroyed; `DR` after a yield: re-enqueued or destroyed); `rd_kafka_consume_cb`'s result (always `HANDLED`, op destroyed); `rd_kafka_op_call`'s result (`KEEP`/`YIELD`: an `rko_op_cb` candidate holds it; `HANDLED`: destroyed here; `PASS`: untouched) and `rd_kafka_metadata_update_op`'s | `PASS`: untouched (the caller keeps it); every other result: taken | on `PASS` the event setup may replace `rko_u.err.errstr` in place (freed and re-duplicated: the member still holds one owned string) and initialise `rko_u.dr.msgq2`; it moves nothing out | `quant live #rko: #owner \|-> #callback when #result != #PASS;` with `alias #PASS = @RD_KAFKA_OP_RES_PASS;` | **declared**; its other clauses state the member invocations it makes (the rk_conf callbacks as `rd_kafka_background_queue_serve` states them, the op's `rko_u.offset_commit.cb`, `rko_u.terminated.cb` and `rko_op_cb` (through `rd_kafka_op_call`), and `rk_share_consumer.share_acknowledgement_commit_cb`) |
| `rd_kafka_txn_op_send_offsets_to_transaction` (`rdkafka_txnmgr.c:1935-1993`) | `HANDLED` (under `__DESTROY`; on a state or PID error after `rd_kafka_txn_curr_api_set_result`); `KEEP` after `rd_kafka_AddOffsetsToTxnRequest(..., rko)` took the op as its opaque | `KEEP`: taken (the request holds it); `HANDLED`: borrowed (the handler destroys it) | none: it reads `rko_u.txn.cgmetadata->group_id` only | `quant live #rko: #owner \|-> #callback when #result == @RD_KAFKA_OP_RES_KEEP;` | **declared** |
| `rd_kafka_admin_worker` (`rdkafka_admin.c:1035-1291`) | `KEEP` (five waits); `HANDLED` after `rd_kafka_admin_common_worker_destroy` | `KEEP`: taken; `HANDLED`: borrowed | on `HANDLED` it destroys `rko_u.admin_request.eonce` and, through `rd_kafka_admin_result_fail` -> `rd_kafka_admin_result_new`, moves `rko_u.admin_request.args` and `fanout_parent` into a result op | the KEEP hand-off would fit, but the `HANDLED` arm releases and moves out what the op's members hold (OP-CALLBACK-CANDIDATE-CONTRACTS shape (b)) | **named, not declared** |
| `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` (`rdkafka_partition.c:4156-4162` -> `:4030-4144`) | `KEEP` (the eonce waits for metadata); `HANDLED` (after replying, also under `__DESTROY`) | `KEEP`: taken; `HANDLED`: borrowed | on `HANDLED` it moves `rko_u.leaders.partitions` into the reply op and clears it, and disables and clears `rko_u.leaders.eonce` | as above: shape (b) | **named, not declared** |

The two (b) candidates join the six of `OP-CALLBACK-CANDIDATE-CONTRACTS`:
eight undeclared candidates remain in `rd_kafka_op_s.rko_op_cb`'s run set of
27, so `rd_kafka_op_destroy`'s 131 leaves stay `callback-uncontracted` (the
first named, `rd_kafka_DeleteRecords_leaders_queried_cb`, is unchanged).

## 3. The caller branch check, by reading

The soundness requirement: the code that invokes the callback branches on the
same result with the same constant, keeps or disposes of the op on the
complementary results, and does not touch it where the callback took it.

| Invoker (declaration) | Chain to the invocation | Guard | Reading | Expected check |
| --- | --- | --- | --- | --- |
| `rd_kafka_q_serve0` (`callback`, `rko_serve`) | `rd_kafka_op_handle` calls `callback(rk, rkq, rko, cb_type, opaque)` (after `callback = rko->rko_serve` when set) and returns its result | `!= PASS` | `rd_kafka_assert(NULL, res != RD_KAFKA_OP_RES_PASS)` (a real branch to the noreturn `rd_kafka_crash`); the op is never named again in the iteration; the loop condition rebinds `rko` | admits for the formal `callback`; refuses for `rko_serve` (its call goes through a local copy of the member) |
| `rd_kafka_q_pop_serve0` | the same `rd_kafka_op_handle` | `!= PASS` | branches on `HANDLED`/`KEEP` (`goto retry`) and `YIELD` (`return NULL`); every other value, `PASS` and any value outside the enum, `break`s to `return rko` | **refuses**: no branch on `PASS`; a value `!= PASS` outside the four returns the op |
| `rd_kafka_q_serve`, `_serve_maybe_consume` | forward to `rd_kafka_q_serve0` | `!= PASS` | as `rd_kafka_q_serve0` | as `rd_kafka_q_serve0` |
| `rd_kafka_q_pop_serve`, `_pop_serve_maybe_consume` | forward to `rd_kafka_q_pop_serve0` | `!= PASS` | as `rd_kafka_q_pop_serve0` | refuses |
| the serve family (`rko_op_cb`) | `rd_kafka_op_handle` -> `rd_kafka_op_handle_std` -> `rd_kafka_op_call`: `res = rko->rko_op_cb(rk, rkq, rko)` | `== KEEP` | `rd_kafka_op_call`: `YIELD` (or `rd_kafka_yield_thread`) returns `YIELD`; `res != KEEP` clears `rko_op_cb`, so on `KEEP` it returns `KEEP` untouched; `rd_kafka_op_handle`: `KEEP` and `YIELD` return untouched; the serve loops never touch the op on `KEEP` or `YIELD` | admits |
| `rd_kafka_op_destroy` (`rko_op_cb`) | direct: `res = rko->rko_op_cb(rko->rko_rk, NULL, rko)` | `== KEEP` | `rd_assert(res != RD_KAFKA_OP_RES_KEEP)`, glibc `assert` (no `NDEBUG` in the production compile database), ends the `KEEP` path in `__assert_fail` | admits |
| direct calls of `rd_kafka_poll_cb` | `rd_kafka_consume0` (`rdkafka.c:5045`): `== PASS` breaks and keeps the op, else untouched; `rd_kafka_consumer_close` (`:5292`): `== PASS` destroys it, else untouched; `rd_kafka_q_serve_rkmessages` (`rdkafka_queue.c:803`): branches on `KEEP`/`HANDLED`/`YIELD`, every other value keeps the op; `rd_kafka_background_queue_serve` (`rdkafka_background.c:90`): returns on `HANDLED`/`YIELD` and **destroys the op on every other result, `KEEP` included** | `!= PASS` | the first two agree, the last two do not | not read: a direct call of a hand-off declaration is refused (section 4) |

`rd_kafka_admin_handle_response` (`rdkafka_admin.c:920`) also invokes
`rko_op_cb` directly (`== HANDLED` destroys, otherwise untouched); it has no
declaration and is not an invoker of any clause.

## 4. Mechanism (no new surface word)

Exactly the user-decided surface: `#` value keywords (`#absent`, `#owner`,
`#callback`) and binders, bare clause-kind words, `@NAME` resolved through
Clang, module-local `alias #NAME = @NAME;`, the hand-off clause, and the
`#absent` migration. Readings this card takes, each fail-closed:

* **The hand-off** elaborates into the formal's liveness part (`quant live
  #rko;`, backed by `resource_owned(rko, nonnull)`, else
  `quant-live-unbacked`) and a guarded transfer. A declaration carrying it is
  withheld from the function map; rule 12m reads it as a candidate at a
  `supplied_owned` or `_` position (a value the callee owns) only where the
  invoker's chain passes the check, and at a parameter position only where
  every path of the guard ends inside the invoker. Anything else refuses. A
  candidate whose declaration also carries callback clauses still reads
  uncontracted (rule 13u: a retaining declaration is not in the function
  map), so `rd_kafka_poll_cb`'s declaration decides nothing as a candidate.
* **A direct call** of a hand-off declaration is refused (named reason); the
  caller-side reading of a direct call is a residual.
* **The check** reads Clang's AST of each function on the chain (in the
  analysing unit, or the run's facts read by the pre-pass): the result's
  values are split into the constants the body compares it with and the rest;
  each class is followed from the call to every exit; a class the guard
  admits must not name the object again before a rebinding or a noreturn
  call, and what it returns is followed into the caller. Some function of the
  chain must compare the result with the guard's own constant. A body not
  read, a callback passed where no fact follows it, or an unread construct
  refuses.
* **`@NAME`** resolves, per unit, to an enumerator of the candidate's own
  result enum (Clang's value); otherwise refused. `quirin-audit pin` adds the
  enumeration's text to the declaration's reviewed identity (so the value is
  pinned at both revisions) and refuses a name with no enumeration site or one
  in another type.
* **Reserved keywords**: a declaration that carries a `#` form rejects a
  parameter named `absent`, `owner`, `callback` or `result` (the binder
  `#callback` would read as the keyword). Census: 13 contract parameters carry
  such a name (`callback` 7: the six serve declarations and
  `msgpack_packer_init`; `owner` 6, in test fixtures), none in a declaration
  with a `#` form, so nothing on main rejects.
* **Migration**: `|-> absent` occurs once in the providers (`client.qv:1564`,
  `rd_kafka_op_new`), plus test fixtures and docs; every spelling becomes
  `#absent` (the lower level `resource_callback_member_registered(result,
  TAG.FIELD, #absent)` too); a bare `absent` rejects naming `#absent`.

## 5. Prediction

* **No status move on any route.** Corpus 936 / 9 / 3,889, system 93 / 4 /
  351, slice 52 / 1 / 347 / 4 Pending; the null property identical on all
  three (1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232).
  No Safe gain, so no Safe row to list.
* **Leaves that trade, all on Unknown rows:** the four direct calls of
  `rd_kafka_poll_cb` (`call to uncontracted function` -> the hand-off
  direct-call refusal); `rd_kafka_poll_cb`'s and
  `rd_kafka_txn_op_send_offsets_to_transaction`'s own rows (now analysed under
  their entry custody); `rd_kafka_op_destroy`'s 131 leaves unchanged; the
  serve calls naming `rd_kafka_poll_cb` unchanged (`callback-uncontracted`).
  Report digests change (contract digests, the migrated `rd_kafka_op_new`).
* **Lean** 322 -> 323 (one module). **Ledger**: `OP-APP-RK-CALLBACKS-VIA-POLL-CB`
  retired (stated, read at both revisions, measured);
  `OP-CALLBACK-CANDIDATE-CONTRACTS` amended to the eight shape-(b)
  candidates; residuals registered for the direct-call reading and the
  candidate-with-callback-clauses reading; `SERVE0-CALLBACK-HAND-ON-WITHDRAWN`
  unchanged (no `rd_kafka_op_handle` declaration falls out).
