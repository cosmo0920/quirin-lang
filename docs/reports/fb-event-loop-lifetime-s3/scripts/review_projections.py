#!/usr/bin/env python3
"""Attribute every runtime move change, preserving raw records separately."""
import json
from pathlib import Path
import re
import sys


def normalize(value):
    if isinstance(value, str):
        return re.sub(r"rk_plugin_library@[0-9a-f]{64}", "rk_plugin_library@$DIGEST", value)
    if isinstance(value, list):
        return [normalize(item) for item in value]
    if isinstance(value, dict):
        return {key: normalize(item) for key, item in value.items()}
    return value


route = sys.argv[1]
out = Path(__file__).resolve().parents[1] / "artifacts" / route
comparison = json.loads((out / "projection-comparison.json").read_bytes())
result = {
    "meaning": "Comparison-only digest normalization; complete raw projections and changes remain archived. A reason change is not silently treated as a verdict change.",
    "route": route,
    "null_property_dicts_identical": comparison["null_property_dicts_identical"],
    "scenarios": {},
}
for name, scenario in comparison["runtime_scenarios"].items():
    digest_only, semantic = [], []
    for change in scenario["changed_moves"]:
        destination = digest_only if normalize(change["before"]) == normalize(change["after"]) else semantic
        destination.append(change["key"] if destination is digest_only else change)
    result["scenarios"][name] = {
        "digest_only_changes": digest_only,
        "other_changed_moves": semantic,
        "added_moves": scenario["added_moves"],
        "removed_moves": scenario["removed_moves"],
        "before": scenario["before"],
        "after": scenario["after"],
    }
(out / "runtime-attribution.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({name: {"digest_only": len(value["digest_only_changes"]),
                         "other_changed": len(value["other_changed_moves"]),
                         "added": len(value["added_moves"]), "removed": len(value["removed_moves"])}
                  for name, value in result["scenarios"].items()}, indent=2))
