#!/usr/bin/env python3
"""Apply W47-ALIAS-RESIDUALS-CENSUS's ledger edits once.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: a measurement card discharges no entry;
* amends W47-ACCESS-PATH-POSITION-RESIDUALS and W47-TYPED-ROOT-DECLARATION-READER
  with this card's alias figures, each by one appended sentence starting with
  MARK (skipped when the marker is already present);
* appends every entry of `followups.json` (refusing a duplicate id).
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-alias-residuals-census"
MARK = "AMENDED (W47-ALIAS-RESIDUALS-CENSUS):"
AMEND = {
    "W47-ACCESS-PATH-POSITION-RESIDUALS": (
        " " + MARK + " on the integrated wave-47 baseline (3ba72320, corpus diagnostics byte-identical to "
        "the P3 AFTER), 950 of the 7,440 corpus unknown_alias leaves are pointers null-tested and then "
        "poisoned by an unsupported statement of the same function between the test and the leaf (rule "
        "10a); the first such refusal is IND for 497, UNC 276, COND 98, other 79. 232 Unknown functions are "
        "blocked by unknown_alias plus IND leaves only, 10 by alias plus COND and 10 by alias plus RET "
        "(docs/reports/w47-alias-residuals-census/artifacts/census/tables.json)."
    ),
    "W47-TYPED-ROOT-DECLARATION-READER": (
        " " + MARK + " the same never-bound closure refuses 2,350 corpus unknown_alias leaves on pointer "
        "formals that no statement assigns, because their names are mentioned inside actuals of "
        "uncontracted callees (typed parity rows: root_never_bound false for 2,060 of them); registered "
        "separately as W47-ALIAS-CALLEE-BORROWED-CLOSURE, whose discharge (a Borrowed position of a "
        "contracted or validated callee, a member-address mention) is independent of the declaration reader."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    missing = sorted(set(AMEND) - set(ids))
    if missing:
        raise SystemExit(f"not registered on this base: {missing}")
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    amended = 0
    for entry in entries:
        if entry["id"] in AMEND and MARK not in entry["why"]:
            entry["why"] = entry["why"] + AMEND[entry["id"]]
            amended += 1
    entries.extend(new)
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(entries)}; amended {amended}; registered {len(new)}; retired 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
