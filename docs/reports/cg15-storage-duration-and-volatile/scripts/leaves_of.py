#!/usr/bin/env python3
"""CG15: print each listed function's current unsupported and alias leaves,
with CG00's reason and the original source line (read from the pinned
checkout by the leaf's `origin`), for reading the prediction.

usage: leaves_of.py DIAGNOSTICS POPULATION_JSON [VARIANT]  (with_alias, strict or entry)
"""

import collections
import gzip
import json
import os
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
LEAVES = HERE.parents[1] / "cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"
ROOT = "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"


def main():
    diagnostics = json.load(open(sys.argv[1]))
    population = json.load(open(sys.argv[2]))
    variant = sys.argv[3] if len(sys.argv) > 3 else "with_alias"
    wanted = set(population["entry_functions"] if variant == "entry"
                 else population["freed_alone"][variant])
    index = {}
    for dirpath, _, filenames in os.walk(ROOT):
        rel = os.path.relpath(dirpath, ROOT)
        if rel.startswith("build") or "/.git" in dirpath:
            continue
        for name in filenames:
            index.setdefault(name, []).append(os.path.join(dirpath, name))
    with gzip.open(LEAVES, "rt") as handle:
        rows = [json.loads(line) for line in handle][1:]
    reasons = collections.defaultdict(list)
    for row in rows:
        reasons[(row["file"], row["function"], row["line"])].append(row["reason"])
    texts = {}

    def source(origin):
        name, line = origin.rsplit(":", 1)
        paths = index.get(name, [])
        if len(paths) != 1:
            return "?"
        if paths[0] not in texts:
            with open(paths[0], errors="replace") as handle:
                texts[paths[0]] = handle.read().split("\n")
        return texts[paths[0]][int(line) - 1].strip()

    for unit in diagnostics["units"]:
        for function in unit["functions"]:
            name = f"{unit['file']}::{function['function']}"
            if name not in wanted:
                continue
            print(f"== {name}")
            for leaf in function["diagnostics"]:
                reason = ",".join(reasons.get((unit["file"], function["function"], leaf["line"]), []))
                description = re.sub(r"`[^`]*`", "`X`", leaf.get("description", leaf["category"]))
                print(f"   {leaf['category'][:7]} {reason:28.28} {description[:22]:22} | "
                      f"{source(leaf['origin'])[:110]}")


if __name__ == "__main__":
    sys.exit(main())
