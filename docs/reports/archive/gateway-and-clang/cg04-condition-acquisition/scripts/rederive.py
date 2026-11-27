#!/usr/bin/env python3
"""CG04-CONDITION-ACQUISITION: re-derive CG00's frozen function list on this card's base.

Usage: rederive.py PREDICTION_MD BEFORE_DIR

Reads CG00's frozen `predictions/CG04.md` (the 31 functions it could free
alone, with their blocking leaves) and this card's BEFORE run
(`resource-diagnostics.json`), and prints, per predicted function, its status
on the base and its unsupported leaves now (description, count), so a function
that already moved, or whose blockers changed, is named before implementing.
"""

import collections
import json
import pathlib
import re
import sys


def main() -> int:
    prediction = pathlib.Path(sys.argv[1]).read_text()
    before = json.loads((pathlib.Path(sys.argv[2]) / "resource-diagnostics.json").read_text())
    predicted = []
    for line in prediction.splitlines():
        match = re.match(r"- `([^`]+)::([^`]+)` — (\w+)", line)
        if match:
            predicted.append((match.group(1), match.group(2), match.group(3)))
    index = {}
    for unit in before["units"]:
        for function in unit["functions"]:
            index.setdefault((pathlib.Path(unit["file"]).name, function["function"]), []).append(
                (unit["file"], function))
    rows = []
    for unit, name, predicted_status in predicted:
        key = (pathlib.Path(unit).name, name)
        matches = [m for m in index.get(key, []) if m[0].endswith(unit)]
        if len(matches) != 1:
            rows.append({"unit": unit, "function": name, "predicted": predicted_status,
                         "base": f"unmatched ({len(matches)})"})
            continue
        _, function = matches[0]
        leaves = collections.Counter()
        for diagnostic in function.get("diagnostics", []):
            leaves[diagnostic.get("description") or diagnostic.get("category")] += 1
        rows.append({"unit": unit, "function": name, "predicted": predicted_status,
                     "base": function["status"], "leaves": dict(leaves)})
    print(json.dumps(rows, indent=1, ensure_ascii=False))
    moved = [r for r in rows if r["base"] != "Unknown"]
    print(f"predicted {len(rows)}; already not Unknown on the base: {len(moved)}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
