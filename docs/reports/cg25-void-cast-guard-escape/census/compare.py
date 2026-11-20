"""Compare two probe result files row by row: status moves and refusal changes."""
import sys


def load(path):
    rows = {}
    for line in open(path):
        parts = line.rstrip("\n").split("\t")
        rows[parts[0]] = (parts[1], parts[2] if len(parts) > 2 else "")
    return rows


base = {}
for path in sys.argv[1].split(","):
    base.update(load(path))
tip = load(sys.argv[2])
moves = 0
for label, (status, diags) in tip.items():
    before = base.get(label)
    if before is None:
        print(f"NEW   {label:48} {status}")
        continue
    if before[0] != status:
        moves += 1
        print(f"MOVE  {label:48} {before[0]} -> {status}")
    elif before[1] != diags and "--all" in sys.argv:
        print(f"TEXT  {label:48} {status}: {diags[:160]}")
print(f"{moves} moves over {len(tip)} rows")
