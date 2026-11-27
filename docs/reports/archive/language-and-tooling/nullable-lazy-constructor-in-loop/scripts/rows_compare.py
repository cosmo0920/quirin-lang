#!/usr/bin/env python3
"""rows_compare.py BASE.json TIP.json -- card NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP:
the eight ledger rows in two opt-in `current` reports: status, leaf count, and
the nullability leaves (`... used by a non-null contract`, `... returned under
a non-null result mode`) and identity leaves on each row's named resource."""
import collections
import json
import sys

ROWS = {
    "rd_kafka_cgrp_owned_but_not_exist_partitions": "result",
    "rd_kafka_cgrp_calculate_subscribe_revoking_partitions": "revoking",
    "rd_kafka_cgrp_consumer_assignment_with_metadata": "assignment_with_metadata",
    "rd_kafka_cgrp_handle_SyncGroup_memberstate": "assignment",
    "rd_kafka_mock_cgrp_consumer_member_needs_revocation": "intersection",
    "rd_kafka_mock_handle_ConsumerGroupHeartbeat": "existing_assignment",
    "balance": "leftoverUnassignedPartitions",
    "rd_kafka_mock_handle_ShareFetch": "forgotten_partitions",
}


def rows(path):
    out = {}
    for unit in json.load(open(path))["units"]:
        seen = collections.Counter()
        for function in unit.get("functions", []):
            seen[function["function"]] += 1
            if function["function"] in ROWS and seen[function["function"]] == 1:
                out[function["function"]] = function
    return out


def named(function, name):
    return sorted(
        (leaf["line"], leaf["description"])
        for leaf in function.get("diagnostics", [])
        if f"`{name}`" in leaf["description"]
    )


def main() -> int:
    base, tip = rows(sys.argv[1]), rows(sys.argv[2])
    for function, name in ROWS.items():
        b, t = base[function], tip[function]
        nb = sum("non-null contract" in d for _, d in named(b, name))
        nt = sum("non-null contract" in d for _, d in named(t, name))
        print(f"## {function}: {b['status']} -> {t['status']}; leaves "
              f"{len(b['diagnostics'])} -> {len(t['diagnostics'])}; nullable `{name}` {nb} -> {nt}")
        for side, rec in (("base", b), ("tip", t)):
            for line, description in named(rec, name):
                print(f"   {side} {line} {description}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
