#!/usr/bin/env python3
"""runs.py OUT.json -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: the identity and
totals of every run (before: base binary, base contracts; stable: base
binary, the four stable declarations; after: tip binary, every declaration):
binary sha256, wall time inside the lock, exit, custody report sha256 (the
system and slice reports name the worktree's absolute paths, so their digests
differ from another worktree's), custody and null verdicts, check outcomes,
and the fold's Fluent Bit stable standings."""
import hashlib
import json
import os
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "../../../.."))
P = os.path.join(REPO, "private/fbn")
SLICE = "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1"


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def read(path):
    try:
        return open(path).read().strip()
    except OSError:
        return None


out = {}
for name in ("before", "stable", "after"):
    for route in ("corpus", "system", "slice"):
        stem = name if route == "corpus" else f"{name}-{route}"
        summary_path = os.path.join(P, f"{stem}-null.summary.json")
        if not os.path.exists(summary_path):
            continue
        report = {"corpus": os.path.join(P, name, "resource-diagnostics.json"),
                  "system": os.path.join(P, f"{name}-system", "resource-diagnostics.json"),
                  "slice": os.path.join(SLICE, f"fbn-{name}", "resource-diagnostics.json")}[route]
        summary = json.load(open(summary_path))
        stable = {k: {"standing": v["standing"], "stores_examined": v["stores_examined"],
                      "stores_not_proved": v["stores_not_proved"], "whole_object_stores": v["whole_object_stores"]}
                  for k, v in (summary.get("fold", {}).get("stable_declarations") or {}).items() if k.startswith("flb_")}
        record = {
            "binary_sha256": (read(os.path.join(P, f"{name}.binary.sha256")) or "").split()[0],
            "exit": read(os.path.join(P, f"{stem}.exit")),
            "seconds_inside_lock": read(os.path.join(P, f"{stem}.seconds")),
            "report_sha256": sha(report),
            "custody": summary["custody"],
            "null": summary["null"],
            "checks_by_outcome": summary["checks"]["by_outcome"],
            "fold_withdrawn": summary.get("fold", {}).get("withdrawn"),
            "fluent_bit_stable_declarations": stable,
        }
        if route == "corpus":
            run = json.load(open(os.path.join(P, name, "run.json")))
            record["run_json_binary_sha256"] = run.get("binary_sha256")
        out[f"{name}/{route}"] = record
text = json.dumps(out, indent=1, sort_keys=True)
open(sys.argv[1], "w").write(text + "\n")
print(text[:400])
