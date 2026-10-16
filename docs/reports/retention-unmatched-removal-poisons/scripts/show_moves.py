#!/usr/bin/env python3
"""RETENTION-UNMATCHED-REMOVAL-POISONS: show_moves.py ROUTE... -- print the
totals and every moved row (status and leaves) of artifacts/compare-ROUTE.json.
Read-only."""
import json
import pathlib
import sys

ARTIFACTS = pathlib.Path(__file__).resolve().parents[1] / "artifacts"

for route in sys.argv[1:]:
    data = json.loads((ARTIFACTS / f"compare-{route}.json").read_text())
    print("==", route, data["before"]["rows"], "->", data["after"]["rows"])
    print("   null", data["before"]["null"], "->", data["after"]["null"],
          "status moves", len(data["status_moves"]), "null moves", len(data["null_moves"]),
          "unit moves", data["unit_status_moves"])
    for move in data["status_moves"] + data["leaf_only_moves"]:
        print(move["file"], move["function"], move["status"])
        for kind in ("added", "removed"):
            for leaf in move[kind]:
                print("   ", kind, leaf[1][:120], leaf[2], leaf[3])
