#!/usr/bin/env python3
"""The singleplex pilot's summary: for each W45-FB-RETRY-LIFETIME contract,
at each slice revision, whether it is in force, through which reviewed
identity, and whether its body was analysed (checked against it) in the slice;
plus the OR verdicts. Reads the merged-slice runs and writes JSON to stdout.

usage: singleplex_summary.py            (from the repository root)
"""
import json
import pathlib
import sys

PRIVATE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-contract-revision-mode")
FUNCTIONS = ("flb_output_task_flush", "flb_output_task_singleplex_enqueue",
             "flb_output_thread_pool_flush", "flb_task_retry_destroy", "flb_task_users_dec",
             "flb_task_users_inc", "mk_list_add", "mk_list_del")


def main() -> int:
    result = {}
    for attempt in ("rev-864c3d97", "rev-046accf6"):
        run = PRIVATE / attempt / "merged-slice"
        section = json.loads((run / "resource-diagnostics.contracts.json").read_text())[
            "contract_revision"]
        report = json.loads((run / "resource-diagnostics.json").read_text())
        statuses = {}
        for unit in report["units"]:
            for function in unit["functions"]:
                statuses.setdefault(function["function"], set()).add(function["status"])
        rows = {}
        for record in section["pinned_in_force"]:
            if record["function"] in FUNCTIONS:
                rows[record["function"]] = {
                    "in_force": True,
                    "through": [s.get("revision", s["kind"])[:8]
                                for s in record["matched_reviewed_at"]],
                    "identities": record["identities"],
                    "body_analysed": record["body_analysed"],
                    "slice_status": sorted(statuses.get(record["function"], [])),
                }
        for record in section["pin_mismatches"]:
            if record["function"] in FUNCTIONS:
                rows[record["function"]] = {"in_force": False, "reason": record["reason"]}
        result[attempt] = {"source": section["source"], "counts": section["counts"],
                           "contracts": dict(sorted(rows.items()))}
    json.dump(result, sys.stdout, indent=2)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
