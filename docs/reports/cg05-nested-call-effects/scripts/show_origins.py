#!/usr/bin/env python3
"""CG05-NESTED-CALL-EFFECTS: print the pinned corpus source line of each
`FILE:LINE` origin (FILE as the diagnostics name it, under the pinned source).

usage: show_origins.py ORIGIN... [--context N]
"""

import argparse
import pathlib

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("origins", nargs="+")
    parser.add_argument("--context", type=int, default=0)
    args = parser.parse_args()
    cache = {}
    for origin in args.origins:
        name, line = origin.rsplit(":", 1)
        line = int(line)
        if name not in cache:
            matches = [p for p in ROOT.rglob(pathlib.Path(name).name) if str(p).endswith(name)]
            cache[name] = matches[0] if matches else None
        path = cache[name]
        if path is None:
            print(f"{origin}: not found")
            continue
        lines = path.read_text(errors="replace").splitlines()
        for number in range(max(1, line - args.context), min(len(lines), line + args.context) + 1):
            mark = ">" if number == line else " "
            print(f"{mark}{name}:{number}: {lines[number - 1]}")


if __name__ == "__main__":
    main()
