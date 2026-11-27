#!/usr/bin/env python3
"""Count the differing leaves of a run's JSON artifact against an archived
(gzip) one by path class (array indices elided), after rewriting ROOT to
CANONICAL. A disclosure beside baseline_identity.py, never a substitute.

Usage: leaf_classes.py RUN.json ARCHIVED.json.gz ROOT=CANONICAL
"""

import collections
import gzip
import json
import re
import sys


def main() -> int:
    run = json.load(open(sys.argv[1]))
    base = json.loads(gzip.decompress(open(sys.argv[2], "rb").read()))
    root, canonical = sys.argv[3].split("=", 1)
    run = json.loads(json.dumps(run).replace(root, canonical))
    out = []

    def walk(a, b, path=""):
        if type(a) is not type(b):
            out.append((path, "type"))
        elif isinstance(a, dict):
            for key in sorted(set(a) | set(b)):
                if key not in a or key not in b:
                    out.append((path + "/" + key, "missing"))
                else:
                    walk(a[key], b[key], path + "/" + key)
        elif isinstance(a, list):
            if len(a) != len(b):
                out.append((path, "len"))
            for index, (x, y) in enumerate(zip(a, b)):
                walk(x, y, f"{path}[{index}]")
        elif a != b:
            out.append((path, "value"))

    walk(run, base)
    classes = collections.Counter((re.sub(r"\[\d+\]", "[]", path), kind) for path, kind in out)
    print(json.dumps({"differing_leaves": len(out),
                      "classes": [[count, path, kind] for (path, kind), count in classes.most_common()]},
                     indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
