#!/usr/bin/env python3
"""update_ledger.py -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS` (its DISCHARGE holds:
  the body check follows a static callee of the unit called with the formal,
  and `goto`, each of the three then validated or its residual named,
  measured).
* Registered: `MEMBER-PATH-STATES-THROUGH-INDIRECT-AND-EXTERNAL-CALLS`, the
  two declarations whose stated paths a call the reading does not follow
  reaches."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS"
RUN = ("corpus 66910c10 on main 5b1ceecd5: 71387bd3 936/9/3,889 -> a2e95319 936/9/3,889, no "
       "row moves (statuses, leaves and every row's contract_dependencies identical; only the "
       "body evidence of rdkafka_admin.c and rdkafka_partition.c differs), binary a2 fd6e0ee2 "
       "against before2 33527d73; declarations 428: partly validated 45 -> 46, declared "
       "assumptions 319 -> 318; system 93/4/351 and slice 52/1/347/4 Pending byte-identical "
       "reports; null rows identical on all three")

RETIRE = ["MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS"]

REGISTER = [
    {
        "id": "MEMBER-PATH-STATES-THROUGH-INDIRECT-AND-EXTERNAL-CALLS",
        "title": "Two member-path declarations stay declared assumptions: a call rule 30's reading does not follow reaches their stated paths",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 30's body check now reads the body as a "
            "control-flow graph and follows the unit's static helpers (a header static inline "
            "included) that receive the formal or a member path's address; any other call that "
            "receives such a view leaves what it reaches unknown. "
            "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb (through its static "
            "worker and rd_kafka_replyq_enq) and rd_kafka_DeleteRecords_leaders_queried_cb "
            "(through `goto err`) are now partly validated. Two are declared assumptions, each body "
            "read at both revisions: rd_kafka_admin_worker (rd_kafka_admin_result_new moves "
            "rko_u.admin_request.args with rd_list_move, external in rdlist.c; "
            "rd_kafka_admin_response_parse passes the op to the indirect cbs->parse, and the "
            "worker passes &rko_u.admin_request.args to the indirect cbs->request; the reading "
            "first ends where rd_kafka_admin_result_fail copies rd_kafka_admin_result_new(rko_req), "
            "a pointer-typed call that received the op, into rko_result) and "
            "rd_kafka_admin_fanout_worker (a correction: partly validated before, the base check "
            "read no call; it passes the op to the indirect "
            "rko_fanout->rko_u.admin_request.fanout.cbs->partial_response after clearing "
            "rko_u.admin_result.fanout_parent; its parameter is const-qualified, which the "
            "reading does not trust). Size: 2 declarations; 4 Unknown corpus rows name them "
            "(rd_kafka_admin_worker, rd_kafka_admin_request_op_new, rd_kafka_admin_fanout_worker, "
            "rd_kafka_admin_result_new), no Safe row. DISCHARGE: "
            "the stated paths are validated through the indirect callees' closed candidate sets "
            "(rule 13u's registry or the member closure, each candidate's own reading) and a "
            "checked summary or contract for rd_list_move, or each residual named, measured."
        ),
        "source": "docs/reports/member-path-unit-helpers/README.md",
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = matrix["named_followups"]
    before = len(followups)
    followups[:] = [entry for entry in followups if entry["id"] not in RETIRE]
    known = {entry["id"] for entry in followups}
    for entry in REGISTER:
        if entry["id"] not in known:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(followups)}")


if __name__ == "__main__":
    main()
