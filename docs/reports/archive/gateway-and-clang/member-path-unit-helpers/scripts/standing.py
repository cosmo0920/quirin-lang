#!/usr/bin/env python3
"""standing.py BEFORE AFTER -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: the
corpus sidecar's body standing of the eight member-path declarations in each
run (private/mpuh/NAME/resource-diagnostics.contracts.json), the rows whose
`contract_dependencies` name one of them, every declaration whose standing
differs between the runs, and the dependency counts. Writes
artifacts/standing.json and prints it."""
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
PRIVATE = HERE.parents[3] / "private/mpuh"
ARTIFACTS = HERE.parent / "artifacts"
EIGHT = [
    "rd_kafka_admin_worker",
    "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb",
    "rd_kafka_admin_fanout_worker",
    "rd_kafka_DeleteRecords_leaders_queried_cb",
    "rd_kafka_ListOffsets_leaders_queried_cb",
    "rd_kafka_share_commit_async_fanout_op",
    "rd_kafka_share_fetch_fanout_op",
    "rd_kafka_share_commit_sync_fanout_op",
]


def load(name):
    sidecar = json.loads((PRIVATE / name / "resource-diagnostics.contracts.json").read_text())
    return sidecar["contract_dependencies"]


def rows(name):
    report = json.loads((PRIVATE / name / "resource-diagnostics.json").read_text())
    found = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            names = json.dumps(function.get("contract_dependencies", []))
            hit = [d for d in EIGHT if f'"{d}"' in names]
            if hit:
                found[f'{unit.get("file", unit.get("path", "?"))}:{function.get("function")}'] = {
                    "status": function["status"], "declarations": hit}
    return found


def main():
    before, after = sys.argv[1], sys.argv[2]
    out = {}
    runs = {name: load(name) for name in (before, after)}
    for name, data in runs.items():
        declarations = data["declarations"]
        out[name] = {
            "counts": data["counts"],
            "eight": {d: (declarations[d].get("body_standing"),
                          declarations[d].get("uncovered_clauses"))
                      if d in declarations else "not consulted" for d in EIGHT},
            "dependent_rows": rows(name),
        }
    a, b = runs[before]["declarations"], runs[after]["declarations"]
    out["standing_changes"] = {
        d: [a.get(d, {}).get("body_standing"), b.get(d, {}).get("body_standing")]
        for d in sorted(set(a) | set(b))
        if a.get(d, {}).get("body_standing") != b.get(d, {}).get("body_standing")
    }
    (ARTIFACTS / "standing.json").write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1)[:8000])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
