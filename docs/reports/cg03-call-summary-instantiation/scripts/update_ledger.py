#!/usr/bin/env python3
"""CG03-CALL-SUMMARY-INSTANTIATION: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG03-CALL-SUMMARY-INSTANTIATION (every DISCHARGE clause holds),
resizes the absorbed W46-ACCESS-PATH-PARITY with re-measured sizes (its
plan-vocabulary part for call positions is answered, the access-plan
vocabularies stay distinct), registers the card's three residuals with their
sizes, amends CG04 and CG05 with the instantiation API, and keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes are the parity rows of the AFTER corpus arbiter on the merged base
(main cb20c28e, run private/cg03/after-main), de-duplicated by
scripts/parity_census.py and scripts/position_rows.py.
"""

import json
import sys

REPORT = "docs/reports/cg03-call-summary-instantiation/README.md"
BY = "CG03-CALL-SUMMARY-INSTANTIATION"

RETIRE = ["CG03-CALL-SUMMARY-INSTANTIATION"]

REGISTER = [
    {
        "id": "CG03-RETURN-OPERAND-RESIDUALS",
        "title": "Scalar return operands the one application point consults and refuses (71 consultations over 54 functions)",
        "why": (
            "Found by CG03-CALL-SUMMARY-INSTANTIATION (gateway rule 10u): the scalar `return` operand route lowers a "
            "refused return that is one condition leaf exactly as the same leaf in an `if`. On the corpus arbiter at "
            "main cb20c28e it is consulted 81 times (de-duplicated parity rows `position: call-instantiation`, "
            "`consumer: return`), admits 10 and refuses 71 over 54 functions: 19 a callee with no contract in force in "
            "the caller's unit (an Unknown local body, or a body in another unit: cross-unit summaries are K14, not "
            "carded), 19 a value connective or a connective below the leaf (CG06), 11 a residual the condition leaf "
            "does not decide, 10 an indirect callee (CG22), 10 a call whose actuals neither the lexical gate nor the "
            "W44 typed argument reads admit, 1 a recursive-component callee, 1 a shadowing refusal. DISCHARGE: each "
            "sub-population closed by its owning card or re-registered with its size; the uncontracted-callee rows "
            "move only with their callees' summaries."
        ),
    },
    {
        "id": "CG03-RECURSIVE-COMPONENT-ADMISSIONS",
        "title": "Older consumers admit a recursive-component callee the one acceptance declines (statement 61, whole-call return 16 functions)",
        "why": (
            "Found by CG03-CALL-SUMMARY-INSTANTIATION: rule 10u's acceptance refuses a callee in the caller's recursive "
            "component in every use (constraint 4's provisional summary is not admission evidence), as the condition "
            "desugaring and B01 already did. The statement consumer and the whole-call return (W4) keep their older "
            "admission of such calls (the fixed point's own use); parity rows at main cb20c28e: statement discarded 35 "
            "rows over 23 functions, statement stored 50 over 38, whole-call return 38 over 16; and 3 whole-call "
            "returns of a derived result under the derived-return CFG (returned-alias, 3 functions). Nothing is widened "
            "or narrowed. DISCHARGE: a measured decision whether a provisional summary is admission evidence outside "
            "the statement fixed point (then the acceptance changes with Lean parity), or the older consumers adopt "
            "the acceptance with every moving function attributed."
        ),
    },
    {
        "id": "CG03-TYPED-SUBSTITUTION-RESIDUALS",
        "title": "Call statements whose typed substitution is refused (17 consultations over 12 functions)",
        "why": (
            "Found by CG03-CALL-SUMMARY-INSTANTIATION: the typed per-formal substitution (value -> identity -> formal, "
            "rule 10u) is consulted for 22 call statements whose lexical substitution was refused or incomplete on the "
            "corpus at main cb20c28e; it admits 5 (a pointer cast of a named object, a `sizeof` operand read as a "
            "name) and refuses 17 over 12 functions: 14 pointer arithmetic in an actual (`memcpy(key + n, ...)`, "
            "`(char *)base + i * n`, `dst + (size - 3)`: CG16) and 3 `rd_kafka_curr_msgs_get(rk, &cnt, &size)`, "
            "whose derived summary makes the dereferenced out-pointer formals custody-taking, so an automatic's "
            "address meets a custody formal (W44-CALL-ARGUMENT-CUSTODY-POSITION by design). DISCHARGE: CG16 reads "
            "offset pointers with extents; a derivation that states a written-through scalar out-pointer as `Out` "
            "or `Borrowed`, measured."
        ),
    },
]

AMEND = {
    "W46-ACCESS-PATH-PARITY": (
        " RESIZED by CG03-CALL-SUMMARY-INSTANTIATION (absorbed, plan-vocabulary part; rule 10u): the call positions "
        "now share one application point (`Parser::instantiate_call`) and one acceptance vocabulary (standing, result "
        "class, identity, use) with a typed substitution vocabulary (identity, address, data, place), so a call "
        "admitted in a statement, a condition leaf, a return operand or an elaborated piece is one `ContractCall` "
        "decided by one table (`QCore.CallInstantiation`); every consultation and every older admission the table "
        "declines is a parity row (`position: call-instantiation`: 103 consultations (15 admitted), 126 declined older-admission rows, "
        "CG03-RECURSIVE-COMPONENT-ADMISSIONS). The access-plan vocabularies stay distinct. Re-measured on the corpus "
        "at main cb20c28e (scripts/position_rows.py, de-duplicated): condition-access 1,303 / 196 rows typed "
        "admitting / refusing beside the older admission, return-family 266 / 141, call-argument 230 / 491 (211 / "
        "455 before CG03: the condition and return consumers now ask the W44 argument reads). The 25,432 / 147 "
        "identical-plan figures are carried, not re-run. No route is retired and no parity is claimed."
    ),
    "CG04-CONDITION-ACQUISITION": (
        " NOTE by CG03-CALL-SUMMARY-INSTANTIATION: the application point this card calls is "
        "`Parser::instantiate_call(callee, statement, destination, ResultUse)` (rule 10u); a condition-bound owned "
        "result is its use `Tested` refusing `custody-result` (`call_instantiation::accept`), which this card "
        "widens with the NULL split, never by a consumer-specific gate."
    ),
    "CG05-NESTED-CALL-EFFECTS": (
        " NOTE by CG03-CALL-SUMMARY-INSTANTIATION: the application point this card calls is "
        "`Parser::instantiate_call` (rule 10u); a nested call's value is use `Passed`, accepted only as the identity "
        "with no result (B04), the row this card widens in `call_instantiation::accept` with its footprints. The "
        "elaborator's effectful-actual piece (`effectful_call_piece`) is the sequenced case CG03 closed."
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
    for amended in AMEND:
        assert amended in ids, amended
    for new in REGISTER:
        assert new["id"] not in ids, new["id"]
    followups = [entry for entry in followups if entry["id"] not in RETIRE]
    for entry in followups:
        if entry["id"] in AMEND:
            entry["why"] += AMEND[entry["id"]]
    for new in REGISTER:
        followups.append({**new, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(len(ids), "->", len(followups))


if __name__ == "__main__":
    main()
