#!/usr/bin/env python3
"""attribute.py BEFORE AFTER PREMISES.jsonl.gz UNITS_DIR -- CG10 r3: every
row whose leaves changed between two opt-in `current` reports, by the reading
whose shape its removed access leaves have in the base AST census: `68` (a
path through a pointer member declared through a pointer typedef), `67` (a
statement with two or more direct calls), `12h` (a pointer cast at an
actual), `other`. A row with no access leaf removed is a re-attribution
reached through a callee or a newly read binding."""
import collections
import gzip
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import candidates  # noqa: E402

GATE = "indirect memory access is outside the resource projection"
CAST = re.compile(r"\(\s*[\w\s]+\*+\s*\)\s*\(?\s*\w+\s*(->|\.)")


def leaves(path):
    out = {}
    for unit in json.load(open(path))["units"]:
        for function in unit.get("functions", []):
            out[(unit["file"], function["function"])] = (
                function["status"],
                collections.Counter((leaf["description"], leaf["line"])
                                    for leaf in function.get("diagnostics", [])))
    return out


def main() -> int:
    before, after = leaves(sys.argv[1]), leaves(sys.argv[2])
    census = {(r["file"], r["function"], r["line"]): r
              for r in (json.loads(line) for line in gzip.open(sys.argv[3], "rt"))}
    candidates.UNITS = sys.argv[4]
    groups = collections.defaultdict(list)
    for key, (status, old) in before.items():
        if key not in after or after[key][1] == old and after[key][0] == status:
            continue
        removed = [line for (description, line) in old - after[key][1] if description == GATE]
        tags = set()
        for line in removed:
            row = census.get((key[0], key[1], line))
            if row is None:
                tags.add("unbound")
            elif candidates.typedef_pointer_paths(row):
                tags.add("68")
            elif len(row["calls"]) >= 2:
                tags.add("67")
            elif CAST.search(row["origin_text"]):
                tags.add("12h")
            else:
                tags.add("other")
        label = "+".join(sorted(tags)) or "no access leaf removed"
        groups[label].append((key[1], status, after[key][0], len(removed)))
    for label, items in sorted(groups.items(), key=lambda item: -len(item[1])):
        moved = [f"{f} {s}->{t}" for f, s, t, _ in items if s != t]
        print(f"## {label}: {len(items)} rows, {sum(n for *_, n in items)} access leaves removed"
              + (f"; moves: {', '.join(moved)}" if moved else ""))
        print("   " + ", ".join(sorted(f for f, *_ in items)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
