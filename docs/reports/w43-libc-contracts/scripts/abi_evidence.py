#!/usr/bin/env python3
"""Summarise the declared-ABI evidence behind the reader-ground refusals.

Usage: abi_evidence.py ABI_CENSUS.txt BEFORE_DIR TRIAL_DIR OUT.json

ABI_CENSUS.txt is the output of
`cargo run -p qcore-c-gateway --example declared_abi_census -- MODULE... -- UNIT.i...`
over the 185 pinned units (line-marker-stripped `-E` text), for every module
this card drafted, BEFORE any declaration was dropped: one row per (unit,
contracted name the unit mentions). TRIAL_DIR is the private trial audit run
with every drafted declaration selected. The output counts, per name, the
units that agreed / withdrew / did not declare it, and the call-site leaves the
trial published as `call to `NAME` whose declared contract was withdrawn`.
"""

import collections
import json
import pathlib
import re
import sys


def main() -> int:
    census, before_dir, trial_dir, out = sys.argv[1:5]
    per_name = collections.defaultdict(collections.Counter)
    for line in pathlib.Path(census).read_text().splitlines():
        parts = line.split("\t")
        if len(parts) == 3:
            per_name[parts[1]][parts[2]] += 1
    trial = json.loads((pathlib.Path(trial_dir) / "resource-diagnostics.json").read_text())
    withdrawn = collections.Counter()
    pattern = re.compile(r"call to `([^`]+)` whose declared contract was withdrawn[^(]*\(([^)]+)\)")
    for unit in trial["units"]:
        for entry in unit["functions"]:
            for d in entry.get("diagnostics") or []:
                match = pattern.match(d["description"])
                if match:
                    withdrawn[(match.group(1), match.group(2))] += 1
    record = {
        "abi_census_units_by_name": {n: dict(c) for n, c in sorted(per_name.items())},
        "trial_withdrawn_call_site_leaves": {f"{n} ({r})": c for (n, r), c in sorted(withdrawn.items())},
        "probe": {
            "note": "Five one-line units through the same census tool: `extern int printf (const char *__restrict __format, ...);` "
            "and the same without `...` are withdrawn (abi-unreadable-declaration); `const char * __format` and "
            "`const char *__format` with a trailing `__attribute__` agree. The unread token is a qualifier AFTER the `*`.",
        },
    }
    pathlib.Path(out).write_text(json.dumps(record, indent=1) + "\n")
    print(len(per_name), sum(withdrawn.values()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
