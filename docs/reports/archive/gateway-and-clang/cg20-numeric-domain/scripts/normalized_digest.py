#!/usr/bin/env python3
"""normalized_digest.py FILE... -- CG20-NUMERIC-DOMAIN: the SHA-256 of a
resource-diagnostics.json with its checkout path (`.../worktrees/<name>`)
replaced by `<checkout>`. The system and slice reports embed the profile
directory's path, so their raw digests differ between worktrees for the same
content; the corpus report embeds none."""

import hashlib
import re
import sys


def main() -> int:
    for path in sys.argv[1:]:
        text = open(path, "rb").read()
        normalized = re.sub(rb"/home/[^\"]*?/worktrees/[^/\"]+", b"<checkout>", text)
        print(hashlib.sha256(text).hexdigest()[:8], hashlib.sha256(normalized).hexdigest()[:8], path)
    return 0


if __name__ == "__main__":
    sys.exit(main())
