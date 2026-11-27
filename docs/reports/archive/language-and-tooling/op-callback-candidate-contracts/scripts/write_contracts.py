#!/usr/bin/env python3
"""write_contracts.py -- OP-CALLBACK-CANDIDATE-CONTRACTS: write the card's
section of `qcore-provider-librdkafka/contracts/librdkafka/client.qv`
(between its markers, replacing a previous run's, placed after the
OP-CONSTRUCTOR-DECLARATIONS section): the candidates of the run closures of
`rd_kafka_op_s.rko_op_cb` and of `rd_kafka_op_s.rko_serve` (rule 12n) whose
custody the vocabulary states, every body read by hand at `66910c10` and
`046accf6` (`scripts/body.py`, byte-identical; census
`artifacts/candidate-census.json`). The candidates it does not declare, and
why, are in the section's head and the report. Idempotent."""
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
MODULE = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
BEGIN = "// BEGIN OP-CALLBACK-CANDIDATE-CONTRACTS (scripts/write_contracts.py)"
END = "// END OP-CALLBACK-CANDIDATE-CONTRACTS"
AFTER = "// END OP-CONSTRUCTOR-DECLARATIONS\n"

INTRO = """// # Op callback candidates
//
// Card OP-CALLBACK-CANDIDATE-CONTRACTS (gateway rules 12m, 12n, 13u). Rule
// 12n closes `rd_kafka_op_s.rko_op_cb` over 27 candidates and
// `rd_kafka_op_s.rko_serve` over 4 for the run; this section declares the
// ones whose custody the vocabulary states. Every body was read at
// `66910c10` and `046accf6` (byte-identical, `scripts/body.py` of the card's
// report); each declaration is pinned (C06.2). `rd_kafka_op_res_t` and
// `rd_kafka_q_cb_type_t` are enumerations with no negative value
// (`rdkafka_op.h:255-277`): `u32` (rule 12u).
//
// The op callback protocol (`rdkafka_op.h:249-264`; `rd_kafka_op_call`,
// `rdkafka_op.c:933-943`; `rd_kafka_op_handle_std` and `rd_kafka_op_handle`,
// `:1056-1126`; `rd_kafka_op_destroy`, `:345-354`; the serving loops,
// `rdkafka_queue.c:398-505` and `:558-650`):
//
// * `rko_op_cb(rk, rkq, rko)`, reached through `rd_kafka_op_call` for an op
//   whose type carries `RD_KAFKA_OP_CB`:
//   * `PASS`: not handled; the op stays with `rd_kafka_op_handle`, which hands
//     it to the serve callback (or a pop returns it);
//   * `HANDLED`: `rd_kafka_op_handle` destroys the op (`rd_kafka_op_call`
//     clears `rko_op_cb` first, so the destroy does not call it again): the
//     callback only borrowed it;
//   * `KEEP`: the callback took the op, which `rd_kafka_op_handle` must not
//     destroy; not permitted under `RD_KAFKA_RESP_ERR__DESTROY`;
//   * `YIELD`: `rd_kafka_op_handle` returns without touching the op, which the
//     callback must have re-enqueued or destroyed: taken. `rd_kafka_op_call`
//     also turns any result into `YIELD` when `rd_kafka_yield_thread` is set
//     (only `rd_kafka_yield`, called from an application callback, sets it).
//   `rd_kafka_op_destroy` calls `rko_op_cb(rko->rko_rk, NULL, rko)` with
//   `rko_err = RD_KAFKA_RESP_ERR__DESTROY` and asserts neither `YIELD` nor
//   `KEEP`: there the callback borrows the op.
// * `rko_serve(rk, rkq, rko, cb_type, opaque)`, reached from
//   `rd_kafka_op_handle` after `PASS`: the callback owns the op on every
//   result but `PASS` (`rd_kafka_q_serve0` asserts the result is not `PASS`;
//   `rd_kafka_q_pop_serve0` returns a `PASS` op to its caller).
//
// So an op callback that returns only `HANDLED` (or `PASS`) borrows the op
// (`resource_borrowed(rko)`: the identity rule 13u admits), and a serve
// callback that never returns `PASS` takes it on every return
// (`resource_consumed(rko, nonnull)`: rule 12m's taking candidate). A
// callback that takes the op only on `KEEP` takes it on some return codes
// only: rule 12m refuses that, and a return-code disposition (C04.13) is not
// even stated over the unsigned `rd_kafka_op_res_t`, so it is not declared.
//
// Every declared candidate may log (`rd_kafka_dbg`, `rd_kafka_log` ->
// `rd_kafka_log0` -> `rd_kafka_log_buf`, `rdkafka.c:280-305`), which calls
// the log callback registered in `rk->rk_conf` when no log queue is set:
// Fluent Bit's `out_kafka` registers `cb_kafka_logger` and no log queue
// (`plugins/out_kafka/kafka_config.c:111`). That is an application rk_conf
// callback (the user's decision of 2026-10-14 on `rd_kafka_destroy_app`); each
// declaration states the dependency as the premise `rk_log_callback`, which
// `OP-APP-RK-CALLBACKS-VIA-POLL-CB` discharges by contracting the logger.
//
// Not declared (the card's report, section 3, with each candidate's sizes):
//
// * taking the op on `KEEP` only: `rd_kafka_admin_worker` (`KEEP` while it
//   waits for a broker, the controller, the broker list or a response),
//   `rd_kafka_txn_op_send_offsets_to_transaction` (`KEEP` once the op is the
//   `AddOffsetsToTxn` request's opaque),
//   `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` (`KEEP`
//   while the eonce waits for metadata);
// * releasing or moving out an object the op's members hold, an effect a
//   plain declaration cannot state: `rd_kafka_admin_fanout_worker` (destroys
//   the fanout parent when its last result arrives),
//   `rd_kafka_DeleteRecords_leaders_queried_cb` and
//   `rd_kafka_ListOffsets_leaders_queried_cb` (destroy the fanout op held in
//   `rko_u.leaders.opaque` on an error, `RD_KAFKA_RESP_ERR__DESTROY`
//   included), `rd_kafka_share_commit_async_fanout_op`,
//   `rd_kafka_share_fetch_fanout_op` (hand `ack_batches` on and clear it),
//   `rd_kafka_share_commit_sync_fanout_op` (moves `ack_batches`, `results`
//   and the op's reply-queue reference into the consumer group);
// * calling an application callback (`OP-APP-RK-CALLBACKS-VIA-POLL-CB`):
//   `rd_kafka_oauthbearer_refresh_op` (`rk_conf.sasl.oauthbearer.token_refresh_cb`,
//   which Fluent Bit registers, `src/aws/flb_aws_msk_iam.c:775`) and
//   `rd_kafka_toppar_op_serve` (`rko_u.offset_commit.cb`, which holds
//   `rk_conf.offset_commit_cb` or the callback `rd_kafka_commit_queue` is
//   handed);
// * `rd_kafka_poll_cb` (the fourth `rko_serve` candidate): the app-rk card's."""

