#!/usr/bin/env python3
"""Itemise how two runs' `resource-diagnostics.contracts.json` differ.

Usage: contracts_diff.py BEFORE_DIR AFTER_DIR OUT.json

Walks both JSON documents and lists every leaf path whose value differs (for a
string, whether the difference is confined to one source text), so the
`contracts` artifact's change can be stated exactly rather than as a digest.
"""

import json
import pathlib
import sys

NAME = "resource-diagnostics.contracts.json"


def walk(a, b, path, out):
    if type(a) is not type(b):
        out.append({"path": path, "kind": "type"})
    elif isinstance(a, dict):
        for key in sorted(set(a) | set(b)):
            if key not in a:
                out.append({"path": f"{path}/{key}", "kind": "added"})
            elif key not in b:
                out.append({"path": f"{path}/{key}", "kind": "removed"})
            else:
                walk(a[key], b[key], f"{path}/{key}", out)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append({"path": path, "kind": "length", "before": len(a), "after": len(b)})
        for index, (x, y) in enumerate(zip(a, b)):
            walk(x, y, f"{path}[{index}]", out)
    elif a != b:
        entry = {"path": path, "kind": "value"}
        if isinstance(a, str) and isinstance(b, str):
            entry["before_length"], entry["after_length"] = len(a), len(b)
            entry["after_starts_with_before"] = b.startswith(a[: max(0, len(a) - 3)])
        else:
            entry["before"], entry["after"] = a, b
        out.append(entry)


def main() -> int:
    before = json.loads((pathlib.Path(sys.argv[1]) / NAME).read_text())
    after = json.loads((pathlib.Path(sys.argv[2]) / NAME).read_text())
    out = []
    walk(before, after, "", out)
    pathlib.Path(sys.argv[3]).write_text(json.dumps({"differences": out}, indent=1) + "\n")
    print(json.dumps(out, indent=1)[:3000])
    return 0


if __name__ == "__main__":
    sys.exit(main())
