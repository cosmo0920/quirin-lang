#!/usr/bin/env python3
"""Apply W43-INDEX-TRAVERSAL's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-INDIRECT-POINTER-INCREMENT`, which must be registered,
  recording the retired entry verbatim with its ground beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-indirect-access-general/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-index-traversal"
GROUNDS = {
    "W43-INDIRECT-POINTER-INCREMENT": (
        "Its discharge condition: \"a card that models pointer arithmetic on a bound name as an "
        "interior derivation of the same site, with the release rules of QCore.DerivedResult, and a "
        "measured re-pin\". Card W43-INDEX-TRAVERSAL models `P++`, `P += E`, `P = Q +/- E` and the "
        "step of `*P++` as `LegacyInstruction::PointerStep`, an interior derivation of the source "
        "binding's own sites (Derivation::Derived with a Stepped marker), with DerivedResult's release "
        "rules (a forward-stepped release is an invalid_release candidate, one that may be at the start "
        "refuses, a use after the base's release is a use_after_release candidate), Lean parity in "
        "QCore.IndexTraversal, and a measured re-pin (docs/reports/w43-index-traversal/README.md). Its "
        "`*p++ = c` / `c = *p++` shapes are admitted as the typed access followed by the step. The same "
        "residual classifier over this card's AFTER census measures the class at 257 leaves / 159 "
        "functions (from 335 / 172); the remainder, `*(p + i)` and steps the reader does not type, is "
        "re-registered, narrower, as W43-INDEX-TRAVERSAL-COMPUTED-POINTER-ACCESS."
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
            "discharged_by": "W43-INDEX-TRAVERSAL",
            "ground": ground,
        })
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
    matrix["named_followups"] = [entry for entry in entries if entry["id"] not in GROUNDS] + new

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
            "note": "W43-INDEX-TRAVERSAL, 2026-09-25. BEFORE (merged base binary) reproduces the "
            "W43-INDIRECT-ACCESS-GENERAL diagnostics byte for byte, and so does the census-instrumented "
            "base run. Typed condition reads, pointer steps (interior-derived), stepped accesses, "
            "unbound local pointer copies and the declared-scalar fact join remove 1,479 leaves and add "
            "130, all attributed: 614 condition, 302 step, 16 gated step, 517 scalar/copy alias, 10 "
            "summary, 20 binding-precision; 127 exchanges and 3 scalar-join exchanges; 0 violation "
            "candidates. 11 functions Unknown -> Safe (all forecast; 2 forecast functions stay "
            "Unknown), 0 unit moves, leak pin 271 unchanged. No bounds claim. Provisional Unsafe is not "
            "an upstream defect claim. Previous row preserved in "
            "docs/reports/w43-index-traversal/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-index-traversal/artifacts/",
            "source": "docs/reports/w43-index-traversal/README.md; preceding baseline "
            "docs/reports/w43-indirect-access-general/artifacts/",
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
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
