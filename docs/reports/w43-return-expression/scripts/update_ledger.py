#!/usr/bin/env python3
"""Apply W43-RETURN-EXPRESSION's hand-maintained matrix edits, once.

Usage: update_ledger.py

* appends every entry of `followups.json` to `named_followups` (refusing a
  duplicate id);
* retires `RETURN-CALL-INCOMPLETE-LINE-455`, which must be registered, and
  records the retirement in the report directory;
* rewrites the hand-maintained `corpus.cycle19_raw_units` row from this card's
  primary artifacts, first preserving the exact previous row beside the report.

Derived sections (including `dimensions.named_followups`) are left to the
generator, which the caller runs afterwards.
"""

import gzip
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w43-return-expression"
RETIRED = "RETURN-CALL-INCOMPLETE-LINE-455"


def main() -> int:
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    new = json.loads((REPORT / "followups.json").read_text())
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    if RETIRED not in ids:
        raise SystemExit(f"{RETIRED} is not registered; nothing to retire")
    retired = next(entry for entry in entries if entry["id"] == RETIRED)
    (REPORT / "retired-followups.json").write_text(
        json.dumps(
            [
                {
                    "entry": retired,
                    "discharged_by": "W43-RETURN-EXPRESSION",
                    "ground": "The W43 census classifies every refused return leaf from the "
                    "analyzer's own parser-delimited token view (the complete return "
                    "expression, however many preprocessed lines it spans), not from one "
                    "origin line: 1,444 of 1,444 published leaves are covered and 0 are "
                    "shape-unknown. This is the multi-line-aware classifier the discharge "
                    "condition names; census/census.txt and census/census.json.gz.",
                }
            ],
            indent=2,
        )
        + "\n"
    )
    matrix["named_followups"] = [entry for entry in entries if entry["id"] != RETIRED] + new

    corpus = matrix["corpus"]
    (REPORT / "previous-corpus-row.json").write_text(
        json.dumps(corpus["cycle19_raw_units"], indent=2) + "\n"
    )
    diagnostics = json.loads(gzip.decompress((REPORT / "artifacts/diagnostics.json.gz").read_bytes()))
    functions = {"Safe": 0, "Unsafe": 0, "Unknown": 0}
    units = {"Safe": 0, "Unsafe": 0, "Unknown": 0, "data_only": 0}
    for unit in diagnostics["units"]:
        units[unit["status"]] += 1
        for entry in unit["functions"]:
            functions[entry["status"]] += 1
    counts = diagnostics["diagnostic_counts"]
    row = dict(corpus["cycle19_raw_units"])
    row.update(
        {
            "note": "W43-RETURN-EXPRESSION, 2026-09-23. Same-checkout BEFORE reproduces the W42 "
            "diagnostics byte for byte (the four other artifacts match the canonical-contract-"
            "locations checkpoint after the worktree-path alias; their W42 differences are that "
            "migration's attributed path and digest identities). AFTER lowers 941 whole-call "
            "return refusals to the call classifier's own refusal at the same line (671 "
            "uncontracted call, 267 lexical indirect access, 2 braced initializer, 1 nested "
            "call): 941 removed, 941 added, 0 function or unit moves, every count unchanged. "
            "Leak 272 preserved. Raw units remain unpromoted; provisional Unsafe is not an "
            "upstream defect claim. Previous row preserved in "
            "docs/reports/w43-return-expression/previous-corpus-row.json.",
            "units": sum(units.values()),
            "safe": units["Safe"],
            "provisional_unsafe": units["Unsafe"],
            "unknown": units["Unknown"],
            "data_only": units["data_only"],
            "projected_functions": sum(functions.values()),
            "leak_diagnostics": counts["leak"],
            "baseline": "docs/reports/w43-return-expression/artifacts/",
            "source": "docs/reports/w43-return-expression/README.md; preceding baseline "
            "docs/reports/w42-priority-integration/artifacts/",
            "unknown_alias_diagnostics": counts["unknown_alias"],
            "unsupported_diagnostics": counts["unsupported"],
            "double_release_diagnostics": counts.get("double_release", 0),
            "function_statuses": {
                "safe": functions["Safe"],
                "provisional_unsafe": functions["Unsafe"],
                "unknown": functions["Unknown"],
            },
        }
    )
    corpus["cycle19_raw_units"] = row
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups: {len(entries)} -> {len(matrix['named_followups'])}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
