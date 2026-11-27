#!/usr/bin/env python3
"""update_ledger.py -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `OP-APP-RK-CALLBACKS-VIA-POLL-CB` (its DISCHARGE holds: the user's
  decision on a return-code split beside member invocations, then
  rd_kafka_poll_cb stated, its body read at both revisions, measured on the
  served-queue callers).
* Amended: `OP-CALLBACK-CANDIDATE-CONTRACTS` (its KEEP-only residuals: one
  declared, two named as shape (b)).
* Registered: the caller-side reading of a direct call of a hand-off
  declaration, and a hand-off candidate read uncontracted for its own
  callback clauses or reached through a local copy of the member.
* Unchanged: `SERVE0-CALLBACK-HAND-ON-WITHDRAWN` (no rd_kafka_op_handle
  declaration falls out)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "SUPPLIED-OWNED-RETURN-CODE-HANDOFF"
SOURCE = "docs/reports/supplied-owned-return-code-handoff/README.md"
RUN = ("corpus 66910c10 on main 10eb7692d: e93a68bd 936/9/3,889 -> 99d6362f 936/9/3,889, no "
       "status move, 4 Unknown rows trade one leaf each (the direct calls of rd_kafka_poll_cb: "
       "call to uncontracted function -> callback-handoff-direct), binary a0 b9f13b1b against "
       "before 6e5ecc58; system 93/4/351 and slice 52/1/347/4 Pending no row moves; null rows "
       "identical on all three")

RETIRE = ["OP-APP-RK-CALLBACKS-VIA-POLL-CB"]

AMEND = {
    "OP-CALLBACK-CANDIDATE-CONTRACTS": {
        "title": "Eight op-callback candidates stay undeclared: they release or move out what the op's members hold",
        "why": (
            f"Amended by {CARD} ({RUN}). The user's decision of 2026-10-15 states a hand-off "
            "by return code (`quant live #rko: #owner |-> #callback when #result == "
            "@RD_KAFKA_OP_RES_KEEP;`, C04.19.5, rule 12m with the caller branch check), so the "
            "KEEP-only shape (a) has a clause: rd_kafka_txn_op_send_offsets_to_transaction is "
            "declared with it (body read at both revisions, pinned with rdkafka_op.h's "
            "enumeration; the check admits it at both serve loops' rko_op_cb and at "
            "rd_kafka_op_destroy). The other two KEEP-only candidates are shape (b) as well and "
            "stay undeclared: rd_kafka_admin_worker (on HANDLED it destroys "
            "rko_u.admin_request.eonce and, through rd_kafka_admin_result_fail -> "
            "rd_kafka_admin_result_new, moves rko_u.admin_request.args and fanout_parent into a "
            "result op) and rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb (on "
            "HANDLED it moves rko_u.leaders.partitions into the reply and disables and clears "
            "rko_u.leaders.eonce). Remaining, all shape (b), releasing or moving out an object "
            "at a member path of the borrowed op, which no declaration states: those two, "
            "rd_kafka_admin_fanout_worker, rd_kafka_DeleteRecords_leaders_queried_cb, "
            "rd_kafka_ListOffsets_leaders_queried_cb, rd_kafka_share_commit_async_fanout_op, "
            "rd_kafka_share_fetch_fanout_op, rd_kafka_share_commit_sync_fanout_op. With "
            "rd_kafka_oauthbearer_refresh_op's own callback clause (OP-CALLBACK-CANDIDATES-APP-"
            "CALLBACKS), rd_kafka_op_destroy's 131 leaves stay callback-uncontracted (first named "
            "rd_kafka_DeleteRecords_leaders_queried_cb). DISCHARGE: the user's decision on a "
            "clause for a release or move-out of an object at a member path of a borrowed formal "
            "(as C04.19.2's arms state for the destroyer), each of the eight then declared or "
            "named as refused with its body read at both revisions, measured on "
            "rd_kafka_op_destroy's callers."
        ),
    },
}

REGISTER = [
    {
        "id": "HANDOFF-DIRECT-CALL-READING",
        "title": "A direct call of a hand-off declaration is refused: the caller's branch on the result is not read at the call",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 12m reads a hand-off (`quant live #P: #owner "
            "|-> #callback when #result OP @C;`) only as a candidate of a callback clause; a "
            "direct call refuses (callback-handoff-direct), because the caller's own state "
            "would hold the object on one class of results and not the other, which the per-path "
            "custody state does not split on a result value outside C04.13's signed "
            "dispositions. Size: 4 corpus rows, 4 leaves, each the row's only change (the four "
            "direct calls of rd_kafka_poll_cb). By reading: rd_kafka_consume0 (rdkafka.c:5045; "
            "`== PASS` breaks and keeps the op, every other result leaves it) and "
            "rd_kafka_consumer_close0 (:5292; `== PASS` destroys it) agree with the not-PASS "
            "hand-off; rd_kafka_q_serve_rkmessages (rdkafka_queue.c:803) keeps the op for every "
            "value outside KEEP, HANDLED and YIELD, and rd_kafka_background_queue_serve "
            "(rdkafka_background.c:90) destroys it on every result but HANDLED and YIELD, KEEP "
            "included, so those two would refuse. DISCHARGE: a reading of a direct call of a "
            "hand-off declaration that splits the caller's state on the guard at the caller's "
            "own branch on the result (the caller branch check's reading, applied in the "
            "calling body), with Lean parity, measured on the four rows."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "HANDOFF-CANDIDATE-OWN-CALLBACKS",
        "title": "rd_kafka_poll_cb's hand-off is never consulted as a candidate: its own callback clauses read uncontracted, and rko_serve is called through a local copy",
        "why": (
            f"Registered by {CARD} ({RUN}). rd_kafka_poll_cb is declared with its not-PASS "
            "hand-off and the twelve member invocations it makes (the rk_conf callbacks, the "
            "share consumer's acknowledgement callback, the op's rko_u.offset_commit.cb, "
            "rko_u.terminated.cb and rko_op_cb), so it is a retaining declaration, which rule "
            "13u reads as uncontracted where it is a candidate (4 corpus leaves name it "
            "callback-uncontracted: rd_kafka_offsets_for_times, rd_kafka_op_req, "
            "rd_kafka_op_req2, rd_kafka_query_watermark_offsets). The caller branch check, "
            "measured on the 94 librdkafka units (examples/handoff_census.rs), admits it at "
            "rd_kafka_q_serve0, rd_kafka_q_serve and rd_kafka_q_serve_maybe_consume and refuses "
            "it at the three pop serves (rd_kafka_q_pop_serve0 returns the op for every value "
            "outside HANDLED, KEEP and YIELD) and at the rd_kafka_op_s.rko_serve clause "
            "(rd_kafka_op_handle copies rko_serve into its callback formal before the call, "
            "which the path search does not follow). The served-queue callers refuse first at "
            "their access gate. DISCHARGE: a candidate whose declaration carries member "
            "invocations decided at the invocation as the composition of its own clauses "
            "(or recorded as refused), and a member copied into a local before its call followed "
            "by the check, measured on the serve callers."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in entries:
        if entry["id"] in AMEND:
            entry.update(AMEND[entry["id"]])
    known = {entry["id"] for entry in entries}
    for entry in REGISTER:
        if entry["id"] in known:
            for existing in entries:
                if existing["id"] == entry["id"]:
                    existing.update(entry)
        else:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(len(entries))


if __name__ == "__main__":
    main()
