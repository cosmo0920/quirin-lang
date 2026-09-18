#!/usr/bin/env python3
"""Inventory AGENTS.md's standing constraints: label, order, byte length, sha256.

Usage: constraints.py <path-to-AGENTS.md> [--json out.json]
"""
import re, sys, hashlib, json, pathlib

path = pathlib.Path(sys.argv[1])
text = path.read_text()

start = text.index("## Standing constraints when touching specific subsystems")
end = text.index("## Standing per-change gate")
body = text[start:end]

# a constraint starts at column 0 with "N." or "Na." (e.g. "10b.")
starts = [(m.start(), m.group(1)) for m in re.finditer(r"(?m)^(\d+[a-z]?)\.\s", body)]
rows = []
for i, (pos, label) in enumerate(starts):
    stop = starts[i + 1][0] if i + 1 < len(starts) else len(body)
    chunk = body[pos:stop].rstrip("\n") + "\n"
    rows.append({
        "label": label,
        "order": i,
        "bytes": len(chunk.encode()),
        "sha256": hashlib.sha256(chunk.encode()).hexdigest()[:16],
    })

# group headings, in order
groups = re.findall(r"(?m)^\*\*(.+?)\*\*\s*$", body)

out = {"labels": [r["label"] for r in rows], "count": len(rows),
       "groups": groups, "rows": rows,
       "section_bytes": len(body.encode())}

if "--json" in sys.argv:
    pathlib.Path(sys.argv[sys.argv.index("--json") + 1]).write_text(
        json.dumps(out, indent=1) + "\n")

print(f"constraints: {out['count']}")
print("labels:", " ".join(out["labels"]))
print("groups:", " | ".join(groups))
print(f"section bytes: {out['section_bytes']}")
print()
for r in rows:
    print(f"  {r['label']:>4}  {r['bytes']:>6}  {r['sha256']}")
