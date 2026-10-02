#!/usr/bin/env python3
"""probe_rows.py PROBE_DIR... -- the mk_vhost_set and mk_vhost_set_property rows
(status and leaves) of each probe directory's tip.json and master.json."""
import json
import pathlib
import sys

WANTED = ("mk_vhost_set", "mk_vhost_set_property")


def rows(report):
    data = json.loads(report.read_text())
    for unit in data.get("units", []):
        for function in unit.get("functions", []):
            name = function.get("function") or function.get("name")
            if name in WANTED:
                yield name, function.get("status"), sorted(
                    f"{d.get('origin')}:{d.get('category')}:{d.get('description', '')}"
                    for d in function.get("diagnostics", [])
                )


def main():
    result = {}
    for directory in map(pathlib.Path, sys.argv[1:]):
        for side in ("tip", "master"):
            for name, status, leaves in rows(directory / f"{side}.json"):
                result[f"{directory.name}/{side}/{name}"] = {"status": status, "leaves": leaves}
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
