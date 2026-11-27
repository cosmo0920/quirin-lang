#!/usr/bin/env python3
"""Dump the analysed text of every non-call leaf in the bodies of the callees
reachable (same unit) from Unknown functions blocked only by uncontracted calls.

Usage: cone_lines.py CAPTURE_DIR DIAGNOSTICS.json OUT.jsonl
"""
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, "docs/reports/w44-corpus-callee-summaries/scripts")
import definition_sites  # noqa: E402

CALL = re.compile(r"call to uncontracted function `([^`]+)`")


def main():
    capture, diagnostics, out = sys.argv[1:4]
    data = json.load(open(diagnostics))
    texts = []
    for path in sorted(pathlib.Path(capture, "pp").glob("*-p1.i")):
        text = path.read_text(errors="replace")
        texts.append((path.name, set(definition_sites.definitions(text)), text))
    with open(out, "w") as handle:
        for unit in data["units"]:
            key = unit["library"] + "/" + unit["file"]
            fns = {f["function"]: f for f in unit["functions"]}
            roots = [f for f in unit["functions"] if f["status"] == "Unknown" and f["diagnostics"]
                     and all(CALL.fullmatch(d["description"]) for d in f["diagnostics"])]
            if not roots:
                continue
            depth = {}
            work = collections.deque((f["function"], 0) for f in roots)
            while work:
                name, level = work.popleft()
                if name in depth:
                    continue
                depth[name] = level
                for d in fns[name]["diagnostics"]:
                    m = CALL.fullmatch(d["description"])
                    if m and m.group(1) in fns and fns[m.group(1)]["status"] == "Unknown":
                        work.append((m.group(1), level + 1))
            names = set(fns)
            matches = sorted((t for t in texts if names <= t[1]), key=lambda t: len(t[1]))
            lines = matches[0][2].split("\n") if matches else []
            for name, level in depth.items():
                if level == 0:
                    continue
                for d in fns[name]["diagnostics"]:
                    if CALL.fullmatch(d["description"]):
                        continue
                    text = lines[d["line"] - 1].strip() if 0 < d["line"] <= len(lines) else "?"
                    handle.write(json.dumps({"unit": key, "function": name, "depth": level,
                                             "description": d["description"], "line": d["line"],
                                             "origin": d.get("origin"), "text": text[:240]}) + "\n")


if __name__ == "__main__":
    main()
