#!/usr/bin/env python3
"""FB-QUANT-CONTRACT-CENSUS: committed artifacts from the classified runs.

usage: summarize.py PRIVATE_DIR

Writes artifacts/patterns.json (per pattern and route: function and row
counts, total / alone / via callee, leaf texts, the rows and their sites, the
null rows) and artifacts/runs.json (the measured identities: binary, report
digests, status and null totals, wall times). The per-row leaf dump stays in
PRIVATE_DIR.
"""
import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPORT = os.path.dirname(HERE)
SLICE_RUN = "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/fqc-before"


def sha(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def read(path):
    with open(path) as f:
        return f.read().strip()


def main():
    priv = sys.argv[1]
    patterns = {}
    runs = {"binary": read(os.path.join(priv, "bin", "before.sha256")).split()[0],
            "built_from": read(os.path.join(priv, "build-before.head")), "routes": {}}
    reports = {"corpus": os.path.join(priv, "before", "resource-diagnostics.json"),
               "system": os.path.join(priv, "before-system", "resource-diagnostics.json"),
               "slice": os.path.join(SLICE_RUN, "resource-diagnostics.json")}
    seconds = {"corpus": "before.seconds", "system": "before-system.seconds", "slice": "before-slice.seconds"}
    exits = {"corpus": "before.exit", "system": "before-system.exit", "slice": "before-slice.exit"}
    for route in ("corpus", "system", "slice"):
        d = json.load(open(os.path.join(priv, f"{route}-classified.json")))
        runs["routes"][route] = {
            "report_sha256": sha(reports[route]),
            "custody_status_of_classified_rows": d["custody_status"],
            "null_verdicts_of_classified_rows": d["null_verdicts"],
            "rows_classified": d["rows"], "non_safe_leaves": d["non_safe_leaves"],
            "unclassified_leaves": d["unclassified_leaves"],
            "wall_seconds_inside_lock": float(read(os.path.join(priv, seconds[route]))),
            "exit": int(read(os.path.join(priv, exits[route]))),
            "scope": "flb_input_chunk.c rows only" if route == "corpus" else "every row of the route",
        }
        for name, p in d["patterns"].items():
            q = {k: v for k, v in p.items()}
            patterns.setdefault(name, {})[route] = q
    corpus_run = json.load(open(os.path.join(priv, "before", "run.json")))
    runs["routes"]["corpus"]["run_json_binary_sha256"] = corpus_run["binary_sha256"]
    for name, path in (("patterns.json", patterns), ("runs.json", runs)):
        with open(os.path.join(REPORT, "artifacts", name), "w", encoding="utf-8") as f:
            json.dump(path, f, indent=1, ensure_ascii=False)
            f.write("\n")
    for name, per in patterns.items():
        line = [name]
        for route in ("system", "slice", "corpus"):
            p = per[route]
            line.append(f"{route}: fn {p['custody_functions_total']}/{p['custody_functions_alone']}/"
                        f"{p['custody_functions_via_callee']} rows {p['custody_rows_total']}/{p['custody_rows_alone']} "
                        f"null fn {p['null_functions_total']}/{p['null_functions_alone']}")
        print(" | ".join(line))
    return 0


if __name__ == "__main__":
    sys.exit(main())
