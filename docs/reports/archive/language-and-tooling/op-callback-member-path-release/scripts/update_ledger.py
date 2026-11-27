#!/usr/bin/env python3
"""update_ledger.py -- OP-CALLBACK-MEMBER-PATH-RELEASE: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `OP-CALLBACK-CANDIDATE-CONTRACTS` (its DISCHARGE holds: the user's
  decision on a member-path clause, each of the eight then declared with its
  body read at both revisions, measured on rd_kafka_op_destroy's callers).
* Amended: `HANDOFF-CANDIDATE-OWN-CALLBACKS` (rd_kafka_op_destroy's 131
  leaves now name rd_kafka_oauthbearer_refresh_op alone).
* Registered: the member-path states the body check leaves as declared
  assumptions (cleared in static helpers of the unit, or in a body with a
  `goto`)."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "OP-CALLBACK-MEMBER-PATH-RELEASE"
SOURCE = "docs/reports/op-callback-member-path-release/README.md"
RUN = ("corpus 66910c10 on main 39287f12e: 99d6362f 936/9/3,889 -> 71387bd3 936/9/3,889, no "
       "status move, 83 Unknown rows trade one leaf text each (rd_kafka_op_destroy's 131 "
       "callback-uncontracted leaves name rd_kafka_oauthbearer_refresh_op instead of "
       "rd_kafka_DeleteRecords_leaders_queried_cb), binary a2 d2d91163 against before 2260bdae; "
       "system 93/4/351 and slice 52/1/347/4 Pending no row moves; null rows identical on all three")

RETIRE = ["OP-CALLBACK-CANDIDATE-CONTRACTS"]

AMEND_APPEND = {
    "HANDOFF-CANDIDATE-OWN-CALLBACKS": (
        f" Amended by {CARD} ({RUN}): the eight op-callback candidates of shape (b) are "
        "declared with member-path states (C04.19.6, rule 30), so rd_kafka_oauthbearer_refresh_op "
        "is the one rko_op_cb candidate of the run's 27 that reads uncontracted (its own member "
        "invocation of rd_kafka_s.rk_conf.sasl.oauthbearer.token_refresh_cb): all 131 of "
        "rd_kafka_op_destroy's leaves (83 functions, none decided by them alone) name it."
    ),
}

REGISTER = [
    {
        "id": "MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS",
        "title": "Three member-path declarations stay declared assumptions: their bodies clear the paths in static helpers or behind a goto",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 30's body check validates a member-path state "
            "from the declaration's own body: a top-level null store before every return (no "
            "goto or label) for #released / #moved, one null store for a set, no store for "
            "#stale. Five of the eight librdkafka declarations validate (rd_kafka_admin_fanout_worker, "
            "rd_kafka_ListOffsets_leaders_queried_cb, the three share fan-outs; partly-validated, "
            "evidence 30). Three stay declared assumptions, each body read at both revisions: "
            "rd_kafka_admin_worker (its HANDLED paths clear rko_u.admin_request.eonce in "
            "rd_kafka_admin_common_worker_destroy and move args, fanout_parent and replyq.q in "
            "rd_kafka_admin_result_new, rd_kafka_admin_result_enq -> rd_kafka_replyq_enq and "
            "rd_kafka_admin_fanout_op_distribute, all static in rdkafka_admin.c or inline in "
            "rdkafka_queue.h), rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb "
            "(a wrapper returning the static rd_kafka_topic_partition_list_query_leaders_async_worker, "
            "which clears eonce under `if (eonce)` and moves partitions under `if (replyq.q)`) and "
            "rd_kafka_DeleteRecords_leaders_queried_cb (its stale opaque is never stored, but the "
            "body has `goto err`). Size: 3 declarations; no row moves on any of them (every "
            "rd_kafka_op_destroy leaf names rd_kafka_oauthbearer_refresh_op). DISCHARGE: the body "
            "check follows a static callee of the unit called with the formal (and a structured "
            "goto), each of the three then validated or its residual named, measured."
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
        extra = AMEND_APPEND.get(entry["id"])
        if extra and extra not in entry["why"]:
            entry["why"] += extra
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
