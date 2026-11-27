#!/usr/bin/env python3
"""Summarise one rule-1 census stream (`QCORE_PARENTHESISED_CALLEE_CENSUS`).

Usage: census_summary.py STREAM.jsonl[.gz] [--json OUT]

A missing stream means the sink was never opened: rule 1 rewrote nothing.
Records are deduplicated on (unit_seq, function, line, callee, depth), since an
entry point that tokenizes a unit more than once writes its rewrites again.
"""

import collections
import gzip
import json
import pathlib
import sys


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    records = []
    if path.exists():
        data = path.read_bytes()
        if data[:2] == b"\x1f\x8b":
            data = gzip.decompress(data)
        records = [json.loads(line) for line in data.decode().splitlines() if line]
    distinct = sorted(
        {(r["unit_seq"], r["function"], r["line"], r["callee"], r["depth"]) for r in records}
    )
    summary = {
        "stream": str(path),
        "stream_present": path.exists(),
        "records": len(records),
        "distinct_rewrites": len(distinct),
        "units": len({row[0] for row in distinct}),
        "functions": len({(row[0], row[1]) for row in distinct}),
        "by_callee": dict(collections.Counter(row[3] for row in distinct).most_common()),
        "by_depth": dict(collections.Counter(row[4] for row in distinct)),
    }
    print(json.dumps(summary, indent=1))
    if "--json" in sys.argv[2:]:
        pathlib.Path(sys.argv[sys.argv.index("--json", 2) + 1]).write_text(
            json.dumps(summary, indent=1) + "\n"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
