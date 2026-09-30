#!/usr/bin/env python3
"""B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: status totals and digest of run reports.

Usage: totals.py RUN [RUN ...]   (a run directory or a resource-diagnostics.json)

Prints the report's sha256 prefix, the row totals, and the totals keyed by
(unit file, function) as CG04-LOOP-SITE-GENERATIONS' compare.py keys them (the
system and slice figures the ledger quotes).
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
        rows = collections.Counter()
        keyed = {}
        for unit in data["units"]:
            for function in unit["functions"]:
                rows[function["status"]] += 1
                keyed[(unit["file"], function["function"])] = function["status"]
        deduplicated = collections.Counter(keyed.values())
        print(
            hashlib.sha256(raw).hexdigest()[:8],
            "rows",
            dict(sorted(rows.items())),
            "keyed",
            dict(sorted(deduplicated.items())),
            argument,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
