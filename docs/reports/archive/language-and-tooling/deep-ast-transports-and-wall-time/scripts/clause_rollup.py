#!/usr/bin/env python3
"""clause_rollup.py BEFORE AFTER OUT.json -- RDHTTP-HEADER-SUMMARY-GAP (adapted from DECLARED-ASSUMPTION-BODY-CHECKS)
(adapted from VERDICT-BODY-STANDING-CLAUSE-SCOPE): what the declared-entry
check (rules 13m, 13n) did to the body standing.

From each run's report and contract sidecar: the section counts and Safe
roll-up of both; every declaration whose standing moved, with its evidence
before and after; the checks this card claims:

* every declaration validated BEFORE is still validated AFTER, and every
  partly-validated one is still at least partly validated (the card only adds
  evidence);
* every declaration whose standing rose carries `13m` or `13n` evidence AFTER,
  and none fell;
* every "only validated" Safe row of BEFORE is one AFTER, and every row new to
  that bucket rests on a declaration the card validated (each row's
  declarations read from its own `contract_dependencies` field, which this
  card does not change);
* no unit records a `conflict` failure for a declaration it now validates by
  `13n` (read from the sidecar's per-declaration `bodies`).

Exit 0 when every check holds."""
import json
import pathlib
import sys

ROW_KEY = "contract_dependencies"
RANK = {"declared-assumption": 0, "held": 0, "partly-validated": 1, "validated": 2}
NEW = {"13m", "13n"}


def load(run):
    run = pathlib.Path(run)
    report = json.loads((run / "resource-diagnostics.json").read_text())
    sidecar = json.loads((run / "resource-diagnostics.contracts.json").read_text())
    return report, sidecar["contract_dependencies"]


def safe_rows(report):
    for index, unit in enumerate(report["units"]):
        for function in unit.get("functions", []):
            field = function.get(ROW_KEY)
            if field is None or function["status"] != "Safe":
                continue
            yield (index, unit["file"], function["function"]), field


def only_validated_rows(report, standing):
    rows = {}
    for key, field in safe_rows(report):
        if field.get("held") or field.get("withdrawn"):
            continue
        names = field["declarations"]
        if names and all(standing.get(name) == "validated" for name in names):
            rows[key] = names
    return rows


def main():
    before_report, before = load(sys.argv[1])
    after_report, after = load(sys.argv[2])
    standing_before = {n: d["body_standing"] for n, d in before["declarations"].items()}
    standing_after = {n: d["body_standing"] for n, d in after["declarations"].items()}
    moved = []
    fell = []
    for name in sorted(set(standing_before) | set(standing_after)):
        b, a = standing_before.get(name), standing_after.get(name)
        if b == a:
            continue
        decl_before = before["declarations"].get(name, {})
        decl_after = after["declarations"].get(name, {})
        row = {
            "function": name,
            "standing_before": b,
            "standing_after": a,
            "bodies_before": decl_before.get("bodies"),
            "bodies_after": decl_after.get("bodies"),
            "uncovered_clauses_after": decl_after.get("uncovered_clauses", []),
            "safe_rows": decl_after.get("safe_rows"),
        }
        moved.append(row)
        if RANK.get(a, 0) < RANK.get(b, 0):
            fell.append(name)
    rose_without_new_evidence = [
        row["function"] for row in moved
        if RANK.get(row["standing_after"], 0) > RANK.get(row["standing_before"], 0)
        and not NEW & set((row["bodies_after"] or {}).get("validated_by", []))
    ]
    conflict_beside_13n = [
        name for name, decl in after["declarations"].items()
        if "13n" in (decl.get("bodies") or {}).get("validated_by", [])
        and "conflict" in (decl.get("bodies") or {}).get("failed_by", [])
        and decl["body_standing"] == "validated"
    ]
    before_only = only_validated_rows(before_report, standing_before)
    after_only = only_validated_rows(after_report, standing_after)
    promoted = {row["function"] for row in moved if row["standing_after"] == "validated"}
    new_rows = sorted(set(after_only) - set(before_only))
    new_rows_without_promotion = [
        key for key in new_rows if not promoted & set(after_only[key])
    ]
    record = {
        "before": {"counts": before["counts"],
                   "safe_rollup": {k: v for k, v in before["safe_rollup"].items()
                                   if k not in ("top_declared_assumptions",
                                                "partly_validated_declarations")}},
        "after": {"counts": after["counts"],
                  "safe_rollup": {k: v for k, v in after["safe_rollup"].items()
                                  if k not in ("top_declared_assumptions",
                                               "partly_validated_declarations")}},
        "moved": moved,
        "fell": fell,
        "rose_without_new_evidence": rose_without_new_evidence,
        "validated_with_a_conflict_failure": conflict_beside_13n,
        "only_validated_rows_before": len(before_only),
        "only_validated_rows_after": len(after_only),
        "only_validated_rows_kept": len(set(before_only) & set(after_only)),
        "only_validated_rows_new": [list(key) + [after_only[key]] for key in new_rows],
        "new_rows_without_a_promoted_declaration": [list(key) for key in new_rows_without_promotion],
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({
        "moved": {row["function"]: f'{row["standing_before"]} -> {row["standing_after"]}'
                  for row in moved},
        "fell": fell,
        "rose_without_new_evidence": rose_without_new_evidence,
        "only_validated_rows": [len(before_only), len(after_only)],
        "kept": record["only_validated_rows_kept"],
        "new_without_promotion": len(new_rows_without_promotion),
    }))
    print("after", json.dumps(record["after"]))
    ok = (not fell and not rose_without_new_evidence and not conflict_beside_13n
          and record["only_validated_rows_kept"] == len(before_only)
          and not new_rows_without_promotion)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
