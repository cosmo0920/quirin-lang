#!/usr/bin/env python3
"""Apply W45-RETURN-EXPRESSION-FAMILIES's hand-maintained matrix edits, once.

Usage: update_ledger.py

* appends every entry of `followups.json` to `named_followups` (refusing a
  duplicate id);
* retires `W43-RETURN-CALL-UNDER-CAST` and `W43-RETURN-TYPE-OPERATOR`, which
  must be registered, keeping each entry verbatim in `retired-followups.json`;
* appends W45's re-measured sizes to four W43 return residuals, keeping each
  previous text in `amended-followups.json`;
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  primary artifacts, first preserving the exact previous row beside the report.

Derived sections (including `dimensions.named_followups`) are left to the
generator, which the caller runs afterwards.
"""

import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-return-expression-families"
CARD = "W45-RETURN-EXPRESSION-FAMILIES"

RETIRED = {
    "W43-RETURN-CALL-UNDER-CAST": (
        "W45-RETURN-EXPRESSION-FAMILIES recognises a leading cast positively (identifiers the "
        "unit's declaration reader resolves as a never variably modified type, then `*`s; a "
        "parenthesised function designator is never read as a cast), lowers a refused call "
        "under casts to the classifier's own refusal (22 leaves) and admits an `Owned` result "
        "only under pointer casts of a declared object pointer, refusing an integer cast by "
        "name (QCore.ReturnExpressionFamilies.an_owned_result_under_casts_is_a_pointer_conversion, "
        "an_integer_cast_of_an_owned_result_is_refused); parentheses alone give W4's answer. "
        "Measured: the AFTER census has 0 whole-call leaves under a cast or in parentheses "
        "(census/census-after.txt), 4 admitted (XXH32_createState and XXH64_createState "
        "escape their fresh site)."
    ),
    "W43-RETURN-TYPE-OPERATOR": (
        "In a function whose definition declares a scalar value result, "
        "W45-RETURN-EXPRESSION-FAMILIES admits `sizeof`/`_Alignof` over identifiers the reader "
        "resolves as a never variably modified type (TypedAccessModel::never_variably_modified, "
        "W44-COLLECTION-ALLOCATION's VLA argument) as a constant that carries nothing. Measured: "
        "0 type-operator return leaves after (6 before, 4 sole-blocked, all four now Safe). The "
        "fixed type-query rule in other positions is unchanged; this entry sized only the return "
        "leaves."
    ),
}

AMENDED = {
    "W43-RETURN-ACCESS-OPERANDS": (
        " Narrowed by W45-RETURN-EXPRESSION-FAMILIES, which decides call-free reads in "
        "scalar-result functions without member typing (C converts the value to the declared "
        "scalar result): the family falls from 289 to 144 leaves over 130 Unknown functions, 27 "
        "sole-blocked (docs/reports/w45-return-expression-families/census/census-after.txt); the "
        "rest is the pointer-result half (W45-RETURN-POINTER-RESULT-READS) and bound-base "
        "positions (W45-RETURN-BOUND-BASE-POSITIONS)."
    ),
    "W43-RETURN-COMPOUND-CALL": (
        " Re-measured by W45-RETURN-EXPRESSION-FAMILIES on the 2026-09-25 baseline: 79 leaves "
        "over 73 Unknown functions, 35 sole-blocked, unchanged by that card, which admits no call "
        "inside a larger return expression; 65 of them are in scalar-result functions, 14 in "
        "pointer-result ones (census/residual-sizing.txt of that report)."
    ),
    "W43-RETURN-CONDITIONAL-OPERANDS": (
        " Narrowed by W45-RETURN-EXPRESSION-FAMILIES: 13 call-free conditional or short-circuit "
        "leaves over 13 Unknown functions remain (6 sole-blocked), pointer-valued conditionals and "
        "bound bases in a skipped position."
    ),
    "W43-RETURN-COMMA-MACRO": (
        " Re-measured by W45-RETURN-EXPRESSION-FAMILIES: 69 leaves over 34 Unknown functions (1 "
        "sole-blocked), unchanged."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    for name in list(RETIRED) + list(AMENDED):
        if name not in ids:
            raise SystemExit(f"{name} is not registered")
    retired = [
        {"entry": entry, "discharged_by": CARD, "ground": RETIRED[entry["id"]]}
        for entry in entries
        if entry["id"] in RETIRED
    ]
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
    amended = []
    kept = []
    for entry in entries:
        if entry["id"] in RETIRED:
            continue
        if entry["id"] in AMENDED:
            amended.append({"id": entry["id"], "previous_why": entry["why"], "appended": AMENDED[entry["id"]]})
            entry = dict(entry, why=entry["why"] + AMENDED[entry["id"]])
        kept.append(entry)
    (REPORT / "amended-followups.json").write_text(json.dumps(amended, indent=2) + "\n")
    matrix["named_followups"] = kept + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(
        json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n"
    )
    diagnostics = json.loads(gzip.decompress((REPORT / "artifacts/diagnostics.json.gz").read_bytes()))
    functions = {"Safe": 0, "Unsafe": 0, "Unknown": 0}
    units = {"Safe": 0, "Unsafe": 0, "Unknown": 0, "data_only": 0}
    for unit in diagnostics["units"]:
        units[unit["status"]] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
    counts = diagnostics["diagnostic_counts"]
    row = dict(corpus["cycle19_raw_units"])
    row.update(
        {
            "note": "W45-RETURN-EXPRESSION-FAMILIES, 2026-09-25, on main b160d765. The same-checkout "
            "BEFORE reproduces the W44-COLLECTION-ALLOCATION integration baseline's diagnostics, "
            "chunk-contracts and Monkey source byte for byte (contracts after the root alias; "
            "source-effects differ only in checkout-path digests). AFTER admits the scalar-result "
            "return read and the cast of a whole call: 405 leaves removed, 40 added, every one "
            "attributed (189 direct, 22 exchanges, 4 base obligations, 223 cascade and 7 flow "
            "leaves of callers whose callee now validates a summary); 76 functions Unknown -> Safe "
            "(72 sole-blocked by the return refusal, 4 by cascade), none to provisional Unsafe; "
            "units unchanged; leak 37 preserved. Raw units remain unpromoted; provisional Unsafe is "
            "not an upstream defect claim. Previous row preserved in "
            "docs/reports/w45-return-expression-families/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w45-return-expression-families/artifacts/",
            "source": "docs/reports/w45-return-expression-families/README.md; preceding baseline "
            "docs/reports/w44-collection-allocation/integration/artifacts/",
            "unknown_alias_diagnostics": counts["unknown_alias"],
            "unsupported_diagnostics": counts["unsupported"],
            "double_release_diagnostics": counts.get("double_release", 0),
            "function_statuses": {
                "safe": functions["Safe"],
                "provisional_unsafe": functions["Unsafe"],
                "unknown": functions["Unknown"],
            },
        }
    )
    corpus["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
