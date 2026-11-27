#!/usr/bin/env python3
"""Compare two census runs site by site, and two typed-fact readings key by key.

Usage: compare_census.py BEFORE_DIR AFTER_DIR [BEFORE_FACTS AFTER_FACTS]

For each of the four readings (fixed, previous, raw and configured) prints
the verdict tally, edges and unresolved seeds on both sides, every site whose
verdict or first reason changed, and every changed instance flag. With the
fact directories, reports whether each fact kind of the earlier reading is
unchanged in the later one (the later adds kinds; it must not change any).
Exits 0. Read-only.
"""

import collections
import gzip
import json
import pathlib
import sys


def load(path):
    path = pathlib.Path(path)
    if not path.exists() and path.with_suffix(".json.gz").exists():
        path = path.with_suffix(".json.gz")
    data = path.read_bytes()
    if path.suffix == ".gz":
        data = gzip.decompress(data)
    return json.loads(data)


def main():
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    for name in ("fixed", "previous", "fixed-configured", "previous-configured"):
        b, a = load(before / f"{name}.json"), load(after / f"{name}.json")
        tally = lambda d: dict(collections.Counter(s["result"]["verdict"] for s in d["sites"]))
        print(f"== {name}: sites {tally(b)} -> {tally(a)}; edges {b['edges']} -> {a['edges']}; "
              f"seeds {b['unresolved_seeds']} -> {a['unresolved_seeds']}")
        key = lambda s: (s["file"], s["line"], s["function"], s["callee"])
        bs = {key(s): s for s in b["sites"]}
        for site in a["sites"]:
            old = bs.get(key(site))
            if old is None:
                print("   new site", key(site))
                continue
            ov, nv = old["result"], site["result"]
            if ov["verdict"] != nv["verdict"] or ov.get("reasons", [""])[:1] != nv.get("reasons", [""])[:1]:
                print(f"   {site['file']}:{site['line']} {ov['verdict']} -> {nv['verdict']}")
                if nv["verdict"] == "unresolved":
                    print("      now:", nv["reasons"][0][:400])
        bf = b.get("typed", {}).get("instance_flags", {})
        af = a.get("typed", {}).get("instance_flags", {})
        for function in sorted(set(bf) | set(af)):
            if bf.get(function) != af.get(function):
                print(f"   flag {function}: {bf.get(function, {}).get('derived')} -> "
                      f"{af.get(function, {}).get('derived')}")
        same_rest = {k: v for k, v in b.items() if k not in ("typed", "sites")} == \
            {k: v for k, v in a.items() if k not in ("typed", "sites")}
        print(f"   every other top-level key equal: {same_rest}")
    if len(sys.argv) > 4:
        for name in ("library", "fixed-test", "previous-test"):
            b = load(pathlib.Path(sys.argv[3]) / f"{name}.json")
            a = load(pathlib.Path(sys.argv[4]) / f"{name}.json")
            kinds = {kind: b[kind] == a.get(kind) for kind in b}
            added = sorted(set(a) - set(b))
            print(f"== facts {name}: earlier kinds unchanged {kinds}; added {added}")


if __name__ == "__main__":
    main()
