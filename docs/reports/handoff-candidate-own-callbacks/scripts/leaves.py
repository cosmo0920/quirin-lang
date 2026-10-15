#!/usr/bin/env python3
"""leaves.py REPORT OUT.json -- HANDOFF-CANDIDATE-OWN-CALLBACKS (adapted from OP-CALLBACK-MEMBER-PATH-RELEASE): in one route
report (a run directory or its resource-diagnostics.json), every leaf whose
description names one of the card's eight candidates, the oauthbearer
candidate, or a member-invoking declaration the candidates decide
(`rd_kafka_op_destroy`, the serve and request families), grouped by the
callee the leaf is about and the candidate it names; per function, whether
every leaf is such a leaf (the rows the card alone could decide). Writes
OUT.json and prints a table. A reading aid: the rows are read by hand."""
import collections
import json
import pathlib
import re
import sys

EIGHT = [
    "rd_kafka_admin_worker",
    "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb",
    "rd_kafka_admin_fanout_worker",
    "rd_kafka_DeleteRecords_leaders_queried_cb",
    "rd_kafka_ListOffsets_leaders_queried_cb",
    "rd_kafka_share_commit_async_fanout_op",
    "rd_kafka_share_fetch_fanout_op",
    "rd_kafka_share_commit_sync_fanout_op",
]
OTHER = ["rd_kafka_oauthbearer_refresh_op", "rd_kafka_poll_cb", "rd_kafka_toppar_op_serve"]
INVOKERS = [
    "rd_kafka_op_destroy", "rd_kafka_q_serve0", "rd_kafka_q_serve",
    "rd_kafka_q_serve_maybe_consume", "rd_kafka_q_pop_serve0", "rd_kafka_q_pop_serve",
    "rd_kafka_q_pop_serve_maybe_consume", "rd_kafka_op_req0", "rd_kafka_op_req",
    "rd_kafka_op_req2", "rd_kafka_q_purge0", "rd_kafka_q_wait_result",
]
NAMED = re.compile(r"`([^`]*)`")


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def classify(description):
    """(callee the leaf is about, candidate it names) or None."""
    names = NAMED.findall(description)
    callee = next((n for n in names if n in INVOKERS), None)
    candidate = next((n for n in names if n in EIGHT or n in OTHER), None)
    if callee is None and candidate is None:
        return None
    return callee or "-", candidate or "-"


def main():
    report = load(sys.argv[1])
    groups = collections.Counter()
    functions = collections.defaultdict(set)
    alone = []
    samples = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            diagnostics = function.get("diagnostics") or []
            key = "%s::%s" % (unit["file"], function["function"])
            hits = [classify(d.get("description", "")) for d in diagnostics]
            for diagnostic, hit in zip(diagnostics, hits):
                if hit is None:
                    continue
                groups[hit] += 1
                functions[hit].add(key)
                samples.setdefault(hit, diagnostic.get("description", ""))
            if function["status"] == "Unknown" and diagnostics and all(hits):
                alone.append({"function": key, "leaves": [d.get("description", "") for d in diagnostics]})
    record = {
        "groups": [
            {"callee": c, "candidate": n, "leaves": groups[(c, n)],
             "functions": len(functions[(c, n)]), "sample": samples[(c, n)]}
            for (c, n) in sorted(groups)
        ],
        "alone": alone,
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    for row in record["groups"]:
        print("%-32s %-64s %4d %4d" % (row["callee"], row["candidate"], row["leaves"], row["functions"]))
    print("functions decided only by these leaves:", len(alone))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
