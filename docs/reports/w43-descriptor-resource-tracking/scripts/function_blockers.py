#!/usr/bin/env python3
"""Print every diagnostic of the functions that carry a descriptor/handle leaf.

Usage: function_blockers.py DIAGNOSTICS_JSON_GZ [UNIT:FUNCTION ...]

With no function list, selects every function the descriptor inventory lists.
Used to write PREDICTION.md: a function can move only if every one of its
other blockers is also decided.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from descriptor_inventory import CALLEES, UNCONTRACTED  # noqa: E402


def main() -> int:
    data = json.load(gzip.open(sys.argv[1]))
    wanted = set(sys.argv[2:])
    for unit in data["units"]:
        for entry in unit["functions"]:
            key = f"{unit['file']}:{entry['function']}"
            diagnostics = entry.get("diagnostics") or []
            names = {
                m.group(1)
                for d in diagnostics
                if (m := UNCONTRACTED.fullmatch(d["description"])) and m.group(1) in CALLEES
            }
            if wanted and key not in wanted:
                continue
            if not wanted and not names:
                continue
            counts = collections.Counter(
                (d.get("category", "?"), d["description"]) for d in diagnostics
            )
            print(f"== {key} [{entry.get('status')}] {len(diagnostics)} leaves")
            for (kind, description), count in sorted(counts.items()):
                print(f"   {count:3} {kind}: {description[:150]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
