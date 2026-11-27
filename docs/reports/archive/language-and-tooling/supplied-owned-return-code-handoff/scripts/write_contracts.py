#!/usr/bin/env python3
"""write_contracts.py -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF: (1) migrate the
one `|-> absent` spelling of `qcore-provider-librdkafka/contracts/librdkafka/
client.qv` (`rd_kafka_op_new0`, and the OP-CONSTRUCTOR-DECLARATIONS section's
comment naming it) to the value keyword `|-> #absent` (USER DECISION
2026-10-15: one spelling); (2) write the card's section (between its markers,
replacing a previous run's, after the OP-APP-RK-CALLBACKS-VIA-POLL-CB
section): the module alias `#PASS` and the two hand-off declarations,
`rd_kafka_poll_cb` (not-PASS) and `rd_kafka_txn_op_send_offsets_to_transaction`
(KEEP only), every body read by hand at `66910c10` and `046accf6`
(`scripts/body.py`, byte-identical; the report's census). Idempotent."""
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
MODULE = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
BEGIN = "// BEGIN SUPPLIED-OWNED-RETURN-CODE-HANDOFF (scripts/write_contracts.py)"
END = "// END SUPPLIED-OWNED-RETURN-CODE-HANDOFF"
AFTER = "// END OP-APP-RK-CALLBACKS-VIA-POLL-CB\n"

MIGRATIONS = [
    (
        "    quant live #result: rd_kafka_op_s.rko_op_cb |-> absent;\n",
        "    quant live #result: rd_kafka_op_s.rko_op_cb |-> #absent;\n",
    ),
    (
        "// `quant` state clause `quant live #result: TAG.FIELD |-> absent;` (the\n",
        "// `quant` state clause `quant live #result: TAG.FIELD |-> #absent;` (the\n",
    ),
]

PREMISE = (
    'requires resource_premise(rk_no_plugin_library, "P-RK-NO-PLUGIN-LIBRARY (user decision '
    "2026-10-14, U2): no plugin.library.paths is configured, so no library outside the run "
    "(rd_kafka_plugin_new's dlsym conf_init) registers a callback or an interceptor into a conf, "
    "and every rk_conf callback a call may run, the log callback rd_kafka_log_buf calls included, "
    'is one the run registers or a stated default, each declared (OP-APP-RK-CALLBACKS-VIA-POLL-CB)");'
)

SECTION = f"""{BEGIN}

// # Return-code hand-offs
//
// Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (gateway rules 12m and 13l,
// contract specification C04.19.5), on the user's decisions of 2026-10-15:
// `#` marks binders and value keywords, `@NAME` is a C enumerator Clang
// resolves (an enumerator of the guarded result's own enumeration, its
// enumeration pinned with the declaration at both revisions), a module alias
// is a second spelling of one `@` name, and
// `quant live #rko: #owner |-> #callback when #result != @NAME;` states that
// the handed op is the callback's exactly when the guard holds of the
// callback's result and stays with the caller otherwise. Rule 12m admits
// such a candidate only where the invoker's body branches on the result with
// the guard's own constant and never names the op where the guard holds (the
// caller branch check); a direct call of the declaration refuses. Every body
// was read at `66910c10` and `046accf6` (byte-identical, `scripts/body.py`
// of the card's report); each declaration is pinned (C06.2), its `@` names'
// enumeration (`rdkafka_op.h:255-264`) in its reviewed identity.
//
// Not declared, each named in the report: `rd_kafka_admin_worker` (KEEP on
// five waits, but on HANDLED it destroys `rko_u.admin_request.eonce` and moves
// `rko_u.admin_request.args` and `fanout_parent` into a result op) and
// `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` (KEEP while
// the eonce waits, but on HANDLED it moves `rko_u.leaders.partitions` into
// the reply and disables and clears `rko_u.leaders.eonce`): their HANDLED
// arms release or move out what the op's members hold, which no declaration
// states (`OP-CALLBACK-CANDIDATE-CONTRACTS`, shape (b)).

alias #PASS = @RD_KAFKA_OP_RES_PASS;

// `rd_kafka_poll_cb(rk, rkq, rko, cb_type, opaque)` (`rdkafka.c:5813-6096`),
// an `rko_serve` candidate and the callback the served-queue callers pass:
// `PASS` on seven paths, each leaving the op untouched (an event set up for
// the application, a fetch with no `consume_cb` or on a return call type, an
// offset commit or a share acknowledgement with no callback, a consumer error
// or an admin result on a return call type, a share-fetch response; the event
// set-up may replace `rko_u.err.errstr` in place and initialises
// `rko_u.dr.msgq2`, moving nothing out); every other result takes the op: it
// destroys a `HANDLED` op itself, `TERMINATE` and a yielded delivery report
// destroy or re-enqueue it (`YIELD`), `rd_kafka_consume_cb` destroys it
// (`HANDLED`), and an `rko_op_cb` reached through `rd_kafka_op_call` keeps it
// on `KEEP` or `YIELD`. The callbacks it invokes: the application's rk_conf
// callbacks (as `rd_kafka_background_queue_serve` states them, `consume_cb`
// through `rd_kafka_consume_cb`), the share consumer's acknowledgement
// callback, and the op's `rko_u.offset_commit.cb`, `rko_u.terminated.cb` and
// `rko_op_cb` (through `rd_kafka_op_call` and `rd_kafka_op_destroy`). Its own
// callback clauses make it a retaining declaration, which rule 13u reads as
// uncontracted where it is a candidate; a direct call refuses (rule 12m).
extern fn rd_kafka_poll_cb(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>, cb_type: u32, opaque: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_borrowed(cb_type);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.rebalance_cb, rk, _, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.error_cb, rk, _, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.dr_msg_cb, rk, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.dr_cb, rk, _, _, _, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.throttle_cb, rk, _, _, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.stats_cb, rk, _, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.log_cb, rk, _, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.consume_cb, _, _);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_share_consumer.share_acknowledgement_commit_cb, _, _, _, _);
    ensures normal resource_callback_member_invoked(rko, rd_kafka_op_s.rko_u.offset_commit.cb, rk, _, _, _);
    ensures normal resource_callback_member_invoked(rko, rd_kafka_op_s.rko_u.terminated.cb, rk, _);
    ensures normal resource_callback_member_invoked(rko, rd_kafka_op_s.rko_op_cb, _, _, rko);
    ensures normal resource_return_none();
    quant live #rko: #owner |-> #callback when #result != #PASS;
}};

// `rd_kafka_txn_op_send_offsets_to_transaction(rk, rkq, rko)`
// (`rdkafka_txnmgr.c:1935-1993`, static), an `rko_op_cb` candidate:
// `HANDLED` under `RD_KAFKA_RESP_ERR__DESTROY` and after a state or PID error
// (`rd_kafka_txn_curr_api_set_result` hands the error to `rk`); `KEEP` after
// `rd_kafka_AddOffsetsToTxnRequest(..., rko)` took the op as the request's
// opaque ("the rko is passed to AddOffsetsToTxn"). It reads
// `rko_u.txn.cgmetadata->group_id` and moves nothing out of the op.
extern fn rd_kafka_txn_op_send_offsets_to_transaction(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PREMISE}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_return_none();
    quant live #rko: #owner |-> #callback when #result == @RD_KAFKA_OP_RES_KEEP;
}};

{END}
"""


def main():
    text = MODULE.read_text()
    for old, new in MIGRATIONS:
        if old in text:
            text = text.replace(old, new)
        elif new not in text:
            raise SystemExit(f"migration anchor not found: {old!r}")
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
