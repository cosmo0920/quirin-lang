#!/usr/bin/env python3
"""Print the probe rows of named functions from a contracts-route report.

Usage: probe_rows.py REPORT.json [FUNCTION ...]
(default functions: mk_vhost_set, mk_vhost_set_property). One JSON object per
function: status and every diagnostic's (line, description).
"""

import json
import sys


def rows(report, wanted):
    data = json.load(open(report))
    units = data.get("units") or []
    for unit in units:
        for entry in unit.get("functions") or []:
            if entry.get("function") in wanted:
                yield {
                    "file": unit.get("file"),
                    "function": entry["function"],
                    "status": entry.get("status"),
                    "diagnostics": [
                        [d.get("line"), d.get("description")]
                        for d in entry.get("diagnostics") or []
                    ],
                }


def main():
    report = sys.argv[1]
    wanted = set(sys.argv[2:]) or {"mk_vhost_set", "mk_vhost_set_property"}
    for row in rows(report, wanted):
        print(json.dumps(row, ensure_ascii=False))


if __name__ == "__main__":
    main()
