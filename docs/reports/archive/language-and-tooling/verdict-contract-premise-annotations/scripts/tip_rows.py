#!/usr/bin/env python3
"""tip_rows.py BEFORE_DIR AFTER_DIR -- VERDICT-CONTRACT-PREMISE-ANNOTATIONS: the
round-12 probe's `mk_vhost_set` and `mk_vhost_set_property` rows (status,
diagnostic count, dependencies) on the tip and master copies, BEFORE against
AFTER, and whether every verdict row of each report is identical."""
import hashlib
import json
import pathlib
import sys

NAMES = ("mk_vhost_set", "mk_vhost_set_property")


def rows(path):
    data = json.loads(pathlib.Path(path).read_text())
    return [(u.get("file"), f["function"], f["status"], f.get("diagnostics"), f.get("contract_dependencies"))
            for u in data.get("units", []) for f in u.get("functions", [])]


ok = True
for copy in ("tip", "master"):
    b, a = rows(f"{sys.argv[1]}/{copy}.json"), rows(f"{sys.argv[2]}/{copy}.json")
    digest = lambda rs: hashlib.sha256(json.dumps([r[:4] for r in rs]).encode()).hexdigest()[:12]
    same = [r[:4] for r in b] == [r[:4] for r in a]
    deps_same = [r[4] for r in b] == [r[4] for r in a]
    ok = ok and same and deps_same
    print(copy, "verdict rows", digest(b), digest(a), "equal", same, "dependencies equal", deps_same)
    for r in a:
        if r[1] in NAMES:
            print(" ", r[1], r[2], len(r[3] or []), "diagnostics;", json.dumps(r[4]))
sys.exit(0 if ok else 1)
