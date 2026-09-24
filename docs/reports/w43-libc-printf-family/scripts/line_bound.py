#!/usr/bin/env python3
"""Forecast bound: functions whose EVERY leaf sits on an analysed line that calls
a printf-family name.

Usage: line_bound.py BASELINE_DIAGNOSTICS UNIT_TEXT_DIR OUT.json NAME...

Written and run before any declaration was drafted into the manifest. A
function all of whose leaves are on such lines is the most a declaration of
NAME... could move by line-local effects alone (the declared-callee leaves
themselves, and the same-line refusals the prior libc cards saw decided with
them: branch conditions, lexical indirect access, nested calls). It is an upper
bound on line-local moves, not a prediction: a line can also gain a refusal.
Also counts, per status, the leaves on such lines that are NOT the
declared-callee leaf (the same-line candidates).
"""

import collections
import json
import pathlib
import re
import sys


def main() -> int:
    data = json.loads(pathlib.Path(sys.argv[1]).read_text())
    texts = pathlib.Path(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    names = sys.argv[4:]
    call = re.compile(r"(?<![A-Za-z0-9_])(" + "|".join(map(re.escape, names)) + r")\s*\(")
    own = re.compile(r"call to uncontracted function `(" + "|".join(map(re.escape, names)) + r")`")
    pairing = json.loads((texts / "pairing.json").read_text())
    candidates, same_line = [], collections.Counter()
    for unit in data["units"]:
        entry = pairing.get(f"{unit['library']}/{unit['file']}") or {}
        if not entry.get("text"):
            continue
        lines = (texts / entry["text"]).read_text(errors="replace").split("\n")
        for function in unit["functions"]:
            leaves = function.get("diagnostics") or []
            if not leaves or function["status"] == "Safe":
                continue
            on_line = []
            for leaf in leaves:
                text = lines[leaf["line"] - 1] if 0 < leaf["line"] <= len(lines) else ""
                on_line.append(bool(call.search(text)) or bool(own.fullmatch(leaf["description"])))
                if call.search(text) and not own.fullmatch(leaf["description"]):
                    same_line[(function["status"], leaf["category"], re.sub(r"`[^`]*`", "`_`", leaf["description"]))] += 1
            if all(on_line) and any(own.fullmatch(l["description"]) for l in leaves):
                candidates.append({
                    "unit": f"{unit['library']}/{unit['file']}", "function": function["function"],
                    "status": function["status"],
                    "leaves": collections.Counter(l["description"] for l in leaves),
                })
    record = {
        "names": names,
        "all_leaves_on_family_lines": len(candidates),
        "by_status": dict(collections.Counter(c["status"] for c in candidates)),
        "candidates": candidates,
        "same_line_leaves": [
            {"status": s, "category": c, "shape": d, "leaves": n}
            for (s, c, d), n in sorted(same_line.items(), key=lambda kv: -kv[1])
        ],
    }
    out.write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: v for k, v in record.items() if k not in ("candidates", "same_line_leaves")}, indent=1))
    for c in candidates:
        print(c["status"], c["unit"], c["function"], dict(c["leaves"]))
    for row in record["same_line_leaves"][:25]:
        print(row)
    return 0


if __name__ == "__main__":
    sys.exit(main())
