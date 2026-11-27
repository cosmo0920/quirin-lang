#!/usr/bin/env python3
"""Compare every configuration row's status and diagnostics, retaining duplicates."""

import hashlib
import json
import sys
from pathlib import Path


def projection(path: Path) -> list:
    data = json.loads(path.read_text())
    return [
        {
            "file": unit["file"],
            "functions": [
                (f["function"], f["status"], f.get("diagnostics", []))
                for f in unit["functions"]
            ],
        }
        for unit in data["units"]
    ]


def digest(rows: list) -> str:
    return hashlib.sha256(
        json.dumps(rows, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()


def main() -> int:
    before, after, output = map(Path, sys.argv[1:4])
    base = projection(before)
    tip = projection(after)
    record = {
        "before": str(before),
        "after": str(after),
        "units": [len(base), len(tip)],
        "configuration_rows": [sum(len(u["functions"]) for u in base),
                               sum(len(u["functions"]) for u in tip)],
        "row_projection_sha256": [digest(base), digest(tip)],
        "identical": base == tip,
    }
    output.write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps(record, indent=1))
    return 0 if record["identical"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
