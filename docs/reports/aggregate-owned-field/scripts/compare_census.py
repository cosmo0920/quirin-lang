#!/usr/bin/env python3
"""Compare two custody_census outputs program by program.

Prints, for the programs present in both, how many are byte-identical across
every published field (profile, custody operation list, emitted C,
observation) and lists every difference by file and field. A program the card
tree admits that the base refused — or the reverse — is listed separately:
the card's own new example is expected there and nothing else is.
"""
import sys


def parse(path):
    records = {}
    current = None
    field = None
    c_lines = []
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("=== "):
            current = line[4:]
            records[current] = {}
            field = None
            continue
        if current is None:
            continue
        if field == "c":
            if line == "--- end c":
                records[current]["c"] = "\n".join(c_lines)
                field = None
            else:
                c_lines.append(line)
            continue
        if line.startswith("--- c ("):
            field = "c"
            c_lines = []
        elif line == "--- no c":
            records[current]["c"] = None
        elif line.startswith("refused "):
            records[current]["refused"] = line
        elif line.startswith("profile "):
            records[current]["profile"] = line
        elif line.startswith("custody "):
            records[current]["custody"] = line
        elif line.startswith("observation "):
            records[current]["observation"] = line
    return records


base = parse(sys.argv[1])
card = parse(sys.argv[2])
identical = 0
differ = []
only = []
refused_both = 0
for name in sorted(set(base) | set(card)):
    b = base.get(name)
    c = card.get(name)
    if b is None or c is None:
        only.append((name, "base" if c is None else "card"))
        continue
    if "refused" in b and "refused" in c:
        refused_both += 1
        if b["refused"] != c["refused"]:
            differ.append((name, "refusal codes", b["refused"], c["refused"]))
        continue
    if ("refused" in b) != ("refused" in c):
        differ.append((name, "admission", b.get("refused", "admitted"), c.get("refused", "admitted")))
        continue
    same = True
    for key in ("profile", "custody", "c", "observation"):
        if b.get(key) != c.get(key):
            same = False
            differ.append((name, key, "(base)", "(card)"))
    if same:
        identical += 1

print(f"programs in both trees: {len(set(base) & set(card))}")
print(f"admitted in both and byte-identical on profile, custody, C and observation: {identical}")
print(f"refused in both (same codes unless listed): {refused_both}")
print(f"differ: {len(differ)}")
for entry in differ:
    print("  DIFFER", *entry)
print(f"present in one tree only: {len(only)}")
for name, tree in only:
    print("  ONLY", tree, name)
sys.exit(0 if not differ else 1)
