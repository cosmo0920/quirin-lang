#!/usr/bin/env python3
"""Compare the executed test sets and outcomes of two test runs, per binary.

Usage: compare.py <base> <other> [--expect-added KEY::LINE]...

Each side is either a timestamped plain `cargo test` log (output.log, see
ts.py) or a runner `summary.json` (scripts/run_test_binaries.py). A binary is
keyed by its executable stem (hash stripped); every `Doc-tests` section counts
as the one `doc-tests` unit. The comparison is of sorted
`test <name> ... <outcome>` lines per key, then of the summed summaries
(passed/failed/ignored/measured/filtered and the number of summaries).

`--expect-added` declares a test line that the other side is expected to have
in addition (a test the change itself adds, which must pass). Each declared
line must be present in `other`, absent from `base`, and `ok`; it is then
removed from `other` (lines and its binary's passed count) before comparing.
Anything else that differs is a difference. Exit 0 only if nothing differs.
"""

import argparse
import json
from pathlib import Path
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


def counts(entry: dict) -> dict:
    return {k: sum(int(s[k]) for s in entry["summaries"]) for k in COUNT_KEYS}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("base", type=Path)
    parser.add_argument("other", type=Path)
    parser.add_argument("--expect-added", action="append", default=[])
    args = parser.parse_args()
    base, other = load(args.base), load(args.other)
    problems = []
    removed = {}
    for declared in args.expect_added:
        key, line = declared.split("::", 1)
        if not line.endswith(" ... ok"):
            problems.append(f"declared addition is not a passing test line: {declared}")
        elif key not in other or line not in other[key]["tests"]:
            problems.append(f"declared addition missing from other: {declared}")
        elif key in base and line in base[key]["tests"]:
            problems.append(f"declared addition already in base: {declared}")
        else:
            other[key]["tests"].remove(line)
            removed[key] = removed.get(key, 0) + 1
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
        sa, sb = counts(base[key]), counts(other[key])
        sb["passed"] -= removed.get(key, 0)
        if sa != sb or len(base[key]["summaries"]) != len(other[key]["summaries"]):
            problems.append(f"{key}: summaries differ {sa} vs {sb}")
    lines_a = sum(len(entry["tests"]) for entry in base.values())
    lines_b = sum(len(entry["tests"]) for entry in other.values())
    total_a = {k: sum(counts(e)[k] for e in base.values()) for k in COUNT_KEYS}
    total_b = {k: sum(counts(e)[k] for e in other.values()) for k in COUNT_KEYS}
    total_b["passed"] -= sum(removed.values())
    total_a["summaries"] = sum(len(e["summaries"]) for e in base.values())
    total_b["summaries"] = sum(len(e["summaries"]) for e in other.values())
    print(f"base : {len(base)} keys, {lines_a} test lines, totals {total_a}")
    print(f"other: {len(other)} keys, {lines_b} test lines, totals {total_b} "
          f"(after removing {sum(removed.values())} declared additions)")
    if total_a != total_b:
        problems.append(f"totals differ: {total_a} vs {total_b}")
    for problem in problems:
        print("DIFF", problem)
    if problems:
        print(f"{len(problems)} differences")
    elif removed:
        print(f"IDENTICAL apart from {sum(removed.values())} declared, passing additions")
    else:
        print("IDENTICAL")
    return 0 if not problems else 1


if __name__ == "__main__":
    sys.exit(main())
