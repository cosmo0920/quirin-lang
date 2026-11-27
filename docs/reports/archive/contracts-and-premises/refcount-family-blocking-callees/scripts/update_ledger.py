#!/usr/bin/env python3
"""REFCOUNT-FAMILY-BLOCKING-CALLEES: amend W45-CALLEE-CASCADE-RESIDUAL,
W48-CALLEE-DECISION-TABLE, REFCOUNT-REFERENCE-RESULT-BODY-SHAPES,
RETAINING-CALL-MEMBER-ACTUAL and CG22-CALLBACK-REGISTRY-CONTRACT; register the
residuals with sizes. The card has no ledger entry of its own (it was briefed
directly), so nothing is retired. Acceptance matrix, indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "REFCOUNT-FAMILY-BLOCKING-CALLEES"
MARK = "AMENDED (" + CARD
SOURCE = "docs/reports/refcount-family-blocking-callees/README.md"

RUN = ("corpus 66910c10 79dc429d 880/9/3,945 -> 36fd1649 881/9/3,944, base 652dc085f, binary "
       "f0646e94; system 87/4/357 and slice 45/1/354/4 Pending, no row moves")

AMEND = {
    "W48-CALLEE-DECISION-TABLE": (
        f" {MARK}, {RUN}, scripts/withdrawers.py and scripts/family_closure.py over "
        "artifacts/withdrawers-a2-corpus.json): class 3 gains a shape, a callee that KEEPS (or "
        "drops) a reference on the object it is handed and stores it in an owner it builds or "
        "reaches (an op's rko_rktp or rko_replyq, a forwarding queue, the desired list, the "
        "transaction list, a broker migration op). The brief forbids reading such a member as "
        "plain borrowed, and C04.17 has no clause for it (a reference clause cannot stand beside "
        "a retention of the same parameter, and rd_kafka_q_keep into an owner is not a "
        "retention: the object outlives the holder by the kept reference, not by the caller); "
        "inside each body the keep is REFCOUNT-KEEP-INTO-PLACE's shape (`rko->rko_rktp = "
        "rd_kafka_toppar_keep(rktp)`). "
        "Members, each body read at both revisions (withdrawer appearances in the 62 declared, "
        "withdrawn reference families / families it breaks first / families it is the only "
        "withdrawer of): rd_kafka_consumer_close_q 2/2/1, "
        "rd_kafka_share_fetch_reply_handle_partition_error 2/2/0, rd_kafka_op_set_replyq 2/1/0, "
        "rd_kafka_consume_stop0 1/1/1, rd_kafka_share_msgset_parse 20/0/0, rd_kafka_msgset_parse "
        "2/0/0, rd_kafka_fetch_reply_handle_partition_error 2/0/0, "
        "rd_kafka_fetch_preferred_replica_handle 2/0/0, rd_kafka_txn_add_partition 2/0/0, "
        "rd_kafka_toppar_enq_msg 2/0/0, rd_kafka_toppar_broker_update 1/0/0, "
        "rd_kafka_offset_validate 1/0/0, rd_kafka_op_req0 1/0/0 (13 callees; 6 families broken "
        "first; 1 function the reference route would decide if one member were declared alone, "
        "rd_kafka_consume_stop over rd_kafka_consume_stop0; 0 Safe). The relink rd_kafka_txn_partition_registered (moves rktp between transaction "
        "lists without a reference, 1/0/0) is a retention, not this shape. DISCHARGE: a user "
        "decision on a clause for a reference kept into an owner (or the caller-side reading "
        "that the caller's own references are untouched), measured."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f" {MARK}, {RUN}, scripts/rank.py, artifacts/rank-a2-corpus.json): the 523 / 367 "
        "reproduce. Four pinned declarations (rd_kafka_q_wait_result, rd_kafka_q_purge0, "
        "rd_kafka_toppar_needs_query, rd_kafka_toppar_fetch_backoff) free 1 "
        "(rd_kafka_toppar_purge_and_disable_queues, decided by rd_kafka_q_purge0 alone). "
        "Remaining: 522 Unknown functions blocked by uncontracted calls alone, 366 by one callee "
        "alone; system 26 / 18, slice 38 / 27 unchanged."
    ),
    "REFCOUNT-REFERENCE-RESULT-BODY-SHAPES": (
        f" {MARK}, {RUN}): one more member, rd_kafka_toppar_broker (rdkafka_partition.c:2716-2729): "
        "it returns rd_kafka_broker_keep(rktp->rktp_broker) under a condition, a returned broker "
        "reference of a member read (rule 13d's conditional-keep shape), which every caller drops "
        "through rd_kafka_broker_destroy, a macro over the rd_refcnt primitive no declaration "
        "reads. It breaks 1 declared toppar family first and alone "
        "(rd_kafka_topic_partition_available); 0 Safe. Undeclared."
    ),
    "RETAINING-CALL-MEMBER-ACTUAL": (
        f" {MARK}, {RUN}, artifacts/withdrawers-a2-corpus.json): the largest single breaker of "
        "declared reference families: 13 of the 62 withdrawn families are broken first at their "
        "own acquiring rd_kafka_q_new(X->f) (rd_kafka_q_new0 is withheld: it retains rk by its "
        "result), in rd_kafka_consume_stop0, rd_kafka_seek (2), rd_kafka_op_req and nine "
        "rdunittest_msgset_errors.c tests (response_q). Reading the member actual is not enough: "
        "rule 12t refuses a retained P that is a member read (retention-undecided), so each "
        "also needs a decided P. 3 more families are broken first by that retention-undecided "
        "at an acquiring rd_kafka_q_new0 (rd_kafka_share_commit_sync, rd_kafka_commit_queue 2)."
    ),
    "CG22-CALLBACK-REGISTRY-CONTRACT": (
        f" {MARK}, {RUN}): rd_kafka_q_wait_result and rd_kafka_q_purge0 are declared (borrowed "
        "rkq) under a stated served-queue premise: a popped op's own callback they may run "
        "(rd_kafka_op_handle -> rd_kafka_op_call; rd_kafka_op_destroy's ERR__DESTROY call) takes "
        "over no reference the caller holds on the served queue (constraint 4). The callbacks are "
        "not modelled; rd_kafka_q_serve (2 withdrawn families), rd_kafka_q_pop_serve (2) and "
        "rd_kafka_poll_cb (1), which take or are a callback, stay undeclared here."
    ),
}

NEW = [
    {
        "id": "REFCOUNT-BY-VALUE-AGGREGATE-ABI",
        "title": "Callees that take or return a C structure by value stay undeclarable, and rd_kafka_replyq_t carries a kept reference",
        "why": "Registered by " + CARD + " (" + RUN + "; scripts/withdrawers.py, "
               "scripts/family_closure.py over artifacts/withdrawers-a2-corpus.json). The "
               "declared-ABI reader projects scalars, pointers, va_list and enumerations (rule "
               "12u), never a structure passed or returned by value. Six of the twelve callees "
               "that break a declared reference family first are such callees: "
               "rd_kafka_toppar_op_seek (rd_kafka_fetch_pos_t and rd_kafka_replyq_t; 4 withdrawn "
               "families / 3 broken first), rd_kafka_ListOffsetsRequest (replyq; 2/2), "
               "rd_kafka_replyq_make (returns the replyq; 3/1), rd_kafka_broker_purge_queues "
               "(replyq; 1/1), rd_kafka_offset_store0 (pos; 1/1, the only withdrawer of "
               "rd_kafka_offset_store's family), rd_kafka_toppar_op_pause_resume (replyq; 1/1), "
               "rd_kafka_toppar_pid_change (rd_kafka_pid_t; 1/1), and later withdrawers "
               "rd_kafka_commit0 (replyq; 2/0), rd_kafka_msgset_create_ProduceRequest (pid; 1/0), "
               "rd_kafka_toppar_op_fetch_stop (replyq; 1/0). rd_kafka_replyq_t "
               "(RD_KAFKA_REPLYQ(q, v) is rd_kafka_replyq_make, rdkafka_queue.h:674-691) holds "
               "rd_kafka_q_keep(q): the callee's op takes that reference over, a transfer through "
               "a by-value aggregate C04.17 does not spell. Size: 10 callees, 10 families broken "
               "first, 1 function the reference route would decide (rd_kafka_offset_store), 0 "
               "Safe. DISCHARGE: an ABI projection of by-value records (CG14-AGGREGATE-VALUE-POSITIONS "
               "on the caller's side) and a decision on a reference carried inside a by-value "
               "record, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-Q-POP-OWNED-OP",
        "title": "rd_kafka_q_pop's owned op result is undeclared: declared, it publishes one leak on an infeasible path",
        "why": "Registered by " + CARD + " (" + RUN + "; first AFTER a1, binary d59b325d, "
               "corpus bddf4d59 881/19/3,934, artifacts/compare-a1-corpus.json). "
               "rd_kafka_q_pop(rkq, timeout_us, version) (rdkafka_queue.c:537-540, through the "
               "static rd_kafka_q_pop_serve0) borrows rkq and hands its caller the popped op or "
               "NULL: resource_borrowed(rkq) and resource_return_owned(nullable) state the body. "
               "Declared, it moved 10 Unknown rows to Unsafe and freed none: "
               "rd_kafka_metadata (rdkafka_metadata.c:155 to :111) on an infeasible path (the "
               "route takes the err == 0 edge of `if (err)` and then the err == "
               "RD_KAFKA_RESP_ERR__DESTROY_BROKER edge of the do-while; the two tests of err are "
               "not correlated), and nine rdunittest_msgset_errors.c tests whose RD_UT_ASSERT "
               "on a popped op's member returns 1 with the op live (a leak on a failing test's "
               "path only, provisional, never an upstream defect). It appears in 15 of the 62 "
               "withdrawn reference families, never first and never alone. Withdrawn before "
               "the measured AFTER. DISCHARGE: branch correlation of a variable tested twice "
               "with constants (or a no-leak reading of the unit-test assertion arm), then the "
               "declaration, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-FAMILY-NON-CALLEE-BREAKERS",
        "title": "Most declared reference families are withdrawn by a refusal that no contract removes",
        "why": "Registered by " + CARD + " (" + RUN + "; scripts/withdrawers.py, "
               "artifacts/withdrawers-a2-corpus.json, lexical and approximate). Of 127 plain-name "
               "reference-drop-unheld leaves, 65 have an origin no declaration covers (an "
               "undeclared call 35, a member read or list walk 26, a formal 4) and 62 a declared "
               "origin whose family a refused statement withdrew (rule 13b's refused hand-on). "
               "Their first withdrawer: an uncontracted callee 17 (12 callees: "
               "W48-CALLEE-DECISION-TABLE class 3 and REFCOUNT-BY-VALUE-AGGREGATE-ABI); the access "
               "gate at an acquiring rd_kafka_q_new(X->f) 13 (RETAINING-CALL-MEMBER-ACTUAL); an "
               "unresolved identity at an acquiring ut_create_mock_toppar(rkshare->...) 9; an "
               "unresolved identity in the null test of a name two acquisitions join "
               "(`rktp = rd_kafka_toppar_get(...); if (!rktp) rktp = "
               "rd_kafka_toppar_desired_get(...); if (unlikely(!rktp))`) 5; the access gate's "
               "address class at `mtx_lock(&(rktp)->rktp_lock)` (the rd_kafka_toppar_lock and "
               "rd_kafka_topic_*lock macros; W47-ACCESS-PATH-POSITION-RESIDUALS) 4, and 91 "
               "withdrawer appearances across all families, so a family with a contracted callee "
               "is withdrawn again at its next lock line; retention-undecided at an acquiring "
               "rd_kafka_q_new0 3; loop re-acquisition 3; an unresolved identity at "
               "rd_kafka_toppar_get2 2; a branch condition 2; others 4. The 11 "
               "reference-unbalanced leaves (10 functions) sit in the same functions or behind "
               "access-gate refusals in keep bodies. DISCHARGE: none by contracts; each tracker "
               "named above re-runs scripts/withdrawers.py on its arbiter.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    known = {entry["id"] for entry in entries}
    missing = [name for name in AMEND if name not in known]
    if missing:
        raise SystemExit("not registered: %s" % missing)
    for entry in entries:
        text = AMEND.get(entry["id"])
        if text and MARK not in entry["why"]:
            entry["why"] += text
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(entries))


if __name__ == "__main__":
    main()
