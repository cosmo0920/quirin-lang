#!/usr/bin/env python3
"""What still blocks the functions that call the `va_list` members.

Usage: valist_callers.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS OUT.json

The callers are the functions whose BEFORE leaves name `vsnprintf`, `vprintf`,
`vfprintf` or `vsprintf`. For each, the AFTER leaves are listed with their
descriptions, and the `__builtin_va_*` leaves are counted, so the residual the
card registers (`W44-VA-START-END-BUILTINS`) is sized from primary evidence.
"""

import collections
import json
import pathlib
import re
import sys

MEMBERS = ("vsnprintf", "vprintf", "vfprintf", "vsprintf")


def load(path):
    return json.loads(pathlib.Path(path).read_text())


def main() -> int:
    before, after = load(sys.argv[1]), load(sys.argv[2])
    callers = set()
    for unit in before["units"]:
        for entry in unit["functions"]:
            if any(any(f"`{m}`" in d["description"] for m in MEMBERS) for d in entry.get("diagnostics") or []):
                callers.add((unit["library"], unit["file"], entry["function"]))
    rows = []
    shapes = collections.Counter()
    for unit in after["units"]:
        for entry in unit["functions"]:
            key = (unit["library"], unit["file"], entry["function"])
            if key not in callers:
                continue
            descriptions = [d["description"] for d in entry.get("diagnostics") or []]
            builtin = [d for d in descriptions if "__builtin_va_" in d]
            for d in descriptions:
                shapes[re.sub(r"`[^`]*`", "`_`", d)] += 1
            rows.append({
                "function": list(key), "status": entry["status"], "leaves": len(descriptions),
                "builtin_va_leaves": len(builtin),
                "only_builtin_va": bool(descriptions) and len(builtin) == len(descriptions),
            })
    summary = {
        "callers": len(rows),
        "with_builtin_va_leaf": sum(1 for r in rows if r["builtin_va_leaves"]),
        "builtin_va_leaves": sum(r["builtin_va_leaves"] for r in rows),
        "blocked_only_by_builtin_va": sum(1 for r in rows if r["only_builtin_va"]),
        "leaf_shapes": dict(shapes.most_common()),
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps({"summary": summary, "callers": rows}, indent=1) + "\n")
    print(json.dumps({k: v for k, v in summary.items() if k != "leaf_shapes"}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
