#!/usr/bin/env python3
"""registrations.py PARITY.jsonl REPORT OUT.json -- CG22-CALLBACK-REGISTRY-CONTRACT:
the parity census's callback registrations (rule 11p: every call passing a
function designator, `position: "callback-registration"`) grouped by callee,
each with the status of the calling function in REPORT (a run directory or
its resource-diagnostics.json) and the designators it passes. A function
named in several units takes the worst status (Safe < Unknown < Unsafe). The
`Safe` callers of a declared callee are the rows a callback clause on that
declaration can move (a registration or invocation no longer read as
borrowed)."""
import collections
import json
import pathlib
import sys

ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def main():
    report = pathlib.Path(sys.argv[2])
    if report.is_dir():
        report = report / "resource-diagnostics.json"
    status = {}
    for unit in json.loads(report.read_text())["units"]:
        for function in unit["functions"]:
            name, value = function["function"], function["status"]
            if name not in status or ORDER[value] > ORDER[status[name]]:
                status[name] = value
    callees = collections.defaultdict(lambda: {"declared": False, "sites": 0,
                                               "callers": {}, "designators": set()})
    seen = set()
    for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
        row = json.loads(line)
        if row.get("position") != "callback-registration":
            continue
        key = (row["callee"], row["function"], row["line"], row.get("source_sha256"))
        if key in seen:
            continue
        seen.add(key)
        entry = callees[row["callee"]]
        entry["declared"] |= bool(row.get("callee_declared"))
        entry["sites"] += 1
        entry["callers"][row["function"]] = status.get(row["function"], "not-reported")
        entry["designators"].update(row.get("designators", []))
    out = {}
    for callee, entry in sorted(callees.items(), key=lambda kv: -kv[1]["sites"]):
        counts = collections.Counter(entry["callers"].values())
        out[callee] = {"declared": entry["declared"], "sites": entry["sites"],
                       "caller_statuses": dict(counts),
                       "safe_callers": sorted(f for f, s in entry["callers"].items() if s == "Safe"),
                       "designators": sorted(entry["designators"])}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + "\n")
    print("distinct registrations %d, callees %d" % (len(seen), len(out)))
    for callee, entry in out.items():
        if entry["declared"] or entry["safe_callers"]:
            print("%-44s declared=%-5s sites=%3d %s safe=%s" % (
                callee, entry["declared"], entry["sites"], entry["caller_statuses"],
                entry["safe_callers"][:6]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
