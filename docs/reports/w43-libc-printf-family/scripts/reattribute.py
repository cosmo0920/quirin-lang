#!/usr/bin/env python3
"""Close `attribute.py`'s rows with one further, body-anchored class.

Usage: reattribute.py ATTRIBUTION.json UNIT_TEXT_DIR OUT.json NAME...

`w43-libc-contracts/scripts/attribute.py` anchors a changed leaf at a decided
callee leaf, a newly Safe callee, the leaf's own line, or (same-function) any
other changed leaf of a function that has one of those. A declaration of NAME
also changes the C02-P closure of every body that CALLS NAME, wherever the call
sits: `borrowed_actual::seed_call_arguments` refuses, as an access base, every
name passed to an uncontracted call, and stops refusing a name passed only to a
declared `Borrowed` or declared-tail position. A body whose NAME call is in a
position that never reported an uncontracted-callee leaf (`hof += snprintf(…)`
reports `unsupported assignment expression`) can therefore move a leaf on
another line with no anchor in the first four classes.

This script assigns such a row the class `same-body-closure` when, and only
when, the analysed text of its own function body (from the definition's line
to its matching `}`) calls a name in NAME... . Every other unattributed row
stays unattributed. It recomputes `attribution_complete` from the original
summary's other conditions, which it does not touch.
"""

import json
import pathlib
import re
import sys


def body_text(lines, function):
    pattern = re.compile(r"(?<![A-Za-z0-9_])" + re.escape(function) + r"\s*\(")
    for index, line in enumerate(lines):
        if not pattern.search(line) or line.rstrip().endswith(";"):
            continue
        depth, seen, chunk = 0, False, []
        for text in lines[index:]:
            chunk.append(text)
            depth += text.count("{") - text.count("}")
            seen = seen or "{" in text
            if seen and depth <= 0:
                return "\n".join(chunk)
            if not seen and text.rstrip().endswith(";"):
                break
    return None


def main() -> int:
    data = json.loads(pathlib.Path(sys.argv[1]).read_text())
    texts = pathlib.Path(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    names = sys.argv[4:]
    call = re.compile(r"(?<![A-Za-z0-9_])(" + "|".join(map(re.escape, names)) + r")\s*\(")
    pairing = json.loads((texts / "pairing.json").read_text())
    closed = []
    for row in data["leaves"]:
        if row["class"] is not None:
            continue
        entry = pairing.get(f"{row['library']}/{row['unit']}") or {}
        if not entry.get("text"):
            continue
        lines = (texts / entry["text"]).read_text(errors="replace").split("\n")
        body = body_text(lines, row["function"])
        if body and call.search(body):
            row["class"] = "same-body-closure"
            row["body_calls"] = sorted(set(call.findall(body)))
            closed.append(row)
    summary = data["summary"]
    unattributed = [r for r in data["leaves"] if r["class"] is None]
    summary["same_body_closure"] = len(closed)
    summary["unattributed"] = len(unattributed)
    classes = {}
    for r in data["leaves"]:
        key = f"{r['direction']} {r['class']}"
        classes[key] = classes.get(key, 0) + r["count"]
    summary["by_class"] = dict(sorted(classes.items()))
    summary["attribution_complete"] = (
        not unattributed
        and not summary["unanchored_moves"]
        and not summary["unexplained_unit_changes"]
        and not [t for t in summary["top_level_changes"] if t != "diagnostic_counts"]
    )
    out.write_text(json.dumps(data, indent=1) + "\n")
    print(json.dumps({k: summary[k] for k in (
        "leaves_removed", "leaves_added", "by_class", "status_moves", "same_body_closure",
        "unattributed", "attribution_complete")}, indent=1))
    for row in closed:
        print("closed:", row["unit"], row["function"], row["line"], row["direction"], row["description"][:90])
    return 0 if summary["attribution_complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
