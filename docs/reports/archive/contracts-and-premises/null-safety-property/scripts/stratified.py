#!/usr/bin/env python3
"""stratified.py CANDIDATES.json OUT.json [PER_KIND] -- NULL-SAFETY-PROPERTY:
a deterministic sample of the ranked candidate list beyond its top 30: for
each of the bulk source kinds (memory-field, formal, uncontracted-result,
null-literal), PER_KIND (default 6) entries evenly spaced through that kind's
part of the ranking, for the hand reading of what the bulk looks like."""
import json
import sys

entries = json.load(open(sys.argv[1]))
per_kind = int(sys.argv[3]) if len(sys.argv) > 3 else 6
sample = []
for kind in ("memory-field", "formal", "uncontracted-result", "null-literal"):
    part = [e for e in entries if e["rank_kind"] == kind]
    if not part:
        continue
    step = max(1, len(part) // per_kind)
    sample.extend(part[i] for i in range(0, len(part), step)[:per_kind])
json.dump(sample, open(sys.argv[2], "w"), indent=1)
for e in sample:
    print(e["rank_kind"], e["origin"], e["function"], e["subject"], e["sources"])
