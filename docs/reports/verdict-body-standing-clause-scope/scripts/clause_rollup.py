#!/usr/bin/env python3
"""clause_rollup.py BEFORE AFTER OUT.json -- VERDICT-BODY-STANDING-CLAUSE-SCOPE:
what the per-clause standing did to the previous card's standing.

From each run's report and contract sidecar: the section counts and Safe
roll-up of both; every declaration BEFORE labelled `validated`, with its
evidence and its AFTER standing (and uncovered clauses); the subset BEFORE
validated by a structural check alone (no W4 or entry evidence); every
declaration AFTER labels `validated` or `partly-validated`; the check that
AFTER's validated set is a subset of BEFORE's; and how many of BEFORE's
"only validated" Safe rows stay "only validated" AFTER (each row's
declarations read from its own `contract_dependencies` field, which this card
does not change)."""
import json
import pathlib
import sys

ROW_KEY = "contract_dependencies"
STRUCTURAL = {"11r", "12w", "13a", "13c", "13d"}


def load(run):
    run = pathlib.Path(run)
    report = json.loads((run / "resource-diagnostics.json").read_text())
    sidecar = json.loads((run / "resource-diagnostics.contracts.json").read_text())
    return report, sidecar["contract_dependencies"]


def only_validated_rows(report, standing):
    rows = []
    for index, unit in enumerate(report["units"]):
        for function in unit.get("functions", []):
            field = function.get(ROW_KEY)
            if field is None or function["status"] != "Safe":
                continue
            if field.get("held") or field.get("withdrawn"):
                continue
            names = field["declarations"]
            if names and all(standing.get(name) == "validated" for name in names):
                rows.append((index, unit["file"], function["function"]))
    return rows


def main():
    before_report, before = load(sys.argv[1])
    after_report, after = load(sys.argv[2])
    standing_before = {n: d["body_standing"] for n, d in before["declarations"].items()}
    standing_after = {n: d["body_standing"] for n, d in after["declarations"].items()}
    validated_before = sorted(n for n, s in standing_before.items() if s == "validated")
    rows = []
    for name in validated_before:
        decl_before = before["declarations"][name]
        decl_after = after["declarations"][name]
        by = decl_before.get("bodies", {}).get("validated_by", [])
        rows.append({
            "function": name,
            "validated_by_before": by,
            "structural_alone": bool(by) and set(by) <= STRUCTURAL,
            "validated_by_after": decl_after.get("bodies", {}).get("validated_by", []),
            "standing_after": decl_after["body_standing"],
            "uncovered_clauses": decl_after.get("uncovered_clauses", []),
            "clauses": decl_after.get("clauses", {}),
            "safe_rows": decl_after["safe_rows"],
        })
    validated_after = sorted(n for n, s in standing_after.items() if s == "validated")
    partly_after = sorted(n for n, s in standing_after.items() if s == "partly-validated")
    newly_partly = sorted(n for n in partly_after if standing_before.get(n) != "validated")
    before_only = only_validated_rows(before_report, standing_before)
    after_only = only_validated_rows(after_report, standing_after)
    record = {
        "before": {"counts": before["counts"],
                   "safe_rollup": {k: v for k, v in before["safe_rollup"].items()
                                   if k != "top_declared_assumptions"}},
        "after": {"counts": after["counts"],
                  "safe_rollup": {k: v for k, v in after["safe_rollup"].items()
                                  if k not in ("top_declared_assumptions",
                                               "partly_validated_declarations")}},
        "validated_before": len(validated_before),
        "validated_after": len(validated_after),
        "partly_validated_after": len(partly_after),
        "validated_after_is_subset_of_before": set(validated_after) <= set(validated_before),
        "structural_alone_before": sum(r["structural_alone"] for r in rows),
        "before_validated": rows,
        "partly_validated_not_validated_before": [
            {"function": n,
             "standing_before": standing_before.get(n),
             "validated_by_after": after["declarations"][n].get("bodies", {}).get("validated_by"),
             "uncovered_clauses": after["declarations"][n].get("uncovered_clauses")}
            for n in newly_partly],
        "only_validated_rows_before": len(before_only),
        "only_validated_rows_after": len(after_only),
        "only_validated_rows_kept": len(set(before_only) & set(after_only)),
        "only_validated_rows_after_not_before": sorted(set(after_only) - set(before_only)),
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: record[k] for k in (
        "validated_before", "validated_after", "partly_validated_after",
        "validated_after_is_subset_of_before", "structural_alone_before",
        "only_validated_rows_before", "only_validated_rows_after", "only_validated_rows_kept")}))
    print("after", json.dumps(record["after"]))
    ok = record["validated_after_is_subset_of_before"] and not record[
        "only_validated_rows_after_not_before"]
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
