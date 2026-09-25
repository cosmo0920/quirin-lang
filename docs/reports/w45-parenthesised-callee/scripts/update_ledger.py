#!/usr/bin/env python3
"""Apply W45-PARENTHESISED-CALLEE's hand-maintained matrix edits, once.

Usage: update_ledger.py

* appends every entry of `followups.json` to `named_followups`, refusing a
  duplicate id;
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row in
  `previous-corpus-row.json`.

The card retires nothing. Derived sections are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs
afterwards. Adapted from docs/reports/w45-sub-object-stores/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w45-parenthesised-callee"
NOTE = (
    "W45-PARENTHESISED-CALLEE, 2026-09-26. BEFORE (base 1db9a569) reproduces the W45-SUB-OBJECT-STORES "
    "integration diagnostics byte for byte. A parenthesised name callee `(f)(x)` is rewritten to `f(x)` in "
    "every function body before any reader runs; every other callee expression that is not a name refuses "
    "where its position would otherwise be admitted; a `for` init or increment clause whose original "
    "spelling carries grouping tokens refuses unless its stripped classification already did (gateway "
    "constraint 10c). Every delta is attributed: 44 for-clause refusals added and 31 `unresolved resource "
    "identity` reports of the same clauses removed, all in functions already Unknown; rule 1 rewrote only "
    "casts the typedef reader had missed on the first run and no call in the pinned corpus, and rules 1 "
    "and 2 move no leaf. Functions: 0 moves; 0 unit moves; leak 37 unchanged, every leak and violation leaf "
    "identical. No bounds claim. Provisional Unsafe is not an upstream defect claim. Previous row preserved "
    "in docs/reports/w45-parenthesised-callee/previous-corpus-row.json."
)


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    before = len(entries)
    entries.extend(new)

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
            "note": NOTE,
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w45-parenthesised-callee/artifacts/",
            "source": "docs/reports/w45-parenthesised-callee/README.md; preceding baseline "
            "docs/reports/w45-sub-object-stores/integration/artifacts/",
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
    print(f"named_followups: {before} -> {len(entries)}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only",
                                          "function_statuses", "leak_diagnostics",
                                          "unknown_alias_diagnostics", "unsupported_diagnostics")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
