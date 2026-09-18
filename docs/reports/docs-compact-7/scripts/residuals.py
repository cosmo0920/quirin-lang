#!/usr/bin/env python3
"""Residual-shaped names in AGENTS.md / CLAUDE.md / README.md vs named_followups."""
import re, json, pathlib, sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ae267574ed4f22b74")
FILES = ["AGENTS.md", "CLAUDE.md", "README.md"]

tick = re.compile(r"`([^`\n]+)`")
# residual-shaped: ALLCAPS (letters/digits) with at least one hyphen, >= 2 segments
resid = re.compile(r"^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$")

m = json.load(open(ROOT / "docs/reviews/r0_3/acceptance_matrix.json"))
ids = {e["id"] for e in m["named_followups"]}
# also index the free text of the matrix entries so a name mentioned inside another
# entry's `why` counts as "reaches a tracker"
blob = json.dumps(m["named_followups"])

found = {}
for f in FILES:
    text = (ROOT / f).read_text()
    for mm in tick.finditer(text):
        s = mm.group(1).strip()
        if resid.match(s):
            found.setdefault(s, set()).add(f)

missing_id = []
missing_all = []
for name in sorted(found):
    if name in ids:
        continue
    missing_id.append(name)
    if name not in blob:
        missing_all.append(name)

print(f"residual-shaped names in the three files: {len(found)}")
print(f"not a named_followups id:                 {len(missing_id)}")
print(f"absent from the matrix entirely:          {len(missing_all)}")
print("\n=== not an id, but mentioned somewhere in the matrix text ===")
for n in missing_id:
    if n not in missing_all:
        print("  ", n, "   <-", ",".join(sorted(found[n])))
print("\n=== absent from the matrix entirely ===")
for n in missing_all:
    print("  ", n, "   <-", ",".join(sorted(found[n])))
