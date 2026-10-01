#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVE-RESIDUALS (sibling-effect part): the frozen prediction's
function list, from the census run (the composition computed, nothing admitted)
and the BEFORE diagnostics.

usage: predict.py CENSUS-PARITY.jsonl BEFORE-RUN [--json OUT]

A function is a candidate when one of its rule-10z consultations is class (a)
(the composition commutes on both arms). It can move to Safe only if every one
of its leaves stands on a line of a class-(a) consultation (upper bound A), or
on a class-(a) line or a line whose consultation refused only an edge (upper
bound B: rule 11m also lets rule 10y's consumers read an edge's own node).
Leaves are matched by (function name, line); the parity rows carry no unit, so
a name defined in several units is matched in each (named in the output).
"""

import argparse
import collections
import json
import pathlib

MARK = "; composed with CG05's footprints: "
EDGE = "an arm's expression an older route refuses"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("before", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    rows = {}
    with args.parity.open() as stream:
        for line in stream:
            row = json.loads(line)
            position = str(row.get("position", ""))
            if position.startswith("value-connective"):
                rows[(row.get("function"), row.get("line"), position,
                      tuple(row.get("texts", [])))] = row
    a_lines = collections.defaultdict(set)
    edge_lines = collections.defaultdict(set)
    for (function, line, _, _), row in rows.items():
        reason = str(row.get("new_refusal") or "")
        if MARK in reason and reason.split(MARK, 1)[1].startswith("commutes on both arms"):
            a_lines[function].add(line)
        elif reason == EDGE:
            edge_lines[function].add(line)
    data = json.loads((args.before / "resource-diagnostics.json").read_text())
    units_of = collections.defaultdict(list)
    candidates = []
    for unit in data["units"]:
        for function in unit["functions"]:
            name = function["function"]
            units_of[name].append(unit["file"])
            if name not in a_lines:
                continue
            leaves = [(d.get("line"), d.get("category"), d.get("description"))
                      for d in function.get("diagnostics", [])]
            outside_a = [leaf for leaf in leaves if leaf[0] not in a_lines[name]]
            outside_b = [leaf for leaf in outside_a if leaf[0] not in edge_lines[name]]
            candidates.append({
                "unit": unit["file"],
                "function": name,
                "status": function["status"],
                "class_a_consultations": len(a_lines[name]),
                "leaves": len(leaves),
                "leaves_off_class_a_lines": len(outside_a),
                "leaves_off_class_a_or_edge_lines": len(outside_b),
                "upper_bound_a": function["status"] == "Unknown" and not outside_a,
                "upper_bound_b": function["status"] == "Unknown" and not outside_b,
            })
    record = {
        "functions_with_a_class_a_consultation": len(a_lines),
        "class_a_consultations": sum(len(v) for v in a_lines.values()),
        "status_of_those_functions": dict(collections.Counter(c["status"] for c in candidates)),
        "upper_bound_a": sorted(f"{c['unit']}::{c['function']}" for c in candidates if c["upper_bound_a"]),
        "upper_bound_b": sorted(f"{c['unit']}::{c['function']}" for c in candidates if c["upper_bound_b"]),
        "names_in_several_units": sorted(n for n in a_lines if len(units_of[n]) > 1),
        "candidates": candidates,
    }
    print(json.dumps({k: v for k, v in record.items() if k != "candidates"}, indent=1))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")


if __name__ == "__main__":
    main()
