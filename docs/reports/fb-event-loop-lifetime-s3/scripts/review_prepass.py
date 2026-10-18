#!/usr/bin/env python3
"""Retain the source-read expansion and its existing closure consequences."""
from collections import Counter
import gzip
import json
from pathlib import Path
import sys

route = sys.argv[1]
out = Path(__file__).resolve().parents[1] / "artifacts" / route
contracts = [json.loads(gzip.decompress((out / side / "contracts.json.gz").read_bytes()))
             for side in ("before", "after")]
left, right = [item["callback_members"] for item in contracts]
a, b = left["prepass"], right["prepass"]
protocols = b.get("lifetime_protocols", {})
result = {
    "meaning": "Source observation, not a verdict or protocol validation. Existing RunMemberFacts policy opens every closure when any relevant run unit is unread.",
    "route": route,
    "before": {key: a[key] for key in ("units", "read", "rounds", "unread", "unread_reasons", "scope")},
    "after": {key: b[key] for key in ("units", "read", "rounds", "unread", "unread_reasons", "scope")},
    "added_tokens": sorted(set(b["tokens"]) - set(a["tokens"])),
    "removed_tokens": sorted(set(a["tokens"]) - set(b["tokens"])),
    "dropped_scope_trigger_counts": dict(Counter(b["scope"]["dropped"].values())),
    "changed_closures": {key: {"before": left["members"].get(key), "after": right["members"].get(key)}
                         for key in left["members"].keys() | right["members"].keys()
                         if left["members"].get(key) != right["members"].get(key)},
    "protocol_counts": {key: len(value) for key, value in protocols.items()},
    "protocol_standing": [{"host": item["host"], "protocol": item["protocol"],
                           "standing_kind": list(item["standing"]),
                           "premise": item["standing"].get("DeclaredAssumption", {}).get("premise"),
                           "unchecked_reason_count": len(item["standing"].get("DeclaredAssumption", {}).get("reasons", []))}
                          for item in protocols.get("protocols", [])],
    "exercised_existing_controls": [
        "member_run_facts::tests::a_registration_reads_the_setters_actuals_at_every_call (unread unit opens closure)",
        "member_prepass_routes::the_no_build_route_decides_over_the_runs_set (broken relevant source opens every member)",
        "QCore.AppRkCallbacks runDecisionVectors (open closure refuses)",
    ],
    "followup": "S3-PREPASS-UNREAD-GRAPH-ISOLATION; only independently proved source/include/layout/alias/exposure noninterference or readable inputs may restore the affected closure",
}
(out / "prepass-review.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({"route": route, "added_tokens": result["added_tokens"],
                  "changed_closures": len(result["changed_closures"]),
                  "before_read": a["read"], "after_read": b["read"],
                  "before_unread": len(a["unread"]), "after_unread": len(b["unread"])}, indent=2))
