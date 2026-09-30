#!/usr/bin/env python3
"""entries.py CG00_LEAVES_JSONL_GZ RUN [RUN…] -- card CG14-AGGREGATE-VALUES-AND-INITIALIZERS:
CG00's M08–M11 leaves, found again in each run by their stable identity
(unit file, function, origin `file:line`, description), per entry and CG00
reason: how many are still present. CG00 measured on `ae17152b`; a leaf that
is gone may have been lifted by any card since, so the BEFORE run on this
card's base is the comparison, not CG00's own count.
"""

import collections
import gzip
import json
import pathlib
import sys

ENTRIES = ("M08", "M09", "M10", "M11")


def present(run):
    data = json.loads((pathlib.Path(run) / "resource-diagnostics.json").read_text())
    counts = collections.Counter()
    for unit in data["units"]:
        name = pathlib.Path(unit["file"]).name
        for function in unit["functions"]:
            for diagnostic in function.get("diagnostics", []):
                if diagnostic.get("category") != "unsupported":
                    continue
                counts[(name, function["function"], diagnostic.get("origin"),
                        diagnostic.get("description"))] += 1
    return counts


def main() -> int:
    leaves = collections.Counter()
    with gzip.open(sys.argv[1], "rt", encoding="utf-8") as stream:
        next(stream)
        for line in stream:
            row = json.loads(line)
            if row.get("entry") in ENTRIES:
                key = (row["file"], row["function"], row["origin"], row["description"])
                leaves[(row["entry"], row["reason"], key)] += 1
    record = {}
    for run in sys.argv[2:]:
        counts = present(run)
        table = collections.defaultdict(lambda: {"cg00": 0, "present": 0})
        for (entry, reason, key), count in leaves.items():
            cell = table[f"{entry} {reason}"]
            cell["cg00"] += count
            cell["present"] += min(count, counts.get(key, 0))
        record[run] = dict(sorted(table.items()))
    print(json.dumps(record, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
