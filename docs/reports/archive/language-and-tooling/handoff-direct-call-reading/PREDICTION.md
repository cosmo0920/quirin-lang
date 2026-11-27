# HANDOFF-DIRECT-CALL-READING: prediction (frozen before implementation code)

Card HANDOFF-DIRECT-CALL-READING (wave 48, card-high). Base: local `main`
`b2fbde2c5` (corpus 938 / 9 / 3,887, system 93 / 4 / 351, slice 52 / 1 /
347 / 4 Pending, ledger 786, Lean 327). Rule 12m amended in place; no new
rule id expected. No new surface syntax: the reading applies the existing
clause `quant live #P: #owner |-> #callback when #result OP @C;` at a direct
call, so there is no pause point.

## 1. The four bodies, at both revisions

The corpus copy of Fluent Bit `66910c10`
(`build/cycle19/fluent-bit-clang-source`), the current-revision copy and the
`046accf6` checkout are byte-identical for `rdkafka.c`, `rdkafka_queue.c`,
`rdkafka_background.c`, `rdkafka_op.h`, `rdkafka_op.c` and `rdkafka_int.h`
(compared with `filecmp`, `shallow=False`). `rd_kafka_op_res_t`
(`rdkafka_op.h:255-264`): `PASS` 0, `HANDLED` 1, `KEEP` 2, `YIELD` 3. The
declaration: `rd_kafka_poll_cb` hands `rko` (parameter 2) to the callback
`when #result != #PASS`.

| row | the call and the caller's branch | reading by result class (guard `!= PASS`) | prediction |
| --- | --- | --- | --- |
| `rd_kafka_consume0` (`rdkafka.c:5045`) | `res = rd_kafka_poll_cb(rk, rkq, rko, RD_KAFKA_Q_CB_RETURN, NULL);` then, as the next statement, `if (res == RD_KAFKA_OP_RES_PASS) break;` | `PASS` (kept): breaks; the op is the caller's after the loop (`rd_kafka_message_get(rko)`). `YIELD` (taken): `return NULL`, op not named. The rest (taken): `return NULL` or `continue`, whose loop condition rebinds `rko` from `rd_kafka_q_pop` before naming it | **agrees**; the test is read where the value is stored and tested by the next statement (rule 11r's form) |
| `rd_kafka_consumer_close0` (`rdkafka.c:5292`) | `res = rd_kafka_poll_cb(rk, rkq, rko, RD_KAFKA_Q_CB_RETURN, NULL);` then `if (res == RD_KAFKA_OP_RES_PASS) rd_kafka_op_destroy(rko);` | `PASS` (kept): destroyed. The rest (taken): back to the loop condition, which rebinds `rko` | **agrees**; same form |
| `rd_kafka_q_serve_rkmessages` (`rdkafka_queue.c:803`) | `res = rd_kafka_poll_cb(...)`; `if (res == KEEP \|\| res == HANDLED) continue; else if (unlikely(res == YIELD \|\| rd_kafka_yield_thread)) break;` then `rd_kafka_op_is_ctrl_msg(rko)`, `TAILQ_INSERT_TAIL(&ctrl_msg_q, rko, …)`, `rd_kafka_message_get(rko)` | `KEEP`, `HANDLED`: continue (untouched). `YIELD`: break (untouched). The rest, `PASS` and every value outside the enumeration: names `rko` again; the guard admits the values outside the enumeration and `PASS` is never compared | **refuses** (the caller keeps the op on results the guard gives the callback) |
| `rd_kafka_background_queue_serve` (`rdkafka_background.c:90`) | `res = rd_kafka_poll_cb(..., RD_KAFKA_Q_CB_CALLBACK, opaque);` `if (res == HANDLED \|\| res == YIELD) return res;` then `rd_kafka_op2str(rko->rko_type)`, `rd_kafka_op_destroy(rko)` | `HANDLED`, `YIELD`: returned (untouched). The rest, `KEEP` included: reads and destroys `rko`, which `KEEP` gave the callback | **refuses** |

## 2. Mechanism to be built

At a direct call of a hand-off declaration, the caller's per-path state for
the handed object forks: **taken** where the guard holds (the object is
gone, the handed actual consumed) and **kept** where it fails (the
declared `resource_owned(P, nonnull)`). The two are joined only at the
caller's own test of the call's result, as rule 11r lowers a return-code
disposition: the call tested where it stands, or stored by `NAME = f(…);`
and tested by the next statement, against an integer constant or an
enumerator Clang resolves in the callee's result type; each edge takes the
one disposition every class it can reach agrees on, and an edge reaching
both refuses. Before that, the caller branch check's own reading of the
calling body (`handoff_branch::function_handling`, the analysing unit's
Clang AST) must admit the call: every direct site of the callee in the
body is read, no class the guard admits names the object again, the body
compares the result with the guard's own constant, and the result is not
stored through a conversion. A disagreeing reading refuses
`callback-handoff-unbranched`; a reading not made (not read, not branched,
not tested where it stands) refuses `callback-handoff-direct`, as today.

## 3. Predicted measurement

* **Corpus:** 938 / 9 / 3,887, **no status move**. The four rows each trade
  their one `callback-handoff-direct` leaf:
  * `rd_kafka_consume0`, `rd_kafka_consumer_close0`: the hand-off is read;
    the call then refuses at its next clause, the member invocation of
    `rd_kafka_op_s.rko_u.offset_commit.cb` (`callback-member-open`: the
    union's type-keyed opaque writes, `UNION-OPAQUE-RECORD-WRITES-BY-TYPE`),
    as `rd_kafka_poll_cb`'s composition refuses at the candidate rows.
  * `rd_kafka_q_serve_rkmessages`, `rd_kafka_background_queue_serve`:
    `callback-handoff-unbranched` (the reading disagrees with the guard).
    If the body walk does not read a construct of either body, the key
    stays `callback-handoff-direct` with the unread reason; either way the
    call refuses.
* **No Safe gain**: every one of the four rows carries other leaves
  (unresolved identity, uncontracted `rd_kafka_q_pop` and friends,
  indirect access). **No Safe → non-Safe change.**
* **System** 93 / 4 / 351 and **slice** 52 / 1 / 347 / 4 Pending:
  byte-identical (no direct call of a hand-off declaration in their units).
* **Null property:** tallies and rows identical on all three routes.
* **Next blocker per row:** the two agreeing rows,
  `UNION-OPAQUE-RECORD-WRITES-BY-TYPE` at the call, then their other leaves;
  the two refusing rows, the disagreement itself (real behaviour of the
  bodies: no contract or reading change can admit it without changing the
  declaration's guard).
* **Lean:** one new module (`QCore.HandOffDirect`): the split is sound per
  result class, a disagreeing caller never decides, an unbranched one never
  decides; 327 → 328 audits.
* **Ledger:** `HANDOFF-DIRECT-CALL-READING` retired if its DISCHARGE holds
  (786 → 785 before any registration); a later-branch residual (a stored
  result tested other than by the next statement) registered if the
  mechanism leaves it, sized at 0 corpus rows.
