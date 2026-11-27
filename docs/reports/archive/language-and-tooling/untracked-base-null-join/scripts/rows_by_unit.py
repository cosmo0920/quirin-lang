#!/usr/bin/env python3
"""rows_by_unit.py BASE.json TIP.json -- card RETURN-STRICT-MEMBER-NULL-
UNTRACKED-ROOT: every row of two opt-in `current` reports whose status,
diagnostics or contract dependencies (null premises included) differ, keyed
by unit *index* and function position, so two units of one file (the route
reports `src/flb_input_chunk.c` twice) are not collapsed as `moved.py` and
`changed_rows.py` collapse them. `hypothesis_context`, `plugin_links` and
`reading_site_evidence` carry the measured revision and package digests and
are not compared."""
import json
import sys


def main() -> int:
    base = json.load(open(sys.argv[1]))
    tip = json.load(open(sys.argv[2]))
    assert len(base["units"]) == len(tip["units"])
    changed = 0
    for index, (before, after) in enumerate(zip(base["units"], tip["units"])):
        assert before["file"] == after["file"]
        rows_b, rows_t = before.get("functions", []), after.get("functions", [])
        assert [r["function"] for r in rows_b] == [r["function"] for r in rows_t]
        for row_b, row_t in zip(rows_b, rows_t):
            keys = ("status", "diagnostics", "contract_dependencies")
            if any(row_b.get(key) != row_t.get(key) for key in keys):
                changed += 1
                print(json.dumps({
                    "unit": index, "file": before["file"], "function": row_b["function"],
                    "status": [row_b["status"], row_t["status"]],
                    "null_premises": [
                        row_b["contract_dependencies"].get("null_premises", []),
                        row_t["contract_dependencies"].get("null_premises", []),
                    ],
                    "diagnostics_equal": row_b["diagnostics"] == row_t["diagnostics"],
                }))
    print(json.dumps({"changed_rows": changed}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
