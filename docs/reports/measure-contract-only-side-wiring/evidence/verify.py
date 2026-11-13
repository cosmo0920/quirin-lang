#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card MEASURE-CONTRACT-ONLY-SIDE-WIRING: the --verify-reuse check, applied to
the records compare.sh wrote. The harness keeps its own timing records
(started.txt, ended.txt, time.txt) in each route directory, where the tool's
layout has none; they are dropped here and from nothing else.

Usage: verify.py OUT_PREFIX  (reads OUT_PREFIX-a and OUT_PREFIX-b)
Exit 0 only when, for both tips, every report artifact of the composed side
equals the full side's byte for byte with no substitution and the
base->composed comparison equals base->full (directories aside)."""

import json
import sys

HARNESS = {"started.txt", "ended.txt", "time.txt"}


def clean(record):
    record = {k: v for k, v in record.items() if k != "after_directory"}
    artifacts = dict(record["artifacts"])
    for key in ("identical", "differing"):
        artifacts[key] = [e for e in artifacts[key] if e["file"] not in HARNESS]
    record["artifacts"] = artifacts
    return record


ok = True
for tip in "ab":
    record = json.load(open(f"{sys.argv[1]}-{tip}/native-comparison.json"))
    routes = {c["comparison"]: c["routes"]["corpus"] for c in record["comparisons"]}
    same = clean(routes["composed->full"])["artifacts"]
    byte_identical = (not same["differing"] and not same["only_before"] and not same["only_after"]
                      and not same["identical_after_named_substitution"] and same["identical"])
    equal = clean(routes["base->composed"]) == clean(routes["base->full"])
    rows = routes["composed->full"]["function_rows"]
    print(json.dumps({"tip": tip, "status": record["status"],
                      "composed_vs_full_artifacts_byte_identical": bool(byte_identical),
                      "artifacts": sorted(e["file"] for e in same["identical"]),
                      "composed_vs_full_function_rows": rows,
                      "base_to_composed_equals_base_to_full": equal,
                      "base_to_composed_function_rows": routes["base->composed"]["function_rows"],
                      "base_to_composed_moves": len(routes["base->composed"].get("moves", []))}))
    ok &= bool(byte_identical) and equal and record["status"] == "complete"
sys.exit(0 if ok else 1)
