#!/usr/bin/env python3
"""write_contracts.py -- OP-CALLBACK-MEMBER-PATH-RELEASE: write the card's
section of `qcore-provider-librdkafka/contracts/librdkafka/client.qv`
(between its markers, replacing a previous run's, after the
SUPPLIED-OWNED-RETURN-CODE-HANDOFF section): the eight `rko_op_cb` candidates
of shape (b), each stating the state it leaves at member paths of the op
(USER DECISION 2026-10-15, "B: state + set"), every body read by hand at
`66910c10` and `046accf6` (`scripts/body.py`, byte-identical; the census in
PREDICTION.md section 2). No move destination is stated: where a moved object
goes is named in each declaration's comment only. Idempotent."""
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
MODULE = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
BEGIN = "// BEGIN OP-CALLBACK-MEMBER-PATH-RELEASE (scripts/write_contracts.py)"
END = "// END OP-CALLBACK-MEMBER-PATH-RELEASE"
AFTER = "// END SUPPLIED-OWNED-RETURN-CODE-HANDOFF\n"

PREMISE = (
    'requires resource_premise(rk_no_plugin_library, "P-RK-NO-PLUGIN-LIBRARY (user decision '
    "2026-10-14, U2): no plugin.library.paths is configured, so no library outside the run "
    "(rd_kafka_plugin_new's dlsym conf_init) registers a callback or an interceptor into a conf, "
    "and every rk_conf callback a call may run, the log callback rd_kafka_log_buf calls included, "
    'is one the run registers or a stated default, each declared (OP-APP-RK-CALLBACKS-VIA-POLL-CB)");'
)

ACK_PREMISE = (
    'requires resource_premise(share_ack_batches_nonempty, "P-SHARE-ACK-BATCHES-NONEMPTY: the '
    "op's rko_u.share_fetch_fanout.ack_batches is NULL or non-empty: its producer "
    "rd_kafka_share_build_ack_details (rdkafka_share_acknowledgement.c:686-842) creates the list "
    "only at its first rd_list_add (:822-826), and rd_kafka_share_fetch_fanout (rdkafka.c:3404) "
    "stores only that value; a non-NULL empty list with a selected broker would be cleared "
    'without its release (has_fanout_acks ? ack_batches : NULL, rdkafka.c:3801-3803)");'
)

KEEP = "@RD_KAFKA_OP_RES_KEEP"

