#!/usr/bin/env python3
"""predict.py RECENSUS_JSON PP_CLASSES_ROWS OUT_JSON -- K1: the rows rule 75 could free.

A K1 leaf is a rule-75 candidate when its analysed statement (pp_classify.py) holds an
address of an arrow member over a parenthesised name (`&(p)->f`). A row is a rule-75
ceiling row when every item it carries is such a leaf (no other blocker, no other K1
leaf). Rows touched: any rule-75 candidate leaf. Upper bounds: the rule also needs the
root bound by no other event and every other premise of the position to hold.
"""
import collections
import gzip
import json
import sys

K1 = {"expr:statement:call-arg", "expr:cond:call-arg", "expr:return:call-arg"}


def main():
    recensus, pp_rows, out = sys.argv[1:4]
    cls = {}
    for line in open(pp_rows):
        r = json.loads(line)
        cls[(r["file"], r["function"], r["line"])] = r["class"]
    rows = []
    with gzip.open(recensus + ".rows.jsonl.gz", "rt") as handle:
        for line in handle:
            rows.append(json.loads(line))
    touched, freed = [], []
    candidate_leaves = 0
    for r in rows:
        if r["status"] != "Unknown" or not r["items"]:
            continue
        marks = []
        for _, tags, line in r["items"]:
            k1 = bool(set(tags) & K1)
            c = cls.get((r["file"], r["function"], line)) if k1 else None
            marks.append(c == "address-paren-root")
        if any(marks):
            touched.append(r)
            candidate_leaves += sum(marks)
            if all(marks):
                freed.append(r)
    by_lib = collections.Counter(r.get("library") for r in freed)
    summary = {
        "candidate_leaves_on_unknown_rows": candidate_leaves,
        "rows_touched": len(touched),
        "rows_freed_ceiling": len(freed),
        "freed_by_library": dict(by_lib),
        "freed": sorted(f"{r.get('library')}::{r['file']}::{r['function']}" for r in freed),
    }
    open(out, "w").write(json.dumps(summary, indent=1) + "\n")
    print(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