LOG = (
    'requires resource_premise(rk_log_callback, "a line the callback logs through '
    "rd_kafka_log0 may call the log callback registered in rk_conf (Fluent Bit out_kafka "
    "registers cb_kafka_logger and no log queue), which receives the handle, a level and two "
    'strings and is assumed to take no custody until OP-APP-RK-CALLBACKS-VIA-POLL-CB contracts it");'
)

OP_CB_PARAMS = "(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, {op}: ptr<mut, u8>) -> u32"
SERVE_PARAMS = "(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>, cb_type: u32, opaque: ptr<mut, u8>) -> u32"


def block(comment, signature, clauses):
    body = "\n".join("    " + clause for clause in ["requires resource_contract_v1;", *clauses])
    return f"{comment}\n{signature}\ncontract {{\n{body}\n}};"


def op_cb(name, comment, premises=(), op="rko"):
    return block(
        comment,
        f"extern fn {name}{OP_CB_PARAMS.format(op=op)}",
        [
            *premises,
            LOG,
            "ensures normal resource_borrowed(rk);",
            "ensures normal resource_borrowed(rkq);",
            f"ensures normal resource_borrowed({op});",
            "ensures normal resource_return_none();",
        ],
    )


def serve(name, comment, premises=()):
    return block(
        comment,
        f"extern fn {name}{SERVE_PARAMS}",
        [
            *premises,
            LOG,
            "ensures normal resource_borrowed(rk);",
            "ensures normal resource_borrowed(rkq);",
            "ensures normal resource_consumed(rko, nonnull);",
            "ensures normal resource_borrowed(cb_type);",
            "ensures normal resource_borrowed(opaque);",
            "ensures normal resource_return_none();",
        ],
    )


