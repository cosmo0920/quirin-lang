#!/usr/bin/env python3
"""update_ledger.py -- LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED` (its DISCHARGE: the user
  decided, the null views are ruled independent except the scoped member
  forms, which name the premise, and the model contract names it, measured)
  and `LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED` (the user decided, both name
  the premise, re-pinned, measured; rd_kafka_toppar_get_avail keeps it).
* Registered: `LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED`, rule 14t's five
  plain `nonnull_member` clauses, which the census's rule would premise but
  decision 1 does not admit the clause beside."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER"
SOURCE = "docs/reports/librdkafka-plugin-premise-remainder/README.md"
RETIRE = ["LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED", "LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED"]
RUN = ("corpus 66910c10 on main 711287771: 82214412 936/9/3,889 -> 82214412 (byte-identical report), "
       "null rows identical apart from the premise fields and the bounds-environment digest; binary a1 "
       "b11509e5 against before 352276f2 (built from an exported base tree); system 93/4/351 and slice "
       "52/1/347/4 Pending unchanged")

REGISTER = [
    {
        "id": "LIBRDKAFKA-NONNULL-MEMBER-PREMISE-UNADMITTED",
        "title": "Rule 14t's plain nonnull_member declarations in librdkafka's null view would carry P-RK-NO-PLUGIN-LIBRARY by the member census's rule, but the premise is admitted only beside the scoped forms",
        "why": (
            "Registered by " + CARD + " (" + RUN + "). The user decided (2026-10-15, verbatim) \"Only "
            "nonnull_member* need it\", and the brief carried it as a premise clause admitted beside "
            "nonnull_member_stable / nonnull_member_scope only. The card applied the member census's "
            "rule (docs/reports/librdkafka-plugin-premise-remainder/scripts/member_reach.py: a store to "
            "TAG.FIELD reachable on reach.py's over-approximating graph from a function a plugin's "
            "callback may call, or the declared body reaching an rk_conf or interceptor call) to every "
            "member declaration of null_deref.qv. Rule 14t's five plain nonnull_member clauses on four "
            "declarations (rd_kafka_toppar_new0 rd_kafka_toppar_s.rktp_rkt, rd_kafka_topic_new0 "
            "rd_kafka_topic_s.rkt_rk and .rkt_topic, rd_kafka_cgrp_new rd_kafka_cgrp_s.rkcg_rk, "
            "rd_kafka_transport_new rd_kafka_transport_s.rktrans_rkb) meet the same rule: each member has "
            "a store reachable from an external function and each host body reaches an rk_conf or "
            "interceptor call. Their fact is checked against the analysed units' stores exactly as a "
            "stable member's is (rule 14t), so a store made by a plugin's code could falsify it too. "
            "The loader rejects the premise beside them (premise-beside-member), as the brief directs. "
            "Size: 4 declarations, 5 clauses. DISCHARGE: the user decides whether the plain "
            "nonnull_member form also admits the premise; if so, the five name it (re-pinned, measured, "
            "every null row resting on member:TAG.FIELD listing it); if not, the decision is recorded "
            "beside rule 14t."
        ),
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    entries[:] = [e for e in entries if e["id"] not in RETIRE]
    known = {e["id"] for e in entries}
    for entry in REGISTER:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")


main()
