#!/usr/bin/env python3
"""CG08-CASTS-AND-TYPE-QUERIES: the elaborator's CG08 refusals in one run's
parity stream, de-duplicated by (function, line, position, refusal).

usage: cast_refusals.py PARITY_JSONL [--json OUT]
"""
import argparse
import collections
import json
import pathlib


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    rows = set()
    with args.parity.open() as stream:
        for line in stream:
            if '"elaboration-' not in line or "CG08" not in line:
                continue
            row = json.loads(line)
            refusal = str(row.get("new_refusal"))
            if "CG08" not in refusal:
                continue
            rows.add((row["function"], row["line"], row["position"], refusal))
    tally = collections.Counter((position, refusal) for _, _, position, refusal in rows)
    functions = collections.defaultdict(set)
    for function, _, position, refusal in rows:
        functions[(position, refusal)].add(function)
    record = [
        {"position": p, "refusal": r, "consultations": n, "functions": len(functions[(p, r)])}
        for (p, r), n in tally.most_common()
    ]
    print(f"{len(rows)} consultations over {len({f for f, *_ in rows})} functions")
    for entry in record:
        print(f"  {entry['consultations']:5} / {entry['functions']:4}  {entry['position']}: {entry['refusal']}")
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
