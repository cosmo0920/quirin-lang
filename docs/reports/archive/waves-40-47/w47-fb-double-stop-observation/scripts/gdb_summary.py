#!/usr/bin/env python3
"""Summarize a run_gdb.sh directory: per scenario, each pthread_join's thread
argument with its caller (frame #1, function and file:line), the harness's
`worker=` readings, and the inferior's outcome.

Usage: gdb_summary.py RUN_DIR OUT_JSON  (prints the summary)
"""

import hashlib
import json
import pathlib
import re
import sys


def main():
    run_dir = pathlib.Path(sys.argv[1])
    result = {}
    for log in sorted(run_dir.glob("gdb-*.log")):
        text = log.read_text(errors="replace")
        joins = []
        for m in re.finditer(r"GDBJOIN thread=(\S+)\n#0 [^\n]*\n#1 +0x[0-9a-f]+ in (\w+) \([^\n]*\) at (\S+)", text):
            joins.append({"thread": m.group(1), "caller": m.group(2),
                          "at": re.sub(r"^.*/attempt-1/(src|harness)-[^/]+/", "", m.group(3))})
        outcome = re.findall(r"\[Inferior 1 \(process \d+\) (exited [^\]]+)\]|Program received signal (\w+)", text)
        result[log.stem.removeprefix("gdb-")] = {
            "gdb_exit": int((run_dir / f"{log.stem}.exit").read_text().strip()),
            "joins": joins,
            "harness_worker": re.findall(r"HARNESS (before-\S+) status=\S+ worker=(\S+)", text),
            "inferior": [a or b for a, b in outcome],
            "log_sha256": hashlib.sha256(log.read_bytes()).hexdigest(),
        }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(result, indent=1) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=1))


if __name__ == "__main__":
    main()
