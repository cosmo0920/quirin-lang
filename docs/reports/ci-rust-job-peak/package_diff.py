#!/usr/bin/env python3
"""Explain why two provider executables of the same sources differ.

    package_diff.py <provider A> <provider B>

Card CI-RUST-JOB-PEAK (PACKAGE-BYTES-DEPEND-ON-CHECKOUT). Prints the sizes,
the number of differing bytes, absolute paths left in each file, and the
mangled-symbol hashes (`17h<16 hex>E` in legacy Rust mangling) that differ,
grouped by crate prefix, so a path-dependent crate hash is visible as such.
"""

import collections
import re
import subprocess
import sys
from pathlib import Path


def symbols(path):
    out = subprocess.run(["nm", "--defined-only", str(path)], capture_output=True, text=True).stdout
    names = [line.split()[-1] for line in out.splitlines() if line.strip()]
    return names


def crate_of(symbol):
    match = re.match(r"_ZN(?:\d+)?(\d+)([A-Za-z_][A-Za-z0-9_]*)", symbol)
    if not match:
        return "?"
    length = int(match.group(1))
    return match.group(2)[:length]


def main(a, b):
    left, right = Path(a).read_bytes(), Path(b).read_bytes()
    print(f"sizes {len(left)} {len(right)}")
    differing = sum(x != y for x, y in zip(left, right)) + abs(len(left) - len(right))
    print(f"differing bytes {differing}")
    for name, data in (("A", left), ("B", right)):
        paths = sorted(set(re.findall(rb"/(?:media|home|tmp)/[\x21-\x7e]{4,}", data)))
        print(f"{name}: {len(paths)} absolute host paths; first: {[p.decode()[:100] for p in paths[:3]]}")
        remapped = len(set(re.findall(rb"/qcore/[\x21-\x7e]{4,}", data)))
        print(f"{name}: {remapped} distinct /qcore/ (remapped) paths")
    strip_hash = lambda s: re.sub(r"17h[0-9a-f]{16}E$", "", s)
    sa, sb = symbols(a), symbols(b)
    ha = {strip_hash(s): s for s in sa}
    hb = {strip_hash(s): s for s in sb}
    print(f"symbols {len(sa)} {len(sb)}; names equal after removing hashes: {set(ha) == set(hb)}")
    changed = collections.Counter()
    same = collections.Counter()
    for key in set(ha) & set(hb):
        (changed if ha[key] != hb[key] else same)[crate_of(key)] += 1
    print("symbols whose hash differs, by leading path segment:")
    for crate, count in changed.most_common(15):
        print(f"  {crate}: {count} differ, {same[crate]} equal")
    print("leading segments with no differing hash (first 15):",
          sorted(c for c in same if c not in changed)[:15])


if __name__ == "__main__":
    main(*sys.argv[1:])
