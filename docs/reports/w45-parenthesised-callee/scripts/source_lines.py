#!/usr/bin/env python3
"""Print the pinned source line behind every leaf of one refusal description.

Usage: source_lines.py DIAGNOSTICS DESCRIPTION_PREFIX [--out FILE]

Reads a run's `resource-diagnostics.json`, selects every leaf whose
description starts with DESCRIPTION_PREFIX, and prints its unit, function,
origin (`file:line` in the pinned Fluent Bit source) and that source line,
read from the pinned read-only checkout. Reading only; used to classify the
card's added leaves by reading.
"""

import json
import pathlib
import sys

PIN = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
)


def main() -> int:
    document = json.loads(pathlib.Path(sys.argv[1]).read_text())
    prefix = sys.argv[2]
    rows = []
    for unit in document["units"]:
        source = PIN / unit["file"] if not pathlib.Path(unit["file"]).is_absolute() else pathlib.Path(unit["file"])
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                if not diagnostic.get("description", "").startswith(prefix):
                    continue
                origin = diagnostic.get("origin", "")
                name, _, line = origin.rpartition(":")
                text = ""
                candidates = [source] if source.is_file() and source.name == name else []
                candidates += sorted(PIN.rglob(name))
                for path in candidates:
                    try:
                        lines = path.read_text(errors="replace").splitlines()
                        text = lines[int(line) - 1].strip()
                        break
                    except (OSError, ValueError, IndexError):
                        continue
                rows.append(f"{unit['file']}\t{function['function']}\t{origin}\t{text}")
    output = "\n".join(sorted(rows)) + "\n"
    if "--out" in sys.argv[3:]:
        pathlib.Path(sys.argv[sys.argv.index("--out", 3) + 1]).write_text(output)
    print(output, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
