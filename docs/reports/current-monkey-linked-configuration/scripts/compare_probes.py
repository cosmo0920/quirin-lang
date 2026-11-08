#!/usr/bin/env python3
"""Per-function comparison of the two prediction probes.

usage: compare_probes.py A_REPORT B_REPORT A_PREPASS B_PREPASS OUT_JSON

A is the standalone Monkey run (as the user's run reads it, label
`monkey-libevent`). B is the linked configuration through an explicit
selection (label `monkey`). Both ran the base binary. For every Monkey file,
the script pairs each function's row by name and reports its status in each
run and whether its diagnostics (reason codes and texts, order-insensitive)
are equal. It also compares the two member pre-pass sections (`units`,
`read`, and every other top-level key) to show what the standalone units
added to the run's set. Auxiliary; decides nothing.
"""
import collections
import json
import sys

a_path, b_path, a_prepass, b_prepass, out = sys.argv[1:6]
a = json.load(open(a_path))
b = json.load(open(b_path))


def rows(report, library):
    found = {}
    for unit in report["units"]:
        if unit["library"] != library:
            continue
        functions = {}
        for function in unit.get("functions", []):
            diagnostics = sorted(
                json.dumps(
                    {key: value for key, value in diagnostic.items() if key in ("reason_code", "reason", "message", "kind", "code")},
                    sort_keys=True,
                )
                for diagnostic in function.get("diagnostics", [])
            )
            functions[function["function"]] = {"status": function["status"], "diagnostics": diagnostics}
        found[unit["file"]] = {
            "status": unit["status"],
            "reason": unit.get("reason"),
            "functions": functions,
            "not_projected": unit.get("not_projected", []),
        }
    return found


standalone = rows(a, "monkey-libevent")
linked = rows(b, "monkey")
units = []
moves = collections.Counter()
for file in sorted(set(standalone) | set(linked)):
    s = standalone.get(file)
    l = linked.get(file)
    row = {"file": file, "standalone": s and s["status"], "linked": l and l["status"]}
    changed = []
    if s and l:
        for name in sorted(set(s["functions"]) | set(l["functions"])):
            fs = s["functions"].get(name)
            fl = l["functions"].get(name)
            if fs is None or fl is None:
                changed.append({"function": name, "standalone": fs and fs["status"], "linked": fl and fl["status"]})
                moves[f"{fs and fs['status']} -> {fl and fl['status']}"] += 1
            elif fs != fl:
                changed.append(
                    {
                        "function": name,
                        "standalone": fs["status"],
                        "linked": fl["status"],
                        "diagnostics_equal": fs["diagnostics"] == fl["diagnostics"],
                        "only_standalone": sorted(set(fs["diagnostics"]) - set(fl["diagnostics"])),
                        "only_linked": sorted(set(fl["diagnostics"]) - set(fs["diagnostics"])),
                    }
                )
                moves[f"{fs['status']} -> {fl['status']}" + ("" if fs["status"] != fl["status"] else " (diagnostics)")] += 1
            else:
                moves["unchanged"] += 1
        row["functions_changed"] = changed
    else:
        side = s or l
        row["functions"] = {name: value["status"] for name, value in side["functions"].items()}
        row["not_projected"] = side["not_projected"]
    units.append(row)

prepass = {}
pa = json.load(open(a_prepass))
pb = json.load(open(b_prepass))
for key in sorted(set(pa) | set(pb)):
    if pa.get(key) != pb.get(key):
        value_a, value_b = pa.get(key), pb.get(key)
        if isinstance(value_a, (int, str, float)) or isinstance(value_b, (int, str, float)):
            prepass[key] = {"standalone": value_a, "linked": value_b}
        else:
            prepass[key] = "differs"

result = {"function_moves": dict(moves), "units": units, "prepass_differences": prepass}
json.dump(result, open(out, "w"), indent=1, sort_keys=True)
print(json.dumps(result["function_moves"], indent=1))
for row in units:
    if row.get("functions_changed") or "functions" in row:
        print(row["file"], row["standalone"], "->", row["linked"], json.dumps(row.get("functions_changed", row.get("functions")))[:900])
print(json.dumps(prepass, indent=1)[:1500])
