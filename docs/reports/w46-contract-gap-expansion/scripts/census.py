#!/usr/bin/env python3
"""Inventory requested-family uncontracted-call leaves, without inferring safety.

Usage: census.py DIAGNOSTICS OUTPUT [--root REPO]
Input is a plain or gzip audit report. Declaration inputs come from git's index;
an existing declaration may belong to an observation interpreter, be unselected,
or be withdrawn, so declaration presence is not contract availability.
"""

import argparse
import collections
import gzip
import hashlib
import json
import pathlib
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("diagnostics", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    args = parser.parse_args()
    root = args.root.resolve()
    paths = subprocess.check_output(
        ["git", "ls-files", "-z", "providers"], cwd=root
    ).decode().split("\0")
    declarations = collections.defaultdict(list)
    libc = set()
    for name in paths:
        if not name.endswith((".qcore", ".qv")):
            continue
        text = (root / name).read_text()
        for symbol in re.findall(r"\bextern\s+fn\s+(\w+)\s*\(", text):
            declarations[symbol].append(name)
            if "/contracts/libc/" in name:
                libc.add(symbol)
    libc_names = root / "providers/quirin-provider-libc/libc-names.txt"
    if libc_names.exists():
        libc.update(re.findall(r"^[A-Za-z_]\w*$", libc_names.read_text(), re.M))

    def family(name):
        if name in libc:
            return "libc"
        for prefixes, group in [
            (("event_", "evutil_", "evbuffer_", "bufferevent_"), "libevent"),
            (("mk_", "_mk_", "MK_"), "monkey"),
            (("cmt_",), "cmetrics"),
            (("ctr_",), "ctraces"),
            (("flb_input_chunk_", "input_chunk_"), "input_chunk"),
            (("flb_output_", "flb_task_", "flb_engine_", "flb_sched_"), "flushing_events"),
        ]:
            if name.startswith(prefixes):
                return group
        return None

    raw = args.diagnostics.read_bytes()
    report = json.loads(gzip.decompress(raw) if args.diagnostics.suffix == ".gz" else raw)
    rows = {}
    for unit_index, unit in enumerate(report["units"]):
        for function in unit.get("functions", []):
            diagnostics = function.get("diagnostics", [])
            matches = [re.fullmatch(r"call to uncontracted function `([^`]+)`", d["description"])
                       for d in diagnostics]
            for diagnostic, match in zip(diagnostics, matches):
                if not match or diagnostic["category"] != "unsupported":
                    continue
                name = match[1]
                group = family(name)
                if group is None:
                    continue
                row = rows.setdefault(name, {
                    "family": group, "callee": name, "sites": [],
                    "alone_blocked_functions": [], "existing_declaration_modules": declarations[name],
                })
                identity = {"unit_index": unit_index, "file": unit["file"],
                            "function": function["function"], "status": function["status"]}
                row["sites"].append(dict(identity, line=diagnostic.get("line"), origin=diagnostic.get("origin")))
                if (function["status"].lower() == "unknown" and all(matches)
                        and all(m[1] == name for m in matches)
                        and identity not in row["alone_blocked_functions"]):
                    row["alone_blocked_functions"].append(identity)
    ordered = sorted(rows.values(), key=lambda r: (
        -len(r["alone_blocked_functions"]), -len(r["sites"]), r["family"], r["callee"]))
    totals = {}
    for group in ["libc", "libevent", "monkey", "cmetrics", "ctraces", "input_chunk", "flushing_events"]:
        selected = [r for r in ordered if r["family"] == group]
        totals[group] = {"callees": len(selected), "call_leaves": sum(len(r["sites"]) for r in selected),
                         "alone_blocked_functions": sum(len(r["alone_blocked_functions"]) for r in selected)}
    result = {"schema_version": 1, "input": str(args.diagnostics),
              "input_sha256": hashlib.sha256(raw).hexdigest(), "unit_count": len(report["units"]),
              "scope": "Exact uncontracted-call diagnostics in requested symbol families; not an exhaustive API inventory or safety coverage.",
              "families": totals, "callees": ordered}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(totals, indent=2))


if __name__ == "__main__":
    main()
