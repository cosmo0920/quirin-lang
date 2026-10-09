#!/usr/bin/env python3
"""readings.py AFTER-PARITY.jsonl BEFORE-PARITY.jsonl [OUT.json] -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: the rules' own consultations on
one route, deduplicated by (function, line, tokens) (the W4 fixed point reads a
body more than once): rule 14a and 14b readings (`residual-access`, admitted or
not), and the call actuals the typed reader admitted in AFTER but never in
BEFORE, split into element addresses (14c) and interior offsets (14d)."""
import collections
import json
import pathlib
import sys


def main() -> int:
    readings = {}
    admitted_after, admitted_before = set(), set()
    for path, sink in ((sys.argv[1], admitted_after), (sys.argv[2], admitted_before)):
        with open(path, encoding="utf-8") as stream:
            for line in stream:
                if '"residual-access"' not in line and '"call-argument"' not in line:
                    continue
                row = json.loads(line)
                key = (row.get("function"), row.get("line"), tuple(row.get("texts") or []))
                if row.get("position") == "residual-access" and path == sys.argv[1]:
                    readings[key] = (row.get("rule"), bool(row.get("new_admitted")))
                elif row.get("position") == "call-argument" and row.get("new_admitted"):
                    sink.add(key)
    counts = collections.Counter((rule, admitted) for rule, admitted in readings.values())
    new = admitted_after - admitted_before
    elements = sorted(k for k in new if k[2][:1] == ("&",) and "[" in k[2])
    offsets = sorted(k for k in new if k not in set(elements))
    record = {
        "readings": {f"{rule} {'admitted' if a else 'declined'}": n
                     for (rule, a), n in sorted(counts.items())},
        "new_actuals_14c": len(elements),
        "new_actuals_14d": len(offsets),
        "examples_14c": [[f, l, " ".join(t)] for f, l, t in elements[:10]],
        "examples_14d": [[f, l, " ".join(t)] for f, l, t in offsets[:10]],
    }
    print(json.dumps(record, indent=1))
    if len(sys.argv) > 3:
        pathlib.Path(sys.argv[3]).write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
