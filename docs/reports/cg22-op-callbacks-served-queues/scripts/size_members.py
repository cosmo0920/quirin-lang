#!/usr/bin/env python3
"""size_members.py REPORT OUT.json -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES (adapted from CG22-CALLBACK-REGISTRY-CONTRACT, from
CALLEE-RETENTION-CLASS1's size_members.py): for every member the card names,
in one route report (a run directory or its resource-diagnostics.json), the
leaves that name it (``call to uncontracted function `F` ``, or a refusal of
this card or of rules 11r/12t/13b that names `F` as the call), the functions
carrying such a leaf, and the functions it decides alone (every leaf of the
function names it), listed with their units. Writes OUT.json and prints a
table."""
import collections
import json
import pathlib
import re
import sys

MEMBERS = [
    # the ledger entry's members
    "rd_kafka_op_destroy", "rd_kafka_q_serve0", "rd_kafka_q_pop_serve0",
    "rd_kafka_q_serve_maybe_consume", "rd_kafka_q_serve", "rd_kafka_q_pop_serve",
    "rd_kafka_poll_cb",
    # their neighbours: the invoking layer, the registrars and constructors,
    # the served-queue members and the wrappers of the served queues
    "rd_kafka_q_pop_serve_maybe_consume", "rd_kafka_q_pop", "rd_kafka_op_handle",
    "rd_kafka_op_handle_std", "rd_kafka_op_call", "rd_kafka_op_new_cb", "rd_kafka_op_new0",
    "rd_kafka_op_reply", "rd_kafka_op_error_reply", "rd_kafka_q_wait_result", "rd_kafka_q_purge0",
    "rd_kafka_event_destroy",
]
NAMED = re.compile(r"`([^`]*)`")


def named(description):
    """The callee a leaf names, or None."""
    if description.startswith("call to uncontracted function `"):
        return NAMED.search(description).group(1)
    if description.startswith("call to `"):
        return NAMED.search(description).group(1)
    return None


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def main():
    report = load(sys.argv[1])
    leaves = collections.Counter()
    carrying = collections.defaultdict(set)
    alone = collections.defaultdict(list)
    for unit in report["units"]:
        for function in unit["functions"]:
            diagnostics = function.get("diagnostics") or []
            names = [named(d.get("description", "")) for d in diagnostics]
            key = "%s::%s" % (unit["file"], function["function"])
            for name in names:
                if name in MEMBERS:
                    leaves[name] += 1
                    carrying[name].add(key)
            if function["status"] == "Unknown" and diagnostics and len(set(names)) == 1 \
                    and names[0] in MEMBERS:
                alone[names[0]].append(key)
    record = {member: {"leaves": leaves[member], "functions": len(carrying[member]),
                       "alone": len(alone[member]), "alone_functions": sorted(alone[member])}
              for member in MEMBERS}
    pathlib.Path(sys.argv[2]).write_text(json.dumps(record, indent=1) + "\n")
    for member in MEMBERS:
        row = record[member]
        print("%-34s %4d %4d %3d" % (member, row["leaves"], row["functions"], row["alone"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
