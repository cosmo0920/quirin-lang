#!/usr/bin/env python3
"""Apply W46-ACCESS-PATH-CENSUS's matrix edits once.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: a measurement card discharges no entry;
* amends eleven entries with this card's figures, each by an appended
  sentence that starts with MARK (skipped when the marker is already present):
  the ten registered access classes (sized on the current baseline by their
  own registering scripts, and split by the typed census) and
  W44-CALLEE-CROSS-UNIT-SUMMARY (the closure figure);
* appends every entry of `followups.json` (refusing a duplicate id).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w46-access-path-census"
MARK = "AMENDED (W46-ACCESS-PATH-CENSUS):"
PRE = (" " + MARK + " re-measured on the w46-external-callee-contracts baseline by its registering "
       "script, run unchanged over this card's census run (classes.json): ")
TYPED = ("; Clang's typed AST of the analysed text types every step, and splits its IND/COND/RET leaves ")

AMEND = {
    "W43-INDIRECT-UNTYPED-SINGLE-ACCESS": PRE + "2,771 leaves / 1,071 Unknown functions / 69 sole-blocked"
    + TYPED + "1,711 P2-expressible / 0 P3 / 622 refused (pointer-valued store 434, aggregate value 107, "
    "union 100); 435 leaves are in families outside the typed census. Every base or member the lexical "
    "reader left untyped has a Clang type.",
    "W44-CHAIN-NESTED-AGGREGATE": PRE + "1,442 leaves / 548 / 33" + TYPED
    + "455 P2 / 450 P3 expressible / 489 refused (union 424, pointer-valued store 111); 48 outside.",
    "W44-CHAIN-TYPEDEF-ROOT": PRE + "588 leaves / 302 / 17" + TYPED
    + "287 P2 / 87 P3 expressible / 159 refused (pointer-valued store 97, union 77); 55 outside.",
    "W44-CHAIN-UNDECLARED-ROOT": PRE + "222 leaves / 63 / 4" + TYPED
    + "85 P2 / 79 P3 expressible / 51 refused (union 36, pointer-valued store 18); 7 outside.",
    "W44-CHAIN-INCOMPLETE-RECORD": PRE + "224 leaves / 159 / 10" + TYPED
    + "89 P2 / 91 P3 expressible / 39 refused (union 23, pointer-valued store 9); 5 outside. Every "
    "record on these chains is complete to Clang: the refusal is the lexical reader's.",
    "W44-CHAIN-CALL": PRE + "389 leaves / 258 / 15" + TYPED
    + "3 P2 / 252 P3 expressible / 131 refused (union 108, indirect call 36); 3 outside.",
    "W43-INDIRECT-POINTER-VALUE-STORE": PRE + "350 leaves / 239 / 25" + TYPED
    + "263 refused as pointer-valued stores and 10 that are scalar or null stores to Clang (P2-expressible); "
    "77 outside.",
    "W43-INDIRECT-OTHER-STATEMENTS": PRE + "1,414 leaves / 706 / 37" + TYPED
    + "649 P2 / 2 P3 expressible / 148 refused (pointer-valued store 101, union 23); 610 outside.",
    "W44-CONDITION-CALL-INDIRECT-ARGUMENT": PRE + "1,729 leaves / 648 / 19" + TYPED
    + "1,454 P3-expressible (call-argument position) / 275 refused (pointer-valued store 208, union 44).",
    "W44-CALL-RESULT-INDIRECT-DESTINATION": PRE + "943 leaves / 513 / 28 (family_residuals.py `destination`)"
    "; the typed reading (a store through a path whose value is a call) holds 1,592 IND leaves: 101 P2 / "
    "481 P3 expressible / 1,010 refused (pointer-valued store 965, union 127).",
    "W44-CALLEE-CROSS-UNIT-SUMMARY": (
        " " + MARK + " the callee closure over the typed prediction (PREDICTION.md): a cross-unit summary "
        "route would add 3 Safe and 5 Unknown functions under P3 (8 freed), and 3 Safe under P2; with every "
        "freed function taken Safe it reaches 227 + 15 in two rounds. After P3, 870 Unknown functions are "
        "blocked only by in-corpus callees, whose own blockers are mostly non-access families."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    missing = sorted(set(AMEND) - set(ids))
    if missing:
        raise SystemExit(f"not registered on this base: {missing}")
    amended = 0
    for entry in entries:
        if entry["id"] in AMEND and MARK not in entry["why"]:
            entry["why"] = entry["why"] + AMEND[entry["id"]]
            amended += 1
    entries.extend(new)
    matrix["named_followups"] = entries
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(entries)}; amended {amended}; registered {len(new)}; retired 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
