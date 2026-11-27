#!/usr/bin/env python3
"""update_ledger.py -- OP-APP-RK-CALLBACKS-VIA-POLL-CB: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent.

* Retired: `CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS` (its DISCHARGE holds: a
  store through an embedded record is keyed by the embedded record's own
  TAG.FIELD, a record value copied into a field carries only values that key
  already counts, tests for both, member sections re-measured) and
  `LIBRDKAFKA-LOG-CALLBACK-PREMISE-UNSTATED` (its second branch holds: the
  logger's run-closed candidates are declared identities and `rk_log_callback`
  is retired everywhere, restated as `rk_no_plugin_library`).
* Amended in place: the card's own entry (rd_kafka_poll_cb is not stated:
  its op custody splits by return code, which no clause states beside its
  callback clauses; decision S1), `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`
  (rd_kafka_oauthbearer_refresh_op declared; rd_kafka_toppar_op_serve
  remains) and `W48-CALLEE-DECISION-TABLE` (the rd_kafka_destroy_app note).
* Registered: the plugin premise not yet listed on the earlier librdkafka
  declarations, the routes without the pre-pass, the pre-pass's wall time,
  and `rko_op_cb`'s run set read open through its union-keyed dependency."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "OP-APP-RK-CALLBACKS-VIA-POLL-CB"
SOURCE = "docs/reports/op-app-rk-callbacks/README.md"
RUN = ("corpus 66910c10 0a38c3b3 936/9/3,889 -> 620fbd75 936/9/3,889, no status move, 97 "
       "Unknown rows trade leaves, binary a5 6e51a944 against before2 d2bbd2e5 built from main "
       "b7facc899; system 93/4/351 and slice 52/1/347/4 Pending no row moves; null verdicts "
       "identical on all three; corpus wall 515 -> 668 s")

RETIRE = ["CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS", "LIBRDKAFKA-LOG-CALLBACK-PREMISE-UNSTATED"]

AMEND = {
    CARD: {
        "title": "rd_kafka_poll_cb's op custody splits by return code, which no clause states beside its rk_conf member invocations",
        "why": (
            f"Amended by {CARD} ({RUN}). USER DECISIONS 2026-10-14: \"Closed over FB's "
            "registrations\", then \"Build the agent's recommended set\" (D1 function defaults "
            "with |->, Q2 setter registrations, Q3 innermost-record keys and union members "
            "sharing stores, U2 the premise P-RK-NO-PLUGIN-LIBRARY, R1 a facts pre-pass so a unit "
            "decides over the run's set, F1 every Fluent Bit production unit read, E1 a run-closed "
            "empty set invokes nothing, S1 rule 12m kept). Done: the seventeen rk_conf callback "
            "setters declared as registrations (premises conf_property_writer and "
            "rk_no_plugin_library), rd_kafka_conf_new with its three function defaults, "
            "rd_kafka_consume_cb and rd_kafka_background_queue_serve stated with their member "
            "invocations (each body read at 66910c10 and 046accf6, pinned); the run's sets close: "
            "dr_msg_cb {cb_kafka_msg}, log_cb {cb_kafka_logger, rd_kafka_log_print}, "
            "sasl.oauthbearer.token_refresh_cb {oauthbearer_token_refresh_cb and librdkafka's "
            "five refreshers}, consume_cb, dr_cb, error_cb, rebalance_cb, stats_cb, throttle_cb, "
            "background_event_cb and consume_ctx.consume_cb empty; every candidate declared the "
            "identity. The served-queue callers gain nothing (rd_kafka_poll and "
            "rd_kafka_queue_poll_callback refuse first at the access gate) and the C04.18 premise "
            "of rd_kafka_q_wait_result stays (it refuses first at the uncontracted "
            "rd_kafka_q_pop). Not done: rd_kafka_poll_cb (rdkafka.c:5813-6096) keeps the op on "
            "PASS (seven paths) and takes it on HANDLED, KEEP and YIELD; C04.13 states no "
            "disposition over the unsigned rd_kafka_op_res_t and none stands beside a callback "
            "clause, so it stays uncontracted (rd_kafka_poll_cb 4 leaves / 4 functions / 0 alone, "
            "unchanged; rd_kafka_background_queue_serve's direct call of it 1). DISCHARGE: the "
            "user's decision on a return-code split beside member invocations (or on keeping "
            "rd_kafka_poll_cb uncontracted), then rd_kafka_poll_cb stated or named, its body read "
            "at both revisions, measured on the served-queue callers."
        ),
    },
    "OP-CALLBACK-CANDIDATES-APP-CALLBACKS": {
        "title": "rd_kafka_toppar_op_serve calls rko_u.offset_commit.cb and reaches rk_conf.open_cb",
        "why": (
            f"Amended by {CARD} ({RUN}). Done: rd_kafka_oauthbearer_refresh_op declared "
            "(op borrowed, HANDLED only; member invocation of "
            "rd_kafka_s.rk_conf.sasl.oauthbearer.token_refresh_cb, whose run set closes over "
            "Fluent Bit's and librdkafka's six refreshers, each declared the identity), and the "
            "premises rk_log_callback (19 declarations) and rk_closesocket_callback "
            "(rd_kafka_mock_cluster_op_serve) restated as rk_no_plugin_library (log_cb closes "
            "over two declared identities, closesocket_cb closes empty), re-pinned at both "
            "revisions. A candidate carrying its own callback clause is still read as "
            "uncontracted at rule 13u (a retaining declaration is not in the function map), so "
            "rd_kafka_op_destroy's decisions do not change on it. Remaining: "
            "rd_kafka_toppar_op_serve (rdkafka_partition.c:2159, static; an rko_serve "
            "candidate) calls rko->rko_u.offset_commit.cb, a member under the op's rko_u union, "
            "whose run key rd_kafka_op_s.rko_u.* now shares the union's stores, and releases or "
            "moves partition references in several arms. DISCHARGE: rd_kafka_toppar_op_serve "
            "declared (or named as refused) against the union-keyed closure, its body read at "
            "both revisions, measured."
        ),
    },
}

TABLE_NOTE = (
    f" AMENDED ({CARD}, {RUN}): rd_kafka_destroy_app stays undeclared, named as a residual of "
    "the user's \"Closed over FB's registrations\": it releases rk only without "
    "RD_KAFKA_DESTROY_F_IMMEDIATE (a literal premise flags = 0 on rd_kafka_destroy is "
    "existing vocabulary), but its consumer close serves through rd_kafka_poll_cb "
    "(uncontracted, OP-APP-RK-CALLBACKS-VIA-POLL-CB) and rd_kafka_destroy_final runs the "
    "interceptors' on_destroy, an rd_list of methods read through a union "
    "(rdkafka_interceptor.c:116, :412) that only a plugin registers (P-RK-NO-PLUGIN-LIBRARY: "
    "empty). rd_kafka_destroy_app 2 functions alone, unchanged."
)

NEW = [
    {
        "id": "LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED",
        "title": "The earlier librdkafka declarations whose bodies may log do not list P-RK-NO-PLUGIN-LIBRARY",
        "why": (
            f"Registered by {CARD} ({RUN}). Every librdkafka body that logs (rd_kafka_dbg, "
            "rd_kafka_log -> rd_kafka_log_buf, rdkafka.c:280-305) may call rk_conf.log_cb, whose "
            "run set closes over the declared identities cb_kafka_logger and rd_kafka_log_print "
            "only under the premise P-RK-NO-PLUGIN-LIBRARY (user decision U2: \"List it in every "
            "dependent verdict\"). The card lists it on its 26 declarations that rest on an "
            "rk_conf callback and on the 19 OP-CALLBACK-CANDIDATE-CONTRACTS declarations whose "
            "rk_log_callback / rk_closesocket_callback premises it restated; 73 other client.qv "
            "declarations and 7 in refcount.qv, typed_queue.qv, queue_destructor.qv and "
            "topic_partition.qv (80, not all of whose bodies log) do not, nor do the 86 "
            "null_deref.qv views. DISCHARGE: each of those declarations whose body may reach a "
            "log call (read at both revisions) names the premise, re-pinned, measured (expected: "
            "dependency premises only)."
        ),
    },
    {
        "id": "MEMBER-PREPASS-OTHER-ROUTES",
        "title": "Only the pinned route reads the run's member facts before analysis",
        "why": (
            f"Registered by {CARD} ({RUN}). The member-facts pre-pass (rule 12n, user decisions "
            "R1 and F1, qcore_audit::member_prepass) runs on the pinned corpus route; the "
            "current, no-build and checkpointed routes keep each unit's own set (an empty set "
            "refused), so a member invocation decides there only where the unit holds the "
            "stores itself. Sizes: the system and tail-pause selections hold no librdkafka unit "
            "and no member-invoking call decides on them (0 leaves). DISCHARGE: the pre-pass on "
            "the current and no-build routes over their compile databases (and a checkpoint "
            "that carries the run's facts), measured on the system and slice routes."
        ),
    },
    {
        "id": "MEMBER-PREPASS-WALL-TIME",
        "title": "The member-facts pre-pass adds about 150 s to the corpus arbiter",
        "why": (
            f"Registered by {CARD} ({RUN}). The pre-pass preprocesses all 1,444 production units "
            "with their own commands and reads 845 whole Clang ASTs (its bootstrap round, before "
            "any record is read, selects by member names as well as record tags), measured "
            "105 s alone (private/oprk/a5-prepass.seconds); corpus 515.3 -> 668.2 s at --jobs 4. "
            "DISCHARGE: a bootstrap that reads the declared records' defining units first (or "
            "reuses the analysed units' facts), so only units naming a final token are read, "
            "with the run's closures byte-identical, measured."
        ),
    },
    {
        "id": "RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION",
        "title": "rd_kafka_op_s.rko_op_cb reads open over the run once the op's rko_u union members share their stores",
        "why": (
            f"Registered by {CARD} ({RUN}). Q3 keys a union's members together (rd_kafka_op_s"
            ".rko_u.*), and rko_op_cb's closure depends on a store read from the union; over the "
            "run's facts that union key is open: a CStyleCastExpr value, a memmove into a struct "
            "rd_kafka_group_member_s, whole-object assignments of struct rd_kafka_fetch_pos_s and "
            "struct rd_kafka_replyq_s (records the union holds by value). Before, the analysed "
            "units' run set closed over 27 candidates. Effect: rd_kafka_op_destroy's 13 decided "
            "leaves (OP-CALLBACK-CANDIDATE-CONTRACTS) refuse again (callback-member-open 118 -> "
            "131 leaves), no status move. DISCHARGE: each opening store classified by reading "
            "(the cast's source, whether the memmove and the assignments reach a function-pointer "
            "member of the union), the union's closure closed or named, measured."
        ),
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = [entry for entry in matrix["named_followups"] if entry["id"] not in RETIRE]
    seen = set()
    for entry in followups:
        if entry["id"] in AMEND:
            amend = AMEND[entry["id"]]
            entry["title"] = amend["title"]
            entry["why"] = amend["why"]
            entry["source"] = SOURCE
            seen.add(entry["id"])
        if entry["id"] == "W48-CALLEE-DECISION-TABLE":
            if TABLE_NOTE not in entry["why"]:
                entry["why"] += TABLE_NOTE
            seen.add(entry["id"])
    missing = (set(AMEND) | {"W48-CALLEE-DECISION-TABLE"}) - seen
    if missing:
        raise SystemExit(f"not in the ledger: {sorted(missing)}")
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
