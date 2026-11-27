#!/usr/bin/env python3
"""clause_rollup.py BEFORE AFTER OUT.json -- CG22-CALLBACK-REGISTRY-CONTRACT
(from RDHTTP-HEADER-SUMMARY-GAP's clause_rollup.py): the contract-dependency
section of two runs (rules 13f-13k): each run's declaration counts and corpus
Safe roll-up, and every declaration whose body standing moved, with its
evidence before and after and its clauses left uncovered. The card adds `13y`
evidence (rule 13y) and withholds sixteen declarations; nothing here decides a
verdict."""
import json
import pathlib
import sys

SKIP = ("top_declared_assumptions", "partly_validated_declarations")


def load(run):
    run = pathlib.Path(run)
    sidecar = json.loads((run / "resource-diagnostics.contracts.json").read_text())
    return sidecar["contract_dependencies"]


def main():
    before, after = load(sys.argv[1]), load(sys.argv[2])
    standing = [{n: d["body_standing"] for n, d in run["declarations"].items()}
                for run in (before, after)]
    moved = []
    for name in sorted(set(standing[0]) | set(standing[1])):
        b, a = standing[0].get(name), standing[1].get(name)
        if b == a:
            continue
        decl = after["declarations"].get(name, {})
        moved.append({
            "function": name, "standing_before": b, "standing_after": a,
            "bodies_before": before["declarations"].get(name, {}).get("bodies"),
            "bodies_after": decl.get("bodies"),
            "uncovered_clauses_after": decl.get("uncovered_clauses", []),
            "safe_rows_after": decl.get("safe_rows"),
        })
    record = {
        "before": {"counts": before["counts"],
                   "safe_rollup": {k: v for k, v in before["safe_rollup"].items() if k not in SKIP}},
        "after": {"counts": after["counts"],
                  "safe_rollup": {k: v for k, v in after["safe_rollup"].items() if k not in SKIP}},
        "moved": moved,
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    print("before", json.dumps(record["before"]))
    print("after ", json.dumps(record["after"]))
    for row in moved:
        print("%s: %s -> %s (%s)" % (row["function"], row["standing_before"], row["standing_after"],
                                     (row["bodies_after"] or {}).get("validated_by")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
