#!/usr/bin/env python3
"""sole_detail.py PREMISES.jsonl.gz PPDIR [PREMISE...] -- AG1: every
sole-blocker function (its only unsupported family the access refusal) with
each access leaf's premise, its path summaries and the analysed statement,
optionally narrowed to functions whose leaves all carry one of PREMISE."""
import collections
import gzip
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from leaves import statement  # noqa: E402

IND = "indirect memory access is outside the resource projection"


def main() -> int:
    rows = [json.loads(line) for line in gzip.open(sys.argv[1], "rt")]
    ppdir = sys.argv[2]
    wanted = set(sys.argv[3:])
    by = collections.defaultdict(list)
    for row in rows:
        if set(row["families"]) <= {IND} and row["status"] == "Unknown":
            by[(row["file"], row["function"])].append(row)
    cache = {}
    for (file, function), leaves in sorted(by.items()):
        premises = {leaf["premise"] for leaf in leaves}
        if wanted and not premises <= wanted:
            continue
        path = os.path.join(ppdir, f"unit-{leaves[0]['unit']}.c")
        if path not in cache and os.path.exists(path):
            cache[path] = open(path, errors="replace").read().split("\n")
        lines = cache.get(path, [])
        print(f"== {file} {function} {sorted(premises)}")
        for leaf in leaves:
            text = statement(lines, leaf["line"] - 1)[0] if lines else leaf["origin_text"]
            paths = "; ".join(
                f"{p['access']}/{p['value']}/{p['root_kind']}/{p.get('rhs', '')}/{p.get('callee', '')}"
                for p in leaf["paths"]
            )
            print(f"   [{leaf['premise']}] {text[:150]}")
            print(f"      {paths[:200]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
