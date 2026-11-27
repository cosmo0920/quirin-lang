#!/usr/bin/env python3
"""Attribute every `test result:` summary to the `Running`/`Doc-tests` section
it follows, so a section that prints more than one summary is visible."""
import re
import sys

path = sys.argv[1]
sections = []
cur = None
for line in open(path, encoding="utf-8"):
    s = line.strip()
    if s.startswith("Running ") or s.startswith("Doc-tests "):
        cur = [s, 0]
        sections.append(cur)
    elif s.startswith("test result:") and cur is not None:
        cur[1] += 1
run = sum(1 for s in sections if s[0].startswith("Running "))
doc = sum(1 for s in sections if s[0].startswith("Doc-tests "))
print(f"sections: {len(sections)} (Running {run}, Doc-tests {doc})")
print(f"summaries attributed: {sum(s[1] for s in sections)}")
for name, count in sections:
    if count != 1:
        print(f"  {count} summaries: {name}")
