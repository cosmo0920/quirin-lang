#!/usr/bin/env python3
"""Apply W47-ACCESS-PATH-POSITIONS's ledger edits once.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires W46-ACCESS-PATH-POSITIONS: all three positions it names are
  implemented with its discharge conditions and measured (README);
* amends W43-INDIRECT-UNTYPED-SINGLE-ACCESS (the enum arm it asked for) and
  W46-ACCESS-PATH-PARITY (the position rows now in the parity stream), each
  by one appended sentence starting with MARK (skipped when present);
* appends every entry of `followups.json` (refusing a duplicate id).
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-access-path-positions"
MARK = "AMENDED (W47-ACCESS-PATH-POSITIONS):"
RETIRE = {"W46-ACCESS-PATH-POSITIONS"}
AMEND = {
    "W43-INDIRECT-UNTYPED-SINGLE-ACCESS": (
        " " + MARK + " the owner-aware enum arm landed as its own measured step (typed_access_path.rs "
        "ENUM_ARM; QCore.TypedAccessPathPositions.the_enum_arm_only_adds_enumerations): `enum TAG` for a "
        "declared tag, or a typedef whose declaration is directly an EnumType, is a scalar; anonymous "
        "aggregate, union and pointer-to-enum typedefs keep the refusal (unit and integration controls). "
        "Measured alone (step A -> step B arbiter): 13 more corpus functions Unknown -> Safe, among them "
        "the four P2 enum misses otel_span_set_kind, LZ4F_resetDecompressionContext, "
        "rd_kafka_cgrp_clear_last_err and ut_set_types; the system slice does not move. An anonymous "
        "inline enum member (`enum { ... } state;`) stays refused and is tracked by "
        "W47-ACCESS-PATH-POSITION-RESIDUALS."
    ),
    "W46-ACCESS-PATH-PARITY": (
        " " + MARK + " the parity stream now also carries condition, return and call-actual rows with a "
        "`position` key: every typed fallback answer where the older routes refused, and the typed "
        "candidate beside every condition-access, return-family and lexical call-argument admission "
        "(AFTER corpus, de-duplicated: 1,270 / 219 condition-access rows with the typed candidate "
        "admitting / refusing, 266 / 142 return-family, 187 / 463 call-argument). They are observations "
        "in distinct vocabularies; no route is retired and no parity is claimed."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    missing = sorted((set(AMEND) | RETIRE) - set(ids))
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
    kept = [entry for entry in entries if entry["id"] not in RETIRE]
    retired = len(entries) - len(kept)
    kept.extend(new)
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; amended {amended}; registered {len(new)}; retired {retired}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
