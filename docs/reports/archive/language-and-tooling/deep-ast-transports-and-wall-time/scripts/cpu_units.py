#!/usr/bin/env python3
"""cpu_units.py BEFORE-UNITS BEFORE-CPU AFTER-UNITS AFTER-CPU [OUT.json] --
DEEP-AST-TRANSPORTS-AND-WALL-TIME: each unit analysis's own CPU seconds (its
thread's utime + stime between the polls nearest its start and end,
cpu_poll.py, 100 ticks a second) on both sides; the sums with and without
rdhttp.c and the median AFTER/BEFORE ratio of the units using more than 5 CPU
seconds."""
import bisect
import json
import pathlib
import statistics
import sys

TICKS = 100.0


def cpu_seconds(units_path, cpu_path):
    # Per thread, its (time, ticks) series as polled while it existed.
    series = {}
    for line in open(cpu_path):
        poll = json.loads(line)
        for tid, ticks in poll["cpu"].items():
            series.setdefault(tid, ([], []))
            series[tid][0].append(poll["unix"])
            series[tid][1].append(ticks)

    def before(thread, when):
        # The last poll at or before `when` (0 before the thread's first).
        times, ticks = series.get(thread, ([], []))
        i = bisect.bisect_right(times, when) - 1
        return ticks[i] if i >= 0 else 0

    def after(thread, when):
        # The first poll at or after `when`, else the thread's last poll.
        times, ticks = series.get(thread, ([], []))
        if not times:
            return 0
        i = bisect.bisect_left(times, when)
        return ticks[min(i, len(ticks) - 1)]

    result = {}
    for line in open(units_path):
        unit = json.loads(line)
        name = unit["file"].split("/lib/")[-1]
        result[name] = (after(unit["thread"], unit["end"]) - before(unit["thread"], unit["start"])) / TICKS
    return result


def main():
    before = cpu_seconds(sys.argv[1], sys.argv[2])
    after = cpu_seconds(sys.argv[3], sys.argv[4])
    rd = next(k for k in before if k.endswith("/rdhttp.c"))
    ratios = sorted((after[k] / before[k], k) for k in before if k != rd and before[k] > 5 and k in after)
    summary = {
        "before_sum": round(sum(before.values()), 1),
        "after_sum": round(sum(after.values()), 1),
        "before_without_rdhttp": round(sum(v for k, v in before.items() if k != rd), 1),
        "after_without_rdhttp": round(sum(v for k, v in after.items() if k != rd), 1),
        "rdhttp": [round(before[rd], 1), round(after[rd], 1)],
        "median_ratio_other_units_over_5s": round(statistics.median(r for r, _ in ratios), 3),
        "units_over_5s": len(ratios),
        "slowest": [(k, round(before[k], 1), round(after[k], 1)) for _, k in ratios[::-1][:8]],
        "fastest": [(k, round(before[k], 1), round(after[k], 1)) for _, k in ratios[:5]],
    }
    text = json.dumps(summary, indent=1)
    if len(sys.argv) > 5:
        pathlib.Path(sys.argv[5]).write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
