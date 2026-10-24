#!/usr/bin/env python3
"""W48-RK-LOG-CALLER-FRAME: the named premises each changed row rests on.

usage: premises.py COMPARE_JSON TIP_DIR OUT_JSON

For every status change in COMPARE_JSON (compare.py), resolves the TIP row's
`contract_dependencies.hypothesis_dependencies` (declaration -> hypothesis
atoms) through the TIP sidecar's per-declaration hypothesis graph
(resource-diagnostics.contracts.json: contract_dependencies.declarations.
NAME.hypothesis_graph.premises[*].children / erasure.name) to premise names,
and records each declaration's body standing.
"""
import json
import pathlib
import sys


def main():
    compare = json.loads(pathlib.Path(sys.argv[1]).read_text())
    tip = pathlib.Path(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    sidecar = json.loads((tip / "resource-diagnostics.contracts.json").read_text())
    declarations = sidecar["contract_dependencies"]["declarations"]

    def premise_names(name, atoms):
        graph = (declarations.get(name) or {}).get("hypothesis_graph") or {}
        names = set()
        for premise in (graph.get("premises") or {}).values():
            if set(premise.get("children") or []) & set(atoms):
                names.add((premise.get("erasure") or {}).get("name"))
        return sorted(n for n in names if n)

    rows = []
    for change in compare["status_changes"]:
        deps = change.get("tip_dependencies") or {}
        hyp = deps.get("hypothesis_dependencies") or {}
        rows.append({
            "file": change["file"], "function": change["function"],
            "base": change["base"], "tip": change["tip"],
            "declarations": deps.get("declarations"),
            "summaries": deps.get("summaries"),
            "premises": {name: premise_names(name, atoms) for name, atoms in sorted(hyp.items())},
            "standing": {name: (declarations.get(name) or {}).get("body_standing")
                         for name in sorted(hyp)},
        })
    out.write_text(json.dumps({"rows": rows}, indent=1, sort_keys=True) + "\n")
    for row in rows:
        print(row["function"], row["base"], "->", row["tip"], json.dumps(row["premises"], sort_keys=True))


if __name__ == "__main__":
    main()
