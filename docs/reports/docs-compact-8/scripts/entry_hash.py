#!/usr/bin/env python3
"""Split AGENTS.md's numbered entries (hard invariants + standing constraints)
into per-label blocks and hash each, so a compaction pass can prove which
entries it left byte-identical."""
import hashlib
import re
import sys

path = sys.argv[1] if len(sys.argv) > 1 else "AGENTS.md"
lines = open(path, encoding="utf-8").read().split("\n")

# Section bounds: from "## Hard invariants" to "## Standing per-change gate".
start = next(i for i, l in enumerate(lines) if l.startswith("## Hard invariants"))
split = next(i for i, l in enumerate(lines) if l.startswith("## Standing constraints"))
end = next(i for i, l in enumerate(lines) if l.startswith("## Standing per-change gate"))

head = re.compile(r"^(\d+[a-z]?)\. ")
blocks = []
cur = None
for i in range(start, end):
    m = head.match(lines[i])
    if m:
        if cur:
            blocks.append(cur)
        prefix = "INV-" if i < split else "C-"
        cur = [prefix + m.group(1), i, [lines[i]]]
    elif cur is not None:
        # a new "**Group heading**" or blank-then-heading ends nothing; keep
        # appending until the next numbered head or the section end.
        cur[2].append(lines[i])
if cur:
    blocks.append(cur)

seen = {}
for label, ln, body in blocks:
    text = "\n".join(body).rstrip("\n") + "\n"
    h = hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]
    assert label not in seen, f"duplicate entry label {label}"
    seen[label] = 1
    print(f"{label}\t{ln+1}\t{len(text)}\t{h}")
print(f"#count\t{len(blocks)}", file=sys.stderr)
