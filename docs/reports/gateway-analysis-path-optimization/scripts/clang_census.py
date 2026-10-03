#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION: summarise a clang-census.sh log.

Usage: clang_census.py LOG.jsonl [OUT.json] [--by-parent]

Reports launches, Clang wall seconds and output bytes per mode, and the
repeated launches: a launch is a repeat when an earlier launch had the same
argument vector digest and the same standard-input digest (so the same
output, Clang being deterministic on the same input and flags). The repeat
seconds are what an exact in-process memo of launches could save.
With --by-parent, the same per parent command (the test binary for a test
run), sorted by Clang seconds.
"""
import collections
import json
import re
import sys


def rows(path):
    for line in open(path, encoding="utf-8", errors="replace"):
        try:
            yield json.loads(line)
        except json.JSONDecodeError:
            # a parent command with control characters: keep the numbers
            fixed = re.sub(r'"parent":".*"\}$', '"parent":"?"}', line.strip())
            yield json.loads(fixed)


def summarise(launches):
    modes = collections.defaultdict(lambda: {"launches": 0, "seconds": 0.0, "out_bytes": 0,
                                             "repeats": 0, "repeat_seconds": 0.0})
    seen = set()
    for row in launches:
        mode = modes[row["mode"]]
        mode["launches"] += 1
        mode["seconds"] += row["seconds"]
        mode["out_bytes"] += row["out_bytes"]
        key = (row["args"], row["in"])
        if key in seen and row["in"]:
            mode["repeats"] += 1
            mode["repeat_seconds"] += row["seconds"]
        seen.add(key)
    total = {"launches": sum(m["launches"] for m in modes.values()),
             "seconds": round(sum(m["seconds"] for m in modes.values()), 1),
             "repeats": sum(m["repeats"] for m in modes.values()),
             "repeat_seconds": round(sum(m["repeat_seconds"] for m in modes.values()), 1),
             "distinct_inputs": len({row["in"] for row in launches if row["in"]})}
    for mode in modes.values():
        mode["seconds"] = round(mode["seconds"], 1)
        mode["repeat_seconds"] = round(mode["repeat_seconds"], 1)
    return {"total": total, "modes": dict(sorted(modes.items(), key=lambda kv: -kv[1]["seconds"]))}


def main():
    launches = list(rows(sys.argv[1]))
    result = summarise(launches)
    if "--by-parent" in sys.argv:
        parents = collections.defaultdict(list)
        for row in launches:
            parent = row["parent"].split(" ")[0].rsplit("/", 1)[-1]
            parent = re.sub(r"-[0-9a-f]{16}$", "", parent)
            parents[parent].append(row)
        result["parents"] = {name: summarise(group)["total"] for name, group in
                             sorted(parents.items(), key=lambda kv: -sum(r["seconds"] for r in kv[1]))}
    # the filtered-dump names asked more than once for one input
    names = collections.Counter((row["in"], row["filter"]) for row in launches if row["mode"] == "ast-filtered")
    result["filtered_name_repeats_top"] = [
        {"filter": f, "count": c} for (i, f), c in names.most_common(15) if c > 1]
    per_input = collections.Counter(row["in"] for row in launches if row["in"])
    result["launches_per_input"] = {
        "max": max(per_input.values(), default=0),
        "median": sorted(per_input.values())[len(per_input) // 2] if per_input else 0}
    text = json.dumps(result, indent=1)
    if len(sys.argv) > 2 and not sys.argv[2].startswith("--"):
        open(sys.argv[2], "w").write(text + "\n")
    print(text[:6000])


if __name__ == "__main__":
    main()
