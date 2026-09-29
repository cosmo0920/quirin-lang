#!/usr/bin/env python3
"""CG02-ELABORATION-CORE: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG02-ELABORATION-CORE and the absorbed W43-RETURN-COMMA-MACRO,
registers the card's residuals with their measured sizes, amends
CG01-T-CONSUMER-ADMISSIONS, CG03, CG04, CG05, CG06 and CG08 with the named
refusals the elaborator measured for them, and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).
"""

import json
import sys

REPORT = "docs/reports/cg02-elaboration-core/README.md"
BY = "CG02-ELABORATION-CORE"

REGISTER = [
    {
        "id": "CG02-COMMA-OPERAND-RESIDUALS",
        "title": "Comma returns the elaborator splits but whose operands stay refused by other cards' forms (67 leaves over 32 Unknown functions)",
        "why": (
            "Re-registers W43-RETURN-COMMA-MACRO (absorbed and retired by CG02-ELABORATION-CORE, gateway rule 10s). "
            "Re-measured on the corpus arbiter at CG02's final tip on main f1ba8da2 from the elaborator's parity rows "
            "(`position: elaboration-return`, deduplicated): 67 comma returns over 32 Unknown functions "
            "(W43's 67/32; W45's 69/34 counted two brace leaves outside this population), 0 admitted. Every one "
            "is nanopb's PB_RETURN_ERROR in a `bool` function, `return (stream->errmsg = stream->errmsg ? "
            "stream->errmsg : \"...\"), 0;`: the elaborator refuses first at the comma value's `IntegralToBoolean` "
            "conversion (CG08), and past it the left operand is a value `?:` (CG06) whose result is a "
            "pointer-valued member store (CG11). The comma itself is sequenced and splits (rule 10s). "
            "DISCHARGE: CG08 reads the ToBoolean conversion of a comma's value, CG06 the value `?:` and CG11 the "
            "pointer member store, each measured on these 67 leaves."
        ),
    },
    {
        "id": "CG02-FOR-CLAUSE-SEQUENCES",
        "title": "For-loop init and increment clauses are not connected to the elaborator (109 leaves over 71 functions, lexical)",
        "why": (
            "Found by CG02-ELABORATION-CORE: rule 10s hooks statements, returns and controlling expressions, but a "
            "`for` header's init and increment clauses are lowered by `classify_for_clause` into one instruction "
            "each (`Statement::For { init, increment }`), so a clause with a comma (`i++, j--`) or a value "
            "increment keeps its refusal. Measured lexically on the corpus arbiter at CG02's final tip "
            "(scripts/for_clause_census.py): 109 unsupported leaves over 71 functions on `for` headers whose init "
            "or increment clause holds a comma or an increment (91 `unmodeled increment or decrement expression`). "
            "DISCHARGE: the For statement carries a clause block, the elaborator lowers each clause as a "
            "statement with its order model, measured."
        ),
    },
    {
        "id": "CG02-OLDER-SCALAR-ROUTE-UNSEQUENCED",
        "title": "The older scalar route admits `x = (x = 1)` (unsequenced writes of one object) as one scalar write",
        "why": (
            "Found by CG02-ELABORATION-CORE's control (tests/clang_gap_phase1.rs, e04): the lexical scalar route "
            "admits `x = (x = 1)` before the elaborator is asked, as one ForgetScalar. C leaves the two writes "
            "unsequenced (undefined behaviour); the admission is custody-neutral, so no custody verdict rests on "
            "it. A lexical census of the pinned corpus sources (scripts/same_object_census.py) finds 0 such "
            "statements (4 matches, each a member of the same spelling). DISCHARGE: the older route refuses a "
            "same-object unsequenced pair by name, or defers to the elaborator's order model."
        ),
    },
]

AMEND = {
    "CG01-T-CONSUMER-ADMISSIONS": (
        " AMENDED by CG02-ELABORATION-CORE: route T also names CG02's value forms (assignment value, compound "
        "assignment value, pre/post increment and decrement, comma) in its refusal (`typed-source value form "
        "... is decided by no typed-source consumer (CG02)`); an admission of one needs the same per-consumer "
        "reading."
    ),
    "CG03-CALL-SUMMARY-INSTANTIATION": (
        " MEASURED by CG02-ELABORATION-CORE (rule 10s; parity rows at its final tip on main f1ba8da2): the elaborator refuses 3 "
        "statement consultations as a call whose actuals carry effects (`cg_sink(i++)`, CG03), and every call in "
        "a sequenced piece keeps the older classifier's application; the elaborator's `Obligation::Contract` "
        "list is the interface this card's single application point reads."
    ),
    "CG04-CONDITION-ACQUISITION": (
        " MEASURED by CG02-ELABORATION-CORE: rule 10s admits call-free controlling expressions without "
        "connectives (120 consultations over 52 functions on the corpus at main f1ba8da2); the condition desugar still refuses an increment "
        "anywhere in a connective condition before it splits leaves, so E13's increment inside a leaf is left "
        "to this card (41 consultations refused as value `&&`, 4 as `?:`)."
    ),
    "CG05-NESTED-CALL-EFFECTS": (
        " MEASURED by CG02-ELABORATION-CORE: the elaborator refuses 98 consultations as a call unsequenced with "
        "another event or nested in an actual (CG05), mostly `P = f(...)` where P's address read is unsequenced "
        "with the call; the order model (`EvalGraph::check`, `QCore.ElaborationCore.interleave_run`) is the "
        "base this card extends with contract footprints."
    ),
    "CG06-VALUE-CONNECTIVES-AND-CONDITIONAL": (
        " MEASURED by CG02-ELABORATION-CORE: the elaborator refuses 72 consultations by name as value `&&`, `||` "
        "or `?:` (CG06); `Event::Branch` and `Event::Merge` are reserved in its vocabulary for this card."
    ),
    "CG08-CASTS-AND-TYPE-QUERIES": (
        " MEASURED by CG02-ELABORATION-CORE: the elaborator refuses 165 consultations by a cast kind (CG08), 67 "
        "of them the `IntegralToBoolean` of nanopb's comma return (`CG02-COMMA-OPERAND-RESIDUALS`)."
    ),
}


def main():
    path = sys.argv[1]
    matrix = json.load(open(path))
    followups = matrix["named_followups"]
    ids = [entry["id"] for entry in followups]
    for retired in ("CG02-ELABORATION-CORE", "W43-RETURN-COMMA-MACRO"):
        if retired not in ids:
            raise SystemExit(f"{retired} missing (already applied?)")
    followups[:] = [
        entry for entry in followups
        if entry["id"] not in ("CG02-ELABORATION-CORE", "W43-RETURN-COMMA-MACRO")
    ]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    found = {entry["id"] for entry in followups} & set(AMEND)
    if found != set(AMEND):
        raise SystemExit(f"missing amendment targets: {set(AMEND) - found}")
    for entry in REGISTER:
        followups.append(dict(entry, source=REPORT, recorded_by=BY))
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w") as handle:
        json.dump(matrix, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(len(followups))


if __name__ == "__main__":
    main()
