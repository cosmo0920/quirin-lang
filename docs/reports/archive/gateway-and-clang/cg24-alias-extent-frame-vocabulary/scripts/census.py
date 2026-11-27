#!/usr/bin/env python3
"""CG24-ALIAS-EXTENT-FRAME-VOCABULARY census of one measured route.

usage: census.py CUSTODY_REPORT BOUNDS_JSONL OUT.json

Custody (resource-diagnostics.json), by diagnostic description:
  sds        call to uncontracted function `cfl_sds_cat_safe` (W46-SDS-CAT-SAFE-IN-PLACE-REALLOC)
  backlog    a refusal naming one of the storage-backlog functions of
             W45-FB-BACKLOG-REFERENCE-VOCABULARY (sb_segregate_chunks,
             sb_append_chunk_to_segregated_backlogs, cb_queue_chunks, ...)
  derived    a refusal naming a derived/interior result or a result alias
For each family: leaves, functions (not Safe), and functions whose every leaf is
in the family (sole).

Bounds (the QCORE_BOUNDS_PROPERTY rows): the tally, the functions with a pointer
passed to a callee that states no bounds contract (the reach of callee
declarations, frames and relations included) and those for which that is the
only obstacle, and the functions whose rows record a declared fact at all (0
without bounds declarations: no call can then withdraw a fact). Nothing is
piped; the JSON is the record.
"""

import collections
import json
import pathlib
import re
import sys

SDS = "call to uncontracted function `cfl_sds_cat_safe`"
BACKLOG = re.compile(
    r"sb_segregate_chunks|sb_append_chunk_to_segregated_backlogs|cb_queue_chunks|"
    r"sb_remove_chunk_from_segregated_backlogs|sb_release_output_queue_space"
)
DERIVED = re.compile(r"derived (result|pointer)|interior|result alias|return_interior|return_exact", re.I)
NO_CONTRACT = re.compile(r"states no bounds contract")


def family(description):
    if description == SDS:
        return "sds"
    if BACKLOG.search(description):
        return "backlog"
    if DERIVED.search(description):
        return "derived"
    return None


def custody(path):
    report = json.loads(pathlib.Path(path).read_text())
    leaves = collections.Counter()
    funcs = collections.defaultdict(set)
    sole = collections.defaultdict(set)
    statuses = {}
    for unit in report["units"]:
        for entry in unit.get("functions", []):
            key = (unit.get("library", ""), unit["file"], entry["function"])
            statuses[key] = entry["status"]
            if entry["status"] == "Safe":
                continue
            descriptions = [d.get("description", "") for d in entry.get("diagnostics") or []]
            fams = [family(d) for d in descriptions]
            for f in fams:
                if f:
                    leaves[f] += 1
                    funcs[f].add(key)
            present = set(fams)
            if descriptions and len(present) == 1 and None not in present:
                sole[present.pop()].add(key)
    tally = collections.Counter(statuses.values())
    return {
        "functions": len(statuses),
        "tally": dict(sorted(tally.items())),
        "families": {
            f: {
                "leaves": leaves[f],
                "functions": len(funcs[f]),
                "sole": len(sole[f]),
                "sole_functions": sorted("/".join(k) for k in sole[f]),
            }
            for f in ("sds", "backlog", "derived")
        },
    }


def bounds(path):
    rows = [json.loads(line) for line in open(path, encoding="utf-8") if line.strip()]
    tally = collections.Counter()
    reach = 0
    reach_sole = 0
    declared = 0
    for row in rows:
        b = row.get("bounds") or {}
        verdict = b.get("verdict", "absent")
        tally[verdict] += 1
        accesses = b.get("accesses", [])
        if b.get("assumptions"):
            declared += 1
        unresolved = [a for a in accesses if a["actual"] not in ("in-bounds", "excluded: proved unreachable")]
        uncontracted = [a for a in unresolved if a["kind"] == "call-precondition" and NO_CONTRACT.search(a["subject"])]
        if uncontracted:
            reach += 1
            other_reasons = [
                r for r in b.get("reasons", [])
                if not (r.startswith("bounds-missing-extent") and NO_CONTRACT.search(r))
            ]
            if len(uncontracted) == len(unresolved) and not other_reasons:
                reach_sole += 1
    return {
        "rows": len(rows),
        "tally": dict(sorted(tally.items())),
        "pointer_to_uncontracted_callee": reach,
        "pointer_to_uncontracted_callee_sole": reach_sole,
        "rows_with_declared_assumptions": declared,
    }


def main():
    record = {"custody": custody(sys.argv[1]), "bounds": bounds(sys.argv[2])}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")


if __name__ == "__main__":
    main()
