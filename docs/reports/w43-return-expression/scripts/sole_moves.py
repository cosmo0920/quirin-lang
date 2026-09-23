#!/usr/bin/env python3
"""Where the BEFORE sole-return-blocked functions stand AFTER.

Usage: sole_moves.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS   (plain or .gz)

For every Unknown function whose only distinct BEFORE reason is the
return-expression refusal, prints its AFTER status and the normalised set of
its AFTER distinct reasons, counted.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

RETURN = "unmodeled call, aggregate or indirect access in return expression"


def load(path):
    data = pathlib.Path(path).read_bytes()
    return json.loads(gzip.decompress(data) if path.endswith(".gz") else data)


def functions(data):
    out = {}
    for unit in data["units"]:
        for entry in unit["functions"]:
            out[(unit["library"], unit["file"], entry["function"])] = (
                entry.get("status"),
                {d["description"] for d in entry.get("diagnostics") or []},
            )
    return out


def normalise(text):
    return re.sub(r"`[^`]*`", "`_`", text)


def main() -> int:
    before = functions(load(sys.argv[1]))
    after = functions(load(sys.argv[2]))
    sole = [k for k, (status, reasons) in before.items() if status == "Unknown" and reasons == {RETURN}]
    statuses = collections.Counter(after[k][0] for k in sole)
    shapes = collections.Counter(
        " + ".join(sorted({normalise(r) for r in after[k][1]})) for k in sole
    )
    print(f"BEFORE sole-return-blocked Unknown functions: {len(sole)}")
    print(f"AFTER statuses: {dict(statuses)}")
    print(f"{'functions':>9}  AFTER distinct reasons (normalised)")
    for shape, count in shapes.most_common():
        print(f"{count:9}  {shape}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
