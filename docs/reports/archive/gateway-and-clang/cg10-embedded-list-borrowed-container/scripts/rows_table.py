#!/usr/bin/env python3
"""rows_table.py ROWS.json OUT.md -- the 23 embedded-list sole rows as a table
(ROWS.json is rows.py's output)."""
import json
import sys

CONTRACT = {
    "rd_list_add": "borrowed(rl), element_added(elem, rl), return borrowed_from(rl)",
    "rd_list_destroy": "borrowed(rl), registry invoked over the elements, elements dropped, "
    "unregistered, released_if_constructed(rl)",
    "rd_list_clear": "borrowed(rl), registry invoked over the elements, elements dropped",
    "rd_kafka_interceptor_method_add": "all borrowed, retained_by(func, list), "
    "retained_by(ic_opaque, list), return none",
    "cfl_list_size": "borrowed(head), return none; the call is nested in mpack_start_array's actual",
}

rows = json.load(open(sys.argv[1]))
out = [
    "# The 23 embedded-list sole rows (round 3's tip)",
    "",
    "`scripts/rows.py` over round 3's tip premises (`census-tip2/premises.jsonl.gz`) and "
    "preprocessed units, `refine.py`'s class `embedded-list-address` alone; "
    "`scripts/rows_table.py`. Contract: the declaration in force "
    "(`qcore-provider-librdkafka/contracts/librdkafka/client.qv`, "
    "`quirin-provider-cfl/contracts/cfl/sds_lifetime.qv`).",
    "",
    "| # | file | function | leaves | helper | embedded list | helper contract |",
    "|---:|---|---|---:|---|---|---|",
]
for number, row in enumerate(rows, 1):
    paths = [p for leaf in row["leaves"] for p in leaf["address_paths"] if p["callee"]]
    callees = sorted({p["callee"] for p in paths})
    lists = sorted({"&" + p["root"] + "".join(s[0] + s[1] for s in p["steps"]) for p in paths})
    shown = lists if len(lists) <= 2 else lists[:2] + [f"... ({len(lists)} lists)"]
    out.append(
        f"| {number} | `{row['file'].split('/')[-1]}` | `{row['function']}` | {len(row['leaves'])} "
        f"| `{', '.join(callees)}` | {', '.join('`' + x + '`' for x in shown)} "
        f"| {'; '.join(CONTRACT[c] for c in callees)} |"
    )
open(sys.argv[2], "w").write("\n".join(out) + "\n")
