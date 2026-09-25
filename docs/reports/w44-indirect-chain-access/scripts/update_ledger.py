#!/usr/bin/env python3
"""Apply W44-INDIRECT-CHAIN-ACCESS's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-INDIRECT-CHAIN-ACCESS` and `W43-INDEX-TRAVERSAL-CONDITION-CHAIN`,
  each of which must be registered, recording each retired entry verbatim with
  its ground beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-index-traversal/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-indirect-chain-access"
GROUNDS = {
    "W43-INDIRECT-CHAIN-ACCESS": (
        "Its discharge condition: \"a card that extends the route to record-pointer member chains with "
        "each intermediate dereference checked against field links, Lean parity and a measured re-pin; "
        "or a measured decision that the reader classes do not justify it\". Card "
        "W44-INDIRECT-CHAIN-ACCESS extends W43's typed statement route to member chains of two to four "
        "selections whose every link is exactly `struct POINTEE *` and whose final member is scalar. "
        "Each dereferenced link is checked flow-sensitively: an untracked owner's link is untracked, and "
        "a tracked owner's link must be the member place an admitted store bound, with the recorded field "
        "link and a live child (`indirect_chain_access::chain_resolves`). It has Lean parity in "
        "QCore.IndirectChainAccess and a measured re-pin (docs/reports/w44-indirect-chain-access/README.md): "
        "74 leaves admitted, 24 removed, 50 exchanged for 49 unknown_alias, 0 function moves. The class "
        "measured 5,522 / 2,028 / 213 at the AFTER census, of which 3,315 leaves carry no member chain. "
        "The remainder is re-registered by measured shape: W44-CHAIN-NESTED-AGGREGATE, "
        "W44-CHAIN-TYPEDEF-ROOT, W44-CHAIN-UNDECLARED-ROOT, W44-CHAIN-SUBSCRIPT, "
        "W44-CHAIN-INCOMPLETE-RECORD, W44-CHAIN-CALL, W44-CHAIN-NON-SCALAR-MEMBER, W44-CHAIN-BOUND-ROOT "
        "and W44-CHAIN-CLASS-NOT-A-CHAIN."
    ),
    "W43-INDEX-TRAVERSAL-CONDITION-CHAIN": (
        "Its discharge condition: \"the same card that extends the statement route to record-pointer "
        "chains (each intermediate dereference checked against field links) extends the condition read, "
        "measured together\". Card W44-INDIRECT-CHAIN-ACCESS extends W43-INDEX-TRAVERSAL's typed "
        "condition read with the same chain reader and the same per-link obligation, measured together "
        "with the statement route (26 condition leaves admitted: 2 removed and 24 exchanged). The "
        "condition class measured 1,227 / 622 / 1 at the AFTER census, of which 249 leaves carry no "
        "member chain. The remainder is re-registered with the statement residuals, each entry giving "
        "its condition share."
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
            "discharged_by": "W44-INDIRECT-CHAIN-ACCESS",
            "ground": ground,
        })
    (REPORT / "retired-followups.json").write_text(json.dumps(retired, indent=2) + "\n")
    before = len(entries)
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
            "note": "W44-INDIRECT-CHAIN-ACCESS, 2026-09-25. BEFORE (base c1e69bb5) reproduces the "
            "W43-INDEXED-RESOURCE-COLLECTIONS integration diagnostics byte for byte, and the "
            "census-instrumented base run reproduces all five BEFORE artifacts. Typed record-pointer "
            "member chains in statements and conditions remove 74 unsupported leaves (24 admitted, 50 "
            "refused by the chain obligation) and add 49 unknown_alias exchanges, all attributed; 0 "
            "downstream, 0 violation candidates. 0 function and 0 unit moves, leak pin 271 unchanged. "
            "No bounds claim. Provisional Unsafe is not an upstream defect claim. Previous row preserved "
            "in docs/reports/w44-indirect-chain-access/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-indirect-chain-access/artifacts/",
            "source": "docs/reports/w44-indirect-chain-access/README.md; preceding baseline "
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
    print(f"follow-ups: {before} -> {len(matrix['named_followups'])}")
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
