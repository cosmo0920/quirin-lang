#!/usr/bin/env python3
"""Print every leaf naming one of the given callees, with its analysed line.

Usage: callee_lines.py DIAGNOSTICS(.json|.json.gz) UNIT_TEXTS_DIR OUT.json NAME...

A leaf is selected when its description names NAME in backticks. For each one
the analysed `-E -P` line (see `../../w43-libc-contracts/scripts/unit_texts.py`)
is recorded with the function's status and its total leaf count, so a
prediction can read every call shape rather than a sample.
"""

import gzip
import json
import pathlib
import sys


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def main() -> int:
    data = load(sys.argv[1])
    texts_dir = pathlib.Path(sys.argv[2])
    names = sys.argv[4:]
    pairing = json.loads((texts_dir / "pairing.json").read_text())
    rows = []
    for unit in data["units"]:
        key = f"{unit['library']}/{unit['file']}"
        text = pairing.get(key, {}).get("text")
        lines = (texts_dir / text).read_text(errors="replace").split("\n") if text else None
        for entry in unit["functions"]:
            diagnostics = entry.get("diagnostics") or []
            for diagnostic in diagnostics:
                hit = [n for n in names if f"`{n}`" in diagnostic["description"]]
                if not hit:
                    continue
                line = diagnostic["line"]
                rows.append(
                    {
                        "unit": key,
                        "function": entry["function"],
                        "status": entry["status"],
                        "function_leaves": len(diagnostics),
                        "description": diagnostic["description"],
                        "line": line,
                        "origin": diagnostic.get("origin"),
                        "text": lines[line - 1].strip()[:400] if lines and 0 < line <= len(lines) else None,
                    }
                )
    with open(sys.argv[3], "w") as handle:
        json.dump(rows, handle, indent=1)
        handle.write("\n")
    for row in rows:
        print(f"{row['status']:7} {row['function_leaves']:4} {row['function']:40} {row['text']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
