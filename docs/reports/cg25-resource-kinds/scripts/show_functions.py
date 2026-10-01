#!/usr/bin/env python3
"""Print the status and every diagnostic of named functions.

Usage: show_functions.py DIAGNOSTICS_JSON[.gz] NAME [NAME ...]
"""

import gzip
import json
import sys


def main() -> int:
    path = sys.argv[1]
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        data = json.load(handle)
    wanted = set(sys.argv[2:])
    for unit in data["units"]:
        for entry in unit["functions"]:
            if entry["function"] in wanted:
                print(unit["file"], entry["function"], entry.get("status"))
                for diagnostic in entry.get("diagnostics") or []:
                    print("   ", diagnostic.get("line"), diagnostic.get("category"), diagnostic["description"][:160])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
