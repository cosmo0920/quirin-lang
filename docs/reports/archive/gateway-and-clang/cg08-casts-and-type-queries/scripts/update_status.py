#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES: the STATUS rows this card moves, edited once.

usage: update_status.py STATUS_MD

Each edit replaces the head of one row's value cell and asserts it matched.
"""
import sys

EDITS = [
    (
        "| Projected functions | **586 Safe / 10 provisional Unsafe / 4,238 Unknown**, 4,834 total (CG04-LOOP-SITE-GENERATIONS",
        "| Projected functions | **602 Safe / 10 provisional Unsafe / 4,222 Unknown**, 4,834 total "
        "(CG08-CASTS-AND-TYPE-QUERIES, gateway 11b: casts read as representations with no custody effect "
        "(`PointerToBoolean` the null test, a pointer's integer data only when never bound, an integer's "
        "pointer with no identity), an explicit object-pointer cast at a pointer destination lowered as the "
        "uncast statement, type-query operands read by evaluation class from Clang; **+16 Unknown → Safe** "
        "(11 uncast pointer statements, 4 elaborated statements, conditions and `?:` edges holding type "
        "queries, 1 Clang-confirmed `sizeof` operand at the gate), no correction; a wrong Safe of the `return` "
        "and store routes repaired (a pointer returned or stored as a truth value escaped or bound it), no "
        "corpus instance; 586 / 10 / 4,238 before it, at main `9290ff1a`; CG04-LOOP-SITE-GENERATIONS",
    ),
    (
        "| Corpus diagnostics | ",
        "| Corpus diagnostics | CG08-CASTS-AND-TYPE-QUERIES: leak 40 → 41 (the `then` twin of "
        "`mk_sched_add_connection`'s existing provisional leak), summary conflicts 1 → 2, unknown_alias +352, "
        "unsupported −711 (its own per-category counts, `artifacts/diagnostic-totals.txt`); before it: ",
    ),
    (
        "**237 functions: 34 Safe / 2 provisional Unsafe / 198 Unknown / 3 not projected** at ",
        "**237 functions: 37 Safe / 2 provisional Unsafe / 195 Unknown / 3 not projected** at "
        "CG08-CASTS-AND-TYPE-QUERIES (gateway 11b: `calculate_chunk_capacity_percent`'s floating return, "
        "`flb_input_chunk_get_name`'s and `flb_task_from_fs_storage`'s uncast pointer statements: +3; "
        "34 / 2 / 198 before it); at ",
    ),
    (
        "| Registered follow-ups | **638** (wave 48: ",
        "| Registered follow-ups | **633** (wave 48: CG08-CASTS-AND-TYPE-QUERIES retires its own entry and "
        "the absorbed `W44-SIZEOF-OPERAND-RESIDUALS`, `W44-SIZEOF-CAST-SCALAR-QUERY`, "
        "`SIZEOF-TYPE-FOLD-NO-LEAN`, `CAST-RETURN-UNCOMMITTED-OPERAND` and `W45-CALLEE-CAST-LEXICAL-PREMISE`, "
        "amends `CG02-COMMA-OPERAND-RESIDUALS`, and registers `CG08-CAST-AND-QUERY-RESIDUALS`: 638 → 633; ",
    ),
]


def main():
    path = sys.argv[1]
    text = open(path, encoding="utf-8").read()
    for old, new in EDITS:
        assert text.count(old) == 1, old
        text = text.replace(old, new, 1)
    open(path, "w", encoding="utf-8").write(text)


if __name__ == "__main__":
    main()
