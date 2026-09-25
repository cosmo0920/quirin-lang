#!/usr/bin/env python3
"""Apply W45-CALLEE-SUMMARY-CASCADE' hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires nothing: no registered entry's discharge condition is met by this card
  (see the report's ledger section);
* replaces this card's own entries with `followups.json` (refusing any other card's
  duplicate id), so a rerun after a repair is idempotent;
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w44-corpus-callee-summaries/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-callee-summary-cascade"
CARD = "W45-CALLEE-SUMMARY-CASCADE"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    # Idempotent (review repair): this card's own earlier entries are replaced,
    # never duplicated; every other card's entry is refused as a duplicate.
    entries = [entry for entry in matrix["named_followups"] if entry.get("recorded_by") != CARD]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    matrix["named_followups"] = entries + new

    corpus = matrix["corpus"]
    previous = REPORT / "previous-corpus-row.json"
    if not previous.exists():  # the row before this card, kept from the first run
        previous.write_text(json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n")
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
            "note": "W45-CALLEE-SUMMARY-CASCADE, 2026-09-26, after its integration review. BEFORE (base binary "
            "b160d765) reproduces the W44-COLLECTION-ALLOCATION integration diagnostics, chunk-contracts and source "
            "byte for byte. AFTER admits `B->F` at a Released { nullable } position only as the last use of its "
            "place (gateway constraint 5k premises P1-P4), lowers a pointer member null test to the typed read of "
            "its base, obliges every dereferencing null test's bases (the AGGREGATE-NULL-TEST-AFTER-RELEASE "
            "repair), and guards calls to undeclared unit-defined callees that may release through a formal: 350 "
            "changed leaves, all attributed (27 member releases, 247 member null tests, 10 desugared operands, 1 "
            "decided statement, 6 decided callees, 4 lifted poisonings removed; 34 base exchanges, 4 decided "
            "same-line identities and 17 aggregate-base obligations added); 7 functions and the unit "
            "chunkio/cio_error.c move Unknown -> Safe; no function leaves Safe or Unsafe; 0 leak or violation "
            "leaves added or removed. Leak 37 preserved. Raw units remain unpromoted; provisional Unsafe is not "
            "an upstream defect claim. Previous row preserved in "
            "docs/reports/w45-callee-summary-cascade/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w45-callee-summary-cascade/artifacts/",
            "source": "docs/reports/w45-callee-summary-cascade/README.md; preceding baseline "
            "docs/reports/w44-collection-allocation/integration/artifacts/",
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
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only",
                                          "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
