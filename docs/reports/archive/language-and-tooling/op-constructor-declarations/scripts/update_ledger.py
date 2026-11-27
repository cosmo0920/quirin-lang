#!/usr/bin/env python3
"""update_ledger.py -- OP-CONSTRUCTOR-DECLARATIONS: retire the card's own
entry (its DISCHARGE, "rd_kafka_op_new0, rd_kafka_op_new_cb (a registration of
cb in the result's rko_op_cb, rule 13w) and the rd_kafka_op_req* constructors
declared, with a clause stating the result's discriminant from the decided
type actual (rule 12o's decided arm, the discriminant's run-wide stability
checked as nonnull_member_stable is), each body read at both revisions,
measured", holds) and register the residuals with their sizes. Acceptance
matrix, indent=1, ensure_ascii=False; `dimensions.named_followups` = list
length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "OP-CONSTRUCTOR-DECLARATIONS"
SOURCE = "docs/reports/op-constructor-declarations/README.md"
RUN = ("corpus 66910c10 527a4cad 935/9/3,890 -> ec4ccfeb 936/9/3,889 (one Unknown -> Safe, "
       "read by hand), binary 47492b59, system 93/4/351 and slice 52/1/347/4 Pending, no row "
       "moves, null verdicts identical; base main a7e7e8cee")

NEW = [
    {
        "id": "OP-CONSTRUCTORS-UNDECLARED-RESIDUAL",
        "title": "Three op constructors stay undeclared: rd_kafka_op_new_reply, _new_fetch_msg, _new_ctrl_msg",
        "why": (
            f"Registered by {CARD} ({RUN}). The card declared rd_kafka_op_new0, rd_kafka_op_new_cb "
            "and the request family; the other op constructors stay undeclared, and an op they "
            "create is untracked: rd_kafka_op_new_reply 2 leaves / 1 function "
            "(rdkafka_op.c:770-783: rd_kafka_op_new(rko_orig->rko_type | RD_KAFKA_OP_REPLY), a "
            "computed type actual, so its discriminant would stay undecided), "
            "rd_kafka_op_new_fetch_msg 1/1 and rd_kafka_op_new_ctrl_msg 1/1 (corpus, AFTER). "
            "DISCHARGE: each declared (an owned result; the discriminant stated where the body "
            "decides it; members stated absent only where rule 14e's body check validates them), "
            "each body read at 66910c10 and 046accf6, pinned, measured."
        ),
    },
    {
        "id": "OP-NEW-CB-RK-RETENTION",
        "title": "rd_kafka_op_new_cb's op keeps rk: seven callers' rk lifetimes are undecided",
        "why": (
            f"Registered by {CARD} ({RUN}). rd_kafka_op_new_cb stores rk in the op's rko_rk "
            "(rdkafka_op.c:791, a plain pointer, no reference), declared "
            "resource_retained_by(rk, result) (C04.14.2). Every caller hands rk as a formal or a "
            "member read (rkcg->rkcg_rk), so the retention is undecided at the call: 3 leaves "
            "'call to rd_kafka_op_new_cb retains rk ... (retention-undecided)' "
            "(rd_kafka_share_commit_async, rd_kafka_share_commit_sync, "
            "rd_kafka_share_set_acknowledgement_commit_cb) and 4 'a holder retaining rk may "
            "outlive this function's frame (retention-undecided)' (rd_kafka_admin_request_op_new, "
            "rd_kafka_send_offsets_to_transaction, rd_kafka_share_fetch_fanout, "
            "rd_kafka_topic_partition_list_query_leaders_async), each row also refused for other "
            "reasons. DISCHARGE: a stated, checked premise that the client handle outlives every "
            "op it creates (rd_kafka_destroy drains the op queues before freeing rk), or a "
            "per-caller proof, measured on these callers."
        ),
    },
    {
        "id": "OP-STATE-FOLD-PER-ROW-READS",
        "title": "The result-state fold demotes every row resting on a constructor, not only rows that read a state",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 14e's run fold "
            "(qcore_audit::callback_member_fold, constructed_states) demotes a Safe or Pending row "
            "whose contract dependencies name a constructor whose stated member the run's stores "
            "do not keep, whether or not the row read the per-path state: sound and conservative. "
            "On this run both stated members (rd_kafka_op_s.rko_op_cb, rd_kafka_op_s.rko_type) "
            "are kept and nothing is demoted; on the a3 build (before the sub-object and "
            "null-rebinding classification) one row, ut_create_fetch_op_with_offset, which reads "
            "no state, was demoted this way. DISCHARGE: the unit records which rows a per-path "
            "state decided (by declaration and member), and the fold demotes exactly those, with "
            "a Lean statement that a row reading no state is unaffected by the member's "
            "stability, measured."
        ),
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = [entry for entry in matrix["named_followups"] if entry["id"] != CARD]
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
