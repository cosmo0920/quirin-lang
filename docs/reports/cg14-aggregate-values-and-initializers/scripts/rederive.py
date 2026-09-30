#!/usr/bin/env python3
"""rederive.py CG00_FUNCTIONS_JSON RUN -- card CG14-AGGREGATE-VALUES-AND-INITIALIZERS:
re-derive CG00's frozen function lists for CG14 on this base.

CG00 measured on `ae17152b`. For CG14 it lists the functions freed alone
(strict and with an alias leaf) and those freed together with exactly one
other card. Cards landed since then may have removed that other card's
blockers. Prints, for every listed function, its status and diagnostic leaves
in RUN.
"""

import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from compare import load  # noqa: E402

def main() -> int:
    cg00 = json.loads(pathlib.Path(sys.argv[1]).read_text())
    _, run = load(pathlib.Path(sys.argv[2]))
    by_name = {}
    for (unit, function), value in run.items():
        by_name.setdefault(f"{pathlib.Path(unit).name}::{function}", []).append(value)
    record = {"alone": [], "with_one_other": []}
    alone = set(cg00["freed_alone"]["with_alias"].get("CG14", []))
    for name in sorted(alone):
        record["alone"].append(row(name, by_name))
    for name in cg00["freed_with_one_other_card"].get("CG14", []):
        record["with_one_other"].append(row(name, by_name))
    print(json.dumps(record, indent=1))
    return 0


def row(name, by_name):
    found = by_name.get(name, [])
    return {
        "function": name,
        "status": [status for status, _ in found],
        "leaves": [
            {f"{category}: {description}": count for (category, description), count in leaves.items()}
            for _, leaves in found
        ],
    }


if __name__ == "__main__":
    raise SystemExit(main())
