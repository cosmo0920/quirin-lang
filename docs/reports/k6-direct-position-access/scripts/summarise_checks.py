#!/usr/bin/env python3
"""Card K6: summarise private/k6/PREFIX-*.log test counts and exits.

usage: summarise_checks.py PRIVATE_DIR PREFIX STEP...
"""
import pathlib
import re
import sys

base = pathlib.Path(sys.argv[1])
prefix = sys.argv[2]
for step in sys.argv[3:]:
    log = (base / f"{prefix}-{step}.log").read_text(errors="replace")
    code = (base / f"{prefix}-{step}.exit").read_text().strip()
    results = re.findall(r"^test result: \w+\. (\d+) passed; (\d+) failed", log, re.M)
    passed = sum(int(p) for p, _ in results)
    failed = sum(int(f) for _, f in results)
    print(f"{step}: exit {code}, {len(results)} binaries, {passed} passed, {failed} failed")
