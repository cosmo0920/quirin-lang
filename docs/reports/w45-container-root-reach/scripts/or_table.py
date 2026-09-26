#!/usr/bin/env python3
"""The retry OR table's clause verdicts (`user_count_clauses`) in two slice
reports, side by side, with the rows of the OR functions.

usage: or_table.py BEFORE AFTER OUT.json
"""
import json
import sys

OR_FUNCTIONS = [
    "flb_output_task_singleplex_enqueue", "flb_output_task_flush",
    "flb_output_task_queue_flush_one", "flb_output_task_singleplex_flush_next",
    "flb_output_task_queue_enqueue", "flb_task_retry_destroy", "flb_task_queue_destroy",
]


def rows(path):
    report = json.load(open(path))
    out = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["function"] not in OR_FUNCTIONS:
                continue
            key = f"{unit['file']}::{function['function']}"
            out.setdefault(key, []).append({
                "status": function["status"],
                "leaves": len(function["diagnostics"]),
                "user_count_clauses": function.get("user_count_clauses"),
            })
    return out


def main():
    before, after = rows(sys.argv[1]), rows(sys.argv[2])
    table = {key: {"before": before.get(key), "after": after.get(key),
                   "clauses_identical": [r.get("user_count_clauses") for r in before.get(key, [])]
                   == [r.get("user_count_clauses") for r in after.get(key, [])]}
             for key in sorted(set(before) | set(after))}
    json.dump(table, open(sys.argv[3], "w"), indent=1, sort_keys=True)
    for key, value in table.items():
        b = [(r["status"], r["leaves"]) for r in value["before"] or []]
        a = [(r["status"], r["leaves"]) for r in value["after"] or []]
        print(key, b, "->", a, "clauses identical:", value["clauses_identical"])
        for r in value["after"] or []:
            for clause in r["user_count_clauses"] or []:
                print("    ", json.dumps(clause, sort_keys=True)[:220])
            break


main()
