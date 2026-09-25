#!/usr/bin/env python3
"""Show the analysed (`-E -P`) text of changed leaves, and which validated
callee each one's statement names.

Usage: show_lines.py CAPTURE_DIR DIAGNOSTICS.json ATTRIBUTION.json SCOPE_CENSUS.jsonl CLASS...

A `-P` capture is joined to a unit as `definition_sites.py` joins line-marked
ones (smallest capture whose definitions contain the unit's analysed rows). For
each changed leaf of the given attribution classes it prints the analysed line
and the validated callees of that unit the line calls.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import definition_sites  # noqa: E402

CALLS = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")


def main() -> int:
    capture, diagnostics, attribution, scope_path, *classes = sys.argv[1:]
    data = json.load(open(diagnostics))
    changes = [c for c in json.load(open(attribution))["changes"] if c["class"] in classes]
    wanted = {c["unit"] for c in changes}
    rows = {}
    for line in (gzip.open if scope_path.endswith(".gz") else open)(scope_path, "rt"):
        row = json.loads(line)
        rows.setdefault(tuple(sorted(row["selected"])), row)
    texts = []
    for path in sorted(pathlib.Path(capture, "pp").glob("*-p1.i")):
        text = path.read_text(errors="replace")
        texts.append((path.name, set(definition_sites.definitions(text)), text))
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        if key not in wanted:
            continue
        names = {f["function"] for f in unit["functions"]}
        matches = sorted((t for t in texts if names <= t[1]), key=lambda t: len(t[1]))
        if not matches:
            print("no -P capture for", key)
            continue
        lines = matches[0][2].split("\n")
        row = rows.get(tuple(sorted(names)), {"callees": []})
        validated = {c["name"] for c in row["callees"] if c["outcome"] == "validated"}
        for change in changes:
            if change["unit"] != key:
                continue
            text = lines[change["line"] - 1].strip() if 0 < change["line"] <= len(lines) else "?"
            named = sorted(set(CALLS.findall(text)) & validated)
            print(json.dumps({"unit": key, "function": change["function"], "line": change["line"],
                              "change": change["change"], "description": change["description"],
                              "validated_callees_on_line": named, "text": text[:220]}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
