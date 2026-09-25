#!/usr/bin/env python3
"""Tabulate `/usr/bin/time -v` records (card W44-AUDIT-PARALLEL-UNITS).

Usage: timing_table.py TIME_DIR NAME[=LABEL] ...

For each NAME reads TIME_DIR/NAME.time (GNU time -v output) and, when present,
TIME_DIR/NAME.load (an `uptime` line taken just before the run), and prints
JSON rows with wall, user and system seconds, CPU percentage, peak RSS and the
one-minute load average before the run.
"""

import json
import pathlib
import re
import sys


def seconds(text: str) -> float:
    parts = [float(part) for part in text.split(":")]
    total = 0.0
    for part in parts:
        total = total * 60 + part
    return total


def main() -> int:
    directory = pathlib.Path(sys.argv[1])
    rows = []
    for spec in sys.argv[2:]:
        name, _, label = spec.partition("=")
        text = (directory / f"{name}.time").read_text()
        field = lambda key: re.search(rf"^\s*{re.escape(key)}: (.+)$", text, re.M).group(1).strip()
        row = {
            "run": label or name,
            "wall_s": round(seconds(field("Elapsed (wall clock) time (h:mm:ss or m:ss)")), 2),
            "user_s": float(field("User time (seconds)")),
            "system_s": float(field("System time (seconds)")),
            "cpu_percent": field("Percent of CPU this job got"),
            "peak_rss_kib": int(field("Maximum resident set size (kbytes)")),
            "exit_status": int(field("Exit status")),
        }
        load = directory / f"{name}.load"
        if load.exists():
            row["load_1min_before"] = float(load.read_text().split("load average:")[1].split(",")[0])
        rows.append(row)
    print(json.dumps(rows, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
