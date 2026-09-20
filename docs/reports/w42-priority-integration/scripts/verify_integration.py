"""Verify combined artifact deltas against the isolated scalar-store result.

Usage: python3 verify_integration.py BEFORE AFTER ISOLATED_INDIRECT
Directories contain the five raw Cycle19 audit artifacts. This checks exact
attribution, not the soundness of the rules that produced the artifacts.
"""
import hashlib
import json
from collections import Counter
from pathlib import Path
import sys

before, after, isolated = map(Path, sys.argv[1:])
names = [
    "resource-diagnostics.json",
    "resource-diagnostics.chunk-contracts.json",
    "resource-diagnostics.contracts.json",
    "resource-diagnostics.source-effects.json",
    "monkey-source.json",
]
rows = []
for name in names:
    old, new = (before / name).read_bytes(), (after / name).read_bytes()
    row = {
        "file": name,
        "before_sha256": hashlib.sha256(old).hexdigest(),
        "after_sha256": hashlib.sha256(new).hexdigest(),
        "byte_identical": old == new,
    }
    if name == "resource-diagnostics.json":
        row["identical_to_isolated_indirect"] = new == (isolated / name).read_bytes()
        assert row["identical_to_isolated_indirect"], "unattributed diagnostics"
        left, right = json.loads(old), json.loads(new)
        inventory = lambda data: {
            (unit["library"], unit["file"]): {
                key: unit[key]
                for key in ("source_functions", "not_projected", "enumeration")
            }
            for unit in data["units"]
        }
        assert inventory(left) == inventory(right), "source coverage changed"
        row["source_inventory_unchanged"] = True
        row["unit_reason_codes"] = dict(Counter(
            unit.get("reason_code") or "none" for unit in right["units"]
        ))
    elif name == "monkey-source.json":
        left, right = json.loads(old), json.loads(new)
        assert left["schema_version"] == 2 and right["schema_version"] == 3
        right["schema_version"] = 2
        coverage = right.pop("callback_coverage")
        assert right == left, "unattributed source-sidecar change"
        assert coverage["selected_callbacks"] == coverage["accounted_callbacks"] == 3
        assert coverage["selected_paths"] == 8 and coverage["checked_paths"] == 0
        assert len(coverage["callbacks"]) == 3
        assert all(
            callback["cycle18_composition"]["status"] == "unknown"
            and callback["cycle18_composition"]["reason"].strip()
            for callback in coverage["callbacks"]
        )
        row["callback_coverage"] = coverage
        row["only_schema_and_callback_coverage_changed"] = True
    else:
        assert old == new, f"unattributed change: {name}"
    rows.append(row)
print(json.dumps({"artifacts": rows, "attribution_complete": True}, indent=2))
