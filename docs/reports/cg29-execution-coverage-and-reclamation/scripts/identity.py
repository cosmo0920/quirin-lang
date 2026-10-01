#!/usr/bin/env python3
"""CG28-WEAK-MEMORY-EVENTS: status totals and sha256 of one run's report.

usage: identity.py REPORT_JSON [REPORT_JSON ...]
Prints, per report: sha256, Safe / Unsafe / Unknown / Pending counts.
"""

import collections
import hashlib
import json
import sys

for path in sys.argv[1:]:
    raw = open(path, "rb").read()
    data = json.loads(raw)
    counts = collections.Counter(
        function["status"] for unit in data["units"] for function in unit["functions"]
    )
    print(
        hashlib.sha256(raw).hexdigest(),
        " / ".join(f"{counts.get(s, 0)} {s}" for s in ("Safe", "Unsafe", "Unknown", "Pending")),
        path,
    )
