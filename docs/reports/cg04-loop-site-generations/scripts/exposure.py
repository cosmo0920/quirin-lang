#!/usr/bin/env python3
"""CG04-LOOP-SITE-GENERATIONS: the re-acquisition exposure census.

Usage: exposure.py PARITY.jsonl DIAGNOSTICS [--json OUT]

Reads the `position: "loop-site-generation"` rows of a parity stream (one per
acquisition node an earlier generation of its own site reaches, from the
settled states of every flow run, W4 derivation runs included) and joins them
by function name to the run's reported rows (resource-diagnostics.json, a run
directory or the file). A function is *exposed* when some row says an earlier
generation may be live and carries no unresolved effect. Prints, per reported
status, the exposed functions with their acquisition lines, and the rows of
functions that are not reported rows (callee bodies W4 analysed).
"""

import argparse
import collections
import json
import pathlib


def rows(path):
    with open(path, encoding="utf-8") as stream:
        for line in stream:
            row = json.loads(line)
            if row.get("position") == "loop-site-generation":
                yield row


def reported(path: pathlib.Path):
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    functions = collections.defaultdict(list)
    for unit in data["units"]:
        for function in unit["functions"]:
            functions[function["function"]].append((unit["file"], function["status"]))
    return functions


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("diagnostics", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    functions = reported(args.diagnostics)
    by_function = collections.defaultdict(set)
    for row in rows(args.parity):
        live = row.get("live") and not row.get("unresolved", False)
        key = (row["source_sha256"][:12], row["function"])
        by_function[key].add((
            row["line"], row["site"], row["status"], bool(live),
            bool(row.get("referenced")), bool(row.get("held", False)),
            row.get("step", "-"), bool(row.get("unresolved", False)),
        ))
    record = {"exposed": [], "not_exposed": [], "unreported": []}
    for (source, name), facts in sorted(by_function.items()):
        entry = {
            "function": name,
            "source": source,
            "reported": functions.get(name, []),
            "acquisitions": [
                {"line": f[0], "site": f[1], "status": f[2], "live": f[3],
                 "referenced": f[4], "held": f[5], "step": f[6], "unresolved": f[7]}
                for f in sorted(facts)
            ],
        }
        if not functions.get(name):
            record["unreported"].append(entry)
        elif any(f[3] for f in facts):
            record["exposed"].append(entry)
        else:
            record["not_exposed"].append(entry)
    counts = collections.Counter()
    for entry in record["exposed"]:
        for _, status in entry["reported"]:
            counts[status] += 1
    record["exposed_by_status"] = dict(counts)
    text = json.dumps(record, indent=1, ensure_ascii=False)
    print(text)
    if args.json:
        args.json.write_text(text + "\n", encoding="utf-8")
    for kind in ("exposed", "not_exposed", "unreported"):
        print(f"== {kind} {len(record[kind])}")
        for entry in record[kind]:
            facts = [(a["line"], a["status"], a["live"], a["referenced"], a["unresolved"], a["step"])
                     for a in entry["acquisitions"]]
            print(" ", entry["function"], entry["reported"], facts)
    print("exposed by status", record["exposed_by_status"])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
