#!/usr/bin/env python3
"""Apply W43-LIBC-PRINTF-FAMILY's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `LIBC-PRINTF-FAMILY`, which must be registered,
  recording every retired entry verbatim with its ground beside the report;
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections (including `dimensions.named_followups`) are left to
`docs/reports/remaining-d02/build_matrix.py`, which the caller runs afterwards.
Adapted from docs/reports/w43-libc-result-alias/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-libc-printf-family"
GROUNDS = {
    "LIBC-PRINTF-FAMILY": (
        "Its discharge condition is a dedicated leaf-directed card landing the family together with its "
        "own leaf-by-leaf attribution. Card W43-LIBC-PRINTF-FAMILY declared printf, fprintf, snprintf and "
        "sprintf in providers/quirin-provider-libc/contracts/libc/stdio.qcore (all-borrowed, "
        "resource_variadic_data_tail(), resource_return_none()), each agreeing with the declared ABI in "
        "all 168 units that declare it, and ran a four-stage arbiter attributed per member: 2,212 "
        "uncontracted-callee leaves removed (printf 22, fprintf 1,138, snprintf 1,044, sprintf 8), 2,229 "
        "leaves removed and 1,283 added in all, every one attributed; five functions Unknown -> Safe "
        "(cmt_errno_print, ctr_errno_print, cio_errno_print, rd_kafka_mk_nodename, cJSON_Version), "
        "neither forecast exchange firing; 0 unit moves; leak 272. The va_list members are refused by "
        "name on a measured reader ground (LIBC-PRINTF-VA-LIST), and the exchange is registered as "
        "LIBC-PRINTF-TAIL-ACTUALS and LIBC-PRINTF-NESTED-TAIL-CALLEES. "
        "docs/reports/w43-libc-printf-family/README.md."
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
            "discharged_by": "W43-LIBC-PRINTF-FAMILY",
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
            "note": "W43-LIBC-PRINTF-FAMILY, 2026-09-25. Staged arbiter, one release binary of the base: BEFORE "
            "reproduces the W43-LIBC-RESULT-ALIAS diagnostics byte for byte; four cumulative stages declare "
            "printf, fprintf, snprintf and sprintf and are attributed per member (2,229 leaves removed, 1,283 "
            "added, all attributed; 1,179 of the added are declared-tail refusals and 100 nested-call "
            "refusals). 5 functions Unknown -> Safe, each relative to the declared premises; 0 unit status "
            "moves; 0 double_release, invalid_release or use_after_release candidates. Leak 272 preserved. "
            "Raw units remain unpromoted; provisional Unsafe is not an upstream defect claim. Previous row "
            "preserved in docs/reports/w43-libc-printf-family/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-libc-printf-family/artifacts/",
            "source": "docs/reports/w43-libc-printf-family/README.md; preceding baseline "
            "docs/reports/w43-libc-result-alias/artifacts/",
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
