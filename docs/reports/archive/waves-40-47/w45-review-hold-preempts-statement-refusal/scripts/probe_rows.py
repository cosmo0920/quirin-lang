#!/usr/bin/env python3
"""probe_rows.py OUT RUN... -- the rows of the instance-flow probe runs that
call or define a declaration of W47-LOOP-OWNER-INSTANCE-FLOW's
input_instance.qv (flb_create, flb_input, flb_input_new, flb_service_set),
plus every Pending row, with status and diagnostics, per run directory, and
each run's function tally. Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL."""
import json
import pathlib
import sys
from collections import Counter

NAMES = {"flb_create", "flb_input", "flb_input_new", "flb_service_set"}


def main() -> int:
    out = {}
    for run in sys.argv[2:]:
        report = json.loads((pathlib.Path(run) / "resource-diagnostics.json").read_text())
        rows, tally = [], Counter()
        for unit in report["units"]:
            for function in unit.get("functions", []):
                tally[function["status"]] += 1
                if function["function"] in NAMES or function["status"] == "Pending":
                    diagnostics = function["diagnostics"]
                    rows.append({
                        "file": unit["file"],
                        "function": function["function"],
                        "status": function["status"],
                        "diagnostics": len(diagnostics),
                        "first_diagnostics": [
                            {"line": d["line"], "description": d["description"]}
                            for d in diagnostics[:3]
                        ],
                    })
        out[pathlib.Path(run).name] = {"functions": dict(sorted(tally.items())), "rows": rows}
    pathlib.Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
