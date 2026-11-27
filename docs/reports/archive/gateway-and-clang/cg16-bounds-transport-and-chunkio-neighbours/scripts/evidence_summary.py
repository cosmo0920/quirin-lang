#!/usr/bin/env python3
"""CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: what moved between two
bounds-selected corpus runs beyond bounds_compare.py's fields (the rows'
`contract_dependencies`), the calls to the declared chunkio accessors, and the
AFTER run's body evidence.

Usage: evidence_summary.py BEFORE AFTER [OUT.json]"""
import collections
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
OUT = REPO / "private/cg16-bounds-transport-and-chunkio-neighbours"


def rows(name):
    return [json.loads(line) for line in (OUT / f"{name}-bounds.jsonl").read_text().splitlines()]


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    assert len(before) == len(after)
    differing = 0
    other = 0
    dependencies = collections.Counter()
    for old, new in zip(before, after):
        if old == new:
            continue
        differing += 1
        bounds = dict(new["bounds"] or {})
        found = bounds.pop("contract_dependencies", None)
        if found:
            dependencies[", ".join(found)] += 1
        stripped = dict(new)
        stripped["bounds"] = bounds
        if stripped != old:
            other += 1
    names = {}
    record = json.loads((OUT / f"{sys.argv[2]}-bounds.contracts.json").read_text())
    for name in record["declarations"]:
        names[name] = collections.Counter()
    calls = []
    for row in after:
        for access in (row["bounds"] or {}).get("accesses", []):
            if access["kind"] != "call-precondition":
                continue
            for name in names:
                if name in access["subject"]:
                    names[name][access["actual"].split(":")[0]] += 1
                    calls.append({"file": row["file"], "function": row["function"],
                                  "callee": name, "line": access["line"],
                                  "actual": access["actual"]})
    evidence = record["body_evidence"]
    summary = {
        "rows": len(after),
        "rows_differing": differing,
        "rows_differing_other_than_contract_dependencies": other,
        "rows_with_contract_dependencies": dict(dependencies),
        "calls_to_declared_accessors": {name: dict(c) for name, c in names.items()},
        "calls_total": len(calls),
        "calls": calls,
        "body_evidence_counts": evidence["counts"],
        "safe_rollup": evidence["safe_rollup"],
        "declarations": {
            name: {
                "body_standing": d["body_standing"],
                "clauses": d["clauses"],
                "units_examined": d["units_examined"],
                "units_validated": d["units_validated"],
                "units_failed": d["units_failed"],
                "files": sorted({u["file"] for u in d["units"]}),
                "body_verdicts": sorted({u["body_verdict"] for u in d["units"]}),
            }
            for name, d in evidence["declarations"].items()
        },
        "refutation_table": record.get("refutation_table"),
        "pins": {"in_force": record["pins"]["in_force"], "held": record["pins"]["held"]},
    }
    text = json.dumps(summary, indent=1, sort_keys=True) + "\n"
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(text)
    print(text)


main()
