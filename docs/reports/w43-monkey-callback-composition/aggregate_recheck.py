#!/usr/bin/env python3
"""Compare the frozen broader-integration aggregate with two reruns.

Usage: aggregate_recheck.py FROZEN BASE_RERUN CARD_RERUN OUTPUT

FROZEN is docs/reports/cycle18-broader-integration/aggregate.json. BASE_RERUN
is the aggregate report produced by the unmodified base binary over the
retained evidence; CARD_RERUN is the same configuration run by this card's
refactored `cycle18-composition` route. Every differing aggregate field is
listed; the verdict-bearing fields are compared separately.
"""

import hashlib
import json
import sys

frozen_path, base_path, card_path, output = sys.argv[1:5]
frozen, base, card = (json.load(open(path)) for path in (frozen_path, base_path, card_path))
VERDICT_BEARING = [
    "verdict",
    "universal_safety_certificate",
    "joined_evidence_families",
    "refused_evidence_families",
    "provisional_unsafe_scenarios",
    "unresolved_obligations",
    "bounds",
    "scope",
    "property",
    "semantic_profile",
    "evidence_version",
    "policy_sha256",
    "source_revision",
    "checked_caller_size_bounds",
]


def differences(left, right):
    changed = {}
    for key in sorted(set(left) | set(right)):
        if left.get(key) == right.get(key):
            continue
        if isinstance(left.get(key), dict) and isinstance(right.get(key), dict):
            for inner in sorted(set(left[key]) | set(right[key])):
                if left[key].get(inner) != right[key].get(inner):
                    changed[key + "." + inner] = [left[key].get(inner), right[key].get(inner)]
        else:
            changed[key] = [left.get(key), right.get(key)]
    return changed


def digest(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


frozen_vs_base = differences(frozen["aggregate"], base["aggregate"])
result = {
    "frozen": {"path": frozen_path, "sha256": digest(frozen_path)},
    "base_rerun": {"path": base_path, "sha256": digest(base_path)},
    "card_rerun": {"path": card_path, "sha256": digest(card_path)},
    "verdict_bearing_fields_equal_frozen_vs_base": all(
        frozen["aggregate"].get(key) == base["aggregate"].get(key) for key in VERDICT_BEARING
    ),
    "frozen_vs_base_changed_fields": frozen_vs_base,
    "frozen_vs_base_changed_field_count": len(frozen_vs_base),
    "base_vs_card_aggregate_equal": base["aggregate"] == card["aggregate"],
    "base_vs_card_other_changed_fields": sorted(
        key for key in set(base) | set(card) if key != "aggregate" and base.get(key) != card.get(key)
    ),
}
with open(output, "w") as handle:
    json.dump(result, handle, indent=2)
    handle.write("\n")
print(json.dumps({key: result[key] for key in [
    "verdict_bearing_fields_equal_frozen_vs_base",
    "frozen_vs_base_changed_field_count",
    "base_vs_card_aggregate_equal",
    "base_vs_card_other_changed_fields",
]}))
