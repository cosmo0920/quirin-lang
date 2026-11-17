#!/usr/bin/env python3
"""changes.py NATIVE-COMPARISON.json -- CG22-CALLBACK-CANDIDATE-CONTRACTS: per
moved or content-changed corpus row, its component key, status pair, the
leaves removed and added, the other fields changed, and the summaries and
null premises its contract dependencies gained or lost."""
import json
import pathlib
import sys


def diff(before, after):
    before, after = set(before or []), set(after or [])
    return sorted(after - before), sorted(before - after)


def main():
    record = json.loads(pathlib.Path(sys.argv[1]).read_text())
    route = record["comparisons"][0]["routes"]["corpus"]
    for kind in ("moves", "content_changes"):
        print("==", kind, len(route.get(kind, [])))
        for row in route.get(kind, []):
            print(row["row"], "%s -> %s" % tuple(row["status"]))
            for leaf in row.get("leaves_removed", []):
                print("   - %s" % json.dumps(leaf)[:300])
            for leaf in row.get("leaves_added", []):
                print("   + %s" % json.dumps(leaf)[:300])
            print("   other fields:", row.get("other_fields_changed"))
            deps = row.get("contract_dependencies")
            if deps:
                for key in ("summaries", "null_premises", "declarations"):
                    gained, lost = diff(deps["before"].get(key), deps["after"].get(key))
                    if gained or lost:
                        print("   %s gained %s lost %s" % (key, gained, lost))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
