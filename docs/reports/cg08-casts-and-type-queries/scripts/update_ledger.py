#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES: the ledger edits, applied once.

usage: update_ledger.py MATRIX_JSON

Retires CG08-CASTS-AND-TYPE-QUERIES (every DISCHARGE clause holds) and the
five follow-ups it absorbs (W44-SIZEOF-OPERAND-RESIDUALS,
W44-SIZEOF-CAST-SCALAR-QUERY, SIZEOF-TYPE-FOLD-NO-LEAN,
CAST-RETURN-UNCOMMITTED-OPERAND, W45-CALLEE-CAST-LEXICAL-PREMISE); amends
CG02-COMMA-OPERAND-RESIDUALS (where the nanopb shape now refuses); registers
CG08-CAST-AND-QUERY-RESIDUALS with its measured sizes; keeps
`dimensions.named_followups` equal to the list length. Writes the matrix back
in its own form (indent=1, ensure_ascii=False, trailing newline).

Sizes are the AFTER corpus arbiter's (see the report, section 4), from
scripts/families.py, scripts/cast_refusals.py and scripts/callee_census.py.
"""

import json
import sys

REPORT = "docs/reports/cg08-casts-and-type-queries/README.md"
BY = "CG08-CASTS-AND-TYPE-QUERIES"

RETIRE = [
    "CG08-CASTS-AND-TYPE-QUERIES",
    "W44-SIZEOF-OPERAND-RESIDUALS",
    "W44-SIZEOF-CAST-SCALAR-QUERY",
    "SIZEOF-TYPE-FOLD-NO-LEAN",
    "CAST-RETURN-UNCOMMITTED-OPERAND",
    "W45-CALLEE-CAST-LEXICAL-PREMISE",
]

RESIDUAL_WHY = (
    "Found by CG08-CASTS-AND-TYPE-QUERIES (gateway rule 11b), measured on its AFTER corpus arbiter "
    "(AFTER-IDENTITY). (a) `sizeof` families still refused: SIZEOF-ANALYSIS leaves of `C operator `sizeof` "
    "requires type/operand analysis` (W44-SIZEOF-CAST-SCALAR-QUERY's family, 458/121 at BEFORE; mostly librdkafka "
    "`rd_kafka_buf_read_*` macro statements holding calls, and `id_bytes = (int) sizeof(uint16_t);` whose "
    "destination the lexical scalar-local premise of rule 10s's event lowering does not read), and SIZEOF-LINE "
    "other unsupported leaves on a line spelling `sizeof` (W44-SIZEOF-OPERAND-RESIDUALS' neighbourhood, 373/241 "
    "at BEFORE; most carry another access, a pointer-arithmetic actual or a call). (b) Explicit casts in refused "
    "assignments: ASSIGNMENT-CAST `unsupported assignment expression` leaves on a line spelling a pointer cast "
    "(193/153 at BEFORE): a cast over pointer arithmetic (CG16), an `IntegralToPointer` destination, a cast of "
    "a call. (c) The elaborator's remaining CG08 refusals: CAST-REFUSALS de-duplicated consultations, almost all "
    "`ArrayToPointerDecay` of an array actual (an address: CG16/CG10), `NullToPointer`/`BitCast` into a function "
    "pointer (CG22) and `PointerEscape` (a pointer read from a call or path converted to an integer). (d) A "
    "tracked pointer's integer used only in a comparison is refused (the brief's data-only use, unproved here). "
    "(e) An explicit `(bool)p` condition is admitted by an older condition route with no null refinement (the "
    "accusing direction; pinned by `an_explicit_truth_value_condition_is_not_yet_refined`). (f) Route T: the "
    "Monkey binding (`quirin-source-analysis` `monkey_binding.rs`) evaluates `SizeOf` from an LP64 host table, "
    "not Clang's constant (`cast_representation::clang_constant`). (g) From W44-SIZEOF-OPERAND-RESIDUALS: its "
    "(a), (b) and the operand side of (c) and (d) are admitted (Clang's node decides the operand; pinned by "
    "`tests/sizeof_operand.rs`), but the lexical declarator reader still refuses a function's declarations on a "
    "leading `( NAME * sizeof …` (its other readers keep that refusal), and (e), a `T **` allocation line with "
    "another access, is counted inside (a)'s SIZEOF-LINE. DISCHARGE: each sub-population closed by its owning "
    "card (CG16, CG10, CG22, CG20 for numbers) and re-measured, (d) with a data-use proof, (e) by an older-route "
    "amendment reading `PointerToBoolean` as the null refinement, (f) by the Monkey binding taking Clang's "
    "constant for the observed target, each with Lean parity."
)

REGISTER = [
    {
        "id": "CG08-CAST-AND-QUERY-RESIDUALS",
        "title": "Casts and type queries rule 11b leaves refused, by owner",
        "why": RESIDUAL_WHY,
    },
]

AMEND = {
    "CG02-COMMA-OPERAND-RESIDUALS": (
        " AMENDED (CG08-CASTS-AND-TYPE-QUERIES, gateway rule 11b): the elaborator now reads the comma value's "
        "`IntegralToBoolean`; the first refusal of the same 67 returns (32 functions, de-duplicated "
        "`elaboration-return` rows on its AFTER corpus) is the place reader's refusal of the pointer-valued member "
        "store `stream->errmsg = …` (`place: the typed access-path reader refused it (CG10)`, the store being "
        "CG11's), and rule 10z still refuses each by position (a region under the comma). CG08's clause of the "
        "DISCHARGE holds."
    ),
}


def fill(text, sizes):
    for key, value in sizes.items():
        text = text.replace(key, value)
    return text


def main():
    path = sys.argv[1]
    sizes = json.loads(sys.argv[2]) if len(sys.argv) > 2 else {}
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
        entry = {key: fill(value, sizes) for key, value in registered.items()}
        followups.append({**entry, "source": REPORT, "recorded_by": BY})
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    with open(path, "w", encoding="utf-8") as stream:
        json.dump(matrix, stream, indent=1, ensure_ascii=False)
        stream.write("\n")
    print(len(ids), "->", len(followups))


if __name__ == "__main__":
    main()
