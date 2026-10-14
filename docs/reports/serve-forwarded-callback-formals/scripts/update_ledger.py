#!/usr/bin/env python3
"""update_ledger.py -- SERVE-FORWARDED-CALLBACK-FORMALS: retire the card's
entry and register its residual. The DISCHARGE ("a rule that reads a callback
formal handed to the same clause position of a callee as the caller's own
declaration's clause (as rule 12s forwards dispositions), with Lean, measured
on the four forwarders") holds: gateway rule 15w, Lean QCore.CallbackForward,
measured on the corpus, system and slice with the null property (the four
forwarders read and their rows attributed in the report). The residual is the
reason the four rows do not move: their callees stay withdrawn. Acceptance
matrix, indent=1, ensure_ascii=False; `dimensions.named_followups` = list
length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
RETIRE = {"SERVE-FORWARDED-CALLBACK-FORMALS"}
SOURCE = "docs/reports/serve-forwarded-callback-formals/README.md"
CARD = "SERVE-FORWARDED-CALLBACK-FORMALS"

REGISTER = [
    {
        "id": "SERVE0-CALLBACK-HAND-ON-WITHDRAWN",
        "title": "rd_kafka_q_serve0 and rd_kafka_q_pop_serve0 hand their callback to the "
        "undeclared rd_kafka_op_handle and to themselves, so rule 15w's forwarders stay "
        "withdrawn",
        "why": "Registered by SERVE-FORWARDED-CALLBACK-FORMALS (corpus 66910c10 ec4ccfeb "
        "936/9/3,889 -> MEASURED_CORPUS, system 93/4/351 and slice 52/1/347/4 Pending, "
        "null property byte-identical; base main 2afce9973). Rule 15w validates "
        "rd_kafka_q_serve, rd_kafka_q_serve_maybe_consume, rd_kafka_q_pop_serve and "
        "rd_kafka_q_pop_serve_maybe_consume only on a callee whose declaration is in force "
        "in rdkafka_queue.c, and both callees are withdrawn there by rule 13y independently "
        "of them: rd_kafka_q_pop_serve0 hands callback to rd_kafka_op_handle "
        "(rdkafka_queue.c:450-452, sixth actual) and to itself (:498-499, the forward-queue "
        "recursion); rd_kafka_q_serve0 to rd_kafka_op_handle (:627-628) and to itself "
        "(:585-586). rd_kafka_op_handle (rdkafka_op.c:1096-1124) has no declaration, and "
        "the standing set is a least fixed point, so a recursive forward is never its own "
        "support. Size: 6 declarations withdrawn in 1 unit; 5 corpus rows carry a "
        "callback-withdrawn leaf on them (rd_kafka_q_pop, rd_kafka_q_pop_serve, "
        "rd_kafka_q_pop_serve0, rd_kafka_q_serve, rd_kafka_q_serve0), 3 of them alone "
        "(rd_kafka_q_pop, rd_kafka_q_pop_serve, rd_kafka_q_serve); the two "
        "_maybe_consume rows refuse earlier, alone on the indirect access "
        "rkq->rkq_rk->rk_type. Even with both callees "
        "standing, the forwarders' rows also need the member clauses they repeat "
        "(rd_kafka_op_s.rko_serve, rd_kafka_op_s.rko_op_cb) decided over contracted "
        "candidates (OP-CALLBACK-CANDIDATE-CONTRACTS, OP-APP-RK-CALLBACKS-VIA-POLL-CB), the "
        "_maybe_consume rows keep that access refusal, and "
        "rd_kafka_q_pop passes a null callback; at most rd_kafka_q_serve and "
        "rd_kafka_q_pop_serve could move. DISCHARGE: a declaration of rd_kafka_op_handle "
        "stating its invocation of the callback (and its substitution of rko_serve) with "
        "the body read at 66910c10 and 046accf6, and a user decision on reading a "
        "recursive exact forward (the coinductive reading is sound for this safety clause "
        "but is a stronger claim than the least fixed point), measured on the six serve "
        "declarations.",
        "source": SOURCE,
        "recorded_by": CARD,
    }
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    before = len(matrix["named_followups"])
    entries = [e for e in matrix["named_followups"] if e.get("id") not in RETIRE]
    known = {e.get("id") for e in entries}
    for entry in REGISTER:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"named_followups {before} -> {len(entries)}")


if __name__ == "__main__":
    main()
