#!/usr/bin/env python3
"""summary.py BEFORE AFTER -- NULL-MEMBER-INVARIANT-SCOPE: print the per-route
null tallies, moves, check changes, candidate counts, premise standings and
wall times routes.sh left in private/nmis/."""
import json
import pathlib
import sys

here = pathlib.Path(__file__).resolve().parent
private = here.parents[3] / "private" / "nmis"
before, after = sys.argv[1], sys.argv[2]
for route in ["corpus", "system", "slice"]:
    suffix = "" if route == "corpus" else "-" + route
    compare = json.loads((private / f"compare-{route}-{after}.json").read_text())
    cb = json.loads((private / f"counts-{route}-{before}.json").read_text())
    ca = json.loads((private / f"counts-{route}-{after}.json").read_text())
    sb = json.loads((private / f"{before}{suffix}-null.summary.json").read_text())
    sa = json.loads((private / f"{after}{suffix}-null.summary.json").read_text())
    print("==", route)
    print(" null before", sb["null"])
    print(" null after ", sa["null"])
    print(" moves", compare["moves"], "custody column moves", compare["custody_column_moves"],
          "safe->non-safe", compare["safe_to_non_safe"])
    print(" check changes", compare["check_changes"])
    print(" candidates", cb["candidates"], "->", ca["candidates"], "own", cb["own"], "->", ca["own"],
          "travelled", cb["travelled"], "->", ca["travelled"])
    print(" memory-field own", cb["memory_field_own"], "->", ca["memory_field_own"])
    print(" member-sourced", cb["member_sourced"], "->", ca["member_sourced"])
    print(" scope checks", ca["scope_entry_checks_by_outcome"], "stable proofs", ca["checks_with_a_stable_proof_by_outcome"])
    print(" premises", sb["custody_null_premises"], "->", sa["custody_null_premises"])
    print(" findings", sb["checks"]["by_outcome"].get("finding"), "->", sa["checks"]["by_outcome"].get("finding"))
    print(" fold stable", json.dumps(sa["fold"].get("stable_declarations", {}))[:300])
    for name in (before, after):
        seconds = private / f"{name}{suffix}.seconds"
        print(" seconds", name, seconds.read_text().strip() if seconds.exists() else "?")
