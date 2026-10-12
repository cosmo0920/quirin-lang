#!/usr/bin/env python3
"""attribute.py PARITY-DIFF.json [BEFORE-REPORT AFTER-REPORT] --
GAPO-RETURN-CORRECTNESS: attribute every parity row parity.py found on one
side only, by class: rule 15y's `no-return-node` rows (callee, decision,
functions), rule 15z's `uncast-return-actuals-*` rows, the
`return-route-gap` census, and the rows of other positions a changed
lowering removed or added (rule 12i declined before its row, and what that
consultation asked downstream). With two reports, print the verdicts of the
functions those rows name (they must be equal on both sides when the run
moved nothing)."""
import collections
import json
import pathlib
import sys

diff = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
only_before, only_after = diff["only_before"], diff["only_after"]


def show(label, rows, fields):
    print(f"{label}: {len(rows)}")
    for key, count in collections.Counter(
            tuple(json.dumps(row.get(field), sort_keys=True) for field in fields)
            for row in rows).most_common():
        print(f"  {count:5d} " + "  ".join(f"{f}={k}" for f, k in zip(fields, key)))


ends = [r for r in only_after if r.get("position") == "no-return-node"]
show("15y no-return-node", ends, ["callees", "decision", "whole_discarded_call", "resolution"])
uncast = [r for r in only_after if str(r.get("answer", "")).startswith("uncast-return-actuals")]
show("15z uncast-return-actuals", uncast, ["answer", "removed"])
gaps = [r for r in only_after if r.get("position") == "return-route-gap"]
show("15z return-route-gap", gaps, ["gap", "function", "line"])
access_before = [r for r in only_before if r.get("position") == "return-call-access"]
show("12i rows no longer recorded", access_before, ["form", "admitted"])
for side, rows in (("before", only_before), ("after", only_after)):
    other = [r for r in rows if r.get("position") not in
             ("no-return-node", "return-route-gap", "return-call-access")
             and not str(r.get("answer", "")).startswith("uncast-return-actuals")]
    show(f"other rows only {side}", other, ["position", "function", "line"])
functions = sorted({r["function"] for r in ends + uncast + gaps + access_before})
print(f"functions named: {len(functions)}")
if len(sys.argv) > 3:
    def verdicts(path):
        path = pathlib.Path(path)
        if path.is_dir():
            path = path / "resource-diagnostics.json"
        report = json.loads(path.read_text(encoding="utf-8"))
        return {f["function"]: f["status"] for u in report["units"] for f in u["functions"]}
    before, after = verdicts(sys.argv[2]), verdicts(sys.argv[3])
    status = collections.Counter((before.get(f), after.get(f)) for f in functions)
    print("verdicts of the named functions (before, after):", dict(status))
    by_class = collections.Counter()
    for label, rows in (("15y", ends), ("15z-uncast", uncast), ("12i", access_before)):
        for f in sorted({r["function"] for r in rows}):
            by_class[(label, before.get(f))] += 1
    print("functions by class and verdict:", dict(by_class))
