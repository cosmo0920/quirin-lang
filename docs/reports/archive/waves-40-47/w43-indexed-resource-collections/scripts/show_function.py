#!/usr/bin/env python3
"""Print one function's analysed -E -P text with its line numbers and leaves.

Usage: show_function.py RUN_DIR UNIT_TEXTS_DIR UNIT_KEY FUNCTION
"""

import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import collection_census as census  # noqa: E402


def main() -> int:
    run, texts, key, name = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3], sys.argv[4]
    text = (texts / key).read_text(errors="replace")
    tokens = census.tokenize(text)
    bodies = census.function_bodies(tokens)
    head, _, close = bodies[name]
    first, last = tokens[head][1], tokens[close][1]
    leaves = {}
    diagnostics = json.loads((run / "resource-diagnostics.json").read_text())
    for unit in diagnostics["units"]:
        if f"{unit['library']}__{unit['file'].replace('/', '__')}.i" != key:
            continue
        for function in unit.get("functions", []):
            if function["function"] == name:
                print("status", function["status"])
                for leaf in function.get("diagnostics", []):
                    leaves.setdefault(leaf["line"], []).append(leaf["description"])
    lines = text.split("\n")
    for number in range(first, last + 1):
        marks = " | ".join(leaves.get(number, []))
        print(f"{number:6} {lines[number - 1]}" + (f"    <<< {marks}" if marks else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
