#!/usr/bin/env python3
"""For every SCREAMING-KEBAB name that stopped appearing in the three compacted
files, say where it still lives: elsewhere in the same files (any case), a
report directory, the tracker, or the verbatim checkpoint page. A name in none
of those would be a loss."""
import json
import os
import pathlib

W = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a9936abd188804098"
)
dropped = [l.strip() for l in open(W / "measure/dropped.txt") if l.strip()]
now = "".join(
    (W / f).read_text(encoding="utf-8") for f in ("AGENTS.md", "CLAUDE.md", "README.md")
).lower()
ckpt = (
    W / "docs/development/increments/wave-40-compaction-checkpoints.md"
).read_text(encoding="utf-8")
tracker = {
    e["id"]
    for e in json.loads(
        (W / "docs/reviews/r0_3/acceptance_matrix.json").read_text(encoding="utf-8")
    )["named_followups"]
}
reports = {d.lower() for d in os.listdir(W / "docs/reports")}

print(f"{'name':34s} {'in files':9s} {'report dir':11s} {'tracker':8s} {'checkpoint':10s}")
lost = []
for n in dropped:
    a = n.lower() in now
    b = n.lower() in reports
    c = n in tracker
    d = n in ckpt
    print(f"{n:34s} {str(a):9s} {str(b):11s} {str(c):8s} {str(d):10s}")
    if not (a or b or c or d):
        lost.append(n)
print("\nLOST (in none of the four):", lost or "none")
