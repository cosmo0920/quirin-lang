#!/usr/bin/env python3
"""log0.py PARITY... -- the `rd_kafka_log0` value-connective statements (rule
10z/11m rows), de-duplicated per unit (source digest, function, line, tokens),
by refusal, for each run."""
import collections
import json
import sys

for path in sys.argv[1:]:
    seen = {}
    for line in open(path, encoding="utf-8"):
        if '"value-connective-statement"' not in line or '"rd_kafka_log0"' not in line:
            continue
        row = json.loads(line)
        if row.get("position") != "value-connective-statement" or row["texts"][:1] != ["rd_kafka_log0"]:
            continue
        seen.setdefault((row["source_sha256"], row["function"], row["line"], tuple(row["texts"])), row)
    counts = collections.Counter((row.get("new_refusal") or "admitted")[:60] for row in seen.values())
    print(json.dumps({"run": path, "statements": len(seen), "by_refusal": dict(counts.most_common(5))}, indent=1))
