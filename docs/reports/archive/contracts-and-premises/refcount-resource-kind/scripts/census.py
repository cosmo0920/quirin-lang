#!/usr/bin/env python3
"""census.py REPORT OUT.json -- card REFCOUNT-RESOURCE-KIND: size the
reference-count members on one route report.

A leaf *names* a member when its description is
``call to uncontracted function `F` `` or a refusal of this card's route
(``reference-…``) naming `F`, or any description quoting `F` in backquotes.
Per member: the leaves naming it, the functions carrying such a leaf, and the
functions it decides **alone** (every leaf of the function names it). For the
member set: the functions whose every leaf names some member (`by_set`),
listed with their leaves, and the Unknown functions carrying a member leaf
beside other leaves (`mixed`, with their other leaf families).
"""
import collections
import json
import re
import sys

MEMBERS = {
    "keep": ["rd_kafka_toppar_keep0", "rd_kafka_q_keep", "rd_kafka_q_keep_nolock",
             "rd_kafka_topic_keep", "rd_kafka_topic_keep_app", "rd_kafka_cgrp_keep"],
    "drop": ["rd_kafka_toppar_destroy0", "rd_kafka_q_destroy0", "rd_kafka_q_destroy_owner",
             "rd_kafka_topic_destroy0", "rd_kafka_topic_destroy_app", "rd_refcnt_sub0",
             "rd_kafka_enq_once_trigger", "rd_kafka_enq_once_del_source",
             "rd_kafka_enq_once_del_source_return", "rd_kafka_toppar_destroy_free"],
    "primitive": ["rd_atomic32_add", "rd_atomic32_sub", "rd_atomic32_set", "rd_atomic32_get",
                  "rd_atomic32_init"],
    "constructor": ["rd_kafka_queue_new0"],
    "destructor": ["rd_kafka_topic_partition_list_destroy", "rd_kafka_topic_partition_destroy0",
                   "rd_kafka_topic_partition_private_destroy",
                   "rd_kafka_topic_partition_result_destroy",
                   "rd_kafka_AlterConsumerGroupOffsets_destroy",
                   "rd_kafka_DeleteConsumerGroupOffsets_destroy",
                   "rd_kafka_ListConsumerGroupOffsets_destroy",
                   "rd_kafka_txn_clear_partitions_flag", "rd_kafka_queue_destroy",
                   "rd_kafka_msgbatch_destroy", "rd_kafka_topic_partition_destroy",
                   "rd_kafka_topic_partition_list_clear"],
}
QUOTED = re.compile(r"`([^`]*)`")


def main() -> int:
    report = json.load(open(sys.argv[1]))
    names = {name: group for group, members in MEMBERS.items() for name in members}
    leaves, carrying, alone = collections.Counter(), collections.Counter(), collections.Counter()
    alone_rows = collections.defaultdict(list)
    by_set, mixed = [], []
    statuses = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            statuses[function["status"]] += 1
            if function["status"] != "Unknown":
                continue
            diagnostics = function.get("diagnostics") or []
            named = []
            for leaf in diagnostics:
                quoted = [q for q in QUOTED.findall(leaf.get("description", "")) if q in names]
                named.append(quoted[0] if quoted else None)
            present = {n for n in named if n}
            if not present:
                continue
            key = "%s::%s" % (unit["file"], function["function"])
            for name in present:
                carrying[name] += 1
            for name in named:
                if name:
                    leaves[name] += 1
            if all(named):
                by_set.append({"function": key, "members": sorted(present),
                               "leaves": [d.get("description") for d in diagnostics]})
                if len(present) == 1:
                    only = next(iter(present))
                    alone[only] += 1
                    alone_rows[only].append(key)
            else:
                others = collections.Counter(
                    QUOTED.sub("`_`", d.get("description", ""))
                    for d, n in zip(diagnostics, named) if not n)
                mixed.append({"function": key, "members": sorted(present),
                              "others": dict(others.most_common(6))})
    record = {
        "report": sys.argv[1],
        "statuses": dict(statuses),
        "members": {name: {"group": names[name], "leaves": leaves[name],
                           "functions": carrying[name], "alone": alone[name],
                           "alone_functions": sorted(alone_rows[name])}
                    for name in sorted(names, key=lambda n: (names[n], n))
                    if carrying[name]},
        "by_set": by_set,
        "mixed": mixed,
    }
    with open(sys.argv[2], "w") as out:
        out.write(json.dumps(record, indent=1) + "\n")
    print("statuses %s" % dict(statuses))
    for name, row in record["members"].items():
        print("  %-12s %-44s leaves %4d functions %4d alone %3d" % (
            row["group"], name, row["leaves"], row["functions"], row["alone"]))
    print("decided by the member set: %d; mixed: %d" % (len(by_set), len(mixed)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
