#!/usr/bin/env python3
"""Apply W44-C11-GENERIC-SELECTION's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires nothing: no registered follow-up names the generic selection (the
  card checked `named_followups` for `_Generic`, `generic selection` and the
  card id before editing);
* appends every entry of `followups.json` (refusing a duplicate id);
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  archived AFTER artifacts, first preserving the exact previous row.

Derived sections are left to `docs/reports/remaining-d02/build_matrix.py`,
which the caller runs afterwards. Adapted from
docs/reports/w44-libc-residuals/scripts/update_ledger.py.
"""

import collections
import gzip
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w44-c11-generic-selection"
README = "docs/reports/w44-c11-generic-selection/README.md"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    for entry in entries:
        text = json.dumps(entry)
        if "_Generic" in text or "generic selection" in text.lower() or "C11-GENERIC" in text:
            raise SystemExit(f"{entry['id']} names the generic selection; decide it before appending")
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
            "note": "W44-C11-GENERIC-SELECTION, 2026-09-25. Arbiter on main 70ce6b44: BEFORE reproduces the "
            "W44-LIBC-RESIDUALS integration diagnostics, chunk-contracts and source byte for byte (contracts "
            "after the checkout-root alias). AFTER lowers the 85 selection-invariant glibc __glibc_const_generic "
            "selections (36 functions, 25 units) to their calls: 37 leaves removed and 34 added, every one "
            "attributed; the 14 `call to uncontracted function _Generic` leaves go (6 exchanged for "
            "`bsearch`), 22 condition refusals go, 15 class-(a) test-failure-path leaks of "
            "ut_sasl_oauthbearer_oidc_assertion_create (already provisional Unsafe) and 13 unknown_alias are "
            "added. 1 function Unknown -> Safe (mk_string_char_search), relative to the declared memchr premise; "
            "0 unit status moves; 0 double_release, invalid_release or use_after_release candidates. Leak "
            "19 -> 34. Raw units remain unpromoted; provisional Unsafe is not an upstream defect claim. Previous "
            "row preserved in docs/reports/w44-c11-generic-selection/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w44-c11-generic-selection/artifacts/",
            "source": README + "; preceding baseline "
            "docs/reports/w44-libc-residuals/integration/artifacts/",
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
    print(json.dumps({k: row[k] for k in ("safe", "provisional_unsafe", "unknown", "data_only", "function_statuses", "leak_diagnostics", "unknown_alias_diagnostics", "unsupported_diagnostics", "unit_reason_codes")}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
