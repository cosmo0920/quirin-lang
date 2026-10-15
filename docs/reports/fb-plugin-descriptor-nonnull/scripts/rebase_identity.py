#!/usr/bin/env python3
"""rebase_identity.py OUT.json ROUTE=FIRST.json,REBASED.json... -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE:
whether the movement compare.py measured at the first base (BEFORE ->
AFTER) is the movement measured after the rebase (main -> rebased tip):
the same moving rows (function, configuration, move), the same check-outcome
changes, and the same added caller candidates, per route."""
import json
import sys

out = {}
for arg in sys.argv[2:]:
    route, paths = arg.split("=")
    first_path, rebased_path = paths.split(",")
    first, rebased = json.load(open(first_path)), json.load(open(rebased_path))
    move = lambda m: (m["function"], m["configuration"], m["move"])
    added = lambda a: (a["function"], a["configuration"], a["subject"])
    out[route] = {
        "moves_rebased": rebased["moves"],
        "verdicts_main": rebased["verdicts_before"],
        "verdicts_tip": rebased["verdicts_after"],
        "custody_column_moves": rebased["custody_column_moves"],
        "same_moving_rows": sorted(map(move, first["moving"])) == sorted(map(move, rebased["moving"])),
        "same_check_changes": first["check_changes"] == rebased["check_changes"],
        "same_added_candidates": sorted(map(added, first["added_candidates"])) == sorted(map(added, rebased["added_candidates"])),
    }
text = json.dumps(out, indent=1, sort_keys=True)
open(sys.argv[1], "w").write(text + "\n")
print(text)
