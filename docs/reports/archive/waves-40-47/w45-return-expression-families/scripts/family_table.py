#!/usr/bin/env python3
"""The re-census by the brief's families, BEFORE and AFTER, with the result
carrier.

Usage: family_table.py BEFORE_SIZING AFTER_SIZING

Both inputs are `predict.py --json` outputs (BEFORE: census/prediction.json.gz,
AFTER: census/residual-sizing.json.gz), whose leaves carry W43's family and
detail from the analyzer's token view and the enclosing function's result
carrier read from the pinned source's definition line (`scalar` or
`pointer`). Only leaves on Unknown functions count; `sole` is the blocker
census's (the refusal is the function's only distinct reason).
"""

import collections
import gzip
import json
import pathlib
import sys


def load(path):
    data = pathlib.Path(path).read_bytes()
    if path.endswith(".gz"):
        data = gzip.decompress(data)
    return json.loads(data)


def brief_family(row):
    family, detail = row["family"], row["detail"]
    if family.startswith(("A.", "B.")):
        return "1 whole call under a cast or in parentheses"
    if family.startswith("C."):
        if "conditional" in detail or "short-circuit" in detail:
            return "4 conditional or short-circuit over a call"
        return "2 call inside a larger expression"
    if family.startswith("E."):
        return "5 conditional or short-circuit, no call"
    if family.startswith("F."):
        return "3 member, subscript or dereference read"
    if family.startswith("D."):
        return "6 type operator"
    if "comma" in detail:
        return "7 comma operator"
    return "8 statement expression or compound literal"


def table(rows):
    out = {}
    for name in sorted({brief_family(r) for r in rows}):
        members = [r for r in rows if brief_family(r) == name]
        functions = {(r["library"], r["file"], r["function"]) for r in members}
        sole = {(r["library"], r["file"], r["function"]) for r in members if r["sole"]}
        carriers = collections.Counter(r["carrier"] for r in members)
        out[name] = {
            "leaves": len(members),
            "functions": len(functions),
            "sole_functions": len(sole),
            "scalar_result_leaves": carriers.get("scalar", 0),
            "pointer_result_leaves": carriers.get("pointer", 0),
        }
    return out


def main():
    before = [r for r in load(sys.argv[1])["leaves"] if r["status"] == "Unknown"]
    after = [r for r in load(sys.argv[2])["leaves"] if r["status"] == "Unknown"]
    tb, ta = table(before), table(after)
    print(f"{'family':48} {'BEFORE leaves/fns/sole (scalar|pointer)':>40}   {'AFTER leaves/fns/sole':>22}")
    for name in sorted(set(tb) | set(ta)):
        b = tb.get(name, {"leaves": 0, "functions": 0, "sole_functions": 0,
                          "scalar_result_leaves": 0, "pointer_result_leaves": 0})
        a = ta.get(name, {"leaves": 0, "functions": 0, "sole_functions": 0})
        print(f"{name[2:]:48} {b['leaves']:6}/{b['functions']:4}/{b['sole_functions']:4} "
              f"({b['scalar_result_leaves']:3}|{b['pointer_result_leaves']:3})"
              f"{a['leaves']:14}/{a['functions']:4}/{a['sole_functions']:4}")
    total = lambda rows: (len(rows), len({(r['library'], r['file'], r['function']) for r in rows}),
                          len({(r['library'], r['file'], r['function']) for r in rows if r['sole']}))
    print(f"{'total':48} {'%d/%d/%d' % total(before):>22}{'%d/%d/%d' % total(after):>30}")


if __name__ == "__main__":
    main()
