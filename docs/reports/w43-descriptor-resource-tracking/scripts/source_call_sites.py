#!/usr/bin/env python3
"""Count raw source call sites of descriptor/handle functions in the audited units.

Usage: source_call_sites.py DIAGNOSTICS_JSON_GZ SOURCE_ROOT [MONKEY_ROOT ...]

Reads the unit list from the diagnostics artifact and scans each unit's own
source text (not its headers) for `NAME (` tokens outside comments and string
literals. This is a lexical census: it says where calls are *spelled*, which
the leaf inventory cannot, because a call inside a statement the gateway
already refused for another reason carries that other leaf instead.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from descriptor_inventory import CALLEES  # noqa: E402

COMMENT = re.compile(r"/\*.*?\*/|//[^\n]*|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'", re.S)


def main() -> int:
    data = json.load(gzip.open(sys.argv[1]))
    roots = [pathlib.Path(p) for p in sys.argv[2:]]
    sites = collections.Counter()
    where = collections.defaultdict(set)
    missing = []
    pattern = re.compile(r"(?<![\w.>])(" + "|".join(map(re.escape, CALLEES)) + r")\s*\(")
    index = collections.defaultdict(list)
    for root in roots:
        for candidate in root.rglob("*.c"):
            index[(root, candidate.name)].append(candidate)
    for unit in data["units"]:
        name = unit["file"]
        path = None
        for root in roots:
            found = sorted(p for p in index[(root, pathlib.Path(name).name)] if str(p).endswith("/" + name))
            if found:
                path = found[0]
                if len(found) > 1:
                    print("ambiguous", name, [str(f) for f in found[:3]])
                break
        if path is None:
            missing.append(name)
            continue
        text = COMMENT.sub(" ", path.read_text(errors="replace"))
        for match in pattern.finditer(text):
            sites[match.group(1)] += 1
            where[match.group(1)].add(name)
    for callee in CALLEES:
        if sites[callee]:
            print(f"{callee:16} {sites[callee]:4}  {sorted(where[callee])}")
    print("units not found:", len(missing), missing[:10])
    return 0


if __name__ == "__main__":
    sys.exit(main())
