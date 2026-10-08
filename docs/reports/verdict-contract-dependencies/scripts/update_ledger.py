#!/usr/bin/env python3
"""VERDICT-CONTRACT-DEPENDENCIES: retire the card's entry and register the
residuals with sizes. Acceptance matrix, indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "VERDICT-CONTRACT-DEPENDENCIES"
SOURCE = "docs/reports/verdict-contract-dependencies/README.md"
RETIRE = {CARD}

NEW = [
    {
        "id": "VERDICT-CONTRACT-PREMISE-ANNOTATIONS",
        "title": "Premises stated only in contract comments are not yet machine-readable resource_premise clauses",
        "why": "Registered by VERDICT-CONTRACT-DEPENDENCIES (contract specification C04.18), which attached the served-queue premise to rd_kafka_q_wait_result and rd_kafka_q_purge0 and reports literal-argument premises and rule 13e's result-holder premises per declaration. Still stated only in comments, so a row resting on the declaration does not name them: the reachability premise of quirin-provider-fluent-bit fluent_bit/output_singleplex.qv; AOM-DESTROY-CORRUPTED-UNPAID-PREMISE in fluent_bit/input_chunk_destroy.qv and input_chunk_destroy_direct_routes.qv; C3's ingestion premise in fluent_bit/loop_owner.qv; O04-TRACE's premises N1-N10 in quirin-provider-libc libc/errno.qv (route premises of the nested-call admission). Size: five modules; each annotation changes a pinned declaration's text, so each needs a review and a re-pin at 66910c10 and 046accf6 (the libc ones are flexible: no pin). DISCHARGE: each premise attached as requires resource_premise(NAME, \"STATEMENT\") to the declarations it covers, re-pinned, measured with no verdict movement.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
    {
        "id": "VERDICT-BODY-STANDING-CLAUSE-SCOPE",
        "title": "A declaration validated by a structural check is validated for the clause that check reads, not for every mode",
        "why": "Registered by VERDICT-CONTRACT-DEPENDENCIES (rule 13g). The run's body standing labels a declaration validated when every unit that examined its body validated it; the evidence is an equal W4 summary, the entry check, or a structural check of rules 11r, 12w, 13a, 13c or 13d. A structural check reads one clause kind (a slot move, a borrowed result, a keep or drop, a returned reference, a guard): the declaration's other modes (resource_borrowed formals beside the checked clause) are not re-derived by it. Size (corpus, binary 851bf3dd): 28 of 73 validated declarations rest on a structural check alone (the rule 13a/13c/13d declarations of librdkafka/client.qv and five 13a borrowed results); system and slice 0. DISCHARGE: a per-clause standing (the clauses each piece of evidence covers), or a decision that a structural check validates its declaration, recorded in C06.5.",
        "source": SOURCE,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    known = {entry["id"] for entry in entries}
    entries[:] = [entry for entry in entries if entry["id"] not in RETIRE]
    for entry in NEW:
        if entry["id"] not in known:
            entries.append(entry)
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(entries))


if __name__ == "__main__":
    main()
