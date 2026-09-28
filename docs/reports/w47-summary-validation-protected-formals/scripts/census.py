#!/usr/bin/env python3
"""census.py RUN_DIR [--out FILE]

Reads one arbiter run (resource-diagnostics.json and the typed-path parity
stream) and reports, for card W47-SUMMARY-VALIDATION-PROTECTED-FORMALS:

* every W4 summary the unit derivation validated, with its function's
  reported row status (a summary whose own row is non-Safe is listed);
* every `summary-premises` decision: the candidate, the formals it protects,
  whether the CFG under the reported premises differs, and whether the
  candidate still validates there;
* the withdrawn summaries (validates_under_reported_premises false) and every
  function that reaches one through the unit's direct-call graph, with its
  reported status: the Safe ones are the forecast corrections.

A unit is matched to its parity rows by `source_sha256` and to its report row
by the function names: a unit's reported rows are a subset of its
derivation's candidates. Duplicate rows (a unit derived more than once by the
mode gate) are folded; conflicting duplicates are reported.
"""
import argparse
import collections
import gzip
import json
import pathlib
import sys


def opener(path):
    return gzip.open(path, "rt") if str(path).endswith(".gz") else open(path)


def find(run, name):
    for candidate in (run / name, run / (name + ".gz")):
        if candidate.exists():
            return candidate
    raise SystemExit(f"missing {name} in {run}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--out", type=pathlib.Path)
    args = parser.parse_args()
    with opener(find(args.run, "resource-diagnostics.json")) as handle:
        report = json.load(handle)
    rows = collections.defaultdict(lambda: {"graphs": [], "premises": []})
    with opener(find(args.run, "parity.jsonl")) as handle:
        for line in handle:
            if '"summary-premises' not in line:
                continue
            row = json.loads(line)
            key = row["source_sha256"]
            if row["position"] == "summary-premises-graph":
                rows[key]["graphs"].append(row)
            elif row["position"] == "summary-premises":
                rows[key]["premises"].append(row)
    units = []
    for unit in report["units"]:
        names = {f["function"] for f in unit.get("functions", [])}
        units.append((unit["file"], unit.get("library"), names,
                      {f["function"]: f["status"] for f in unit.get("functions", [])}))
    result = {"units": [], "withdrawn": [], "non_safe_rows_with_summary": [],
              "forecast_safe_moves": [], "ambiguous_sources": [], "conflicts": []}
    for sha, data in sorted(rows.items()):
        graph = {}
        summaries = {}
        for g in data["graphs"]:
            for name, callees in g["graph"].items():
                graph.setdefault(name, set()).update(callees)
            for s in g["summaries"]:
                summaries.setdefault(s["function"], s)
        matches = [u for u in units if u[2] and u[2] <= set(graph)]
        if len(matches) != 1:
            result["ambiguous_sources"].append({"source_sha256": sha, "matches": [u[0] for u in matches],
                                                "graph_functions": len(graph)})
            if not matches:
                continue
        decisions = {}
        for p in data["premises"]:
            key = (p["function"], p["route"], p["candidate"])
            value = (p["cfg_changed"], p["validates_under_reported_premises"], tuple(p["own_protected"]))
            if key in decisions and decisions[key] != value:
                result["conflicts"].append({"source_sha256": sha, "key": key})
            decisions[key] = value
        withdrawn = sorted({k[0] for k, v in decisions.items() if not v[1]})
        callers = collections.defaultdict(set)
        for name, callees in graph.items():
            for callee in callees:
                callers[callee].add(name)
        reach = set(withdrawn)
        pending = list(withdrawn)
        while pending:
            name = pending.pop()
            for caller in callers[name]:
                if caller not in reach:
                    reach.add(caller)
                    pending.append(caller)
        for file, library, _names, statuses in matches:
            entry = {"file": file, "library": library, "source_sha256": sha,
                     "summaries": len(summaries),
                     "decisions": [
                         {"function": k[0], "route": k[1], "candidate": k[2], "own_protected": list(v[2]),
                          "cfg_changed": v[0], "validates_under_reported_premises": v[1],
                          "reported_status": statuses.get(k[0], "not reported")}
                         for k, v in sorted(decisions.items())],
                     "withdrawn": withdrawn,
                     "reaching_withdrawn": sorted(reach - set(withdrawn))}
            result["units"].append(entry)
            for name, s in sorted(summaries.items()):
                status = statuses.get(name, "not reported")
                if status != "Safe" and not s["declared"]:
                    result["non_safe_rows_with_summary"].append(
                        {"file": file, "function": name, "status": status, "contract": s["contract"],
                         "all_borrowed": s["all_borrowed"],
                         "premise": [d for d in entry["decisions"] if d["function"] == name]})
            for name in withdrawn:
                result["withdrawn"].append({"file": file, "function": name,
                                            "reported_status": statuses.get(name, "not reported"),
                                            "decisions": [d for d in entry["decisions"] if d["function"] == name]})
            for name in sorted(reach):
                if statuses.get(name) == "Safe":
                    result["forecast_safe_moves"].append({"file": file, "function": name,
                                                          "withdrawn_itself": name in withdrawn})
    totals = collections.Counter()
    for unit in result["units"]:
        totals["units_with_derivation"] += 1
        totals["summaries"] += unit["summaries"]
        for d in unit["decisions"]:
            totals["decisions"] += 1
            totals["cfg_changed"] += d["cfg_changed"]
            totals["withdrawn_decisions"] += not d["validates_under_reported_premises"]
            totals[f"route:{d['route']}"] += 1
    result["totals"] = dict(totals)
    result["totals"]["withdrawn_functions"] = len(result["withdrawn"])
    result["totals"]["non_safe_rows_with_summary"] = len(result["non_safe_rows_with_summary"])
    result["totals"]["forecast_safe_moves"] = len(result["forecast_safe_moves"])
    text = json.dumps(result, indent=1, sort_keys=True)
    if args.out:
        args.out.write_text(text + "\n")
    print(json.dumps(result["totals"], indent=1))
    print("ambiguous sources:", len(result["ambiguous_sources"]), "conflicts:", len(result["conflicts"]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
