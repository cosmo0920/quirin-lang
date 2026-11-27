#!/usr/bin/env python3
"""CG26-VARIADICS census over one or two `resource-diagnostics.json` reports.

Usage:
  census.py REPORT OUT.json                 # one report: the variadic families
  census.py BEFORE AFTER OUT.json           # two reports: also every moving function

Families (by diagnostic description):
  va-builtin   call to uncontracted function `__builtin_va_{start,end,copy,arg}`
  tail         arguments beyond the declared parameters of `CALLEE` ...
  va-protocol  va_list protocol refused: ... / va_list operation refused: ...

For each family: leaves, functions, callees (tail), and the functions whose
every leaf is in the family (sole) or in va-builtin + tail together. Nothing is
piped; the JSON is the record.
"""

import collections
import json
import pathlib
import re
import sys

VA = re.compile(r"^call to uncontracted function `(__builtin_va_(start|end|copy|arg))`$")
TAIL = re.compile(r"^arguments beyond the declared parameters of `([^`]+)` are outside the resource projection$")


def family(description):
    if VA.match(description):
        return "va-builtin"
    if TAIL.match(description):
        return "tail"
    if description.startswith("va_list protocol refused:") or description.startswith(
        "va_list operation refused:"
    ):
        return "va-protocol"
    return None


def load(path):
    return json.loads(pathlib.Path(path).read_text())


def functions(report):
    for unit in report["units"]:
        for entry in unit["functions"]:
            key = (unit.get("library", ""), unit["file"], entry["function"])
            descriptions = [d.get("description", "") for d in entry.get("diagnostics") or []]
            yield key, entry["status"], descriptions


def census(report):
    leaves = collections.Counter()
    funcs = collections.defaultdict(set)
    callees = collections.Counter()
    callee_functions = collections.defaultdict(set)
    builtins = collections.Counter()
    protocol = collections.Counter()
    sole = collections.defaultdict(list)
    va_or_tail_only = []
    statuses = collections.Counter()
    for key, status, descriptions in functions(report):
        statuses[status] += 1
        fams = [family(d) for d in descriptions]
        for d, f in zip(descriptions, fams):
            if f is None:
                continue
            leaves[f] += 1
            funcs[f].add(key)
            if f == "tail":
                callee = TAIL.match(d).group(1)
                callees[callee] += 1
                callee_functions[callee].add(key)
            elif f == "va-builtin":
                builtins[VA.match(d).group(1)] += 1
            else:
                protocol[d] += 1
        if descriptions and status != "Safe":
            present = set(fams)
            if len(present) == 1 and None not in present:
                sole[next(iter(present))].append("/".join(key))
            if present and present <= {"va-builtin", "tail"} and None not in present:
                va_or_tail_only.append("/".join(key))
    return {
        "statuses": dict(statuses),
        "leaves": dict(leaves),
        "functions": {f: len(v) for f, v in funcs.items()},
        "va_builtin_leaves": dict(builtins.most_common()),
        "tail_callees": {
            c: {"leaves": n, "functions": len(callee_functions[c])}
            for c, n in callees.most_common()
        },
        "protocol_refusals": dict(protocol.most_common()),
        "sole": {f: sorted(v) for f, v in sole.items()},
        "sole_counts": {f: len(v) for f, v in sole.items()},
        "va_builtin_or_tail_only": sorted(va_or_tail_only),
    }


def moves(before, after):
    old = {key: (status, descriptions) for key, status, descriptions in functions(before)}
    rows = []
    for key, status, descriptions in functions(after):
        if key not in old:
            rows.append({"function": "/".join(key), "before": None, "after": status})
            continue
        previous, previous_descriptions = old[key]
        if previous != status or sorted(previous_descriptions) != sorted(descriptions):
            gone = collections.Counter(previous_descriptions) - collections.Counter(descriptions)
            new = collections.Counter(descriptions) - collections.Counter(previous_descriptions)
            rows.append(
                {
                    "function": "/".join(key),
                    "before": previous,
                    "after": status,
                    "removed": dict(gone),
                    "added": dict(new),
                }
            )
    transitions = collections.Counter((r["before"], r["after"]) for r in rows)
    return {
        "transitions": {f"{a} -> {b}": n for (a, b), n in transitions.most_common()},
        "rows": rows,
    }


def main():
    if len(sys.argv) == 3:
        out = {"report": census(load(sys.argv[1]))}
        target = sys.argv[2]
    elif len(sys.argv) == 4:
        before, after = load(sys.argv[1]), load(sys.argv[2])
        out = {"before": census(before), "after": census(after), "moves": moves(before, after)}
        target = sys.argv[3]
    else:
        print(__doc__)
        return 2
    pathlib.Path(target).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    summary = {k: v for k, v in out.items() if k != "moves"}
    for name, value in summary.items():
        print(name, json.dumps({k: value[k] for k in ("statuses", "leaves", "functions", "sole_counts")}))
    if "moves" in out:
        print("moves", json.dumps(out["moves"]["transitions"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
