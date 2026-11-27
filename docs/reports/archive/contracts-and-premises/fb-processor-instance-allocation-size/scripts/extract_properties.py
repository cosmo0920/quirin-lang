#!/usr/bin/env python3
"""extract_properties.py PROPERTIES_DIR OUT.json -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE:
what the existing custody, bounds and null properties say (properties.sh's
runs) of the allocation's own path: for each route, the totals, every
function with a null finding (Unsafe), and for the path's functions the
custody status and reasons, the bounds row's verdict and reason codes with the
instance's access subjects, and the null row's checks whose subject is the
instance (`instance`, `ins`, `p_ins`) with their outcome, sources and triage.
Rows are kept per unit variant (static, shared) in report order."""
import collections
import json
import os
import sys

PATH = {"flb_processor_instance_create", "flb_processor_instance_destroy",
        "flb_processor_unit_create", "flb_processor_unit_destroy", "flb_processor_unit_init",
        "flb_processor_instance_init", "flb_processor_instance_exit",
        "flb_processor_instance_set_property", "flb_processor_unit_set_property"}
SUBJECTS = ("instance", "ins", "p_ins")

root, out = sys.argv[1:]
result = {}
for route in sorted(os.listdir(root)):
    base = os.path.join(root, route)
    if not os.path.isdir(base):
        continue
    exit_file = os.path.join(root, route + ".exit")
    entry = {"exit": open(exit_file).read().strip() if os.path.exists(exit_file) else None}
    if not os.path.exists(os.path.join(base, "null.jsonl")):
        result[route] = entry
        continue
    diag = json.load(open(os.path.join(base, "resource-diagnostics.json")))
    custody = collections.defaultdict(list)

    def walk(node):
        if isinstance(node, dict):
            if "function" in node and "status" in node:
                diags = collections.Counter(
                    f"{d.get('category')}: {d.get('description')}" for d in node.get("diagnostics", []))
                custody[node["function"]].append({"status": node["status"],
                                                  "diagnostics": sorted(diags.items())})
            for v in node.values():
                walk(v)
        elif isinstance(node, list):
            for v in node:
                walk(v)
    walk(diag)
    null_rows = [json.loads(l) for l in open(os.path.join(base, "null.jsonl"))]
    bounds_rows = [json.loads(l) for l in open(os.path.join(base, "bounds.jsonl"))]
    entry["null_summary"] = json.load(open(os.path.join(base, "null.summary.json"))).get("combined")
    entry["null_findings"] = [{"function": r["function"], "file": r.get("file"), "null": r["null"].get("verdict"),
                               "findings": [c for c in r["null"]["checks"] if c["outcome"] == "finding"],
                               "triage": r["null"].get("triage") or []}
                              for r in null_rows
                              if any(c["outcome"] == "finding" for c in r["null"]["checks"])]
    entry["bounds_summary"] = json.load(open(os.path.join(base, "bounds.summary.json")))["tally"]
    path = []
    for fn in sorted(PATH):
        nulls = [r for r in null_rows if r["function"] == fn]
        bounds = [r for r in bounds_rows if r["function"] == fn]
        for i, n in enumerate(nulls):
            b = bounds[i] if i < len(bounds) else None
            path.append({
                "function": fn, "variant": i,
                "custody": n.get("custody"), "custody_rows": custody.get(fn, [])[i:i + 1],
                "null_verdict": n["null"].get("verdict"),
                "combined_null": n.get("combined"),
                "null_reasons": n["null"].get("reasons"),
                "null_instance_checks": [
                    {k: c.get(k) for k in ("id", "kind", "origin", "outcome", "sources", "proofs", "subject")}
                    for c in n["null"]["checks"]
                    if c.get("subject", "").split("->")[0].lstrip("&") in SUBJECTS],
                "null_triage": n["null"].get("triage") or [],
                "bounds_verdict": None if b is None else b["bounds"].get("verdict", b.get("combined")),
                "bounds_reason_codes": None if b is None else sorted(collections.Counter(
                    a["actual"].split(":")[0] for a in b["bounds"]["accesses"]).items()),
                "bounds_instance_subjects": None if b is None else sorted(collections.Counter(
                    a["actual"] for a in b["bounds"]["accesses"]
                    if "`instance`" in a["actual"] or "`ins`" in a["actual"] or "`p_ins`" in a["actual"]).items()),
            })
    entry["path"] = path
    result[route] = entry
with open(out, "w") as handle:
    json.dump(result, handle, indent=1)
    handle.write("\n")
for route, e in result.items():
    print(route, "exit", e["exit"], "null", e.get("null_summary"), "bounds", e.get("bounds_summary", {}).get("bounds"),
          "null findings in", sorted({f["function"] for f in e.get("null_findings", [])}))
    for p in e.get("path", []):
        print("  ", p["function"], p["variant"], "custody", p["custody"], "null", p["null_verdict"], p["null_reasons"], "joined", p["combined_null"],
              "bounds", p["bounds_verdict"], "instance null checks",
              collections.Counter(c["outcome"] for c in p["null_instance_checks"]))
