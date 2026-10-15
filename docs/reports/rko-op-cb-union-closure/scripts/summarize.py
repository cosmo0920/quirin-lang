#!/usr/bin/env python3
"""summarize.py BEFORE AFTER -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: from the
two runs under private/rkou/ (and the slice runs under the shared prepared
attempt): the custody totals and digests per route, the contract-dependency
counts and Safe roll-up, the member fold section (written to
artifacts/callback-members-corpus.json), and whether the null-property rows
and tallies are byte-identical. Writes artifacts/summary.json and prints it."""
import hashlib
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/rkou"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
ARTIFACTS = HERE.parent / "artifacts"


def report(name, route):
    if route == "slice":
        return SLICE / f"rkou-{name}" / "resource-diagnostics.json"
    suffix = "" if route == "corpus" else "-system"
    return PRIVATE / f"{name}{suffix}" / "resource-diagnostics.json"


def totals(path):
    data = json.loads(path.read_text())
    counts = {}
    for unit in data["units"]:
        for function in unit["functions"]:
            counts[function["status"]] = counts.get(function["status"], 0) + 1
    return counts, hashlib.sha256(path.read_bytes()).hexdigest()


def sidecar(name, route):
    path = report(name, route).with_suffix(".contracts.json")
    return json.loads(path.read_text()) if path.exists() else {}


def main():
    before, after = sys.argv[1], sys.argv[2]
    out = {}
    for route in ("corpus", "system", "slice"):
        entry = {}
        for name in (before, after):
            counts, digest = totals(report(name, route))
            dependencies = sidecar(name, route).get("contract_dependencies", {})
            entry[name] = {"totals": counts, "sha256": digest,
                           "dependency_counts": dependencies.get("counts"),
                           "safe_rollup": dependencies.get("safe_rollup")}
        suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
        rows = [PRIVATE / f"{name}{suffix}-null.jsonl" for name in (before, after)]
        tallies = [json.loads((PRIVATE / f"{name}{suffix}-null.summary.json").read_text())["null"]
                   for name in (before, after)]
        entry["null"] = {"rows_identical": rows[0].read_bytes() == rows[1].read_bytes(),
                         "rows_sha256": [hashlib.sha256(r.read_bytes()).hexdigest() for r in rows],
                         "tallies": tallies}
        members = sidecar(after, route).get("callback_members")
        if members is not None:
            entry["callback_members"] = {k: v for k, v in members.items() if k != "meaning"}
            if route == "corpus":
                (ARTIFACTS / "callback-members-corpus.json").write_text(
                    json.dumps(members, indent=1) + "\n")
        out[route] = entry
    (ARTIFACTS / "summary.json").write_text(json.dumps(out, indent=1) + "\n")
    print(json.dumps(out, indent=1)[:6000])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
