#!/usr/bin/env python3
"""Apply W44-CORPUS-CALLEE-SUMMARIES' hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires nothing: no registered entry's discharge condition is met by this card
  (see the report's section 7);
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-result-alias-returned/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-corpus-callee-summaries"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    matrix["named_followups"] = entries + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n")
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
            "note": "W44-CORPUS-CALLEE-SUMMARIES, 2026-09-25. BEFORE (base binary c1e69bb5) reproduces the "
            "W43-INDEXED-RESOURCE-COLLECTIONS integration diagnostics, chunk-contracts and source byte for "
            "byte. AFTER widens the W4 derivation scope to reached static (internal-linkage), non-reserved "
            "header definitions: 70 changed leaves, all attributed (42 uncontracted-call leaves decided at "
            "16 newly validated callees, 13 same-line unknown_alias replacements at effectful Released/Owned "
            "call sites, 11 condition/indirect refusals whose statement calls a decided callee, 4 "
            "same-function re-attributions); mk_core/mk_event.c mk_event_init and mk_event_backend move "
            "Unknown -> Safe; 0 unit status moves; 0 violation leaves added or removed. Leak 271 preserved. "
            "Raw units remain unpromoted; provisional Unsafe is not an upstream defect claim. Previous row "
            "preserved in docs/reports/w44-corpus-callee-summaries/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-corpus-callee-summaries/artifacts/",
            "source": "docs/reports/w44-corpus-callee-summaries/README.md; preceding baseline "
            "docs/reports/w43-indexed-resource-collections/integration/artifacts/",
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
