#!/usr/bin/env python3
"""Where the functions that named a declared libc callee stand after the arbiter.

Usage: where_they_went.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS MODULE.qcore...

(Plain or gzipped JSON.) For every function whose BEFORE diagnostics name a
declared callee in a `call to uncontracted function` leaf, reports its BEFORE
and AFTER status and, if still not Safe, the distinct AFTER blocker shapes.
Separately for the sole-blocked subset (one distinct BEFORE description, that
callee). Prints JSON.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

CALL = re.compile(r"call to uncontracted function `([^`]+)`")


def load(path):
    data = pathlib.Path(path).read_bytes()
    return json.loads(gzip.decompress(data) if path.endswith(".gz") else data)


def main() -> int:
    before, after = load(sys.argv[1]), load(sys.argv[2])
    declared = set()
    for module in sys.argv[3:]:
        text = "\n".join(l.split("//", 1)[0] for l in pathlib.Path(module).read_text().split("\n"))
        declared |= set(re.findall(r"extern fn (\w+)\(", text))
    fa = {(u["library"], u["file"], f["function"]): f for u in after["units"] for f in u["functions"]}
    groups = {"named": collections.Counter(), "sole": collections.Counter()}
    remaining = {"named": collections.Counter(), "sole": collections.Counter()}
    sole_rows = []
    for unit in before["units"]:
        for f in unit["functions"]:
            descriptions = {d["description"] for d in f.get("diagnostics") or []}
            names = {m.group(1) for d in descriptions if (m := CALL.fullmatch(d))}
            if not names & declared:
                continue
            key = (unit["library"], unit["file"], f["function"])
            new = fa[key]
            shapes = sorted({re.sub(r"`[^`]*`", "`_`", d["description"]) for d in new.get("diagnostics") or []})
            sole = f["status"] == "Unknown" and len(descriptions) == 1
            for group in ("named",) + (("sole",) if sole else ()):
                groups[group][f"{f['status']} -> {new['status']}"] += 1
                if new["status"] != "Safe":
                    remaining[group][" + ".join(shapes)] += 1
            if sole:
                sole_rows.append({"function": "/".join(key), "callee": sorted(names)[0], "after": new["status"], "remaining": shapes})
    print(json.dumps({
        "functions_naming_a_declared_callee": dict(groups["named"]),
        "their_remaining_blockers_top": dict(remaining["named"].most_common(15)),
        "sole_blocked_by_a_declared_callee": dict(groups["sole"]),
        "sole_remaining_blockers": dict(remaining["sole"].most_common()),
        "sole_rows": sole_rows,
    }, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
