#!/usr/bin/env python3
"""null_compare.py BEFORE AFTER [BEFORE AFTER ...] -- OP-CONSTRUCTOR-DECLARATIONS:
compare the null-property rows (JSON lines under private/opctor/, NAME-null.jsonl)
of each BEFORE/AFTER pair: the rows that differ, with the fields that differ,
and whether the null tallies agree. A reading aid for the report."""
import json
import pathlib
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/opctor"


def rows(name):
    out = {}
    for line in (PRIVATE / f"{name}.jsonl").read_text().splitlines():
        row = json.loads(line)
        out.setdefault((row.get("library"), row.get("file"), row.get("function")), []).append(row)
    return out


def main():
    pairs = sys.argv[1:]
    for before, after in zip(pairs[::2], pairs[1::2]):
        x, y = rows(before), rows(after)
        diff = sorted(k for k in set(x) | set(y) if x.get(k) != y.get(k))
        sx = json.loads((PRIVATE / f"{before}.summary.json").read_text())["null"]
        sy = json.loads((PRIVATE / f"{after}.summary.json").read_text())["null"]
        print(f"{before} -> {after}: {len(x)} rows, {len(diff)} differ; tallies equal: {sx == sy} {sy}")
        for key in diff:
            xs, ys = x.get(key, [{}])[0], y.get(key, [{}])[0]
            fields = sorted(f for f in set(xs) | set(ys) if xs.get(f) != ys.get(f))
            print("  ", key, fields,
                  {f: (xs.get(f), ys.get(f)) for f in ("custody", "combined") if f in fields})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
