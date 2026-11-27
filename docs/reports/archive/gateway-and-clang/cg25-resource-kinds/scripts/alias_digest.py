#!/usr/bin/env python3
"""alias_digest.py FILE [FILE ...] -- the SHA-256 of each report with every
checkout root before `/contract/profiles/` rewritten to `ROOT`.

The system route and the tail-pause slice name contract modules by their
absolute path under the profile directory, so two runs of one binary over one
contract tree from two checkouts differ only there. This digest compares them
(card CG25-RESOURCE-KINDS; the BEFORE ran over an extracted `git archive` of
main so that its contract texts could not change under it).
"""

import hashlib
import re
import sys

ROOT = re.compile(rb"/home/cosmo/[^\"]*?(?=/contract/profiles/)")


def main() -> int:
    for path in sys.argv[1:]:
        with open(path, "rb") as handle:
            data = ROOT.sub(b"ROOT", handle.read())
        print(hashlib.sha256(data).hexdigest(), path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
