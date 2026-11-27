#!/usr/bin/env python3
"""Compare two runs of the four census readings (census.py outputs).

Usage: compare_census.py BEFORE AFTER

Per reading: exit, SHA-256, site tallies, edges, seeds; every site whose
verdict or first reason moved; every top-level or `typed` key that differs,
with the fixed test's instance-flag reason printed in full when it differs.
Read-only.
"""

import hashlib
import json
import pathlib
import sys

READINGS = ["fixed", "fixed-configured", "previous", "previous-configured"]
TEST = "tests/runtime/in_tail.c::cb_tail_pause_test"


def load(run, name):
    path = run / f"{name}.json"
    return path, json.loads(path.read_text())


def tally(data):
    counts = {}
    for site in data["sites"]:
        verdict = site["result"]["verdict"]
        counts[verdict] = counts.get(verdict, 0) + 1
    return counts


def main():
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    for name in READINGS:
        (bp, b), (ap, a) = load(before, name), load(after, name)
        bh = hashlib.sha256(bp.read_bytes()).hexdigest()
        ah = hashlib.sha256(ap.read_bytes()).hexdigest()
        print(f"== {name}: exits {(before / f'{name}.exit').read_text().strip()} / "
              f"{(after / f'{name}.exit').read_text().strip()}; sha256 {bh[:12]} / {ah[:12]}"
              f"{' (identical)' if bh == ah else ''}")
        print(f"   sites {tally(b)} / {tally(a)}; edges {b['edges']} / {a['edges']}; "
              f"seeds {b['unresolved_seeds']} / {a['unresolved_seeds']}")
        keyed = lambda d: {(s["file"], s["line"], s["callee"], s["function"]): s["result"] for s in d["sites"]}
        bs, as_ = keyed(b), keyed(a)
        moved = 0
        for key in sorted(set(bs) | set(as_)):
            x, y = bs.get(key), as_.get(key)
            first = lambda r: (r or {}).get("verdict"), lambda r: ((r or {}).get("reasons") or [None])[0]
            if x is None or y is None or x.get("verdict") != y.get("verdict") or \
                    (x.get("reasons") or [None])[0] != (y.get("reasons") or [None])[0]:
                moved += 1
                print(f"   MOVED {key}: {x} -> {y}")
        print(f"   sites moved (verdict or first reason): {moved}")
        for key in sorted(set(b) | set(a)):
            if key == "typed":
                continue
            if b.get(key) != a.get(key):
                print(f"   top-level key differs: {key}")
        bt, at = b.get("typed", {}), a.get("typed", {})
        for key in sorted(set(bt) | set(at)):
            if bt.get(key) != at.get(key):
                print(f"   typed key differs: {key}")
        bf = bt.get("instance_flags", {}).get(TEST)
        af = at.get("instance_flags", {}).get(TEST)
        if bf != af:
            print(f"   instance flag BEFORE: {bf}")
            print(f"   instance flag AFTER:  {af}")


if __name__ == "__main__":
    main()
