#!/usr/bin/env python3
"""Forecast from the base census (census_patch.py): which functions each step
could move.

Usage: forecast.py CENSUS_STDERR BEFORE_DIAGNOSTICS OUT_JSON

A census row is a typed read the unchanged API would admit but for the value
check of a `T *const` value (bit 1) or strip's refusal of a `void *` bit-cast
(bit 2). A function row of BEFORE is a *direct candidate* of a step when it is
Unknown and every leaf is an unsupported COND/RET/IND leaf on the line of one
of its census rows admitted by that step (steps are cumulative: step 2 uses
bits 1, 2 and 3). A candidate whose covering rows include a
`condition-assignment` or pointer-destination `statement` binds a borrowed
alias, whose later identity use can add an `unknown_alias` leaf (rule 10b):
it is forecast as a leaf exchange, not a move.
"""
import collections
import json
import sys

census = collections.defaultdict(list)
seen = set()
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    if not line.startswith("W47CP\t"):
        continue
    _, function, row_line, position, bits, texts = line.rstrip("\n").split("\t", 5)
    key = (function, int(row_line), position, int(bits), texts)
    if key in seen:
        continue
    seen.add(key)
    census[function].append({"line": int(row_line), "position": position,
                             "bits": int(bits), "texts": texts})
diagnostics = json.load(open(sys.argv[2]))


def family(d):
    desc = d["description"]
    if d["category"] != "unsupported":
        return d["category"]
    if desc == "indirect memory access is outside the resource projection":
        return "IND"
    if desc.startswith("unmodeled call, aggregate or indirect access in return"):
        return "RET"
    if desc.startswith("unmodeled call, aggregate or indirect access in"):
        return "COND"
    return "OTHER:" + desc[:60]


def assess(row, rows):
    lines = {r["line"] for r in rows}
    blocking, covering = [], set()
    for d in row.get("diagnostics", []):
        fam = family(d)
        if fam in ("IND", "RET", "COND") and d["line"] in lines:
            covering.add(d["line"])
            continue
        blocking.append(f"{fam}@{d['line']}")
    binds = sorted({r["position"] for r in rows if r["line"] in covering
                    and (r["position"] == "condition-assignment" or r["position"] == "statement")})
    return blocking, sorted(covering), binds


out = []
names = collections.Counter()
for unit in diagnostics["units"]:
    for row in unit.get("functions", []):
        names[row["function"]] += 1
for unit in diagnostics["units"]:
    for row in unit.get("functions", []):
        name = row["function"]
        if name not in census:
            continue
        entry = {"library": unit.get("library"), "file": unit["file"], "function": name,
                 "status": row["status"], "leaves": len(row.get("diagnostics", [])),
                 "rows": census[name], "ambiguous_name": names[name] > 1}
        for step, allowed in (("step1", {1}), ("step2", {1, 2, 3})):
            rows = [r for r in census[name] if r["bits"] in allowed]
            blocking, covering, binds = assess(row, rows)
            entry[step] = {"census_rows": len(rows), "blocking": blocking,
                           "covered_lines": covering, "binding_positions": binds,
                           "direct_candidate": row["status"] == "Unknown" and bool(rows)
                           and not blocking,
                           "forecast": ("Safe" if row["status"] == "Unknown" and rows and not blocking
                                        and not binds else
                                        "leaf exchange" if rows and covering else "no change")}
        out.append(entry)
json.dump(out, open(sys.argv[3], "w"), indent=1)
print("census rows (distinct):", len(seen), "in", len(census), "functions")
print("rows by position and bits:", dict(collections.Counter((r["position"].split(":")[0], r["bits"])
                                                             for rows in census.values() for r in rows)))
print("function rows in BEFORE:", len(out), dict(collections.Counter(e["status"] for e in out)))
for step in ("step1", "step2"):
    forecast = collections.Counter(e[step]["forecast"] for e in out)
    print(step, dict(forecast))
    for e in out:
        if e[step]["forecast"] == "Safe":
            print("   Safe:", e["file"].split("/")[-1], e["function"],
                  [r["position"] for r in e["rows"]])
        elif e[step]["direct_candidate"]:
            print("   exchange:", e["file"].split("/")[-1], e["function"], e[step]["binding_positions"])
missing = sorted(set(census) - {e["function"] for e in out})
print("census functions without a BEFORE row:", missing)
