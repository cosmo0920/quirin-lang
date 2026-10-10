#!/usr/bin/env python3
"""candidates.py ROWS.jsonl OUT_PREFIX -- NULL-SAFETY-PROPERTY: the ranked
NULL-dereference candidate list of a null selection's rows.

Each candidate check is one entry (function, origin, kind, subject, sources);
entries are ranked by the strongest source kind they carry, in the order a
reader is likeliest to find a real defect first:

  allocation > lookup > nullable-result > null-literal > uninitialized >
  uncontracted-result > formal > global > memory-field > other

then by library, file and origin. Writes OUT_PREFIX.json (the full list),
OUT_PREFIX-by-kind.json (counts by kind and by library, and candidate
functions by kind) and OUT_PREFIX-top30.json (the first 30 distinct
(function, subject) pairs of the ranking, for the hand reading)."""
import collections
import json
import sys

ORDER = ["allocation", "lookup", "nullable-result", "null-literal", "uninitialized",
         "uncontracted-result", "formal", "global", "memory-field", "other"]
RANK = {kind: index for index, kind in enumerate(ORDER)}

rows = [json.loads(line) for line in open(sys.argv[1]) if line.strip()]
prefix = sys.argv[2]
entries = []
for row in rows:
    for check in (row.get("null") or {}).get("checks") or []:
        if check["outcome"] != "candidate":
            continue
        kinds = sorted({s.split(":")[0] for s in check.get("sources") or []}, key=lambda k: RANK.get(k, 99))
        entries.append({
            "library": row["library"],
            "file": row["file"],
            "function": row["function"],
            "custody": row.get("custody"),
            "origin": check.get("origin"),
            "line": check["line"],
            "kind": check["kind"],
            "subject": check["subject"],
            "sources": check.get("sources") or [],
            "rank_kind": kinds[0] if kinds else "other",
        })
entries.sort(key=lambda e: (RANK.get(e["rank_kind"], 99), e["library"], e["file"], e["line"]))
by_kind = collections.Counter(e["rank_kind"] for e in entries)
by_library = collections.Counter(e["library"] for e in entries)
functions_by_kind = collections.defaultdict(set)
for e in entries:
    functions_by_kind[e["rank_kind"]].add((e["library"], e["file"], e["function"]))
by_library_kind = collections.defaultdict(collections.Counter)
for e in entries:
    by_library_kind[e["library"]][e["rank_kind"]] += 1
top = []
seen = set()
for e in entries:
    key = (e["file"], e["function"], e["subject"])
    if key in seen:
        continue
    seen.add(key)
    top.append(e)
    if len(top) == 30:
        break
json.dump(entries, open(prefix + ".json", "w"), indent=1)
json.dump({
    "candidates": len(entries),
    "by_strongest_source_kind": {k: by_kind[k] for k in ORDER if by_kind[k]},
    "functions_by_strongest_source_kind": {k: len(functions_by_kind[k]) for k in ORDER if functions_by_kind[k]},
    "by_library": dict(by_library.most_common()),
    "by_library_and_kind": {lib: dict(c.most_common()) for lib, c in sorted(by_library_kind.items())},
}, open(prefix + "-by-kind.json", "w"), indent=1)
json.dump(top, open(prefix + "-top30.json", "w"), indent=1)
print(json.dumps({"candidates": len(entries), "by_kind": dict(by_kind)}, indent=1))
for e in top:
    print(e["rank_kind"], e["origin"], e["function"], e["subject"], e["sources"])
