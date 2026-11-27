#!/usr/bin/env python3
"""Headline comparison of two arbiter runs: counts, statuses, movers, leaf diff.

Usage: compare.py BEFORE.json AFTER.json [OUT.json]
"""
import collections
import json
import re
import sys

CALL = re.compile(r"call to uncontracted function `([^`]+)`")


def load(path):
    return json.load(open(path))


def index(data):
    functions = {}
    units = {}
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        units[key] = unit["status"]
        for entry in unit["functions"]:
            functions[(key, entry["function"])] = entry
    return units, functions


def headline(data):
    units, functions = index(data)
    statuses = collections.Counter(entry["status"] for entry in functions.values())
    unit_statuses = collections.Counter(units.values())
    data_only = sum(1 for unit in data["units"] if not unit["functions"])
    unknown = [entry for entry in functions.values() if entry["status"] == "Unknown"]
    call_leaves = sum(1 for entry in unknown for d in entry["diagnostics"] if CALL.fullmatch(d["description"]))
    with_calls = sum(1 for entry in unknown if any(CALL.fullmatch(d["description"]) for d in entry["diagnostics"]))
    only_calls = [entry for entry in unknown if entry["diagnostics"]
                  and all(CALL.fullmatch(d["description"]) for d in entry["diagnostics"])]
    one_callee = sum(1 for entry in only_calls
                     if len({CALL.fullmatch(d["description"]).group(1) for d in entry["diagnostics"]}) == 1)
    return {
        "functions": dict(statuses),
        "units": dict(unit_statuses),
        "data_only_units": data_only,
        "diagnostic_counts": data["diagnostic_counts"],
        "uncontracted_call_leaves": call_leaves,
        "unknown_with_call_leaves": with_calls,
        "only_calls": len(only_calls),
        "only_one_callee": one_callee,
    }


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    ub, fb = index(before)
    ua, fa = index(after)
    movers = []
    for key in sorted(set(fb) | set(fa)):
        sb = fb.get(key, {}).get("status")
        sa = fa.get(key, {}).get("status")
        if sb != sa:
            movers.append({"unit": key[0], "function": key[1], "before": sb, "after": sa})
    unit_moves = [{"unit": k, "before": ub.get(k), "after": ua.get(k)} for k in sorted(set(ub) | set(ua)) if ub.get(k) != ua.get(k)]
    removed, added = [], []
    for key in sorted(set(fb) | set(fa)):
        lb = collections.Counter((d["line"], d["category"], d["description"], d.get("origin")) for d in fb.get(key, {}).get("diagnostics", []))
        la = collections.Counter((d["line"], d["category"], d["description"], d.get("origin")) for d in fa.get(key, {}).get("diagnostics", []))
        for leaf, n in (lb - la).items():
            removed += [{"unit": key[0], "function": key[1], "line": leaf[0], "category": leaf[1], "description": leaf[2], "origin": leaf[3]}] * n
        for leaf, n in (la - lb).items():
            added += [{"unit": key[0], "function": key[1], "line": leaf[0], "category": leaf[1], "description": leaf[2], "origin": leaf[3]}] * n
    result = {
        "before": headline(before),
        "after": headline(after),
        "function_moves": movers,
        "unit_moves": unit_moves,
        "removed": removed,
        "added": added,
    }
    if len(sys.argv) > 3:
        json.dump(result, open(sys.argv[3], "w"), indent=1)
    print(json.dumps({k: result[k] for k in ("before", "after")}, indent=1))
    print("function moves", collections.Counter((m["before"], m["after"]) for m in movers))
    print("unit moves", unit_moves)
    norm = lambda s: re.sub(r"`[^`]*`", "X", s)
    print("removed", collections.Counter(norm(r["description"]) for r in removed).most_common(12))
    print("added", collections.Counter(norm(r["description"]) for r in added).most_common(12))


if __name__ == "__main__":
    main()
