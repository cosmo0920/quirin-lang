"""Exhaustive leaf-by-leaf diff of two JSON artifacts.

Usage: leafdiff.py LEFT.json RIGHT.json [max_rows]
Reports every differing scalar leaf by its full path, plus added/removed paths.
"""

import json
import sys


def leaves(node, prefix, out):
    if isinstance(node, dict):
        for key, value in node.items():
            leaves(value, f"{prefix}.{key}", out)
    elif isinstance(node, list):
        for index, value in enumerate(node):
            leaves(value, f"{prefix}[{index}]", out)
    else:
        out[prefix] = node


def main():
    left_path, right_path = sys.argv[1], sys.argv[2]
    limit = int(sys.argv[3]) if len(sys.argv) > 3 else 40

    left, right = {}, {}
    leaves(json.load(open(left_path)), "", left)
    leaves(json.load(open(right_path)), "", right)

    removed = sorted(set(left) - set(right))
    added = sorted(set(right) - set(left))
    changed = sorted(k for k in set(left) & set(right) if left[k] != right[k])

    print(f"leaves: left={len(left)} right={len(right)}")
    print(f"removed={len(removed)} added={len(added)} changed={len(changed)}")
    for label, keys in (("REMOVED", removed), ("ADDED", added)):
        for key in keys[:limit]:
            print(f"  {label} {key}")
        if len(keys) > limit:
            print(f"  ... {len(keys) - limit} more {label}")
    for key in changed[:limit]:
        print(f"  CHANGED {key}\n    - {left[key]!r}\n    + {right[key]!r}")
    if len(changed) > limit:
        print(f"  ... {len(changed) - limit} more CHANGED")


if __name__ == "__main__":
    main()
