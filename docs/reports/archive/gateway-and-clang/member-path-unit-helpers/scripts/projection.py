#!/usr/bin/env python3
"""projection.py BEFORE AFTER -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS:
whether the two corpus reports (private/mpuh/NAME/resource-diagnostics.json)
are equal once each row's `contract_dependencies` and each unit's
`contract_body_evidence` are removed, and which units' body evidence differs
(the standing labels this card changes). Prints a JSON summary."""
import copy
import hashlib
import json
import pathlib
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/mpuh"


def load(name):
    return json.loads((PRIVATE / name / "resource-diagnostics.json").read_text())


def strip(report):
    report = copy.deepcopy(report)
    for unit in report["units"]:
        unit.pop("contract_body_evidence", None)
        for function in unit["functions"]:
            function.pop("contract_dependencies", None)
    return report


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True).encode()).hexdigest()


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    evidence = []
    rows = []
    for a, b in zip(before["units"], after["units"]):
        if a.get("contract_body_evidence") != b.get("contract_body_evidence"):
            evidence.append(a["file"])
        for fa, fb in zip(a["functions"], b["functions"]):
            if fa.get("contract_dependencies") != fb.get("contract_dependencies"):
                rows.append(f'{a["file"]}:{fa["function"]}')
    out = {
        "projection_equal": digest(strip(before)) == digest(strip(after)),
        "projection_sha256": [digest(strip(before)), digest(strip(after))],
        "units_whose_body_evidence_differs": evidence,
        "rows_whose_dependencies_differ": rows,
    }
    print(json.dumps(out, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
