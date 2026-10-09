#!/usr/bin/env python3
"""tip_compare.py BEFORE-DIR AFTER-DIR -- DECLARED-ASSUMPTION-BODY-CHECKS: the
mk_vhost_set probe (tip-probe.sh) before and after: for tip.json and
master.json, whether every row's (function, status, diagnostics) is identical,
and mk_vhost_set's and mk_vhost_set_property's status, diagnostic count and
dependencies on both."""
import hashlib
import json
import pathlib
import sys


def rows(path):
    report = json.loads(pathlib.Path(path).read_text())
    out = []
    for unit in report.get("units", []):
        for function in unit.get("functions", []):
            out.append((function["function"], function["status"],
                        json.dumps(function.get("diagnostics", []), sort_keys=True)))
    return out, report


for name in ("tip.json", "master.json"):
    before, _ = rows(pathlib.Path(sys.argv[1]) / name)
    after, report = rows(pathlib.Path(sys.argv[2]) / name)
    digest = hashlib.sha256(json.dumps(after).encode()).hexdigest()[:8]
    print(f"{name}: verdict rows identical: {before == after} ({digest}…)")
    for unit in report.get("units", []):
        for function in unit.get("functions", []):
            if function["function"] in ("mk_vhost_set", "mk_vhost_set_property"):
                print("   ", function["function"], function["status"],
                      len(function.get("diagnostics", [])),
                      json.dumps(function.get("contract_dependencies")))
