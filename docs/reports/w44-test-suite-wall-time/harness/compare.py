#!/usr/bin/env python3
"""Compare the executed test sets and outcomes of two test runs, per binary.

Each side is either a timestamped plain `cargo test` log (output.log, see
ts.py) or a runner `summary.json` (scripts/run_test_binaries.py). A binary is
keyed by its executable stem (hash stripped); every `Doc-tests` section counts
as the one `doc-tests` unit. The comparison is of sorted `test <name> ... <outcome>`
lines per key, then of the summed summaries (passed/failed/ignored/measured/
filtered and the number of summaries). Exit 0 only if both are identical.
"""

import argparse
import json
from pathlib import Path
import re
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cargo_log  # noqa: E402

COUNT_KEYS = ("passed", "failed", "ignored", "measured", "filtered")


def load(path: Path) -> dict:
    if path.suffix == ".json":
        document = json.loads(path.read_text())
        merged = {}
        for unit in document["units"]:
            key = "doc-tests" if unit["uid"] == "doc-tests" else cargo_log.stem(unit["executable"])
            entry = merged.setdefault(key, {"summaries": [], "tests": []})
            entry["summaries"] += unit["summaries"]
            entry["tests"] = sorted(entry["tests"] + unit["tests"])
        return merged
    records, _, _ = cargo_log.split(path)
    return cargo_log.summarize(records)


def summed(side: dict) -> dict:
    total = {key: 0 for key in COUNT_KEYS}
    total["summaries"] = 0
    for entry in side.values():
        for summary in entry["summaries"]:
            total["summaries"] += 1
            for key in COUNT_KEYS:
                total[key] += int(summary[key])
    return total


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("base", type=Path)
    parser.add_argument("other", type=Path)
    args = parser.parse_args()
    base, other = load(args.base), load(args.other)
    problems = []
    for key in sorted(set(base) | set(other)):
        if key not in other:
            problems.append(f"missing from other: {key}")
            continue
        if key not in base:
            problems.append(f"missing from base: {key}")
            continue
        a, b = base[key]["tests"], other[key]["tests"]
        if a != b:
            only_a = sorted(set(a) - set(b))
            only_b = sorted(set(b) - set(a))
            problems.append(f"{key}: test lines differ; base-only {only_a[:5]} other-only {only_b[:5]}")
        sa = {k: sum(int(s[k]) for s in base[key]["summaries"]) for k in COUNT_KEYS}
        sb = {k: sum(int(s[k]) for s in other[key]["summaries"]) for k in COUNT_KEYS}
        if sa != sb or len(base[key]["summaries"]) != len(other[key]["summaries"]):
            problems.append(f"{key}: summaries differ {sa} vs {sb}")
    total_a, total_b = summed(base), summed(other)
    lines_a = sum(len(entry["tests"]) for entry in base.values())
    lines_b = sum(len(entry["tests"]) for entry in other.values())
    print(f"base : {len(base)} keys, {lines_a} test lines, totals {total_a}")
    print(f"other: {len(other)} keys, {lines_b} test lines, totals {total_b}")
    if total_a != total_b:
        problems.append(f"totals differ: {total_a} vs {total_b}")
    for problem in problems:
        print("DIFF", problem)
    print("IDENTICAL" if not problems else f"{len(problems)} differences")
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
