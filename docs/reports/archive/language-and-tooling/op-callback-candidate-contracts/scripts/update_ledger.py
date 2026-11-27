#!/usr/bin/env python3
"""update_ledger.py -- OP-CALLBACK-CANDIDATE-CONTRACTS: the card's own entry
is NOT retired (its DISCHARGE asks for "a contract in force for every
candidate of both closures"; 19 of the 30 non-poll candidates are declared and
9 of the other 11 need vocabulary or a decision), so it is amended in place to
the nine residual candidates that need a clause; the two that call an
application callback, the unlisted member-candidate dependencies and the
unstated log-callback premise of the earlier librdkafka declarations are
registered with their own DISCHARGE. Acceptance matrix, indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "OP-CALLBACK-CANDIDATE-CONTRACTS"
SOURCE = "docs/reports/op-callback-candidate-contracts/README.md"
RUN = ("corpus 66910c10 ec4ccfeb 936/9/3,889 -> 0a38c3b3 936/9/3,889, no status move, binary "
       "bcb78670 reading the card's contracts, system 93/4/351 and slice 52/1/347/4 Pending, no "
       "row moves, null verdicts identical; base main 2afce9973")

AMENDED = {
    "id": CARD,
    "title": "Nine op-callback candidates stay undeclared: they take the op on KEEP only, or release or move out what the op's members hold",
    "why": (
        f"Amended by {CARD} ({RUN}). The card stated the op callback protocol (PASS: the op "
        "stays with rd_kafka_op_handle; HANDLED: the handler destroys it, the callback borrowed "
        "it; KEEP and YIELD: the callback took it; rd_kafka_op_destroy's __DESTROY call asserts "
        "neither, so there every candidate borrows; an rko_serve callback owns the op on every "
        "result but PASS) and declared 19 of the 30 non-poll candidates (17 rko_op_cb identities, "
        "rd_kafka_cgrp_op_serve and rd_kafka_mock_cluster_op_serve taking the op), each a "
        "declared assumption with its body read at both revisions and pinned. Still undeclared: "
        "(a) taking the op on KEEP only, which rule 12m refuses and C04.13 cannot state over the "
        "unsigned rd_kafka_op_res_t (u32): rd_kafka_admin_worker, "
        "rd_kafka_txn_op_send_offsets_to_transaction, "
        "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb (each returns HANDLED "
        "on __DESTROY, never KEEP); (b) releasing or moving out an object the op's members hold, "
        "which a plain declaration cannot state: rd_kafka_admin_fanout_worker (destroys the "
        "fanout parent), rd_kafka_DeleteRecords_leaders_queried_cb and "
        "rd_kafka_ListOffsets_leaders_queried_cb (destroy the fanout op in rko_u.leaders.opaque "
        "on an error), rd_kafka_share_commit_async_fanout_op and rd_kafka_share_fetch_fanout_op "
        "(hand ack_batches on and clear it), rd_kafka_share_commit_sync_fanout_op (moves "
        "ack_batches, results and the op's reply-queue reference into the group). No unit's own "
        "closed set holds any of the nine (they are registered through rd_kafka_op_new_cb), so "
        "no leaf names them: 0 leaves; every rd_kafka_op_destroy call still refuses or is "
        "demoted (118 callback-member-open leaves; the run's 27 rko_op_cb candidates are in no "
        "unit's set). DISCHARGE: the user's decision on a clause for (a) a callback's custody "
        "under its destroy-time call apart from its serve-time return codes and (b) a release or "
        "move-out of an object at a member path of a borrowed formal (as C04.19.2's arms state "
        "for the destroyer), each of the nine then declared or named as refused with its body "
        "read at both revisions, measured on rd_kafka_op_destroy's callers."
    ),
}

NEW = [
    {
        "id": "OP-CALLBACK-CANDIDATES-APP-CALLBACKS",
        "title": "Two op-callback candidates call an application callback, and the 19 declared ones rest on the rk_log_callback premise",
        "why": (
            f"Registered by {CARD} ({RUN}). rd_kafka_oauthbearer_refresh_op (rko_op_cb) calls "
            "rk_conf.sasl.oauthbearer.token_refresh_cb, which Fluent Bit registers "
            "(src/aws/flb_aws_msk_iam.c:775), and rd_kafka_toppar_op_serve (rko_serve) calls "
            "rko_u.offset_commit.cb (rk_conf.offset_commit_cb or the callback "
            "rd_kafka_commit_queue is handed) and reaches rk_conf.open_cb through the offset "
            "file; both stay undeclared. The 19 declared candidates carry the premise "
            "rk_log_callback (rd_kafka_log0 -> rd_kafka_log_buf calls rk_conf.log_cb, Fluent "
            "Bit's out_kafka cb_kafka_logger with no log queue), and "
            "rd_kafka_mock_cluster_op_serve also rk_closesocket_callback (never registered: "
            "NULL). Sizes: 0 leaves name the two (no unit's own set holds them); the premise is "
            "published on 19 declarations. DISCHARGE: after OP-APP-RK-CALLBACKS-VIA-POLL-CB "
            "contracts the rk_conf callbacks Fluent Bit registers (the user's decision of "
            "2026-10-14: closed over Fluent Bit's registrations), the two candidates declared "
            "against those contracts and the rk_log_callback and rk_closesocket_callback "
            "premises discharged or restated against them, each body read at both revisions, "
            "measured."
        ),
    },
    {
        "id": "MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED",
        "title": "A row deciding a member invocation does not list the candidates whose contracts it read",
        "why": (
            f"Registered by {CARD} ({RUN}). contract_dependencies::row_dependencies counts a "
            "lookup for a function only when its body spells the name; the candidates of a "
            "member invocation (rule 12n) are read from the unit's member facts, never spelled "
            "by the caller (rd_kafka_q_purge0 never names rd_kafka_offset_reset_op_cb), so a row "
            "deciding through them does not list them, nor their premises (rk_log_callback), "
            "among its dependencies. Each candidate's own row lists its declaration, so the "
            "sidecar publishes its standing. Size on this run: 0 Safe or Pending rows (the fold "
            "demotes every row whose unit's set does not hold the run's, and no unit's does); "
            "the 8 rows whose op_destroy calls now decide (rdkafka.c, rdkafka_offset.c) are "
            "Unknown. DISCHARGE: the route records every candidate whose contract decided an "
            "invocation (member, synchronous, table, registry) as a dependency of the deciding "
            "function, with the candidates' premises published on the row, and a test that a "
            "row resting on a member decision names them, measured."
        ),
    },
    {
        "id": "LIBRDKAFKA-LOG-CALLBACK-PREMISE-UNSTATED",
        "title": "Earlier librdkafka declarations whose bodies log reach rk_conf.log_cb without naming it",
        "why": (
            f"Registered by {CARD} ({RUN}). rd_kafka_log_buf (rdkafka.c:280-305) calls "
            "rk_conf.log_cb when no log queue is set, and Fluent Bit's out_kafka registers "
            "cb_kafka_logger with no log queue (plugins/out_kafka/kafka_config.c:111): every "
            "librdkafka body that logs (rd_kafka_dbg, rd_kafka_log) may call an application "
            "callback. The card's 19 candidate declarations state it as the premise "
            "rk_log_callback; the librdkafka declarations already in force (qcore-provider-"
            "librdkafka, client.qv and the other modules) do not. DISCHARGE: either every such "
            "declaration names the premise (pinned again at both revisions) or the logger's "
            "contract from OP-APP-RK-CALLBACKS-VIA-POLL-CB is shown to take no custody and the "
            "premise is retired everywhere, measured (expected: no verdict moves; dependency "
            "premises only)."
        ),
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    for index, entry in enumerate(followups):
        if entry["id"] == CARD:
            followups[index] = {**entry, "title": AMENDED["title"], "why": AMENDED["why"],
                                "source": SOURCE}
            break
    else:
        raise SystemExit(f"{CARD} is not in the ledger")
    known = {entry["id"] for entry in followups}
    for entry in NEW:
        record = {"id": entry["id"], "title": entry["title"], "why": entry["why"],
                  "source": SOURCE, "recorded_by": CARD}
        if entry["id"] in known:
            followups = [record if e["id"] == entry["id"] else e for e in followups]
        else:
            followups.append(record)
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
