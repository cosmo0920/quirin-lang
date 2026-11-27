#!/usr/bin/env python3
"""CG05-NESTED-CALL-EFFECTS: the system-route and tail-pause-slice
comparisons of one BEFORE/AFTER pair (run names under private/cg05 and the
shared slice attempt), written as artifacts/{system,slice}-comparison.json.

usage: routes.py BEFORE_NAME AFTER_NAME
"""

import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
PRIVATE = REPO / "private/cg05"
SLICE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
ARTIFACTS = HERE.parent / "artifacts"

before, after = sys.argv[1], sys.argv[2]
pairs = {
    "system": (PRIVATE / f"{before}-system/resource-diagnostics.json",
               PRIVATE / f"{after}-system/resource-diagnostics.json"),
    "slice": (SLICE / f"cg05-{before}/resource-diagnostics.json",
              SLICE / f"cg05-{after}/resource-diagnostics.json"),
}
for name, (b, a) in pairs.items():
    out = ARTIFACTS / f"{name}-comparison.json"
    subprocess.run([sys.executable, str(HERE / "compare.py"), str(b), str(a), "--json", str(out)],
                   check=True, capture_output=True)
    d = json.loads(out.read_text())
    print(name, d["totals"], d["move_counts"], d["same_status_functions_with_changed_diagnostics"])
    print("   ", d["leaf_delta"])
    for path in (b, a):
        print("   ", subprocess.run(["sha256sum", str(path)], capture_output=True, text=True).stdout.strip())