TXN_HEAD = """// The transaction ops (`rdkafka_txnmgr.c`), registered by
// `rd_kafka_txn_op_req` (`rd_kafka_op_new_cb(rk, RD_KAFKA_OP_TXN, op_cb)`):
// each returns `HANDLED` at once when `rko_err` is
// `RD_KAFKA_RESP_ERR__DESTROY`, and otherwise checks and moves the
// transaction state under `rd_kafka_wrlock(rk)` and replies to the waiting
// application call (`rd_kafka_txn_curr_api_set_result`, which takes the
// local `error`); every return is `HANDLED`. The op is only read (`rko_err`,
// `rko_u.txn`): borrowed. `rk` and `rkq` are only read: borrowed."""

TXN = [
    ("rd_kafka_txn_op_init_transactions", "1258-1298",
     "starts the idempotent producer (`rd_kafka_idemp_start`) in `WAIT_PID`"),
    ("rd_kafka_txn_op_ack_init_transactions", "1308-1328",
     "moves `READY_NOT_ACKED` to `READY`"),
    ("rd_kafka_txn_op_begin_transaction", "1413-1449",
     "moves `READY` to `IN_TRANSACTION`, frees the previous `txn_errstr` and wakes the broker threads"),
    ("rd_kafka_txn_op_begin_commit", "2447-2476",
     "moves `IN_TRANSACTION` to `BEGIN_COMMIT`"),
    ("rd_kafka_txn_op_commit_transaction", "2339-2438",
     "sends `EndTxn` (commit) or completes an empty transaction (`rd_kafka_txn_endtxn_complete`), "
     "and records an abortable error"),
    ("rd_kafka_txn_op_commit_transaction_ack", "2485-2508",
     "completes the acknowledged commit (`rd_kafka_txn_complete`)"),
    ("rd_kafka_txn_op_begin_abort", "2608-2646",
     "moves to `BEGIN_ABORT` and clears the pending partitions under `txn_pending_lock`"),
    ("rd_kafka_txn_op_abort_transaction", "2655-2752",
     "sends `EndTxn` (abort), completes an empty transaction, or waits for a PID bump"),
    ("rd_kafka_txn_op_abort_transaction_ack", "2761-2784",
     "completes the acknowledged abort (`rd_kafka_txn_complete`)"),
]


