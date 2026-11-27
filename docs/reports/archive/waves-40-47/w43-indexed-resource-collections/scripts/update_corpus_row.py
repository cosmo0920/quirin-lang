#!/usr/bin/env python3
"""Point `corpus.cycle19_raw_units` at this card's archived AFTER artifacts.

Usage: update_corpus_row.py

Used after the second main merge (d2dd61c1): the follow-ups were merged entry
by entry by hand, so `update_ledger.py` (which appends them) is not rerun. The
previous row (main's, from the W43-DESCRIPTOR-RESOURCE-TRACKING integration)
must already be preserved in `previous-corpus-row.json`.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-indexed-resource-collections"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    previous = json.loads((REPORT / "previous-corpus-row.json").read_text())
    if previous != matrix["corpus"]["cycle19_raw_units"]:
        raise SystemExit("previous-corpus-row.json is not the matrix's current row")
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
    row = dict(previous)
    row.update(
        {
            "note": "W43-INDEXED-RESOURCE-COLLECTIONS on main d2dd61c1 (merge eea0b30b), 2026-09-25. "
            "Measured against the W43-DESCRIPTOR-RESOURCE-TRACKING integration baseline: 0 leaves "
            "removed or added, 0 function or unit status moves, leak 271, 0 violation candidates; the "
            "diagnostics are equal to that baseline except for the report's `scope` string, contracts "
            "are identical after the root alias, and source-effects differ only in checkout-path-derived "
            "digests. 27 element operations are lowered in 19 functions and every one keeps its "
            "fallback refusal. Provisional Unsafe is not an upstream defect claim. Previous row "
            "preserved in docs/reports/w43-indexed-resource-collections/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-indexed-resource-collections/artifacts/",
            "source": "docs/reports/w43-indexed-resource-collections/README.md; preceding baseline "
            "docs/reports/w43-descriptor-resource-tracking/integration/artifacts/",
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
    matrix["corpus"]["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "function_statuses", "leak_diagnostics")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
