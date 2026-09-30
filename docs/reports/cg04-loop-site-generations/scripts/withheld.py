#!/usr/bin/env python3
"""CG04-LOOP-SITE-GENERATIONS: the withheld-obligation census (sibling search).

Usage: withheld.py PARITY.jsonl DIAGNOSTICS [--json OUT]

Reads the `position: "withheld-obligation"` rows of a parity stream: one per
exit of a flow run that answered `Safe` although its settled exit state still
holds a possibly-live identity with an unresolved effect that no diagnostic
reported (W4 derivation runs included). Joins them by function name to the
run's reported rows and prints the functions whose *reported* row is Safe:
their Safe rests on an obligation no diagnostic accounts for.
"""

import argparse
import collections
import json
import pathlib


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("diagnostics", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    path = args.diagnostics
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    reported = collections.defaultdict(list)
    for unit in data["units"]:
        for function in unit["functions"]:
            reported[function["function"]].append((unit["file"], function["status"]))
    rows = collections.defaultdict(set)
    with open(args.parity, encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            if row.get("position") != "withheld-obligation":
                continue
            rows[(row["source_sha256"][:12], row["function"])].add(
                (row["line"], tuple(row["acquisition_lines"]))
            )
    record = {"reported_safe": [], "other": []}
    for (source, name), exits in sorted(rows.items()):
        entry = {
            "function": name,
            "source": source,
            "reported": reported.get(name, []),
            "exits": sorted([line, list(acquired)] for line, acquired in exits),
        }
        if any(status == "Safe" for _, status in reported.get(name, [])):
            record["reported_safe"].append(entry)
        else:
            record["other"].append(entry)
    text = json.dumps(record, indent=1, ensure_ascii=False)
    if args.json:
        args.json.write_text(text + "\n", encoding="utf-8")
    for kind in ("reported_safe", "other"):
        print(f"== {kind} {len(record[kind])}")
        for entry in record[kind]:
            print(" ", entry["function"], entry["reported"], entry["exits"][:4])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
