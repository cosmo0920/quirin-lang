#!/usr/bin/env python3
"""Explain why two provider executables of the same sources differ.

    package_diff.py <provider A> <provider B>

Card CI-RUST-JOB-PEAK (PACKAGE-BYTES-DEPEND-ON-CHECKOUT). Prints, per ELF
section, whether the bytes are equal; the absolute host paths each file holds;
and, per crate named in a v0-mangled symbol (`C s<disambiguator>_ <len><name>`),
whether its crate disambiguator is the same in both. A crate whose
disambiguator differs was compiled with a different `-C metadata`, which cargo
derives from the package id and, for a path package outside the workspace
root, from its absolute path. Needs `readelf` and `nm`.
"""

import re
import subprocess
import sys
from pathlib import Path


def sections(path):
    out = subprocess.run(["readelf", "-S", "-W", str(path)], capture_output=True,
                         text=True, check=True).stdout
    data = Path(path).read_bytes()
    found = {}
    for line in out.splitlines():
        match = re.match(r"\s*\[\s*\d+\]\s+(\S+)\s+(\S+)\s+[0-9a-f]+\s+([0-9a-f]+)\s+([0-9a-f]+)", line)
        if match and match.group(2) != "NOBITS":
            offset, size = int(match.group(3), 16), int(match.group(4), 16)
            found[match.group(1)] = data[offset:offset + size]
    return found


def crate_disambiguators(path):
    out = subprocess.run(["nm", "--defined-only", str(path)], capture_output=True,
                         text=True, check=True).stdout
    found = {}
    for match in re.finditer(r"Cs([0-9A-Za-z]*)_(\d+)([a-z_][a-z0-9_]*)", out):
        length, name = int(match.group(2)), match.group(3)
        if len(name) >= length:
            found.setdefault(name[:length], set()).add(match.group(1))
    return found


def main(a, b):
    left, right = sections(a), sections(b)
    equal = [name for name in left if left[name] == right.get(name)]
    print(f"sections equal: {len(equal)} of {len(left)}")
    for name in left:
        if left[name] != right.get(name):
            other = right.get(name, b"")
            differing = sum(x != y for x, y in zip(left[name], other))
            print(f"  differs: {name} ({differing} of {len(left[name])} bytes; "
                  f"sizes {len(left[name])} / {len(other)})")
    for label, path in (("A", a), ("B", b)):
        data = Path(path).read_bytes()
        paths = sorted(set(re.findall(rb"/(?:media|home|tmp|qcore)/[\x21-\x7e]{4,}", data)))
        roots = sorted({p.split(b"/src/")[0].decode() for p in paths})
        print(f"{label}: {len(paths)} absolute paths, under {roots[:4]}")
    da, db = crate_disambiguators(a), crate_disambiguators(b)
    for name in sorted(set(da) | set(db)):
        same = da.get(name) == db.get(name)
        print(f"  {'same' if same else 'DIFFERS'} crate {name}: "
              f"{sorted(da.get(name, []))} / {sorted(db.get(name, []))}")


if __name__ == "__main__":
    main(*sys.argv[1:])
