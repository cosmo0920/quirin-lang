#!/usr/bin/env python3
"""Compare a private rebuild's compilation database with the pinned one.

Usage: compare_compile_db.py PINNED_SOURCE PINNED_BUILD PRIVATE_SOURCE PRIVATE_BUILD [OUT.json]

Each entry is normalized by replacing the two roots with SRC/BLD. Equality means
the private build compiles the same translation units with the same arguments,
apart from the checkout and build directory spelling. Read-only on both sides.
"""
import json
import pathlib
import sys


def normalized(database, source, build):
    rows = set()
    for entry in json.loads(pathlib.Path(database).read_text()):
        text = []
        for key in ("file", "command", "output"):
            value = entry.get(key, "")
            text.append(value.replace(source, "SRC").replace(build, "BLD"))
        rows.add(tuple(text))
    return rows


def main() -> int:
    pinned_source, pinned_build, private_source, private_build = sys.argv[1:5]
    pinned = normalized(pinned_build + "/compile_commands.json", pinned_source, pinned_build)
    private = normalized(private_build + "/compile_commands.json", private_source, private_build)
    result = {
        "pinned_entries": len(pinned),
        "private_entries": len(private),
        "common": len(pinned & private),
        "only_pinned": sorted(row[0] + " -> " + row[2] for row in pinned - private),
        "only_private": sorted(row[0] + " -> " + row[2] for row in private - pinned),
    }
    result["identical"] = not result["only_pinned"] and not result["only_private"]
    text = json.dumps(result, indent=2) + "\n"
    if len(sys.argv) > 5:
        pathlib.Path(sys.argv[5]).write_text(text)
    print(json.dumps({k: (v if not isinstance(v, list) else len(v)) for k, v in result.items()}))
    return 0 if result["identical"] else 1


if __name__ == "__main__":
    sys.exit(main())
