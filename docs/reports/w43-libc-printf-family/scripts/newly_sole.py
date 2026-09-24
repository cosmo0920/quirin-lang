#!/usr/bin/env python3
"""Functions whose only blocker is one uncontracted callee, before vs after.

Usage: newly_sole.py BEFORE_ARTIFACTS AFTER_ARTIFACTS OUT.json

Same definition as `w43-return-expression/scripts/sole_callees.py`: an Unknown
function whose diagnostics carry exactly one distinct description, and that
description is `call to uncontracted function `NAME``. Lists the functions that
enter and leave that set, with the callee.
"""

import gzip
import json
import pathlib
import re
import sys


def sole(directory):
    data = json.load(gzip.open(pathlib.Path(directory) / "diagnostics.json.gz"))
    found = {}
    for unit in data["units"]:
        for entry in unit["functions"]:
            if entry.get("status") != "Unknown":
                continue
            reasons = {d["description"] for d in entry.get("diagnostics") or []}
            if len(reasons) != 1:
                continue
            match = re.fullmatch(r"call to uncontracted function `([^`]+)`", next(iter(reasons)))
            if match:
                found[f"{unit['library']}/{unit['file']}::{entry['function']}"] = match.group(1)
    return found


def main() -> int:
    before, after = sole(sys.argv[1]), sole(sys.argv[2])
    record = {
        "before": len(before),
        "after": len(after),
        "left": {k: before[k] for k in sorted(set(before) - set(after))},
        "entered": {k: after[k] for k in sorted(set(after) - set(before))},
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps(record, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
