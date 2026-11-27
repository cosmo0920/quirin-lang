#!/usr/bin/env python3
"""B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: the escape census.

Usage: escape.py PARITY.jsonl DIAGNOSTICS [--json OUT]

Reads the `position: "field-link-escape"` rows of a parity stream: one per
escape of an owner that reaches, through a B08 field link, a child that may be
live and either carries an unresolved effect already (on the base: every child
linked on one side of a join) or is not carried on every path (`partial`, the
repaired flow), or has a mixed status. Rows come from every iterate and every
flow run (W4 derivation runs included), so they over-approximate the settled
states. Joins them by function name to the run's reported rows and prints the
functions whose reported row is Safe: their Safe may rest on an escape the
repair refuses.
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
            if row.get("position") != "field-link-escape":
                continue
            rows[(row["source_sha256"][:12], row["function"])].add(
                (
                    row["line"],
                    row["child"],
                    row["status"],
                    row.get("partial"),
                    row["unresolved"],
                    row["mixed"],
                )
            )
    record = {"reported_safe": [], "other": []}
    for (source, name), escapes in sorted(rows.items()):
        entry = {
            "function": name,
            "source": source,
            "reported": reported.get(name, []),
            "escapes": sorted(list(escape) for escape in escapes),
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
            print(" ", entry["function"], entry["reported"], entry["escapes"][:4])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
