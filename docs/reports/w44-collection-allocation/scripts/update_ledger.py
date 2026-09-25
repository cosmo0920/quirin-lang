#!/usr/bin/env python3
"""Apply W44-COLLECTION-ALLOCATION's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION` (refusing if it is not
  registered, so a no-op retirement cannot be reported as one);
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`,
which the caller runs afterwards. Adapted from
docs/reports/w43-indexed-resource-collections/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-collection-allocation"
RETIRED = ["W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION"]


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for retired in RETIRED:
        if retired not in ids:
            raise SystemExit(f"{retired} is not registered")
    entries = [entry for entry in entries if entry["id"] not in RETIRED]
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
            "note": "W44-COLLECTION-ALLOCATION, 2026-09-25, on main c1e69bb5. BEFORE reproduces the "
            "W43-INDEXED-RESOURCE-COLLECTIONS integration diagnostics byte for byte. A `sizeof` "
            "dereference operand (`*NAME`, `**NAME`, `NAME[LIT]`) over a pointer declared with no "
            "array dimension and a never-variably-modified specifier is unevaluated (C11 6.5.3.4p2) "
            "and leaves the lexical access gate of an ordinary call statement. AFTER: 234 leaves "
            "removed (204 withdrawn operand refusals), 370 added; functions 294/85/4455 -> "
            "298/129/4407 (4 Unknown -> Safe, 44 Unknown -> provisional Unsafe), units 3/43/135/4 -> "
            "3/65/113/4; leak 271 -> 396, of which 109 added leak leaves are unwitnessed claims on "
            "identities handed to refused statements or returned under a cast "
            "(W44-REFUSED-HANDOFF-LEAK-CLAIMS) and 16 are witnessed unit-test assertion-failure "
            "returns. No function is Safe through a collection element node. Provisional Unsafe is "
            "not an upstream defect claim. Previous row preserved in "
            "docs/reports/w44-collection-allocation/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-collection-allocation/artifacts/",
            "source": "docs/reports/w44-collection-allocation/README.md; preceding baseline "
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
    print(json.dumps({
        "followups": len(matrix["named_followups"]),
        "units": [row["safe"], row["provisional_unsafe"], row["unknown"], row["data_only"]],
        "functions": row["function_statuses"],
        "leak": row["leak_diagnostics"],
        "reason_codes": row["unit_reason_codes"],
    }))
    return 0


if __name__ == "__main__":
    sys.exit(main())
