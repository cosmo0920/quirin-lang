#!/usr/bin/env python3
"""VERDICT-BODY-STANDING-CLAUSE-SCOPE: retire the card's entry and register the
residuals with sizes. Acceptance matrix, indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "VERDICT-BODY-STANDING-CLAUSE-SCOPE"
SOURCE = "docs/reports/verdict-body-standing-clause-scope/README.md"
RETIRE = {CARD}

NEW = [
    {
        "id": "VERDICT-STRUCTURAL-DECLARATION-CUSTODY-CLAUSES",
        "title": "The custody clauses of a declaration only a structural check validates have no body evidence",
        "why": "Registered by VERDICT-BODY-STANDING-CLAUSE-SCOPE (rules 13i-13j). A structural check (13a, 13c, 13d) covers the clause kind it reads; the declaration's resource_borrowed formals and plain result clause stay uncovered, so the declaration is partly-validated. No existing exact reading covers them: a retaining declaration is never given a W4 summary (rule 12t, derive_custody_summaries), and the function-map borrowed-result and returned-reference declarations have no validated W4 summary and no entry evidence in any unit; a new evidence-only inference would build CFGs whose QCORE_TYPED_PATH_PARITY rows change the parity multisets. Size (corpus, binary 65d7c0d8): 25 declarations partly-validated (13 retaining: ten 13c keeps and drops, rd_slice_reader0, sds_cat_safe, rd_kafka_q_new0 with its two resource_retained_by(_, result) clauses too; 12 in the function map: five 13a borrowed results, seven 13d returned references), 21 corpus Safe rows rest on one of them and no declared assumption (the 21 the only-validated bucket lost); system and slice 0 such declarations. DISCHARGE: an exact body reading of those clauses that adds no parity row, consultation or verdict-bearing output (e.g. a W4 inference of a retaining declaration's custody projection run evidence-only with parity emission and consultation logging suppressed, or a structural formal check exact for resource_borrowed), with Lean parity, measured with no verdict movement.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "VERDICT-COUNTED-USER-CLAUSE-EVIDENCE",
        "title": "Counted-user clauses checked against bodies record no body evidence",
        "why": "Registered by VERDICT-BODY-STANDING-CLAUSE-SCOPE (rule 13i). W4 now covers the custody clauses of a declaration with counted-user or container clauses, but nothing covers the counted-user clauses themselves, although the counted-user pass (legacy_counted_users.rs, gateway rule 10e, contract specification C04.7) checks a body defined in the unit against its resource_user_exit and neutrality clauses. Size (binary 65d7c0d8): partly-validated declarations whose only uncovered clause is counted-user: corpus 4 (flb_input_chunk_is_mem_overlimit, flb_input_chunk_is_up, flb_task_get_route_status, flb_task_set_route_status: resource_user_neutral()), system 5 (the four and flb_task_users_inc: resource_user_added(#0, users)), slice 1 (flb_input_collector_running); Safe rows on them in the partly bucket: corpus 1, system 2, slice 2. DISCHARGE: the counted-user pass's body check recorded as evidence covering exactly the clauses it establishes (or a recorded decision that it establishes none), with Lean parity and no verdict movement.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    known = {entry["id"] for entry in entries}
    assert CARD in known or not RETIRE - known
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(entries))


if __name__ == "__main__":
    main()
