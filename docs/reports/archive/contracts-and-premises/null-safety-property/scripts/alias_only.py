#!/usr/bin/env python3
"""alias_only.py REPORT -- NULL-SAFETY-PROPERTY: the Unknown functions of a
route report (a run directory or its resource-diagnostics.json) whose every
leaf is an `unresolved resource identity` (the population a dereference
reading can free at most), with the seven CG10-INTERIOR-ALIAS-NULLABILITY
functions marked."""
import collections
import json
import pathlib
import sys

NAMED = {
    "rd_kafka_share_ack_type_from_msg_op", "rd_kafka_topic_partition_list_has_duplicates",
    "rd_kafka_topic_partition_list_regex_cnt", "append", "peek", "sink_peek",
    "mk_vhost_fdt_chain_lookup",
}

path = pathlib.Path(sys.argv[1])
if path.is_dir():
    path = path / "resource-diagnostics.json"
report = json.loads(path.read_text())
rows = []
for unit in report["units"]:
    for function in unit["functions"]:
        if function["status"] != "Unknown":
            continue
        leaves = function.get("diagnostics") or []
        if leaves and all(d["description"].startswith("unresolved resource identity") for d in leaves):
            rows.append((unit["file"], function["function"], sorted({d["description"] for d in leaves})))
print("Unknown functions whose every leaf is an unresolved identity:", len(rows))
print("distinct function names:", len({name for _, name, _ in rows}))
print("leaf-description counts:", collections.Counter(len(r[2]) for r in rows))
for row in rows:
    if row[1] in NAMED:
        print("named:", row)
