#!/usr/bin/env python3
"""parity.py BEFORE.jsonl AFTER.jsonl OUT.json [MOVING.json] -- VERDICT-BODY-STANDING-CLAUSE-SCOPE (adapted from VERDICT-CONTRACT-DEPENDENCIES, itself adapted from REFCOUNT-FAMILY-BLOCKING-CALLEES):
the full comparison of two QCORE_TYPED_PATH_PARITY streams (Clang AST node
addresses erased for the multiset comparison).

Every row is compared, first in stream order (byte identity) and then as a
multiset of whole rows: the rows only BEFORE has and only AFTER has, grouped by
(function, position). With MOVING (compare.py's output), each differing row's
function is checked against the functions whose leaves or status moved, so a
parity difference outside a moving function is named."""
import collections
import json
import pathlib
import re
import sys

ADDRESS = re.compile(rb'"0x[0-9a-f]+"')
# CALLEE-RETENTION-CLASS1: the parity rows Debug-print lowered instructions,
# and `ContractCall` gained a `retention` field (rule 12t). Its empty value is
# erased too, so a row differs only where a decision did.
RETENTION_NONE = b", retention: None"


def read(path, erase):
    rows = pathlib.Path(path).read_bytes().splitlines()
    if not erase:
        return rows
    return [ADDRESS.sub(b'"0x"', row).replace(RETENTION_NONE, b"") for row in rows]


def main():
    same_order = read(sys.argv[1], False) == read(sys.argv[2], False)
    # Clang AST node addresses ("0x..." values) are process memory and differ
    # between any two runs; they are erased before the multiset comparison.
    before, after = read(sys.argv[1], True), read(sys.argv[2], True)
    same_order_erased = before == after
    b, a = collections.Counter(before), collections.Counter(after)
    only_before, only_after = b - a, a - b
    moving = set()
    if len(sys.argv) > 4:
        moving = {m["function"] for m in json.loads(pathlib.Path(sys.argv[4]).read_text())["moves"]}
    groups = collections.Counter()
    outside = collections.Counter()
    for side, rows in (("before", only_before), ("after", only_after)):
        for row, count in rows.items():
            data = json.loads(row)
            function = data.get("function", "?")
            key = "%s %s %s" % (side, function, data.get("position", data.get("rule", "?")))
            groups[key] += count
            if moving and function not in moving:
                outside["%s %s" % (side, function)] += count
    record = {
        "rows": [len(before), len(after)],
        "byte_identical_in_order": same_order,
        "identical_in_order_addresses_erased": same_order_erased,
        "multiset_identical": not only_before and not only_after,
        "only_before": sum(only_before.values()),
        "only_after": sum(only_after.values()),
        "functions_with_differences": len({k.split()[1] for k in groups}),
        "differences_outside_moving_functions": dict(outside.most_common()),
        "groups": dict(groups.most_common()),
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: record[k] for k in ("rows", "byte_identical_in_order",
                                             "identical_in_order_addresses_erased", "multiset_identical",
                                             "only_before", "only_after",
                                             "functions_with_differences")}))
    print("outside moving functions:", json.dumps(record["differences_outside_moving_functions"])[:2000])


if __name__ == "__main__":
    main()