def blocks():
    out = [INTRO]
    out.append(op_cb(
        "rd_kafka_broker_monitor_op_cb",
        """// `rd_kafka_broker_monitor_op_cb` (`rdkafka_broker.c:6413-6419`, static):
// unless `rko_err` is `RD_KAFKA_RESP_ERR__DESTROY`, calls the monitor callback
// `rko_u.broker_monitor.cb(rko_u.broker_monitor.rkb)`; returns `HANDLED`. The
// only callbacks registered (`rd_kafka_broker_monitor_add`) are
// `rd_kafka_coord_rkb_monitor_cb` (`rdkafka_broker.c:5497`; runs the
// coordinator request state machines, `rdkafka_coord.c:593-601`) and
// `rd_kafka_txn_coord_monitor_cb` (`rdkafka_txnmgr.c:3248`; starts a
// coordinator query or moves the idempotence state, `:3152-3187`), both
// read by hand: each receives only the broker. The op is only read:
// borrowed.""",
        premises=[
            'requires resource_premise(broker_monitor_callback, "the monitor callback in '
            "rko_u.broker_monitor.cb is rd_kafka_coord_rkb_monitor_cb or rd_kafka_txn_coord_monitor_cb, "
            "the only two rd_kafka_broker_monitor_add registers, which receive only the broker and take "
            'no custody of the op, its handle or its queue");',
        ],
    ))
    out.append(op_cb(
        "rd_kafka_cgrp_assignor_handle_Metadata_op",
        """// `rd_kafka_cgrp_assignor_handle_Metadata_op` (`rdkafka_cgrp.c:2140-2166`,
// static): `HANDLED` at once on `RD_KAFKA_RESP_ERR__DESTROY`, in a join state
// other than `WAIT_METADATA` or without group-leader members; otherwise runs
// the group's assignor over `rko_u.metadata.mdi`, which it only reads
// (`rd_kafka_cgrp_assignor_run`, `:2075`; the assign callback is one
// librdkafka registers: range, roundrobin or cooperative-sticky,
// `rd_kafka_assignor_add`), and returns `HANDLED`. The op is only read:
// borrowed.""",
        premises=[
            'requires resource_premise(assignor_callback, "the assign callback the group assignor '
            "runs is range, roundrobin or cooperative-sticky, the only ones rd_kafka_assignor_add "
            'registers, which read the metadata and members and take no custody of the op");',
        ],
    ))
    out.append(op_cb(
        "rd_kafka_cgrp_consumer_handle_Metadata_op",
        """// `rd_kafka_cgrp_consumer_handle_Metadata_op` (`rdkafka_cgrp.c:3056-3101`,
// static): `HANDLED` at once on `RD_KAFKA_RESP_ERR__DESTROY` or without a next
// target assignment; otherwise builds the assignment with metadata (a list it
// destroys itself) and returns
// `rd_kafka_cgrp_consumer_handle_next_assignment`'s result, `HANDLED` on
// every return (`:2918-2992`). The op is only read (`rko_err`): borrowed.""",
    ))
    out.append(op_cb(
        "rd_kafka_cgrp_handle_Metadata_op",
        """// `rd_kafka_cgrp_handle_Metadata_op` (`rdkafka_cgrp.c:2568-2582`, static):
// `HANDLED` at once on `RD_KAFKA_RESP_ERR__DESTROY`; otherwise, when the
// subscription version still matches, `rd_kafka_cgrp_metadata_update_check`;
// returns `HANDLED`. The op is only read: borrowed.""",
    ))
    out.append(op_cb(
        "rd_kafka_offset_reset_op_cb",
        """// `rd_kafka_offset_reset_op_cb` (`rdkafka_offset.c:767-777`, static): no
// `RD_KAFKA_RESP_ERR__DESTROY` test, so `rd_kafka_op_destroy` runs it in full.
// Locks the op's partition `rko_rktp`, runs a whole offset reset
// (`rd_kafka_offset_reset(rktp, broker_id, pos, rko_err, "%s", reason)`,
// `:792-891`: from another thread it enqueues a new reset op holding its own
// partition reference; otherwise it applies `auto.offset.reset`, may enqueue a
// consumer error op, moves the fetch state and may send an offset request),
// unlocks and returns `HANDLED`. The op and its partition reference are only
// read; nothing the op holds is released or moved: borrowed.""",
    ))
    out.append(op_cb(
        "rd_kafka_offset_validate_op_cb",
        """// `rd_kafka_offset_validate_op_cb` (`rdkafka_offset.c:1098-1106`, static):
// no `RD_KAFKA_RESP_ERR__DESTROY` test. Locks `rko_rktp`, runs
// `rd_kafka_offset_validate(rktp, "%s", reason)` (`:1118-1225`: from another
// thread a new op with its own partition reference; otherwise it may move the
// fetch state and send an `OffsetForLeaderEpoch` request holding a new
// partition reference), unlocks and returns `HANDLED`. The op is only read:
// borrowed.""",
    ))
    out.append(op_cb(
        "rd_kafka_share_ack_commit_cb_register_op",
        """// `rd_kafka_share_ack_commit_cb_register_op` (`rdkafka.c:3243-3256`,
// static): no `RD_KAFKA_RESP_ERR__DESTROY` test. On a share consumer, copies
// `rko_u.share_ack_commit_cb_register.registered` into the handle's flag and
// logs; returns `HANDLED`. The op is only read: borrowed.""",
    ))
    out.append(op_cb(
        "rd_kafka_share_fetch_reply_op_cb",
        """// `rd_kafka_share_fetch_reply_op_cb` (`rdkafka.c:3432-3436`, static):
// `return rd_kafka_share_fetch_reply_op(rk, rko)` (`:3442-3610`). No early
// `RD_KAFKA_RESP_ERR__DESTROY` test. It reads the reply's `share_fetch` fields,
// marks the batches' errors, enqueues an acknowledgement callback op per batch
// for the application queue (each with a partition list it builds,
// `rd_kafka_share_enqueue_ack_commit_cb_op`), updates the consumer group's and
// the target broker's counters and flags, and may enqueue new share fetch ops
// to brokers (each holding its own broker reference) or apply a commit-sync
// result; every return is `HANDLED`. Nothing the op holds is released or
// moved (`rd_kafka_op_destroy`'s `SHARE_FETCH` arm releases `ack_details` and
// `target_broker` afterwards): borrowed.""",
    ))
    out.append(TXN_HEAD)
    for name, lines, effect in TXN:
        out.append(op_cb(
            name,
            f"// `{name}` (`rdkafka_txnmgr.c:{lines}`, static): {effect}.",
        ))
    out.append(serve(
        "rd_kafka_cgrp_op_serve",
        """// `rd_kafka_cgrp_op_serve` (`rdkafka_cgrp.c:7371-7566`, static), the consumer
// group queue's `rkq_serve` (`:500`): `opaque` is the group, only read. Every
// path hands the op on or destroys it: a terminated group destroys it; the
// `NAME`, `CG_METADATA`, `SUBSCRIBE`, `GET_SUBSCRIPTION`, `GET_ASSIGNMENT` and
// `GET_REBALANCE_PROTOCOL` arms reply (`rd_kafka_op_reply`: re-queued to its
// reply queue or destroyed); `OFFSET_FETCH` hands it to the request or its
// handler, `OFFSET_COMMIT` to the commit, `ASSIGN` to
// `rd_kafka_cgrp_handle_assign_op`, `TERMINATE` to
// `rd_kafka_cgrp_terminate0`; every other arm leaves it to the final
// `rd_kafka_op_destroy`. Every return is `HANDLED`: the op is taken on every
// return.""",
    ))
    out.append(serve(
        "rd_kafka_mock_cluster_op_serve",
        """// `rd_kafka_mock_cluster_op_serve` (`rdkafka_mock.c:3158-3184`, static), the
// mock cluster queue's `rkq_serve` (`:3356`): `opaque` is the mock cluster.
// `TERMINATE` clears its run flag, `MOCK` runs the command
// (`rd_kafka_mock_cluster_cmd`, `:2898`, which only reads the op's
// `rko_u.mock`; closing a mock broker's connections may call
// `rk_conf.closesocket_cb`, which Fluent Bit never registers and librdkafka
// leaves NULL, `rdkafka_conf.c:741-746`), then `rd_kafka_op_reply(rko, err)`
// takes the op; returns `HANDLED`: the op is taken on every return.""",
        premises=[
            'requires resource_premise(rk_closesocket_callback, "closing a mock connection may call '
            "rk_conf.closesocket_cb, which Fluent Bit never registers and librdkafka leaves NULL, so "
            'close runs instead, until OP-APP-RK-CALLBACKS-VIA-POLL-CB closes the rk_conf callbacks");',
        ],
    ))
    return out


def main():
    text = MODULE.read_text()
    section = BEGIN + "\n\n" + "\n\n".join(blocks()) + "\n\n" + END + "\n"
    if BEGIN in text:
        start = text.index(BEGIN)
        end = text.index(END, start) + len(END) + 1
        text = text[:start] + section + text[end:]
    else:
        at = text.index(AFTER) + len(AFTER)
        text = text[:at] + "\n" + section + text[at:]
    MODULE.write_text(text)
    print(f"wrote {MODULE.relative_to(REPO)}")


if __name__ == "__main__":
    main()
