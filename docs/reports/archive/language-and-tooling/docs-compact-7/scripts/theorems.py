#!/usr/bin/env python3
"""Extract theorem-shaped identifiers from the three compaction-target files and
resolve each against proof/."""
import re, subprocess, sys, os, pathlib

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-ae267574ed4f22b74")
FILES = ["AGENTS.md", "CLAUDE.md", "README.md"]

# backticked spans
tick = re.compile(r"`([^`\n]+)`")

# theorem-shaped: QCore.Foo(.bar)* OR a bare snake_case identifier with >=2 underscores
qcore = re.compile(r"^QCore\.[A-Za-z0-9_.]+$")
snake = re.compile(r"^[a-z][a-z0-9]*(_[a-z0-9]+){2,}$")
lowerCamelOrSnake = re.compile(r"^[a-zA-Z][A-Za-z0-9]*(_[A-Za-z0-9]+)+$")

cands = {}
for f in FILES:
    text = (ROOT / f).read_text()
    for m in tick.finditer(text):
        s = m.group(1).strip()
        if qcore.match(s) or snake.match(s):
            cands.setdefault(s, set()).add(f)

# build a corpus of proof text
proof_text = {}
for p in (ROOT / "proof").rglob("*.lean"):
    try:
        proof_text[p] = p.read_text()
    except Exception:
        pass

def resolve(name):
    # strip a QCore. namespace prefix down to the final component too
    last = name.split(".")[-1]
    hits = []
    for p, t in proof_text.items():
        if name in t or (re.search(r"\b(theorem|lemma|def|abbrev|structure|inductive)\s+" + re.escape(last) + r"\b", t)):
            hits.append(p.relative_to(ROOT).as_posix())
    return hits

print("name\tfiles\tresolved\twhere")
unresolved = []
for name in sorted(cands):
    hits = resolve(name)
    status = "RESOLVED" if hits else "DANGLING"
    if not hits:
        unresolved.append(name)
    print(f"{name}\t{','.join(sorted(cands[name]))}\t{status}\t{';'.join(hits[:3])}")

print("\n--- summary ---")
print(f"candidates: {len(cands)}")
print(f"resolved:   {len(cands)-len(unresolved)}")
print(f"dangling:   {len(unresolved)}")
for n in unresolved:
    print("  DANGLING:", n, "in", ",".join(sorted(cands[n])))
