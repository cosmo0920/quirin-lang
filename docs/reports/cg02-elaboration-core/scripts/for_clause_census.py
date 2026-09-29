#!/usr/bin/env python3
"""CG02-ELABORATION-CORE: size the for-clause residual.

Usage: for_clause_census.py RUN_DIR SOURCE_ROOT

Counts the unsupported leaves of a corpus run whose original line is a `for`
header with a comma or an increment/decrement in its init or increment clause
and whose description is not the loop-condition refusal: the clause lowerings
(`classify_for_clause`) the elaborator is not connected to. Lexical: a header
split over lines is counted only where the leaf's line holds the clauses.
"""

import collections
import json
import pathlib
import re
import sys

CONDITION = "unmodeled call, aggregate or indirect access in for-loop condition"


def clauses(line):
    start = line.find("(", line.find("for"))
    depth, parts, current = 0, [], []
    for char in line[start + 1:]:
        if char == "(":
            depth += 1
        elif char == ")":
            if depth == 0:
                break
            depth -= 1
        if char == ";" and depth == 0:
            parts.append("".join(current))
            current = []
            continue
        current.append(char)
    parts.append("".join(current))
    return parts


def main() -> int:
    run, root = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    index = {}
    for path in root.rglob("*.[ch]"):
        if path.is_file():
            index.setdefault(path.name, path)
            index.setdefault(str(path.relative_to(root)), path)
    cache = {}
    data = json.loads((run / "resource-diagnostics.json").read_text())
    leaves, functions, reasons = 0, set(), collections.Counter()
    for unit in data["units"]:
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                if diagnostic["category"] != "unsupported" or diagnostic["description"] == CONDITION:
                    continue
                origin = diagnostic.get("origin") or ""
                name, _, number = origin.rpartition(":")
                path = index.get(name) or index.get(pathlib.Path(name).name)
                if path is None or not number.isdigit():
                    continue
                lines = cache.setdefault(path, path.read_text(errors="replace").splitlines())
                line = lines[int(number) - 1] if int(number) <= len(lines) else ""
                if not re.match(r"\s*for\s*\(", line):
                    continue
                parts = clauses(line)
                if len(parts) != 3:
                    continue
                effectful = [parts[0], parts[2]]
                if any("," in part or "++" in part or "--" in part for part in effectful):
                    leaves += 1
                    functions.add((unit["file"], function["function"]))
                    reasons[diagnostic["description"]] += 1
    print(json.dumps({"leaves": leaves, "functions": len(functions),
                      "reasons": dict(reasons.most_common(10))}, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
