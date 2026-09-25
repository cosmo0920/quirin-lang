#!/usr/bin/env python3
"""List every differing JSON leaf between two documents, by path.

Usage: json_leaf_diff.py LEFT RIGHT [--worktree-alias]

Prints one JSON document: the count of differing leaves, the set of differing
key names (the last path component with list indices removed), and the first
differences. `--worktree-alias` first rewrites this worktree's root to the main
checkout root in RIGHT's text, so a raw path difference is separated from a
digest that binds it.
"""

import json
import pathlib
import sys


def leaves(value, path=()):
    if isinstance(value, dict):
        for key, item in value.items():
            yield from leaves(item, path + (key,))
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from leaves(item, path + (index,))
    else:
        yield path, value


def main() -> int:
    left_text = pathlib.Path(sys.argv[1]).read_text()
    right_text = pathlib.Path(sys.argv[2]).read_text()
    if "--worktree-alias" in sys.argv:
        root = pathlib.Path(__file__).resolve().parents[4]
        if root.parent.name == "worktrees" and root.parent.parent.name == ".claude":
            right_text = right_text.replace(str(root), str(root.parents[2]))
    left = dict(leaves(json.loads(left_text)))
    right = dict(leaves(json.loads(right_text)))
    differing = sorted(
        (path for path in left.keys() | right.keys() if left.get(path) != right.get(path)),
        key=str,
    )
    names = sorted(
        {str(next((p for p in reversed(path) if isinstance(p, str)), "")) for path in differing}
    )
    print(
        json.dumps(
            {
                "differing_leaves": len(differing),
                "differing_key_names": names,
                "only_in_left": sum(1 for p in differing if p not in right),
                "only_in_right": sum(1 for p in differing if p not in left),
                "first": [
                    {"path": list(map(str, p)), "left": left.get(p), "right": right.get(p)}
                    for p in differing[:8]
                ],
            },
            indent=1,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
