#!/usr/bin/env python3
"""premises.py RUN FUNCTION... -- HANDOFF-CANDIDATE-OWN-CALLBACKS: for each
named FUNCTION's row in the corpus run private/rrlo/RUN, every declaration its
`contract_dependencies` lists with the premises the contract sidecar publishes
for it (the way a row's premises are read, rule 13f), and the declarations
carrying `rk_no_plugin_library`. A reading aid for the report."""
import json
import pathlib
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/rrlo"


def main():
    run, *functions = sys.argv[1:]
    report = json.loads((PRIVATE / run / "resource-diagnostics.json").read_text())
    sidecar = json.loads((PRIVATE / run / "resource-diagnostics.contracts.json").read_text())
    declared = sidecar["contract_dependencies"]["declarations"]
    out = {}
    for unit in report["units"]:
        for row in unit["functions"]:
            if row["function"] not in functions:
                continue
            names = row.get("contract_dependencies", {}).get("declarations", [])
            premises = {
                name: sorted(p["name"] for p in declared.get(name, {}).get("premises", []))
                for name in names
            }
            carrying = sorted(n for n, p in premises.items() if "rk_no_plugin_library" in p)
            without = sorted(n for n, p in premises.items() if "rk_no_plugin_library" not in p)
            out[f"{unit['file']}::{row['function']}"] = {
                "status": row["status"],
                "declarations": len(names),
                "carrying_rk_no_plugin_library": len(carrying),
                "without": without,
                "premises": premises,
            }
    print(json.dumps(out, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
