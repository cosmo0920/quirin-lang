#!/usr/bin/env python3
"""Sum every `test result:` summary in a cargo test log, counting FAILED
summaries as well as ok ones — the defect GATE-SUMMARY-PARSER-OK-ONLY names."""
import re
import sys

path = sys.argv[1]
tot = {"passed": 0, "failed": 0, "ignored": 0, "measured": 0, "filtered": 0}
n = 0
statuses = {}
pat = re.compile(
    r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; "
    r"(\d+) measured; (\d+) filtered out"
)
for line in open(path, encoding="utf-8"):
    m = pat.match(line)
    if not m:
        continue
    n += 1
    statuses[m.group(1)] = statuses.get(m.group(1), 0) + 1
    for i, k in enumerate(["passed", "failed", "ignored", "measured", "filtered"], 2):
        tot[k] += int(m.group(i))
print("summaries:", n, "statuses:", statuses)
print(tot)
