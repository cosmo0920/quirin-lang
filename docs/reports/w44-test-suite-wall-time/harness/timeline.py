#!/usr/bin/env python3
"""Summarize a runner summary.json: longest units, phase ends, exclusivity check.

Usage: timeline.py <summary.json> [--top N]
Checks from the recorded intervals that no exclusive unit overlapped any other
unit, and prints the parallel/exclusive phase ends relative to the first start.
"""

import argparse
import json
import sys


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("summary")
    parser.add_argument("--top", type=int, default=12)
    args = parser.parse_args()
    document = json.load(open(args.summary))
    intervals = document["intervals"]
    t0 = min(i["start"] for i in intervals)
    for i in sorted(intervals, key=lambda i: i["start"] - i["end"])[: args.top]:
        print(f"{i['end'] - i['start']:8.1f}s  start {i['start'] - t0:6.1f}  end {i['end'] - t0:6.1f}"
              f"  {i['uid']}{' [exclusive]' if i['exclusive'] else ''}")
    parallel = [i for i in intervals if not i["exclusive"]]
    exclusive = [i for i in intervals if i["exclusive"]]
    print(f"parallel phase: {len(parallel)} units, ends {max(i['end'] for i in parallel) - t0:.1f}s")
    if exclusive:
        print(f"exclusive phase: {len(exclusive)} units, "
              f"{min(i['start'] for i in exclusive) - t0:.1f}s to {max(i['end'] for i in exclusive) - t0:.1f}s")
    overlaps = [
        (a["uid"], b["uid"]) for a in exclusive for b in intervals
        if a is not b and a["start"] < b["end"] and b["start"] < a["end"]
    ]
    print("exclusive overlaps:", overlaps if overlaps else "none")
    print("ok:", document["ok"], "totals:", document["totals"])
    return 0 if not overlaps else 1


if __name__ == "__main__":
    sys.exit(main())
