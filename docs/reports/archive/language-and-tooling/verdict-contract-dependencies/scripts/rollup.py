#!/usr/bin/env python3
"""rollup.py RUN OUT.json [FUNCTION...] -- VERDICT-CONTRACT-DEPENDENCIES: what a
run's dependency section and row fields say: the sidecar's counts and Safe
roll-up, the top declared assumptions, the field's size (bytes the row and
unit keys add to the report, measured by removing them), rows without the
field, and the named functions' rows in full."""
import json
import pathlib
import sys

ROW_KEY, UNIT_KEY = "contract_dependencies", "contract_body_evidence"


def main():
    run = pathlib.Path(sys.argv[1])
    report = json.loads((run / "resource-diagnostics.json").read_text())
    sidecar = json.loads((run / "resource-diagnostics.contracts.json").read_text())
    section = sidecar["contract_dependencies"]
    rows = [(u["file"], f) for u in report["units"] for f in u["functions"]]
    missing = [f"{file}::{f['function']}" for file, f in rows if ROW_KEY not in f]
    row_bytes = sum(len(json.dumps(f[ROW_KEY], indent=2)) for _, f in rows if ROW_KEY in f)
    unit_bytes = sum(len(json.dumps(u[UNIT_KEY], indent=2)) for u in report["units"] if UNIT_KEY in u)
    named = {}
    for file, f in rows:
        if f["function"] in sys.argv[3:]:
            deps = f.get(ROW_KEY, {})
            named.setdefault(f["function"], []).append({
                "file": file, "status": f["status"], "dependencies": deps,
                "declarations": {d: {k: section["declarations"].get(d, {}).get(k)
                                     for k in ("mode", "status", "body_standing", "premises")}
                                 for d in deps.get("declarations", [])},
            })
    sizes = [len(f[ROW_KEY]["declarations"]) for _, f in rows if ROW_KEY in f]
    safe_sizes = [len(f[ROW_KEY]["declarations"]) for _, f in rows
                  if ROW_KEY in f and f["status"] == "Safe"]
    record = {
        "rows": len(rows), "rows_without_field": missing,
        "counts": section["counts"],
        "safe_rollup": {k: v for k, v in section["safe_rollup"].items()
                        if k != "top_declared_assumptions"},
        "top_declared_assumptions": [(t["function"], t["safe_rows"], t["mode"], t["status"])
                                     for t in section["safe_rollup"]["top_declared_assumptions"]],
        "validated": sorted(n for n, d in section["declarations"].items()
                            if d["body_standing"] == "validated"),
        "declarations_per_row": {"max": max(sizes, default=0),
                                 "mean": round(sum(sizes) / max(len(sizes), 1), 2)},
        "declarations_per_safe_row": {"max": max(safe_sizes, default=0),
                                      "mean": round(sum(safe_sizes) / max(len(safe_sizes), 1), 2)},
        "field_bytes_approx": {"rows": row_bytes, "units": unit_bytes},
        "named": named,
    }
    pathlib.Path(sys.argv[2]).write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: record[k] for k in ("rows", "counts", "safe_rollup",
                                             "declarations_per_row", "declarations_per_safe_row")}))
    print("missing", len(missing))
    print("top", record["top_declared_assumptions"])
    print("validated", record["validated"])


if __name__ == "__main__":
    main()
