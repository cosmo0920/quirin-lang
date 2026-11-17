#!/usr/bin/env python3
"""leaves.py REPORT -- CG22-CALLBACK-CANDIDATE-CONTRACTS: every
`callback-uncontracted` leaf in REPORT (a run directory or its
resource-diagnostics.json): the function carrying it (unit::function, its
status, whether this key is its only kind of leaf), the callee, and each
candidate the refusal names with that candidate's rows (unit: status) or
`no row`. A candidate is in force in the caller's unit only through a
declaration or a validated summary derived there; a row in another unit is
not a contract in force at the caller (rule 4c needs a stated link premise)."""
import collections
import json
import pathlib
import re
import sys

KEY = "(callback-uncontracted)"
CALLEE = re.compile(r"^call to `([^`]*)`")
CANDIDATE = re.compile(r"callback `([^`]*)` \(from")


def main():
    path = pathlib.Path(sys.argv[1])
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    report = json.loads(path.read_text())
    rows = collections.defaultdict(list)
    for unit in report["units"]:
        for function in unit["functions"]:
            rows[function["function"]].append((unit["file"], function["status"]))
    per_candidate = collections.Counter()
    for unit in report["units"]:
        for function in unit["functions"]:
            leaves = [d["description"] for d in function.get("diagnostics") or []]
            mine = [d for d in leaves if d.endswith(KEY)]
            if not mine:
                continue
            alone = all(d.endswith(KEY) for d in leaves)
            print("%s::%s %s%s (%d leaves)" % (unit["file"], function["function"], function["status"],
                                                " ALONE" if alone else "", len(leaves)))
            for d in mine:
                callee = CALLEE.match(d)
                named = CANDIDATE.findall(d)
                for name in named:
                    per_candidate[name] += 1
                status = "; ".join(
                    "%s=%s" % (n, ",".join("%s:%s" % r for r in rows.get(n, [])) or "no row")
                    for n in named)
                print("    %s -> %s" % (callee.group(1) if callee else "?", status))
    print("candidates named: %d" % len(per_candidate))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
