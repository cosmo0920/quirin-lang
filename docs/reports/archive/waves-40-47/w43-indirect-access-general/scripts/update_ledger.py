#!/usr/bin/env python3
"""Apply W43-INDIRECT-ACCESS-GENERAL's hand-maintained matrix edits, once.

Usage: update_ledger.py

* retires `INDIRECT-CENSUS-FEATURE-UNBUILT`, `BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT`
  and `INDIRECT-EXCHANGE-PROXY-UNVALIDATED`, each of which must be registered,
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
REPORT = ROOT / "docs/reports/w43-indirect-access-general"
GROUNDS = {
    "INDIRECT-CENSUS-FEATURE-UNBUILT": (
        "Its discharge condition is \"a CI job building `-p qcore-c-gateway --features indirect-census`\". "
        "Card W43-INDIRECT-ACCESS-GENERAL added the step `Lint and test the gateway with the "
        "indirect-access census compiled in` to .github/workflows/ci.yml, running `cargo clippy -p "
        "qcore-c-gateway --features indirect-census --all-targets -- -D warnings` and `cargo test -p "
        "qcore-c-gateway --features indirect-census --lib`, and ran both as the `census` step of its own "
        "gate (exit 0; 428 library tests). The card also used and extended the census (statement stream, "
        "declared labels) for its shape census. docs/reports/w43-indirect-access-general/README.md."
    ),
    "BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT": (
        "Its discharge condition is \"a CI job building `cargo clippy -p qcore-c-gateway --features "
        "indirect-census --all-targets -- -D warnings`\", exactly the command card "
        "W43-INDIRECT-ACCESS-GENERAL added to .github/workflows/ci.yml (with the feature's library tests) "
        "and ran green in its gate. docs/reports/w43-indirect-access-general/README.md."
    ),
    "INDIRECT-EXCHANGE-PROXY-UNVALIDATED": (
        "Its discharge condition: \"the first arbiter to admit any shape publishes its measured "
        "three-way split beside this card's prediction for the same shape, and the difference is "
        "recorded\". Card W43-INDIRECT-ACCESS-GENERAL is that arbiter: proxy-validation.txt publishes, "
        "for the 17 census-2 shapes its typed rule touches, the proxy's prediction beside the measured "
        "AFTER outcome. Of 11 functions the proxy calls CLEAR, 4 measured cleared and 7 kept a leaf only "
        "because another statement of the same shape lies outside the typed rule; no CLEAR prediction "
        "was wrong where the rule covered the shape. The 4 EXCH predictions kept a leaf for the same "
        "reason, so the exchange arm is re-registered, narrower, as "
        "INDIRECT-EXCHANGE-PROXY-EXCH-ARM-UNTESTED. The card's own forecast (PREDICTION.md) is "
        "compared leaf by leaf in the report."
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
            "discharged_by": "W43-INDIRECT-ACCESS-GENERAL",
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
            "note": "W43-INDIRECT-ACCESS-GENERAL, 2026-09-24. BEFORE (base binary) reproduces the "
            "W43-LIBC-RESULT-ALIAS diagnostics byte for byte, and so do the census-instrumented base runs. "
            "A typed scalar-access route (statements the classifier refused for their access, destination "
            "or increment, or named as an unsanctioned *p store) removes 1,112 leaves and adds 83, all "
            "attributed: 985 forecast rule leaves, 98 unforecast rule leaves, 1 unreached, 16 summary, 12 "
            "binding-precision; 78 exchanges, 5 summary-side additions, 0 violation candidates. 12 "
            "functions Unknown -> Safe (11 forecast), 1 unit Unknown -> Safe (cmt_time.c). Leak 272 -> "
            "271: snappy.c L1694 withdrawn because the release before it is now credited (a framed store "
            "replaced a blanket poison). Provisional Unsafe is not an upstream defect claim. Previous row "
            "preserved in docs/reports/w43-indirect-access-general/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-indirect-access-general/artifacts/",
            "source": "docs/reports/w43-indirect-access-general/README.md; preceding baseline "
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
