#!/usr/bin/env python3
"""null_compare.py BEFORE.jsonl AFTER.jsonl [OUT.json] -- FB-TEARDOWN-DESTROY-DECLARATIONS
(adapted from RELATIONAL-REWRITE-LINE-ORDER's null_compare.py): compare two
null-property streams (QCORE_NULL_PROPERTY JSON lines; the tallies are the
`.summary.json` beside each): the rows that differ, the fields that differ,
and each row's verdict before and after."""
import json
import pathlib
import sys


def rows(path):
    out = {}
    for line in pathlib.Path(path).read_text().splitlines():
        row = json.loads(line)
        # The run identity (binary, contract digests) differs on every row of
        # two runs; the property's content is the rest.
        row.pop("identity", None)
        out.setdefault((row.get("library"), row.get("file"), row.get("function")), []).append(row)
    return out


def summary(path):
    return json.loads(pathlib.Path(str(path).replace(".jsonl", ".summary.json")).read_text())["null"]


def main():
    before, after = sys.argv[1], sys.argv[2]
    x, y = rows(before), rows(after)
    diff = sorted(k for k in set(x) | set(y) if x.get(k) != y.get(k))
    sx, sy = summary(before), summary(after)
    print(f"{len(x)} -> {len(y)} rows, {len(diff)} differ; tallies {sx} -> {sy}")
    moved = []
    for key in diff:
        xs, ys = x.get(key, [{}]), y.get(key, [{}])
        fields = sorted({f for a, b in zip(xs, ys) for f in set(a) | set(b) if a.get(f) != b.get(f)})
        verdicts = ([r.get("null", {}).get("verdict") for r in xs],
                    [r.get("null", {}).get("verdict") for r in ys])
        moved.append({"row": list(key), "fields": fields, "verdicts": verdicts})
        print("  ", key, fields, verdicts)
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(json.dumps(
            {"before": sx, "after": sy, "differing_rows": moved}, indent=1) + "\n")


if __name__ == "__main__":
    main()
