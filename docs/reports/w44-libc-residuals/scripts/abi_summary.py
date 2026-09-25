#!/usr/bin/env python3
"""Tabulate declared-ABI census outputs into one evidence document.

Usage: abi_summary.py OUT.json LABEL=CENSUS.txt [LABEL=CENSUS.txt ...]

Each CENSUS.txt is the stdout of `declared_abi_census` (one `unit<TAB>name<TAB>
outcome` row per contracted name a unit mentions, then `--- totals`). For every
label the document records the outcome totals and, per name, the outcome
counts. When labels `manifest-base` and `manifest-reader` are both given, it
also records how many (unit, name) rows changed outcome between them, which is
the reader-change-alone measurement.
"""

import collections
import json
import pathlib
import sys


def read(path):
    rows = {}
    for line in pathlib.Path(path).read_text().splitlines():
        if line.startswith("---"):
            break
        parts = line.split("\t")
        if len(parts) == 3:
            unit, name, outcome = parts
            rows[(pathlib.Path(unit).name, name)] = outcome
    return rows


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    censuses = {}
    document = {"censuses": {}}
    for argument in sys.argv[2:]:
        label, path = argument.split("=", 1)
        rows = read(path)
        censuses[label] = rows
        per_name = collections.defaultdict(collections.Counter)
        for (_, name), outcome in rows.items():
            per_name[name][outcome] += 1
        document["censuses"][label] = {
            "source": path,
            "rows": len(rows),
            "totals": dict(collections.Counter(rows.values())),
            "by_name": {name: dict(counts) for name, counts in sorted(per_name.items())},
        }
    if "manifest-base" in censuses and "manifest-reader" in censuses:
        base, reader = censuses["manifest-base"], censuses["manifest-reader"]
        document["reader_change_alone"] = {
            "rows_base": len(base),
            "rows_reader": len(reader),
            "rows_changed": sum(1 for key in set(base) | set(reader) if base.get(key) != reader.get(key)),
        }
    out.write_text(json.dumps(document, indent=1, sort_keys=True) + "\n")
    print(json.dumps({label: block["totals"] for label, block in document["censuses"].items()}, indent=1))
    print(json.dumps(document.get("reader_change_alone")))
    return 0


if __name__ == "__main__":
    sys.exit(main())
