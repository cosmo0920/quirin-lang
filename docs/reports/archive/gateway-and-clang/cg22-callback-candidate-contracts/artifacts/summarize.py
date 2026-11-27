#!/usr/bin/env python3
"""summarize.py NATIVE-COMPARISON.json -- CG22-CALLBACK-CANDIDATE-CONTRACTS:
the declared result, per comparison and route the status and totals, every
moved row and every content-changed row (component key), from the measure
plugin's record."""
import json
import pathlib
import sys


def main():
    record = json.loads(pathlib.Path(sys.argv[1]).read_text())
    print("declared:", json.dumps(record.get("declared"))[:600])
    print("failures:", record.get("failures"))
    timing = record.get("timing") or {}
    print("wall_time_ms:", timing.get("wall_time_ms"))
    for comparison in record["comparisons"]:
        print("comparison", comparison["comparison"])
        for name, route in comparison["routes"].items():
            print(" route", name, "status", route.get("status"))
            for key in ("totals", "before_totals", "after_totals"):
                if key in route:
                    print("  ", key, json.dumps(route[key]))
            for key in sorted(route):
                value = route[key]
                if isinstance(value, list):
                    print("  ", key, len(value))
                    for row in value[:40]:
                        print("     ", json.dumps(row)[:400])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
