#!/usr/bin/env python3
"""Card W44-COLLECTION-ALLOCATION: what the admission did for collections.

Usage: collection_reach.py SIZEOF_CENSUS_JSON DELTA_JSON BEFORE_RUN AFTER_RUN CENSUS_RUN OUT_JSON OUT_TXT

For every self-referential dereference allocation into a destination declared
`T **` (the census's collection-array candidates), it reports whether this
card withdrew the allocation's refusal (from the delta attribution), what
replaced it on the same line, the function's status in both runs, and the
element operations the classifier lowered in that function (the census run's
`statement-census.jsonl`), each with whether its fallback leaf is still
present in AFTER. An element operation whose fallback leaf is gone decided
under the collection rules; one whose fallback is present acted as its
fallback (its array did not resolve to one live, non-null identity).
"""

import collections
import json
import pathlib
import re
import sys


def statuses(run):
    document = json.loads((run / "resource-diagnostics.json").read_text())
    table, leaves = {}, collections.Counter()
    for unit in document["units"]:
        key = f"{unit['library']}/{unit['file']}"
        for function in unit.get("functions", []):
            table[(key, function["function"])] = function["status"]
            for leaf in function.get("diagnostics", []):
                leaves[(function["function"], leaf["line"], leaf["description"])] += 1
    return table, leaves


def main() -> int:
    census = json.loads(pathlib.Path(sys.argv[1]).read_text())
    delta = json.loads(pathlib.Path(sys.argv[2]).read_text())
    before_status, _ = statuses(pathlib.Path(sys.argv[3]))
    after_status, after_leaves = statuses(pathlib.Path(sys.argv[4]))
    census_run = pathlib.Path(sys.argv[5])
    out_json, out_txt = pathlib.Path(sys.argv[6]), pathlib.Path(sys.argv[7])
    withdrawn = {(r["unit"], r["function"], r["line"]) for r in delta["leaves"] if r["class"] == "withdrawn-operand-leaf"}
    substituted = {(r["unit"], r["function"], r["line"]): r["description"] for r in delta["leaves"] if r["class"] == "same-line-substitution"}
    operations = collections.defaultdict(set)
    with open(census_run / "statement-census.jsonl") as handle:
        for line in handle:
            record = json.loads(line)
            if record["outcome"].startswith("element "):
                operations[record["function"]].add((record["line"], record["outcome"]))
    rows = []
    for allocation in census["allocations"]:
        if not (allocation["self_referential"] and allocation["shape"] in ("(*NAME)", "(**NAME)")
                and 2 in allocation["destination_stars"]):
            continue
        unit = allocation["unit"][: -len(".i")].replace("__", "/", 1)
        function = allocation["function"]
        lines = [allocation["line"] - offset for offset in range(3)]
        elements = []
        for line, outcome in sorted(operations.get(function, ())):
            fallback = re.sub(r"^.*; fallback: ", "", outcome)
            elements.append({"line": line, "outcome": outcome,
                             "fallback_leaf_after": after_leaves[(function, line, fallback)] > 0})
        rows.append({
            "unit": unit, "function": function, "line": allocation["line"],
            "destination": allocation["destination"], "callee": allocation["callee"],
            "withdrawn": any((unit, function, line) in withdrawn for line in lines),
            "replaced_by": [substituted[(unit, function, line)] for line in lines if (unit, function, line) in substituted],
            "status_before": before_status.get((unit, function)), "status_after": after_status.get((unit, function)),
            "element_operations": elements,
        })
    summary = {
        "pointer_to_pointer_allocations": len(rows),
        "allocation_refusal_withdrawn": sum(r["withdrawn"] for r in rows),
        "functions_with_lowered_element_operations": sum(bool(r["element_operations"]) for r in rows),
        "element_operations_decided_under_collection_rules": sum(
            1 for r in rows for e in r["element_operations"] if not e["fallback_leaf_after"]),
        "status_moves": dict(collections.Counter(f"{r['status_before']}->{r['status_after']}" for r in rows)),
    }
    out_json.write_text(json.dumps({"summary": summary, "allocations": rows}, indent=1) + "\n")
    text = ["W44-COLLECTION-ALLOCATION: collection-array candidates after the admission", ""]
    text += [f"{k}: {v}" for k, v in summary.items()]
    text.append("")
    for r in rows:
        text.append(f"  {r['status_before']}->{r['status_after']} {r['unit']}::{r['function']} L{r['line']} "
                    f"`{r['destination']}` = {r['callee']}(...): {'withdrawn' if r['withdrawn'] else 'kept'}"
                    + (f"; replaced by {r['replaced_by']}" if r["replaced_by"] else ""))
        for e in r["element_operations"]:
            text.append(f"      L{e['line']} {e['outcome']} (fallback leaf after: {e['fallback_leaf_after']})")
    out_txt.write_text("\n".join(text) + "\n")
    print("\n".join(text))
    return 0


if __name__ == "__main__":
    sys.exit(main())
