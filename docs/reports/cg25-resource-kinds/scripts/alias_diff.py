#!/usr/bin/env python3
"""alias_diff.py A B -- the differences between two route reports that remain
once every checkout root before `/contract/profiles/` is `ROOT` and every
SHA-256 value (a 64-hex string) is `DIGEST`: path-derived digests (contract
source sets, inventories) differ between checkouts by construction.

Prints the count of remaining differing lines and the first few. Zero means the
two runs agree on every verdict, diagnostic, line and count.
"""

import difflib
import re
import sys

ROOT = re.compile(r"/home/cosmo/[^\"]*?(?=/contract/profiles/)")
DIGEST = re.compile(r"\b[0-9a-f]{64}\b")


def normal(path):
    with open(path, encoding="utf-8") as handle:
        return DIGEST.sub("DIGEST", ROOT.sub("ROOT", handle.read())).splitlines()


def main() -> int:
    a, b = normal(sys.argv[1]), normal(sys.argv[2])
    diff = [line for line in difflib.unified_diff(a, b, lineterm="", n=0)
            if line[:1] in "+-" and not line.startswith(("+++", "---"))]
    print(len(diff))
    for line in diff[:20]:
        print(line[:200])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
