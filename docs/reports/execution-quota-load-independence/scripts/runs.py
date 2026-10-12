#!/usr/bin/env python3
"""runs.py BEFORE AFTER -- EXECUTION-QUOTA-LOAD-INDEPENDENCE (from GAPO-RETURN-CORRECTNESS): for two run
names measured by measure.sh, print each route's totals and digest, write
compare.py's movement file per route to private/eqli/compare-AFTER-ROUTE.json,
print the status moves, and say whether each route's null-property stream is
byte-identical."""
import filecmp
import json
import pathlib
import subprocess
import sys

here = pathlib.Path(__file__).resolve().parent
repo = here.parents[3]
private = repo / "private/eqli"
slices = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1")
before, after = sys.argv[1:3]


def run_dir(route, name):
    if route == "corpus":
        return private / name
    if route == "system":
        return private / f"{name}-system"
    return slices / f"eqli-{name}"


def null_stream(route, name):
    suffix = {"corpus": "", "system": "-system", "slice": "-slice"}[route]
    return private / f"{name}{suffix}-null.jsonl"


def null_rows(route, na, nb):
    """The row identity carries the resource contracts' digest, which a
    contract change moves; compare the verdicts themselves and name every row
    whose null, custody or combined field moved."""
    rows = []
    for stream in (na, nb):
        keyed = {}
        for line in stream.read_text().splitlines():
            row = json.loads(line)
            keyed[(row["file"], row["function"])] = row
        rows.append(keyed)
    if set(rows[0]) != set(rows[1]):
        print(f"{route}: null rows differ in keys")
    identity_only = 0
    for key in sorted(set(rows[0]) & set(rows[1])):
        x, y = rows[0][key], rows[1][key]
        fields = ("null", "custody", "combined", "custody_null_premises")
        moved = [f for f in fields if x.get(f) != y.get(f)]
        if moved:
            print(f"{route}: null row {key[0]}::{key[1]} moved {moved}: "
                  f"custody {x.get('custody')} -> {y.get('custody')}, "
                  f"combined {x.get('combined')} -> {y.get('combined')}, "
                  f"null {x['null'].get('verdict')} -> {y['null'].get('verdict')}")
        elif x != y:
            rest = [f for f in set(x) | set(y) if f != "identity" and x.get(f) != y.get(f)]
            if rest:
                print(f"{route}: null row {key[0]}::{key[1]} differs in {sorted(rest)}")
            else:
                identity_only += 1
    print(f"{route}: {identity_only} null rows differ only in their identity")


for route in ("corpus", "system", "slice"):
    a, b = run_dir(route, before), run_dir(route, after)
    if not (b / "resource-diagnostics.json").exists():
        print(f"{route}: no {after} run")
        continue
    subprocess.run([sys.executable, str(here / "totals.py"), str(a), str(b)], check=True)
    out = private / f"compare-{after}-{route}.json"
    subprocess.run([sys.executable, str(here / "compare.py"), str(a), str(b), str(out)], check=True)
    na, nb = null_stream(route, before), null_stream(route, after)
    if na.exists() and nb.exists():
        same = filecmp.cmp(na, nb, shallow=False)
        print(f"{route}: null stream {'identical' if same else 'differs'}")
        if not same:
            null_rows(route, na, nb)
