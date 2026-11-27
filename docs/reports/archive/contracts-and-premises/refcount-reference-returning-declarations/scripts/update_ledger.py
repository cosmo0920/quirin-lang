#!/usr/bin/env python3
"""REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: retire the card's entry and the
absorbed RETAINED-BY-RESULT-HOLDER; amend REFCOUNT-INFERRED-REFERENCE-RESULT,
W48-CALLEE-DECISION-TABLE and W45-CALLEE-CASCADE-RESIDUAL; register the
residuals with sizes. Acceptance matrix, indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "REFCOUNT-REFERENCE-RETURNING-DECLARATIONS"
MARK = "AMENDED (" + CARD
SOURCE = "docs/reports/refcount-reference-returning-declarations/README.md"

RUN = ("corpus 66910c10 7a4595d6 884/9/3,941 -> 79dc429d 880/9/3,945, base e27f98d5b, binary "
       "a3398282; system 87/4/357 and slice 45/1/354/4 Pending, no row moves, parity multisets "
       "identical")

AMEND = {
    "W48-CALLEE-DECISION-TABLE": (
        f" {MARK}, {RUN}): class 3's reference-returning lookups and constructors are declared "
        "(gateway rules 13d, 13e; contract spec C04.17, C04.14.2; QCore.ReferenceResult): nine "
        "returned references (rd_kafka_toppar_get0, _desired_get, _new0, _desired_add, _get2, "
        "_get_avail, rd_kafka_q_new0, ut_create_mock_toppar, ut_create_mock_queue) and one drop "
        "(ut_destroy_mock_queue), each body read at both revisions and pinned; rd_kafka_q_new0 "
        "retains rk and func and rd_kafka_queue_new0 rk by the returned object "
        "(resource_retained_by(P, result)). No Unknown -> Safe; 4 Safe -> Unknown, each a named "
        "correction of REFCOUNT-RESOURCE-KIND (rd_kafka_queue_get_main, _get_consumer, "
        "rd_kafka_queue_get_sasl return a handle that retains their caller's rk; "
        "rd_kafka_share_queue_get_sasl loses its summary). Class 3 unchanged at 2 callees / 4 "
        "functions alone (scripts/tally.py over artifacts/rank-a2-corpus.json: rd_atomic32_add 2, "
        "rd_kafka_txn_clear_partitions_flag 2)."
    ),
    "W45-CALLEE-CASCADE-RESIDUAL": (
        f" {MARK}, {RUN}, scripts/rank.py, artifacts/rank-a2-corpus.json): the 522 / 365 "
        "reproduce; the declarations free none (every caller carries another blocker, "
        "artifacts/origins-before-corpus.json). Remaining: 523 Unknown functions blocked by "
        "uncontracted calls alone, 367 by one callee alone (+1 rd_kafka_share_queue_get_sasl, "
        "now blocked by rd_kafka_queue_get_sasl alone; +1 ut_create_mock_toppar, by "
        "rd_kafka_topic_new alone); system 26 / 18, slice 38 / 27 unchanged."
    ),
    "REFCOUNT-INFERRED-REFERENCE-RESULT": (
        f" {MARK}, {RUN}): not absorbed. No member needed it: every reference-returning body "
        "the card met is declared and checked by rule 13d instead of inferred. Size still 0 "
        "measured functions."
    ),
}

RETIRE = {"REFCOUNT-REFERENCE-RETURNING-DECLARATIONS", "RETAINED-BY-RESULT-HOLDER"}

NEW = [
    {
        "id": "REFCOUNT-REFERENCE-RESULT-BODY-SHAPES",
        "title": "Reference-returning lookups whose bodies rule 13d cannot validate stay undeclared",
        "why": "Registered by " + CARD + " (" + RUN + "; gateway rule 13d). Rule 13d admits a "
               "returned reference only where every return is a null constant, a declared keep or "
               "returned-reference call, or a local obtained from those or from a declared "
               "allocation. Not declared, by shape: rd_kafka_q_fwd_get and "
               "rd_kafka_topic_partition_get_toppar (a conditional keep, `if ((x = "
               "rkq->rkq_fwdq)) rd_kafka_q_keep(x); return x;`, of a member read or an undeclared "
               "lookup), the topic lookups rd_kafka_topic_find_fl, _find0_fl and "
               "_find_by_topic_id (a list walk kept at `break`, NULL when the walk ends), "
               "rd_kafka_topic_new0 and _new_with_id (long constructors with several exits), and "
               "rd_kafka_topic_proper (returns its argument on one path, a new topic on the other: "
               "not a reference on every path, so not declarable as one). Size "
               "(artifacts/origins-a2-corpus.json): 29 reference-drop-unheld leaves through a "
               "plain name from these origins, 25 functions, none alone; rd_kafka_toppar_get_by_id2 "
               "(a by-value rd_kafka_Uuid_t parameter the declared-ABI reader does not project) 0 "
               "leaves. DISCHARGE: a path-sensitive reading of the conditional keep and the list "
               "walk (or a body summary validated against the family balance), Lean, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "REFCOUNT-DROP-OWN-BODY-TRANSFER",
        "title": "A declared drop's own body does not read its parameter as the reference it took over",
        "why": "Registered by " + CARD + " (" + RUN + "; contract spec C04.17 'The transfer'). "
               "ut_destroy_mock_queue is declared resource_reference_dropped(rkq) (rule 13c "
               "validates its forward to rd_kafka_q_destroy_owner): its callers' references leave "
               "their families, and a later drop of an acquired family is an over-drop. Its own "
               "row still refuses its forward as reference-drop-unheld, because route G starts "
               "every body with its formals unbound. Size: 1 function (ut_destroy_mock_queue, "
               "decided alone). DISCHARGE: the body of a declared drop analysed with its "
               "parameter bound to one held reference of the caller's object (a family the body "
               "must drop or hand off), Lean, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "RETAINING-CALL-MEMBER-ACTUAL",
        "title": "The access gate does not read a member actual at a borrowed position of a withheld declaration",
        "why": "Registered by " + CARD + " (" + RUN + "). rd_kafka_queue_new0 left the function "
               "map when it was re-declared retaining rk by its result (rule 13e); its callers "
               "pass a member at the borrowed rkq position (`rk->rk_rep`, "
               "`rk->rk_cgrp->rkcg_q`, `rk->rk_sasl.callback_q`, `rk->rk_background.q`), which "
               "the access gate read for the function-map declaration "
               "(W44-CALL-ARGUMENT-CUSTODY-POSITION) and now refuses (`indirect memory access is "
               "outside the resource projection`). The three Safe -> Unknown corrections "
               "(rd_kafka_queue_get_main, _get_consumer, rd_kafka_queue_get_sasl) report that "
               "leaf; behind it the returned handle's retention of rk refuses at the exit, so "
               "none of them is freeable. Size: 4 corpus functions carry the leaf at a "
               "rd_kafka_queue_new0 call (rd_kafka_queue_get_background too), 0 freeable. "
               "DISCHARGE: the access gate reads a withheld declaration's borrowed positions as "
               "it reads the function map's, measured.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    known = {entry["id"] for entry in entries}
    for entry in entries:
        text = AMEND.get(entry["id"])
        if text and MARK not in entry["why"]:
            entry["why"] += text
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(entries))


if __name__ == "__main__":
    main()
