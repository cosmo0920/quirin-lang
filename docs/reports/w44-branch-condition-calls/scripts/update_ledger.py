#!/usr/bin/env python3
"""Apply W44-BRANCH-CONDITION-CALLS's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-INDEX-TRAVERSAL-CONDITION-CALL`, which must be registered,
  recording the retired entry verbatim with its ground beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`,
which the caller runs afterwards. Adapted from
docs/reports/w43-index-traversal/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-branch-condition-calls"
GROUNDS = {
    "W43-INDEX-TRAVERSAL-CONDITION-CALL": (
        "Its discharge condition: \"a card that composes the B-series isolated call with the typed "
        "read for the residual (both obligations kept), with Lean parity and a measured re-pin; or a "
        "recorded decision that the class belongs to the call routes\". Card W44-BRANCH-CONDITION-CALLS "
        "desugars a refused call-bearing controlling expression into each call as its own ContractCall "
        "(the ordinary statement classifier's instruction, every obligation kept) and one branch per "
        "operand, whose call-free operands decide through the existing null, validity, distinct, neutral "
        "or typed-read route; `&&`, `||` and `?:` become edges. Lean parity is QCore.ConditionCallDesugar "
        "(AuditConditionCallDesugar), and the re-pin is measured and attributed leaf by leaf "
        "(docs/reports/w44-branch-condition-calls/README.md): 292 condition leaves removed. What stays "
        "refused is refused by the call routes themselves and is re-registered by blocker with its size: "
        "W44-CONDITION-CALL-INDIRECT-ARGUMENT, -UNCONTRACTED, -FREE-OPERAND, -TYPE-OPERATOR, -ASSIGNMENT, "
        "-RECURSIVE-LOCAL, -ERRNO-RESIDUAL and -REFUSED-SHAPES (4,479 leaves in total over the five "
        "condition positions at the AFTER run)."
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
    retired = []
    for name, ground in GROUNDS.items():
        if name not in ids:
            raise SystemExit(f"{name} is not registered; nothing to retire")
        retired.append({
            "entry": next(entry for entry in entries if entry["id"] == name),
            "discharged_by": "W44-BRANCH-CONDITION-CALLS",
            "ground": ground,
        })
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
    matrix["named_followups"] = [entry for entry in entries if entry["id"] not in GROUNDS] + new

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
            "note": "W44-BRANCH-CONDITION-CALLS, 2026-09-25. BEFORE (base c1e69bb5) and the "
            "census-instrumented run reproduce the W43-INDEXED-RESOURCE-COLLECTIONS integration "
            "diagnostics byte for byte. Desugaring call-bearing controlling expressions (calls as "
            "ContractCall nodes in evaluation order, short-circuit connectives as edges) removes 292 "
            "condition leaves (276 forecast on Unknown functions, exact; 16 on provisional-Unsafe "
            "functions), exchanges 36 into unknown_alias at the same line (an unresolved identity now "
            "null-tested), moves one unknown_alias coordinate and decides one false leak (snappy.c, the "
            "null edge of rd_malloc). 0 function moves, 0 unit moves, leak 271 -> 270, 0 violation "
            "candidates. Provisional Unsafe is not an upstream defect claim. Previous row preserved in "
            "docs/reports/w44-branch-condition-calls/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-branch-condition-calls/artifacts/",
            "source": "docs/reports/w44-branch-condition-calls/README.md; preceding baseline "
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
