#!/usr/bin/env python3
"""Aggregate replayed descriptor observations into one evidence summary.

Usage: aggregate.py OUT_DIR [AGGREGATE.json]

Reads OUT_DIR/summary.json, each run's report.json and OUT_DIR/sites.json
(from run_scenarios.py) and writes the aggregate: per-run counts, property
checks and violations over every accepted run, each violation with its
symbolized call site, and every non-inherited generation live at exit
grouped by its creating site. Read-only on OUT_DIR.
"""
import collections
import json
import pathlib
import sys


def site_name(sites, site):
    if site["module"] == "observer":
        return "observer:start"
    entry = sites.get(f"{site['module']}:{site['offset']}")
    if not entry:
        return f"{site['module']}+{site['offset']}"
    chain = [entry] + entry.get("inlined_into", [])
    return " <- ".join(f"{frame['function']} ({frame['location']})" for frame in chain)


def stack_functions(sites, stack):
    """Caller chain outside the observer and libc, innermost first."""
    names = []
    for site in stack:
        entry = sites.get(f"{site['module']}:{site['offset']}")
        if not entry or "descriptor_observer.c" in entry["location"] or entry["function"] in ("??", "?"):
            continue
        for frame in [entry] + entry.get("inlined_into", []):
            names.append(f"{frame['function']} ({frame['location'].rsplit(':', 1)[0]})")
    return names


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    summary = json.loads((out / "summary.json").read_text())
    sites = json.loads((out / "sites.json").read_text())
    properties = collections.OrderedDict()
    violations = []
    live = collections.defaultdict(lambda: {"runs": set(), "generations": 0, "kinds": set(), "paths": set()})
    rows = []
    rejected = []
    totals = collections.Counter()
    for run in summary["runs"]:
        name = run["name"]
        if run.get("checker_exit") != 0 or run.get("timed_out"):
            rejected.append({"name": name, "timed_out": run.get("timed_out"),
                             "error": run.get("checker_error")})
            continue
        report = json.loads((out / "runs" / name / "report.json").read_text())["report"]
        totals["records"] += report["records"]
        totals["acquisitions"] += sum(report["acquisitions"].values())
        totals["releases"] += sum(report["releases"].values())
        totals["uses"] += report["uses"]
        totals["reused_identities"] += report["reused_identities"]
        totals["constructor_failures"] += report["constructor_failures"]
        totals["injected_failures"] += report["injected_failures"]
        totals["untracked_uses"] += report["untracked_uses"]
        totals["untracked_releases"] += report["untracked_releases"]
        for key, count in report["out_of_model"].items():
            totals["out_of_model." + key] += count
        run_violations = {}
        for prop, result in report["properties"].items():
            entry = properties.setdefault(prop, {"checks": 0, "violations": 0, "runs_checked": 0,
                                                 "runs_violated": []})
            entry["checks"] += result["checks"]
            entry["violations"] += result["violations"]
            entry["runs_checked"] += int(result["checks"] > 0)
            if result["violations"]:
                entry["runs_violated"].append(name)
                run_violations[prop] = result["violations"]
                for example in result["examples"]:
                    violations.append({"run": name, "property": prop, "sequence": example["sequence"],
                                       "call": example["call"], "site": site_name(sites, example["site"]),
                                       "stack": stack_functions(sites, example["stack"]),
                                       "detail": example["detail"]})
        not_inherited = [g for g in report["live_at_exit"] if not g["inherited"]]
        for generation in not_inherited:
            key = (generation["origin_call"], site_name(sites, generation["origin_site"]),
                   " <- ".join(stack_functions(sites, generation["origin_stack"])[:6]))
            live[key]["runs"].add(name)
            live[key]["generations"] += 1
            live[key]["kinds"].add(generation["kind"])
            if generation["origin_path"]:
                live[key]["paths"].add(generation["origin_path"])
        rows.append({"name": name, "kind": run["kind"], "returncode": run["returncode"],
                     "sigterm": run["sigterm_sent"], "records": report["records"],
                     "acquisitions": report["acquisitions"], "releases": report["releases"],
                     "injected_failures": report["injected_failures"],
                     "injection_reached_function": run.get("injection_reached_function"),
                     "violations": run_violations, "live_at_exit_not_inherited": len(not_inherited),
                     "untracked_uses": report["untracked_uses"],
                     "untracked_releases": report["untracked_releases"]})
    aggregate = {
        "schema_version": 1,
        "runs": len(summary["runs"]),
        "accepted_runs": len(rows),
        "rejected_runs": rejected,
        "totals": dict(sorted(totals.items())),
        "properties": properties,
        "violations": violations,
        "live_at_exit_by_origin": [
            {"call": call, "site": site, "stack": stack, "runs": sorted(value["runs"]),
             "run_count": len(value["runs"]), "generations": value["generations"],
             "kinds": sorted(value["kinds"]), "paths": sorted(value["paths"])}
            for (call, site, stack), value in sorted(live.items(), key=lambda item: (-len(item[1]["runs"]), item[0]))
        ],
        "per_run": rows,
        "static_certificate": False,
    }
    text = json.dumps(aggregate, indent=1) + "\n"
    if len(sys.argv) > 2:
        pathlib.Path(sys.argv[2]).write_text(text)
    print(json.dumps({"runs": aggregate["runs"], "accepted": aggregate["accepted_runs"],
                      "rejected": len(rejected), "totals": aggregate["totals"],
                      "properties": {k: {"checks": v["checks"], "violations": v["violations"],
                                         "runs_violated": len(v["runs_violated"])} for k, v in properties.items()}},
                     indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
