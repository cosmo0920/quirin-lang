#!/usr/bin/env python3
"""CG05-NESTED-CALL-EFFECTS: the nested-call refusal census of one corpus run.

Usage: census.py RUN [--parity PARITY.jsonl] [--json OUT]

Counts every `nested call `X` inside a modeled call is outside the resource
projection` leaf (card O04-B09B10's refusal, the only one the rule-10y consumer
is asked after), by inner callee, with the functions holding them and their
status; the functions whose only non-alias blockers are such leaves; and, with
a parity stream, the `position: "nested-call"` consultations by refusal reason.
"""

import argparse
import collections
import json
import pathlib

HEAD = "nested call `"
TAIL = "` inside a modeled call is outside the resource projection"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    path = args.run / "resource-diagnostics.json" if args.run.is_dir() else args.run
    data = json.loads(path.read_text())
    totals = collections.Counter()
    leaves = collections.Counter()
    functions = {}
    sole = []
    for unit in data["units"]:
        for function in unit["functions"]:
            totals[function["status"]] += 1
            nested = []
            others = []
            for diagnostic in function.get("diagnostics", []):
                description = diagnostic.get("description") or ""
                if description.startswith(HEAD) and description.endswith(TAIL):
                    nested.append(description[len(HEAD):-len(TAIL)])
                elif diagnostic.get("category") != "unknown_alias":
                    others.append(description or diagnostic.get("category"))
            if nested:
                key = f"{unit['file']}::{function['function']}"
                functions[key] = {"status": function["status"], "inner": nested}
                leaves.update(nested)
                if not others:
                    sole.append(key)
    record = {
        "totals": dict(totals),
        "nested_leaves": sum(leaves.values()),
        "nested_functions": len(functions),
        "nested_functions_by_status": dict(
            collections.Counter(entry["status"] for entry in functions.values())
        ),
        "sole_blocked_functions": len(sole),
        "by_inner_callee": dict(leaves.most_common()),
        "functions": functions,
        "sole_blocked": sorted(sole),
    }
    if args.parity and args.parity.exists():
        consultations = []
        with args.parity.open() as stream:
            for line in stream:
                row = json.loads(line)
                if row.get("position") == "nested-call":
                    consultations.append(row)
        unique = {
            (row["function"], row["line"], tuple(row.get("texts", []))): row
            for row in consultations
        }
        reasons = collections.Counter(
            "admitted" if row["new_admitted"] else row["new_refusal"].split(":")[0]
            for row in unique.values()
        )
        record["parity"] = {
            "consultations": len(unique),
            "admitted": sum(1 for row in unique.values() if row["new_admitted"]),
            "by_outcome": dict(reasons.most_common()),
        }
    text = json.dumps(record, indent=1, ensure_ascii=False)
    if args.json:
        args.json.write_text(text + "\n")
    summary = {key: value for key, value in record.items() if key not in ("functions", "sole_blocked", "by_inner_callee")}
    summary["top_inner_callees"] = dict(list(record["by_inner_callee"].items())[:25])
    print(json.dumps(summary, indent=1, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
