#!/usr/bin/env python3
"""Show the analysed text of every leaf of the named functions.

Usage: show_fn.py CAPTURE_DIR DIAGNOSTICS.json FUNCTION...
"""
import json
import pathlib
import sys

sys.path.insert(0, "docs/reports/w44-corpus-callee-summaries/scripts")
import definition_sites  # noqa: E402


def main():
    capture, diagnostics, *want = sys.argv[1:]
    data = json.load(open(diagnostics))
    texts = []
    for path in sorted(pathlib.Path(capture, "pp").glob("*-p1.i")):
        text = path.read_text(errors="replace")
        texts.append((set(definition_sites.definitions(text)), text))
    for unit in data["units"]:
        names = {f["function"] for f in unit["functions"]}
        chosen = [f for f in unit["functions"] if f["function"] in want]
        if not chosen:
            continue
        match = sorted((t for t in texts if names <= t[0]), key=lambda t: len(t[0]))
        lines = match[0][1].split("\n") if match else []
        for entry in chosen:
            print("==", unit["library"] + "/" + unit["file"], entry["function"], entry["status"])
            for d in entry["diagnostics"][:12]:
                text = lines[d["line"] - 1].strip() if 0 < d["line"] <= len(lines) else "?"
                print("  ", d["line"], d["description"][:60], "|", text[:140])


if __name__ == "__main__":
    main()
