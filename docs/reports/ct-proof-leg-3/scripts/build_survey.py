#!/usr/bin/env python3
"""Turn the survey probe's raw output into `survey.json` beside the report.

Inputs: `probes/survey.tsv` (the probe's own stdout rows, shipped) and the
manifest. Output: one row per mapped case with its route, its verdict mode,
whether its default reference run derives into the F02 observable alphabet,
and the derived trace or the refusal. Run from the repository root:

    python3 docs/reports/ct-proof-leg-3/scripts/build_survey.py
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
REPORT = ROOT / "docs/reports/ct-proof-leg-3"
TSV = REPORT / "probes/survey.tsv"
OUT = REPORT / "survey.json"


def main():
    manifest = json.loads((ROOT / "docs/reevaluation/ct_manifest.json").read_text())
    cases = {c["id"]: c for c in manifest["cases"]}

    default_rows = {}
    scripted = {}
    for line in TSV.read_text().splitlines():
        fields = line.split("\t")
        if fields[0] == "SCRIPT":
            scripted.setdefault(fields[1], []).append(
                {"script": fields[2], "outcome": fields[3], "detail": "\t".join(fields[4:])}
            )
        else:
            default_rows[fields[0]] = {
                "outcome": fields[1],
                "detail": "\t".join(fields[2:]),
            }

    rows = []
    for case_id, case in sorted(cases.items(), key=lambda kv: int(kv[0][2:])):
        if case["mapping_status"] != "mapped":
            continue
        probe = default_rows.get(case_id)
        rows.append(
            {
                "id": case_id,
                "route": case["route"],
                "expected": case["expected"],
                "harness_verdict_mode": case["harness_verdict_mode"],
                "proof_stage": case["stages"]["proof"]["status"],
                "default_run": probe["outcome"] if probe else "NOT-PROBED",
                "default_detail": probe["detail"] if probe else "",
                "scripted_arms": scripted.get(case_id, []),
            }
        )

    summary = {}
    for row in rows:
        summary[row["default_run"]] = summary.get(row["default_run"], 0) + 1

    OUT.write_text(
        json.dumps(
            {
                "card": "CT-PROOF-LEG-3",
                "generated": "2026-09-19",
                "method": (
                    "CT-PROOF-LEG-2's method, re-run rather than inherited: every mapped "
                    "case's own source_fixture compiled through qcore_compiler::compile_files; "
                    "where the module lowers to a checked task module, executed by "
                    "qcore_core::task_execution::execute_task_module and handed to "
                    "qcore_core::task_trace_relation::AdmittedCausalOrder::derive (which is "
                    "reference_trace followed by declare). Five failure scripts beside the "
                    "default one. Probe source: probes/survey_probe.rs; raw output: "
                    "probes/survey.tsv."
                ),
                "mapped_cases": len(rows),
                "default_run_summary": summary,
                "rows": rows,
            },
            indent=1,
        )
        + "\n"
    )
    print("wrote", OUT)
    for key, value in sorted(summary.items()):
        print("  ", key, value)


if __name__ == "__main__":
    main()
