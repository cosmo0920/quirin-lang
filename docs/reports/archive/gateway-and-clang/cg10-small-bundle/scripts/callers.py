#!/usr/bin/env python3
"""callers.py CENSUS.jsonl REPORT.json -- CG10-SMALL-BUNDLE: the Unknown rows
of one opt-in `current` report that name a function of the rule-74b census
(verdict `read`) in a `call to uncontracted function` leaf; `SOLE` rows carry
no other leaf (an upper bound on the callers rule 74b can free)."""
import collections
import json
import sys


def main():
    census = [json.loads(line) for line in open(sys.argv[1]) if '"unit"' in line]
    read = {row["function"] for row in census if row["verdict"] == "read"}
    wanted = {"call to uncontracted function `%s`" % name: name for name in read}
    report = json.load(open(sys.argv[2]))
    mention = collections.Counter()
    sole = 0
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["status"] != "Unknown":
                continue
            descriptions = [leaf["description"] for leaf in function["diagnostics"]]
            hits = [text for text in descriptions if text in wanted]
            if not hits:
                continue
            for text in set(hits):
                mention[wanted[text]] += 1
            if len(hits) == len(descriptions):
                sole += 1
                print("SOLE", unit["file"], function["function"], sorted({wanted[t] for t in hits}))
    print(json.dumps({"mentioning_callees": dict(mention.most_common()), "sole_rows": sole},
                     sort_keys=True))


if __name__ == "__main__":
    main()
