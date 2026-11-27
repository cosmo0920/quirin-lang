#!/usr/bin/env python3
"""GAPO-PARITY-DECISIONS-NONDETERMINISTIC: classify the corpus parity rows two
runs disagree on (as multisets, Clang node addresses erased).

Usage: parity_classes.py OUT.json REFERENCE RUN [SUFFIX]

For each row only one side has: its position, and for a `typed-place` row its
`of_position`, function, line, whether the run's own node matched (a row whose
consultation declined has `new_admitted` false) and what its places are (rule
10t `place` decisions, rule 12j `record_object` decisions). The summary counts
rows by side, position and of_position, and lists every `typed-place` row
that differs only by its `places` (a row both sides wrote with different
contents) apart from rows only one side wrote at all.
"""
import collections
import json
import pathlib
import re
import sys

RUNS = pathlib.Path(__file__).resolve().parents[4] / "private/gapo-nd"
ADDRESS = re.compile(r'"0x[0-9a-f]+"')


def load(name, suffix):
    rows = collections.Counter()
    for line in (RUNS / f"{name}{suffix}-parity.jsonl").read_text().splitlines():
        rows[ADDRESS.sub('"0x"', line)] += 1
    return rows


def key(row):
    """A typed-place row's identity without its places."""
    return (row.get("function"), row.get("line"), tuple(row.get("texts") or ()),
            row.get("of_position"), row.get("source_sha256"))


def main():
    out, reference, run = sys.argv[1], sys.argv[2], sys.argv[3]
    suffix = sys.argv[4] if len(sys.argv) > 4 else ""
    b, a = load(reference, suffix), load(run, suffix)
    sides = {"only_reference": b - a, "only_run": a - b}
    summary = {}
    detail = {}
    keyed = {}
    for side, rows in sides.items():
        counts = collections.Counter()
        items = []
        for text, count in rows.items():
            row = json.loads(text)
            position = row.get("position")
            label = (position, row.get("of_position"))
            counts[" / ".join(str(x) for x in label)] += count
            places = row.get("places") or []
            items.append({
                "count": count,
                "position": position,
                "of_position": row.get("of_position"),
                "function": row.get("function"),
                "line": row.get("line"),
                "texts": " ".join(row.get("texts") or []),
                "new_admitted": row.get("new_admitted"),
                "places": [
                    {"rule": p.get("rule", "10t"),
                     "name": (p.get("place") or {}).get("name") or (p.get("record_object") or {}).get("name"),
                     "admitted": p.get("admitted"), "refusal": p.get("refusal")}
                    for p in places
                ],
            })
            if position == "typed-place":
                keyed.setdefault(side, collections.Counter())[key(row)] += count
        summary[side] = dict(counts)
        detail[side] = sorted(items, key=lambda item: (str(item["function"]), item["line"] or 0))
    both = set(keyed.get("only_reference", {})) & set(keyed.get("only_run", {}))
    result = {"reference": reference, "run": run, "suffix": suffix,
              "rows": [sum(b.values()), sum(a.values())], "summary": summary,
              "typed_place_rows_changed_in_content": len(both), "detail": detail}
    pathlib.Path(out).write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k != "detail"}, indent=1))


if __name__ == "__main__":
    main()
