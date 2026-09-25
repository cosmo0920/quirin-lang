#!/usr/bin/env python3
"""Apply W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires nothing: `W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE` is registered on
  card W44-INDIRECT-CALL-ARGUMENT-ACCESS's branch, not on this card's base
  `b4430a14`, and removing a never-registered name would be a no-op. The
  discharge is recorded in the report for the integrator to reconcile;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`,
which the caller runs afterwards. Adapted from
docs/reports/w44-indirect-chain-access/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-assignment-destination-suffix-place"
CARD = "W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    if CARD in ids:
        raise SystemExit(f"{CARD} is registered on this base; retire it instead")
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    before = len(entries)
    matrix["named_followups"] = entries + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(
        json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n"
    )
    diagnostics = json.loads(gzip.decompress((REPORT / "artifacts/diagnostics.json.gz").read_bytes()))
    functions = collections.Counter()
    units = collections.Counter()
    reason_codes = collections.Counter()
    for unit in diagnostics["units"]:
        units[unit["status"]] += 1
        reason_codes[unit.get("reason_code") or "none"] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
    counts = diagnostics["diagnostic_counts"]
    row = dict(corpus["cycle19_raw_units"])
    row.update(
        {
            "note": "W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE, 2026-09-25, a soundness repair. BEFORE "
            "(base b4430a14) reproduces the combined W44-INDIRECT-CHAIN-ACCESS integration diagnostics "
            "byte for byte, and both census runs reproduce their trees' five artifacts. An assignment's "
            "destination is named only when the whole left-hand side is that place or a declarator "
            "prefix precedes it; 1,450 stores into a sub-object that were admitted under a misnamed "
            "destination are refused. Every delta is attributed: 186 gate leaves added, 258 exchanged "
            "descriptions and 1,115 unknown_alias reports of the misnamed aliases removed, 2 leak leaves "
            "withdrawn (unpack_basic_type), and 5/5 caller leaves of the two functions that lost their "
            "W4 summary. Functions: LZ4_write16 and LZ4_write32 Safe -> Unknown, unpack_basic_type "
            "provisional Unsafe -> Unknown. 0 unit moves; leak 271 -> 269. No bounds claim. Provisional "
            "Unsafe is not an upstream defect claim. Previous row preserved in "
            "docs/reports/w44-assignment-destination-suffix-place/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-assignment-destination-suffix-place/artifacts/",
            "source": "docs/reports/w44-assignment-destination-suffix-place/README.md; preceding "
            "baseline docs/reports/w44-indirect-chain-access/integration/artifacts/",
            "unknown_alias_diagnostics": counts["unknown_alias"],
            "unsupported_diagnostics": counts["unsupported"],
            "double_release_diagnostics": counts.get("double_release", 0),
            "function_statuses": {
                "safe": functions["Safe"],
                "provisional_unsafe": functions["Unsafe"],
                "unknown": functions["Unknown"],
            },
            "unit_reason_codes": dict(sorted(reason_codes.items(), key=lambda kv: (-kv[1], kv[0]))),
        }
    )
    corpus["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {before} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only",
                                          "function_statuses", "leak_diagnostics",
                                          "unknown_alias_diagnostics", "unsupported_diagnostics")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
