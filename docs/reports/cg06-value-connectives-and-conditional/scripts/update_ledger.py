#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (every DISCHARGE clause holds)
and the absorbed RETURN-POSITION-CONDITIONAL and
W43-RETURN-CONDITIONAL-OPERANDS; amends W47-ACCESS-PATH-POSITION-RESIDUALS
(its pointer-?: return and ?: actual rows) and CG02-COMMA-OPERAND-RESIDUALS
(how far the nanopb shape now gets); registers
CG06-VALUE-CONNECTIVE-RESIDUALS with its sizes; keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes are the AFTER corpus arbiter's parity rows on the merge of main
41719835 (private/cg06/after3, tip 6be67b0f), de-duplicated by scripts/parity_census.py and
scripts/absorbed.py (artifacts/parity-census.json, artifacts/absorbed.json).
"""

import json
import sys

REPORT = "docs/reports/cg06-value-connectives-and-conditional/README.md"
BY = "CG06-VALUE-CONNECTIVES-AND-CONDITIONAL"

RETIRE = [
    "CG06-VALUE-CONNECTIVES-AND-CONDITIONAL",
    "RETURN-POSITION-CONDITIONAL",
    "W43-RETURN-CONDITIONAL-OPERANDS",
]

REGISTER = [
    {
        "id": "CG06-VALUE-CONNECTIVE-RESIDUALS",
        "title": "Value connectives and conditionals rule 10z consults and refuses (2,219 consultations over 528 functions)",
        "why": (
            "Found by CG06-VALUE-CONNECTIVES-AND-CONDITIONAL (gateway rule 10z): a statement, declaration or return "
            "every older route refused is lowered by branching on each value region's deciding operand. On the corpus "
            "arbiter at its merge of main 41719835 (CG04 and CG05 landed; tip 6be67b0f; the same figures at "
            "main c0ccc66a) it is consulted 2,452 times "
            "(de-duplicated parity rows `position: value-connective-statement`/`-return`), admits 233 in 164 functions "
            "and refuses 2,219 in 528 (at its base 27dc7c9a: 232 / 2,220): "
            "1,133 a sibling of the region with an effect (mostly `rd_kafka_log0(..., f(x), ...)` logging macros: "
            "nested calls: to be decided by composing with CG05's contract footprints, "
            "`EvalGraph::check_footprints`, never by a parallel check); 420 an edge's expression an older route refuses (CG11 owned `?:` stores "
            "`p->f = s ? strdup(s) : NULL`, CG16 pointer arithmetic, uncontracted calls, `sizeof` of an array); 291 a "
            "deciding operand no `if` route decides (CG04 / the condition routes); 284 no unique Clang node (two "
            "identical runs on one line, I04: CG10's occurrence identity serves statement runs only); 67 a region "
            "under a comma (the nanopb `PB_RETURN_ERROR` return, CG02-COMMA-OPERAND-RESIDUALS); 14 under an indirect "
            "callee (`__builtin_assume`, CG22); 4 a deciding call beside a non-inert sibling (CG05); 4 more than 8 "
            "leaves; 2 a nested assignment or a multi-declarator declaration (CG02). Of them, 97 are returns (58 "
            "Unknown functions keep a return leaf on a refused consultation). DISCHARGE: each sub-population closed "
            "by its owning card and re-measured here, or a recorded decision to keep it refused; the leaf cap and the "
            "I04 run identity are this card's to revisit with a measured population."
        ),
    },
]

AMEND = {
    "W47-ACCESS-PATH-POSITION-RESIDUALS": (
        " AMENDED (CG06-VALUE-CONNECTIVES-AND-CONDITIONAL, gateway rule 10z): the pointer-valued `?:` return rows "
        "are discharged (`get_client_rack`, `get_group_id`, `get_group_instance_id`, `get_transactional_id` Safe: "
        "each edge's return is rule 10k's never-bound pointer return or the null constant) and the `?:` actual rows "
        "are absorbed by branching the whole call per edge; re-measured on the AFTER corpus (base 27dc7c9a): rule "
        "10n's 88 conditional call-actual residual-shape rows join on (function, line) to 5 statements rule 10z "
        "admits (3 functions), 42 it refuses (31 functions; sized in CG06-VALUE-CONNECTIVE-RESIDUALS) and 41 it is "
        "never asked about (24 functions: another route decided or declined the statement first). The other "
        "shapes of this entry stand."
    ),
    "CG02-COMMA-OPERAND-RESIDUALS": (
        " AMENDED (CG06-VALUE-CONNECTIVES-AND-CONDITIONAL): the first refusal of these 67 returns (32 functions) is "
        "still the elaborator's `IntegralToBoolean` (CG08). Rule 10z is consulted on the same 67 returns and refuses "
        "each by position (a value region under the comma, which is CG02's to split); past both, the piece "
        "`stream->errmsg = stream->errmsg ? stream->errmsg : \"...\"` is a pointer-valued member store of a pointer "
        "read, CG11's refusal. The value `?:` itself is admitted by rule 10z wherever it stands alone."
    ),
}


def main():
    path = sys.argv[1]
    with open(path, encoding="utf-8") as stream:
        matrix = json.load(stream)
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in RETIRE:
        assert retired in ids, retired
    for registered in REGISTER:
        assert registered["id"] not in ids, registered["id"]
    for amended in AMEND:
        assert amended in ids, amended
    followups = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for registered in REGISTER:
        followups.append({**registered, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(len(ids), "->", len(followups))


if __name__ == "__main__":
    main()
