#!/usr/bin/env python3
"""update_ledger.py -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: retire the card's
own entry (its DISCHARGE, "a clause for a callback that takes over an object
the callee supplies, a type-switched release, and a registration carried by an
op, each measured", holds: C04.19.1-C04.19.3, rules 12m-12o, measured on the
corpus, system and slice) and register every blocker as its own follow-up
with a DISCHARGE (USER DECISION 2026-10-14). Acceptance matrix, indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "CG22-OP-CALLBACKS-AND-SERVED-QUEUES"
SOURCE = "docs/reports/cg22-op-callbacks-served-queues/README.md"
RUN = ("corpus 66910c10 bdf5f1c8 935/9/3,890 -> RUN_AFTER, system 93/4/351 and slice "
       "52/1/347/4 Pending, no status move, null property byte-identical; base main 579c2e6e2")

NEW = [
    {
        "id": "OP-APP-RK-CALLBACKS-VIA-POLL-CB",
        "title": "Every served queue may run rd_kafka_poll_cb, which calls the application's rk_conf callbacks",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 12n's run closure of rd_kafka_op_s.rko_serve "
            "(copied from the destination queue's rkq_serve by rd_kafka_q_enq1) holds "
            "rd_kafka_poll_cb, rd_kafka_cgrp_op_serve, rd_kafka_mock_cluster_op_serve and "
            "rd_kafka_toppar_op_serve; rd_kafka_op_handle substitutes it for the caller's "
            "callback, so every served queue may run rd_kafka_poll_cb whatever its caller "
            "passes. rd_kafka_poll_cb calls the callbacks the application registered in "
            "rk->rk_conf (consume_cb, dr_cb, dr_msg_cb, error_cb, log_cb, rebalance_cb, stats_cb, "
            "throttle_cb), as do the other corpus serve callbacks rd_kafka_consume_cb "
            "(ctx->consume_cb) and rd_kafka_background_queue_serve (background_event_cb, then "
            "rd_kafka_poll_cb). This is the class of the pending rd_kafka_destroy_app decision "
            "(W48-CALLEE-DECISION-TABLE round-3 amendment) and is NOT decided here: none of the "
            "three is declared, so every served-queue call refuses (callback-uncontracted at the "
            "member or the callback). Sizes (leaves / functions / alone, corpus): "
            "rd_kafka_poll_cb SIZE_POLL_CB, rd_kafka_q_serve SIZE_Q_SERVE, "
            "rd_kafka_q_serve_maybe_consume SIZE_Q_SERVE_MC. The served-queue premise (C04.18) on "
            "rd_kafka_q_wait_result reaches the same set. DISCHARGE: after the user's decision on "
            "application-registered rk callbacks, rd_kafka_poll_cb, rd_kafka_consume_cb and "
            "rd_kafka_background_queue_serve stated per that decision (a contract or a named "
            "Pending), each body read at both revisions, measured on the served-queue callers and "
            "on the C04.18 premise of rd_kafka_q_wait_result."
        ),
    },
    {
        "id": "OP-CALLBACK-CANDIDATE-CONTRACTS",
        "title": "The op member closures are closed, but no candidate has a contract in force",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 12n's closure over every unit that recorded "
            "facts: rd_kafka_op_s.rko_op_cb closes over 27 designators (the ten transaction ops, "
            "rd_kafka_admin_worker and _fanout_worker, the three group-metadata ops, "
            "rd_kafka_broker_monitor_op_cb, rd_kafka_oauthbearer_refresh_op, "
            "rd_kafka_offset_reset_op_cb, _validate_op_cb, the five share ops, "
            "rd_kafka_share_fetch_reply_op_cb, the leaders query worker and the two "
            "leaders-queried callbacks), rd_kafka_op_s.rko_serve over 4 "
            "(OP-APP-RK-CALLBACKS-VIA-POLL-CB). None has a contract in force, and 12 have no "
            "RD_KAFKA_RESP_ERR__DESTROY test (rd_kafka_offset_reset_op_cb runs a whole offset reset "
            "when rd_kafka_op_destroy calls it): every decided call of rd_kafka_op_destroy refuses "
            "callback-uncontracted (13 leaves), the rest callback-member-open (the unit sees no "
            "store, 119 leaves). rd_kafka_op_destroy SIZE_OP_DESTROY (leaves / functions / alone). "
            "The served-queue premise (C04.18) on rd_kafka_q_purge0 rests on the same set "
            "(rd_kafka_op_destroy passes rkq = NULL, so no candidate receives the served queue as "
            "an argument). DISCHARGE: a contract in force for every candidate of both closures "
            "(each a declared assumption with its body read at both revisions, or a validated "
            "summary; a candidate calling an application callback waits for "
            "OP-APP-RK-CALLBACKS-VIA-POLL-CB), with the op callback protocol (HANDLED, KEEP, "
            "YIELD) stated, measured on rd_kafka_op_destroy's callers and on the C04.18 premise of "
            "rd_kafka_q_purge0."
        ),
    },
    {
        "id": "OP-CONSTRUCTOR-DECLARATIONS",
        "title": "The op constructors are undeclared, so no caller tracks an op or decides its discriminant",
        "why": (
            f"Registered by {CARD} ({RUN}). rd_kafka_op_new0 SIZE_OP_NEW0, rd_kafka_op_new_cb "
            "SIZE_OP_NEW_CB (leaves / functions / alone; corpus) and the rd_kafka_op_req* family "
            "are undeclared: an op a caller creates is untracked, so rule 13w's per-path "
            "registration (a constructor's result holder) and rule 12o's decided discriminant "
            "never apply, and every rd_kafka_op_destroy call reads the run's closure and the "
            "cautious join over its 60 arms. Of the 96 textual rd_kafka_op_destroy sites, 29 "
            "destroy an op rd_kafka_q_pop returned (REFCOUNT-Q-POP-OWNED-OP), 16 a formal, 25 a "
            "unit-test constructor's. DISCHARGE: rd_kafka_op_new0, rd_kafka_op_new_cb (a "
            "registration of cb in the result's rko_op_cb, rule 13w) and the rd_kafka_op_req* "
            "constructors declared, with a clause stating the result's discriminant from the "
            "decided type actual (rule 12o's decided arm, the discriminant's run-wide stability "
            "checked as nonnull_member_stable is), each body read at both revisions, measured."
        ),
    },
    {
        "id": "SERVE-FORWARDED-CALLBACK-FORMALS",
        "title": "The served-queue forwarders hand a callback formal on and stay Unknown",
        "why": (
            f"Registered by {CARD} ({RUN}). rd_kafka_q_serve, rd_kafka_q_serve_maybe_consume, "
            "rd_kafka_q_pop_serve and rd_kafka_q_pop_serve_maybe_consume are one-line forwarders "
            "of rd_kafka_q_serve0 / rd_kafka_q_pop_serve0 handing their callback formal on; rule "
            "13u reads a formal as an open set (callback-undecided), and in rdkafka_queue.c rule "
            "13y withdraws the declarations whose body hands the callback on (callback-withdrawn). "
            "Four corpus rows are decided alone by these members. The user decided (2026-10-14) no "
            "forwarded-formal rule in this card. DISCHARGE: a rule that reads a callback formal "
            "handed to the same clause position of a callee as the caller's own declaration's "
            "clause (as rule 12s forwards dispositions), with Lean, measured on the four "
            "forwarders."
        ),
    },
    {
        "id": "CALLBACK-MEMBER-DATA-ONLY-UNITS",
        "title": "Data-only units record no member facts, so the run's member closure stays open",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 12n's fold opens every member when an analysed "
            "unit recorded no facts (premise P-CG22-MEMBER-CLOSED-WORLD). On the corpus four "
            "data-only units do (ctraces/ctr_utils.c, librdkafka/opentelemetry/common.pb.c, "
            "metrics.pb.c, resource.pb.c), so both op members read open for the run although the "
            "recorded units close them (27 and 4 candidates). Nothing is demoted today (no row "
            "rests on a decided member invocation). DISCHARGE: member facts collected from a unit "
            "the gateway does not analyse (its initializers and stores), so the run's closure is "
            "exactly the recorded one where every unit is read; measured: both members closed."
        ),
    },
    {
        "id": "RETAINED-RELEASE-OF-A-FORMAL-READ-UNKNOWN",
        "title": "A retaining declaration that releases a formal reads Unknown once a member decision reads the unit's AST",
        "why": (
            f"Registered by {CARD}. In tests/cg22_op_callbacks.rs (measured once while writing "
            "the controls, harness: the card's fixture with a formal actual), a call releasing a "
            "formal through a retaining declaration whose member invocation was decided read "
            "UnknownAlias on the formal, while the same release of a tracked local was Safe and a "
            "retaining declaration with only arms on a formal was Safe; reading the unit's AST "
            "for the member's closure is the difference. The controls use tracked locals. No "
            "corpus row moved by it (every member invocation refuses before the release). "
            "DISCHARGE: the rule that reads the formal as an unknown alias after a decided member "
            "invocation identified, and either justified (named in the rule) or narrowed, with a "
            "control on a formal, measured."
        ),
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = [entry for entry in matrix["named_followups"] if entry["id"] != CARD]
    sizes = json.loads((REPO / "docs/reports/cg22-op-callbacks-served-queues/artifacts/size-after-corpus.json").read_text())
    after = (REPO / "docs/reports/cg22-op-callbacks-served-queues/artifacts/after-run.txt").read_text().strip()

    def size(name):
        row = sizes[name]
        return f"{row['leaves']}/{row['functions']}/{row['alone']}"

    known = {entry["id"] for entry in followups}
    for entry in NEW:
        text = (entry["why"].replace("RUN_AFTER", after)
                .replace("SIZE_POLL_CB", size("rd_kafka_poll_cb"))
                .replace("SIZE_Q_SERVE_MC", size("rd_kafka_q_serve_maybe_consume"))
                .replace("SIZE_Q_SERVE", size("rd_kafka_q_serve"))
                .replace("SIZE_OP_DESTROY", size("rd_kafka_op_destroy"))
                .replace("SIZE_OP_NEW0", size("rd_kafka_op_new0"))
                .replace("SIZE_OP_NEW_CB", size("rd_kafka_op_new_cb")))
        record = {"id": entry["id"], "title": entry["title"], "why": text,
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
