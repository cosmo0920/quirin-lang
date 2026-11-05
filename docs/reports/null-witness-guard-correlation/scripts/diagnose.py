#!/usr/bin/env python3
"""NULL-WITNESS-GUARD-CORRELATION step 1: the findings that appeared between
two corpus null reports, with each finding check's source on both sides.

Usage:
  diagnose.py BASE_NULL_JSONL TIP_NULL_JSONL > diagnosis.json

Read-only; prints one record per finding check that is a finding on the tip
side and not on the base side (keyed by library, file, function and the
check's origin and subject, so a renumbered check still matches).
"""

import json
import sys


def rows(path):
    out = {}
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            row = json.loads(line)
            out[(row["library"], row["file"], row["function"])] = row
    return out


def finding_checks(row):
    triage = row["null"].get("triage") or []
    by_id = {c["id"]: c for c in row["null"]["checks"]}
    for record in triage:
        if record["status"] == "finding":
            yield record, by_id[record["check"]]


def main():
    base, tip = rows(sys.argv[1]), rows(sys.argv[2])
    out = []
    for key in sorted(tip):
        row = tip[key]
        before = base.get(key)
        for record, check in finding_checks(row):
            match = None
            if before is not None:
                match = next(
                    (
                        c
                        for c in before["null"]["checks"]
                        if c["origin"] == check["origin"] and c["subject"] == check["subject"]
                    ),
                    None,
                )
            if match is not None and match["outcome"] == "finding":
                continue
            out.append(
                {
                    "library": key[0],
                    "file": key[1],
                    "function": key[2],
                    "check": check["id"],
                    "origin": check["origin"],
                    "subject": check["subject"],
                    "base": {
                        "outcome": match and match["outcome"],
                        "sources": match and match.get("sources"),
                        "null": before and before["null"]["verdict"],
                        "combined": before and before["combined"],
                        "contract_dependencies": before
                        and before["null"]["contract_dependencies"],
                    },
                    "tip": {
                        "outcome": check["outcome"],
                        "sources": check.get("sources"),
                        "null": row["null"]["verdict"],
                        "combined": row["combined"],
                        "custody": row["custody"],
                        "contract_dependencies": row["null"]["contract_dependencies"],
                    },
                    "witness": record["witness"],
                }
            )
    json.dump(out, sys.stdout, indent=1, sort_keys=True)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
