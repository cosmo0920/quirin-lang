#!/usr/bin/env python3
"""cpu_total.py CPU.jsonl... -- DEEP-AST-TRANSPORTS-AND-WALL-TIME: each run's
own CPU seconds (cpu_poll.py: the greatest tick count seen for every thread,
summed; Clang's processes not counted) and its polled wall span."""
import json
import sys

for path in sys.argv[1:]:
    seen = {}
    first = last = None
    for line in open(path):
        poll = json.loads(line)
        first = poll["unix"] if first is None else first
        last = poll["unix"]
        for tid, ticks in poll["cpu"].items():
            seen[tid] = max(seen.get(tid, 0), ticks)
    print(path, "cpu", round(sum(seen.values()) / 100.0, 1), "s, span", round(last - first, 1), "s")
