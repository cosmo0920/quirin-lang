#!/usr/bin/env python3
"""identity.py BASE.json TIP.json -- FB-BACKLOG-ACCOUNTING: compare two opt-in
`current` reports row by row after blanking what names the measuring checkout.

`evidence_version` carries the enclosing repository's HEAD, so it differs
between any two runs from different commits, and every digest derived from it
(64-hex strings, the reading-site environment keys) differs with it; the
reading-site evidence lists are ordered by those digests. The comparison
therefore blanks `evidence_version` and 64-hex strings, drops each
observation's `environment` key, compares each function's reading-site
evidence as a multiset and every other function field exactly, and compares
unit statuses and environment counts."""
import json
import re
import sys

HEX = re.compile(r"[0-9a-f]{64}")


def norm(value):
    if isinstance(value, dict):
        return {k: ("<v>" if k == "evidence_version" else norm(v))
                for k, v in value.items() if k != "environment"}
    if isinstance(value, list):
        return [norm(v) for v in value]
    if isinstance(value, str):
        return HEX.sub("<h>", value)
    return value


def main() -> int:
    base = json.load(open(sys.argv[1]))
    tip = json.load(open(sys.argv[2]))
    differing = []
    rows = 0
    if len(base["units"]) != len(tip["units"]):
        differing.append("unit count")
    for ua, ub in zip(base["units"], tip["units"]):
        unit = ua.get("file")
        if ua.get("file") != ub.get("file") or ua.get("status") != ub.get("status"):
            differing.append(f"unit {unit}")
        envs = [len(u.get("reading_site_environments", {}).get("environments", {})) for u in (ua, ub)]
        if envs[0] != envs[1]:
            differing.append(f"environment count {unit}")
        if len(ua["functions"]) != len(ub["functions"]):
            differing.append(f"function count {unit}")
        for fa, fb in zip(ua["functions"], ub["functions"]):
            rows += 1
            evidence = [sorted(json.dumps(norm(x), sort_keys=True)
                               for x in f.get("reading_site_evidence") or []) for f in (fa, fb)]
            rest = [norm({k: v for k, v in f.items() if k != "reading_site_evidence"}) for f in (fa, fb)]
            if evidence[0] != evidence[1] or rest[0] != rest[1]:
                differing.append(f"{unit}::{fa.get('function')}")
    print(json.dumps({"function_rows": rows, "differing": differing,
                      "identical": not differing}, indent=1))
    return 0 if not differing else 1


if __name__ == "__main__":
    sys.exit(main())
