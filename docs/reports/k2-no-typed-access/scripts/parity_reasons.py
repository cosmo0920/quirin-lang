#!/usr/bin/env python3
"""parity_reasons.py PARITY_JSONL OUT_JSON FUNCTION... -- K2: the distinct
(line, position, older refusal, reader refusal, tokens) answers the parity
stream (QCORE_TYPED_PATH_PARITY) recorded for the named functions, with counts.
Rows with neither refusal are dropped."""
import collections
import json
import sys

parity, out, *functions = sys.argv[1:]
wanted = set(functions)
answers = collections.Counter()
with open(parity) as handle:
    for line in handle:
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue  # a stream cut by a stopped probe
        if row.get("function") not in wanted or not (row.get("old_refusal") or row.get("new_refusal")):
            continue
        texts = row.get("texts")
        answers[json.dumps([row["function"], row.get("line"), row.get("position"), row.get("old_refusal"),
                            row.get("new_refusal"), " ".join(texts) if isinstance(texts, list) else None])] += 1
rows = [json.loads(key) + [count] for key, count in sorted(answers.items())]
json.dump(rows, open(out, "w"), indent=1)
for row in rows:
    print(row)
