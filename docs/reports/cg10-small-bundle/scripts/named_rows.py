#!/usr/bin/env python3
"""named_rows.py BASE.json TIP.json -- CG10-SMALL-BUNDLE (after CG10-READER-GAPS): the named rows of
the card's three ledger entries, each with its status and leaves on two
opt-in `current` reports (rows keyed by unit file, function and occurrence),
as one JSON object per row."""
import collections
import json
import sys

NAMED = {
    "CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION": [
        "print_string", "pb_dec_bool", "pb_enc_fixed_length_bytes",
    ],
    "CG10-RESULT-HOLDER-RETAINED-BY": [
        "PartitionMemberInfo_new", "rd_kafka_op_new_cb", "rd_kafka_interceptor_method_new",
        "rd_kafka_member_assigned_partitions_pair_new",
    ],
    "CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS": [
        "rd_kafka_buf_set_maker",
    ],
}


def rows(path):
    data = json.load(open(path))
    out = collections.defaultdict(list)
    for unit in data["units"]:
        for function in unit.get("functions", []):
            out[function["function"]].append(
                {
                    "file": unit["file"],
                    "status": function["status"],
                    "leaves": sorted(
                        f"{leaf['line']}: {leaf['description']}"
                        for leaf in function.get("diagnostics", [])
                    ),
                }
            )
    return out


def main() -> int:
    base, tip = rows(sys.argv[1]), rows(sys.argv[2])
    for entry, names in NAMED.items():
        for name in names:
            for index, (before, after) in enumerate(
                zip(base.get(name, [None]), tip.get(name, [None]))
            ):
                print(
                    json.dumps(
                        {
                            "entry": entry,
                            "function": name,
                            "occurrence": index + 1,
                            "base": before,
                            "tip": after,
                        },
                        sort_keys=True,
                    )
                )
    return 0


if __name__ == "__main__":
    sys.exit(main())