SECTION = f"""{BEGIN}

// # Member-path releases of the op-callback candidates
//
// Card OP-CALLBACK-MEMBER-PATH-RELEASE (gateway rule 30, contract
// specification C04.19.6), on the user's decision of 2026-10-15 ("B: state +
// set"): `quant live #P: TAG.PATH |-> STATE;` states what the call leaves at
// a member path of the formal's object: `#released` / `#moved` (the object
// it held is released / handed to another holder, never the caller, and the
// member reads cleared), `#stale` (taken, still designated), `#kept` (the
// state of every path not named), or `{{V, #kept}}` (one of the two on each
// path). A guard, beside a hand-off only, is the hand-off's complement. Rule
// 30 composes each candidate's states with `rd_kafka_op_destroy`'s arms
// (C04.19.2): `rd_kafka_op_destroy` calls `rko_op_cb(rk, NULL, rko)` under
// `RD_KAFKA_RESP_ERR__DESTROY` before its switch, and every candidate below
// returns `HANDLED` there. Every body was read at `66910c10` and `046accf6`
// (byte-identical); every declaration is pinned (C06.2), the hand-offs'
// `@RD_KAFKA_OP_RES_KEEP` enumeration (`rdkafka_op.h:255-264`) in their
// reviewed identity. Every candidate reaches `rd_kafka_log_buf` (the
// application's log callback), so each names `rk_no_plugin_library`.

// `rd_kafka_admin_worker(rk, rkq, rko)` (`rdkafka_admin.c:1035-1262`,
// static; registered by `rd_kafka_admin_request_op_new`, `:632`): `KEEP` on
// the six waits (coordinator, broker, controller, broker list, request sent,
// the trailing return), when the eonce or the request holds the op; `HANDLED`
// only at its `destroy:` label, through `rd_kafka_admin_common_worker_destroy`
// (`:708`: `rd_kafka_enq_once_destroy` drops the op's reference on the
// counted eonce and clears the member, on every `HANDLED` path). Where a
// result is built (`rd_kafka_admin_result_fail` with a reply queue, or one of
// the 23 parse callbacks, each through `rd_kafka_admin_result_new`, `:341`)
// `rko_u.admin_request.args` moves into the result op (`rd_list_move`, the
// count set to 0), `fanout_parent` moves into the result op's
// `admin_result.fanout_parent` (cleared), and `rd_kafka_admin_result_enq`
// hands the result to `rd_kafka_replyq_enq`, which drops and clears
// `replyq.q`; at `WAIT_FANOUTS` the fan-out op is handed to the children
// (`rd_kafka_admin_fanout_op_distribute`, `:1337`, clears `fanout_parent`);
// with no reply queue the three are kept. `reply_buf` is not owned
// (`rdkafka_op.h:497`). Destinations (comment only): the result op enqueued on
// the reply queue; the children ops.
extern fn rd_kafka_admin_worker(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_return_none();
    quant live #rko: #owner |-> #callback when #result == {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.admin_request.eonce |-> #released when #result != {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.admin_request.args |-> {{#moved, #kept}} when #result != {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.admin_request.fanout_parent |-> {{#moved, #kept}} when #result != {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.admin_request.replyq.q |-> {{#released, #kept}} when #result != {KEEP};
}};

// `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb(rk, rkq,
// rko)` (`rdkafka_partition.c:4156-4161`, static; registered `:4192`) returns
// `rd_kafka_topic_partition_list_query_leaders_async_worker(rko)`
// (`:4030-4152`): `KEEP` while the re-enabled eonce waits for metadata;
// `HANDLED` at its `reply:` label (`__DESTROY` goes there directly):
// `rko_u.leaders.eonce` disabled (`rd_kafka_enq_once_disable`, the op's
// reference dropped) and cleared when set; where `replyq.q` is set a reply op
// takes `rko_u.leaders.partitions` (cleared) and a copy of
// `rko_u.leaders.opaque` (left designated), and `rd_kafka_replyq_enq` drops
// and clears `replyq.q`; with no reply queue both are kept. Destinations
// (comment only): the reply op's `rko_u.leaders.partitions` and `.opaque`.
extern fn rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_return_none();
    quant live #rko: #owner |-> #callback when #result == {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.leaders.eonce |-> #released when #result != {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.leaders.replyq.q |-> #released when #result != {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.leaders.partitions |-> {{#moved, #kept}} when #result != {KEEP};
    quant live #rko: rd_kafka_op_s.rko_u.leaders.opaque |-> {{#stale, #kept}} when #result != {KEEP};
}};

// `rd_kafka_admin_fanout_worker(rk, rkq, rko)` (`rdkafka_admin.c:1393-1450`,
// static; registered by `rd_kafka_admin_result_new`, `:350`, on the
// `ADMIN_RESULT` op of a fanned-out request): `HANDLED` only. It clears
// `rko_u.admin_result.fanout_parent` on every path and decrements the fan-out
// op's `outstanding`, destroying the fan-out op when it reaches 0: the op's
// share of the counted fan-out op is released. `partial_response` receives
// the op as `const rd_kafka_op_t *` (`:263`); nothing else of it is touched.
extern fn rd_kafka_admin_fanout_worker(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(rko);
    ensures normal resource_return_none();
    quant live #rko: rd_kafka_op_s.rko_u.admin_result.fanout_parent |-> #released;
}};

// `rd_kafka_DeleteRecords_leaders_queried_cb(rk, rkq, reply)`
// (`rdkafka_admin.c:4398-4487`, static; the reply's `rko_u.leaders.cb`,
// `:4559`): `HANDLED` only. The fan-out op at `rko_u.leaders.opaque` is
// released on an error (`__DESTROY` included: `rd_kafka_admin_common_worker_destroy(…,
// rd_true)`) and handed to the children (`fanout_parent`, counted by
// `outstanding`) otherwise; the member is never cleared. `leaders` and
// `partitions` are read only.
extern fn rd_kafka_DeleteRecords_leaders_queried_cb(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, reply: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(reply);
    ensures normal resource_return_none();
    quant live #reply: rd_kafka_op_s.rko_u.leaders.opaque |-> #stale;
}};

// `rd_kafka_ListOffsets_leaders_queried_cb(rk, rkq, reply)`
// (`rdkafka_admin.c:4294-4390`, static; registered `:4649`): as
// `rd_kafka_DeleteRecords_leaders_queried_cb`, the error path taken on any
// error.
extern fn rd_kafka_ListOffsets_leaders_queried_cb(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, reply: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(reply);
    ensures normal resource_return_none();
    quant live #reply: rd_kafka_op_s.rko_u.leaders.opaque |-> #stale;
}};

// `rd_kafka_share_commit_async_fanout_op(rk, rkq, rko)` (`rdkafka.c:4032-4052`;
// registered `:4377`): `HANDLED` only. A non-empty
// `rko_u.share_commit_async_fanout.ack_batches` is handed to
// `rd_kafka_share_segregate_and_dispatch_acks` (`:3689`: its elements to the
// brokers' ack details, the list `rd_list_destroy`ed) and cleared; otherwise
// it is kept.
extern fn rd_kafka_share_commit_async_fanout_op(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(rko);
    ensures normal resource_return_none();
    quant live #rko: rd_kafka_op_s.rko_u.share_commit_async_fanout.ack_batches |-> {{#released, #kept}};
}};

// `rd_kafka_share_fetch_fanout_op(rk, rkq, rko)` (`rdkafka.c:3745-3809`;
// registered `:3408`): `HANDLED` only. With no broker selected and no acks it
// returns keeping `rko_u.share_fetch_fanout.ack_batches`; otherwise it hands
// the list to `rd_kafka_share_segregate_and_dispatch_acks` when non-empty
// (released as above) and clears the member. A non-NULL empty list is
// cleared without its release, which the premise rules out.
extern fn rd_kafka_share_fetch_fanout_op(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    {ACK_PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(rko);
    ensures normal resource_return_none();
    quant live #rko: rd_kafka_op_s.rko_u.share_fetch_fanout.ack_batches |-> {{#released, #kept}};
}};

// `rd_kafka_share_commit_sync_fanout_op(rk, rkq, rko)` (`rdkafka.c:4251-4330`;
// registered `:4466`): `HANDLED` only; on entry it clears
// `rko_u.share_commit_sync_fanout.ack_batches` and `.results` ("Take
// ownership from op") and, before any return, `rko_replyq.q` ("Ownership
// transferred to rkcg"). The list is released (`RD_IF_FREE` with no acks, or
// `rd_kafka_share_segregate_sync_acks_by_leader`, which pops its elements to
// the brokers and destroys it). Destinations (comment only): `results` and
// the reply queue go to `rkcg->rkcg_commit_sync_request`, reached from `rk`.
extern fn rd_kafka_share_commit_sync_fanout_op(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(rko);
    ensures normal resource_return_none();
    quant live #rko: rd_kafka_op_s.rko_u.share_commit_sync_fanout.ack_batches |-> #released;
    quant live #rko: rd_kafka_op_s.rko_u.share_commit_sync_fanout.results |-> #moved;
    quant live #rko: rd_kafka_op_s.rko_replyq.q |-> #moved;
}};

{END}
"""


def main():
    text = MODULE.read_text()
    if BEGIN in text:
        start = text.index(BEGIN)
        end = text.index(END, start) + len(END) + 1
        text = text[:start] + text[end:].lstrip("\n")
    if AFTER not in text:
        raise SystemExit("anchor section not found")
    at = text.index(AFTER) + len(AFTER)
    text = text[:at] + "\n" + SECTION + text[at:]
    MODULE.write_text(text)
    print("wrote", MODULE)


if __name__ == "__main__":
    main()
