#!/usr/bin/env python3
"""CG15-STORAGE-DURATION-AND-VOLATILE: summarise the card's parity rows of one
run (`QCORE_TYPED_PATH_PARITY`).

usage: parity_summary.py PARITY_JSONL OUT_JSON

Counts, per position (`storage-duration-*`, `volatile-object`,
`volatile-condition`, `volatile-trace`), the answers; for the trace rows, the
access count, how many statements hold an unsequenced pair on one object, and
the functions; for the object and condition rows, the functions each answer
reached. Parity rows repeat when a body is rebuilt (W4 derivation), so the
function sets are the measure; the row counts are not.
"""

import collections
import json
import sys

POSITIONS = ("storage-duration-", "volatile-")


def main():
    answers = collections.defaultdict(collections.Counter)
    functions = collections.defaultdict(set)
    trace = {"statements": 0, "accesses": 0, "unsequenced_conflicts": 0, "functions": set()}
    with open(sys.argv[1]) as handle:
        for line in handle:
            if '"storage-duration-' not in line and '"volatile-' not in line:
                continue
            row = json.loads(line)
            position = row.get("position", "")
            if not position.startswith(POSITIONS):
                continue
            function = row.get("function")
            if position == "volatile-trace":
                trace["statements"] += 1
                trace["accesses"] += row.get("count", 0)
                trace["unsequenced_conflicts"] += row.get("unsequenced_conflict") is not None
                trace["functions"].add(function)
                continue
            answer = row.get("answer")
            answers[position][answer] += 1
            functions[f"{position} {answer}"].add(function)
    record = {
        "schema": "qcore.cg15-parity-summary.v1",
        "rows": {position: dict(counter) for position, counter in sorted(answers.items())},
        "functions": {key: sorted(value) for key, value in sorted(functions.items())},
        "function_counts": {key: len(value) for key, value in sorted(functions.items())},
        "trace": {**trace, "functions": sorted(trace["functions"]),
                  "function_count": len(trace["functions"])},
    }
    with open(sys.argv[2], "w") as handle:
        json.dump(record, handle, indent=1, sort_keys=True)
        handle.write("\n")
    print(json.dumps({"rows": record["rows"], "function_counts": record["function_counts"],
                      "trace": {k: v for k, v in record["trace"].items() if k != "functions"}},
                     indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
