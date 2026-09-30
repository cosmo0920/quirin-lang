#!/usr/bin/env python3
"""CG04-LOOP-SITE-GENERATIONS: status totals and digest of run reports.

Usage: totals.py RUN [RUN ...]   (a run directory or a resource-diagnostics.json)
"""

import collections
import hashlib
import json
import pathlib
import sys


def main() -> int:
    for argument in sys.argv[1:]:
        path = pathlib.Path(argument)
        if path.is_dir():
            path = path / "resource-diagnostics.json"
        raw = path.read_bytes()
        data = json.loads(raw)
        counts = collections.Counter(
            function["status"] for unit in data["units"] for function in unit["functions"]
        )
        print(hashlib.sha256(raw).hexdigest()[:8], dict(sorted(counts.items())), argument)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
