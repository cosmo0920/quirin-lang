#!/usr/bin/env python3
"""Apply W44-INDIRECT-CALL-ARGUMENT-ACCESS' hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `W43-INDIRECT-CALL-ARGUMENT-ACCESS` (writing the entry verbatim with
  its ground to `retired-followups.json`);
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`.
Adapted from docs/reports/w43-indexed-resource-collections/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-indirect-call-argument-access"
RETIRED = "W43-INDIRECT-CALL-ARGUMENT-ACCESS"
GROUND = (
    "Its discharge condition: \"a card that admits a typed scalar access in a Borrowed data argument "
    "or as a call result's scalar destination, with the call's existing obligations retained, and a "
    "measured re-pin; or a recorded decision that the class belongs to the call routes\". Card "
    "W44-INDIRECT-CALL-ARGUMENT-ACCESS admits typed reads (scalar values over typed scalar accesses, "
    "pointer reads through `T **` bases and pointer members, non-pointer sub-object addresses) at "
    "declared Borrowed positions and in declared variadic data tails of contracted calls, retaining "
    "every obligation of the call (the statement is classified again by the ordinary classifier) and "
    "adding the typed read's dereference obligation and a second obligation before it, with Lean "
    "parity in QCore.CallArgumentAccess and a measured re-pin "
    "(docs/reports/w44-indirect-call-argument-access/README.md). The uncontracted part is recorded as "
    "belonging to the call routes (W44-CALL-ARGUMENT-UNCONTRACTED); the call-result destination "
    "(`p->f = g()`) is re-registered, measured, as W44-CALL-RESULT-INDIRECT-DESTINATION; every other "
    "refused shape is re-registered narrower with its AFTER-census size."
)


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    retired = [entry for entry in entries if entry["id"] == RETIRED]
    if len(retired) != 1:
        raise SystemExit(f"{RETIRED} is not registered exactly once")
    (REPORT / "retired-followups.json").write_text(
        json.dumps(
            [{"entry": retired[0], "discharged_by": "W44-INDIRECT-CALL-ARGUMENT-ACCESS", "ground": GROUND}],
            indent=2,
            ensure_ascii=False,
        )
        + "\n"
    )
    entries = [entry for entry in entries if entry["id"] != RETIRED]
    ids = {entry["id"] for entry in entries}
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
            "note": "W44-INDIRECT-CALL-ARGUMENT-ACCESS, 2026-09-25. BEFORE (base c1e69bb5) reproduces the "
            "W43-INDEXED-RESOURCE-COLLECTIONS integration diagnostics byte for byte. Typed reads in the "
            "Borrowed and variadic-tail actuals of contracted calls are admitted under the base's "
            "dereference obligation. AFTER: 186 leaves removed and 47 added, every one attributed; "
            "functions 1 Unknown -> Safe (cio_file_hash_print) and 7 Unknown -> provisional Unsafe; "
            "units 3 Unknown -> provisional Unsafe; leak 271 -> 282, eleven exposures of pre-existing "
            "leak semantics that are false by source reading (W44-CALL-ARGUMENT-EXPOSED-LEAKS); 0 "
            "use_after_release. Provisional Unsafe is not an upstream defect claim. Previous row "
            "preserved in docs/reports/w44-indirect-call-argument-access/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-indirect-call-argument-access/artifacts/",
            "source": "docs/reports/w44-indirect-call-argument-access/README.md; preceding baseline "
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
    sys.exit(main())
