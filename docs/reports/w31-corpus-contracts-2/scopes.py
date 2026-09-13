#!/usr/bin/env python3
"""Compare the two runs' per-scope summary lines and their exit lines."""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent
PATTERN = re.compile(r"^[a-z0-9-]+: \d+ unit\(s\), ")


def scope_lines(path: pathlib.Path) -> list:
    return [line for line in path.read_text().splitlines() if PATTERN.match(line)]


before = scope_lines(ROOT / "runs/audit-before.txt")
after = scope_lines(ROOT / "runs/audit-after.txt")
total_before = sum(int(line.split()[1]) for line in before)
total_after = sum(int(line.split()[1]) for line in after)
print(f"BEFORE scopes: {len(before)}  units summing to {total_before}")
for line in before:
    print(f"  {line}")
print(f"AFTER  scopes: {len(after)}  units summing to {total_after}")
for line in after:
    print(f"  {line}")
print(f"per-scope lines IDENTICAL: {before == after}")
for name in ("before", "after"):
    text = (ROOT / f"runs/audit-{name}.txt").read_text()
    exits = [line for line in text.splitlines() if line.startswith("AUDIT_EXIT=")]
    failures = sum(int(m) for m in re.findall(r"(\d+) gateway failure\(s\)", text))
    print(f"{name}: {exits}  total gateway failures = {failures}")
sys.exit(0)
