#!/usr/bin/env python3
"""FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: totals.py REPORT NULL_JSONL --
row status totals of a resource report (Safe / Pending / Unknown / Unsafe),
its sha256, and the null property's verdict totals."""
import collections
import hashlib
import json
import pathlib
import sys


def main():
    report, null = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    if report.is_dir():
        report = report / "resource-diagnostics.json"
    data = json.loads(report.read_text())
    rows = collections.Counter(f["status"] for u in data["units"] for f in u.get("functions", []))
    verdicts = collections.Counter()
    for line in null.read_text().splitlines():
        n = json.loads(line)["null"]
        v = n["verdict"]
        if v == "Safe":
            v = "Safe-unconditional" if n.get("scope") == "unconditional" else "Safe-conditional"
        verdicts[v] += 1
    print(json.dumps({"report": str(report), "sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
                      "rows": dict(sorted(rows.items())), "null": dict(sorted(verdicts.items()))}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
