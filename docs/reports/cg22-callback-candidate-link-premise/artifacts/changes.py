#!/usr/bin/env python3
"""changes.py RECORD -- CG22-CALLBACK-CANDIDATE-LINK-PREMISE: one line per
content-changed row of the first comparison's corpus route in a
native-comparison record (plain or gzip): status, leaves removed/added, and
the contract-dependency names that appear (+) or disappear (-) per field."""
import gzip
import json
import sys


def main(argv):
    path = argv[1]
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as handle:
        record = json.load(handle)
    route = record["comparisons"][0]["routes"]["corpus"]
    print("moves:", len(route["moves"]), "content changes:", len(route["content_changes"]))
    for change in route["content_changes"]:
        parts = change["row"].split("::")
        dependencies = change.get("contract_dependencies") or {}
        after, before = dependencies.get("after", {}), dependencies.get("before", {})
        fields = []
        for key in sorted(set(after) | set(before)):
            if after.get(key) == before.get(key):
                continue
            new, old = set(map(str, after.get(key, []))), set(map(str, before.get(key, [])))
            fields.append("%s +%s -%s" % (key, sorted(new - old), sorted(old - new)))
        print("%s::%s %s->%s leaves -%d +%d; %s" % (
            parts[1], parts[-1], change["status"][0], change["status"][1],
            len(change["leaves_removed"]), len(change["leaves_added"]), "; ".join(fields)))
        for leaf in change["leaves_removed"]:
            print("    - " + leaf["description"])
        for leaf in change["leaves_added"]:
            print("    + " + leaf["description"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
