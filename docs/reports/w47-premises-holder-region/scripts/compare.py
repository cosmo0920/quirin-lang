#!/usr/bin/env python3
"""Attribute every difference between the BEFORE and AFTER census readings.

Usage: compare.py BEFORE_DIR AFTER_DIR

BEFORE_DIR is W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS's census.py output at the
base (raw, configured with the predefined facts, configured control); AFTER_DIR
is this card's census.py output (raw, configured control, configured with the
predefined facts alone, configured with both facts). For each pair it prints
both SHA-256s, the site counts, edges, functions and seeds, every site whose
verdict or first reason moved, whether the two JSONs are equal once every
instance flag's reason text is set aside, and every instance flag whose
derivation or reason changed. Read-only.
"""

import collections
import copy
import hashlib
import json
import pathlib
import sys

PAIRS = [
    ("fixed.json", "fixed.json"),
    ("fixed-configured-control.json", "fixed-configured-control.json"),
    ("fixed-configured.json", "fixed-configured-predefined.json"),
    ("fixed-configured.json", "fixed-configured.json"),
    ("previous.json", "previous.json"),
    ("previous-configured-control.json", "previous-configured-control.json"),
    ("previous-configured.json", "previous-configured-predefined.json"),
    ("previous-configured.json", "previous-configured.json"),
]


def summary(data):
    counts = collections.Counter(s["result"]["verdict"] for s in data["sites"])
    return (f"discharged {counts['discharged']} / not {counts['not-discharged']} / "
            f"unresolved {counts['unresolved']}; edges {data['edges']}; "
            f"functions {data['functions']}; seeds {data['unresolved_seeds']}")


def without_reasons(data):
    data = copy.deepcopy(data)
    for flag in data.get("typed", {}).get("instance_flags", {}).values():
        flag["reason"] = ""
    data.pop("region_macros", None)
    return data


def main():
    before_dir, after_dir = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    for before_name, after_name in PAIRS:
        a_bytes = (before_dir / before_name).read_bytes()
        b_bytes = (after_dir / after_name).read_bytes()
        a, b = json.loads(a_bytes), json.loads(b_bytes)
        print(f"== {before_name} -> {after_name}")
        print(f"   before {hashlib.sha256(a_bytes).hexdigest()[:16]}: {summary(a)}")
        print(f"   after  {hashlib.sha256(b_bytes).hexdigest()[:16]}: {summary(b)}")
        print(f"   equal but the instance flags' reasons: "
              f"{without_reasons(a) == without_reasons(b)}")
        old = {(s["file"], s["line"], s["callee"]): s["result"] for s in a["sites"]}
        new = {(s["file"], s["line"], s["callee"]): s["result"] for s in b["sites"]}
        if old.keys() != new.keys():
            print("   site sets differ:", sorted(old.keys() ^ new.keys()))

        def first(r):
            return (r.get("reasons") or [None])[0]
        moved = 0
        for key in sorted(old.keys() & new.keys()):
            x, y = old[key], new[key]
            if x["verdict"] != y["verdict"] or first(x) != first(y):
                moved += 1
                print(f"   MOVED {key[0]}:{key[1]} {key[2]}: {x['verdict']} -> {y['verdict']}")
        print(f"   sites moved: {moved}")
        fa = a.get("typed", {}).get("instance_flags", {})
        fb = b.get("typed", {}).get("instance_flags", {})
        for name in sorted(set(fa) | set(fb)):
            x, y = fa.get(name), fb.get(name)
            if x != y:
                print(f"   FLAG {name}: {x and x['derived']} -> {y and y['derived']}")
                if y:
                    print(f"      after: {y['reason'][:600]}")
        if "region_macros" in b:
            print(f"   region_macros: read={b['region_macros'].get('read')} "
                  f"{b['region_macros'].get('reason', '')}")


if __name__ == "__main__":
    main()
