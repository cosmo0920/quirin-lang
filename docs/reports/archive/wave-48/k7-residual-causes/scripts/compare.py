#!/usr/bin/env python3
"""compare.py BASE_RUN TIP_RUN USER_REPORT OUT.json -- K7: the unit-scoped
base and tip reports of the selection, row by row (file, function): status
moves and leaf differences keyed by (category, description, line); and how
many base rows reproduce the user's 11-24 rows (status and leaves)."""
import collections
import json
import pathlib
import sys


def rows(path, files=None):
    report = json.load(open(path))
    out = {}
    for unit in report["units"]:
        if files is not None and unit["file"] not in files:
            continue
        for function in unit["functions"]:
            leaves = collections.Counter((d["category"], d["description"], d["line"])
                                         for d in function["diagnostics"])
            out[(unit["file"], function["function"])] = (function["status"], leaves)
    return out


base_dir, tip_dir, user, out = (pathlib.Path(a) for a in sys.argv[1:5])
base = rows(base_dir / "resource-diagnostics.json")
tip = rows(tip_dir / "resource-diagnostics.json")
files = {f for f, _ in base}
mine = rows(user, files)
reproduced = sum(1 for k, v in base.items() if mine.get(k) == v)
moves, leaf_changes = [], []
for key in sorted(set(base) | set(tip)):
    b, t = base.get(key), tip.get(key)
    if b is None or t is None:
        moves.append({"row": list(key), "base": b and b[0], "tip": t and t[0]})
        continue
    if b[0] != t[0]:
        moves.append({"row": list(key), "base": b[0], "tip": t[0]})
    if b[1] != t[1]:
        leaf_changes.append({"row": list(key), "removed": [list(k) for k in (b[1] - t[1]).elements()],
                             "added": [list(k) for k in (t[1] - b[1]).elements()]})
status = {side: dict(collections.Counter(v[0] for v in data.values())) for side, data in
          (("base", base), ("tip", tip))}
result = {"rows": len(base), "reproduced_user_rows": reproduced, "user_rows_in_selection": len(mine),
          "status": status, "moves": moves, "leaf_changes": leaf_changes}
out.write_text(json.dumps(result, indent=1) + "\n")
print(json.dumps({k: v for k, v in result.items() if k != "leaf_changes"}, indent=1))
for change in leaf_changes:
    print(change["row"], "removed", change["removed"], "added", change["added"])
