#!/usr/bin/env python3
"""CG28-WEAK-MEMORY-EVENTS: sha256 of a system/slice report with the measuring
worktree's path normalised.

usage: normalized.py REPORT_JSON [REPORT_JSON ...]

The configured route embeds the profile directory's absolute path, which names
the worktree (`.claude/worktrees/agent-…/contract/profiles`), so two worktrees'
byte-identical analyses differ in their raw digest. This replaces every
`/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/<name>/` with
`<worktree>/` and hashes the result; the raw digest is printed beside it.
"""

import hashlib
import re
import sys

PATTERN = re.compile(rb"/home/cosmo/Documents/GitHub/qcore-c/\.claude/worktrees/[^/\"]+/")

for path in sys.argv[1:]:
    raw = open(path, "rb").read()
    normal = PATTERN.sub(b"<worktree>/", raw)
    print(hashlib.sha256(normal).hexdigest(), hashlib.sha256(raw).hexdigest()[:8], path)
