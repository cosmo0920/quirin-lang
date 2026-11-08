#!/usr/bin/env python3
"""Card LOOP-REACQUISITION-LEAK-ACCUSATION: keep a census run's evidence.

Copies, from a census run directory, the witness lines of the named functions
(gzip, deterministic), their report rows and the run's unit statuses, and
the run's identity (invocation sidecar digests, slot receipt).

Usage: extract.py RUN_DIR OUT_DIR FUNCTION [FUNCTION ...]
"""
import gzip
import hashlib
import json
import os
import sys


def main(run, out, functions):
    os.makedirs(out, exist_ok=True)
    wanted = set(functions)
    lines = []
    with open(os.path.join(run, "leak-witness.jsonl")) as handle:
        for raw in handle:
            if json.loads(raw).get("function") in wanted:
                lines.append(raw)
    with open(os.path.join(out, "leak-witness.jsonl.gz"), "wb") as raw_out:
        with gzip.GzipFile(fileobj=raw_out, mode="wb", mtime=0) as handle:
            handle.write("".join(lines).encode())
    report_path = os.path.join(run, "resource-diagnostics.json")
    with open(report_path) as handle:
        report = json.load(handle)
    rows = []
    units = []
    for unit in report["units"]:
        units.append({"file": unit.get("file"), "status": unit.get("status"),
                      "functions": {status: sum(1 for f in unit.get("functions", []) if f.get("status") == status)
                                    for status in ("Safe", "Pending", "Unknown", "Unsafe")}})
        for function in unit.get("functions", []):
            if function.get("function") in wanted:
                rows.append({"file": unit.get("file"), "function": function["function"],
                             "status": function.get("status"),
                             "diagnostics": function.get("diagnostics", [])})
    with open(os.path.join(run, "census.slot.json")) as handle:
        receipt = json.load(handle)
    with open(os.path.join(run, "resource-diagnostics.invocation.json")) as handle:
        invocation = json.load(handle)
    with open(report_path, "rb") as handle:
        digest = hashlib.sha256(handle.read()).hexdigest()
    with open(os.path.join(out, "rows.json"), "w") as handle:
        json.dump({"report_sha256": digest, "units": units, "rows": rows,
                   "argv": invocation.get("argv"), "environment": invocation.get("environment"),
                   "clang": invocation.get("clang"),
                   "slot": {key: receipt.get(key) for key in ("class", "exit", "argv_run", "max_rss_kib", "started", "finished")}},
                  handle, indent=1)
        handle.write("\n")
    print(f"{len(lines)} witness lines, {len(rows)} rows -> {out}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], sys.argv[3:])
