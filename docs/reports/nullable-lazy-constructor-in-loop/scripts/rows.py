#!/usr/bin/env python3
"""rows.py REPORT -- card NULLABLE-LAZY-CONSTRUCTOR-IN-LOOP: the status and
leaves of the eight rows the ledger entry names in one opt-in `current`
report, as JSON lines (unit file, function, occurrence, status, nullable-leaf
count, every leaf by line)."""
import collections
import json
import sys

ROWS = (
    "rd_kafka_cgrp_owned_but_not_exist_partitions",
    "rd_kafka_cgrp_calculate_subscribe_revoking_partitions",
    "rd_kafka_cgrp_consumer_assignment_with_metadata",
    "rd_kafka_cgrp_handle_SyncGroup_memberstate",
    "rd_kafka_mock_cgrp_consumer_member_needs_revocation",
    "rd_kafka_mock_handle_ConsumerGroupHeartbeat",
    "balance",
    "rd_kafka_mock_handle_ShareFetch",
)
NULLABLE = "used by a non-null contract"


def main() -> int:
    data = json.load(open(sys.argv[1]))
    for unit in data["units"]:
        seen = collections.Counter()
        for function in unit.get("functions", []):
            seen[function["function"]] += 1
            if function["function"] not in ROWS:
                continue
            leaves = sorted(
                (leaf["line"], leaf["description"]) for leaf in function.get("diagnostics", [])
            )
            print(json.dumps({
                "file": unit["file"],
                "function": function["function"],
                "occurrence": seen[function["function"]],
                "status": function["status"],
                "nullable": sum(1 for _, d in leaves if NULLABLE in d),
                "leaves": leaves,
            }))
    return 0


if __name__ == "__main__":
    sys.exit(main())
