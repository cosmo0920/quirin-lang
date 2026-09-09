"""Report the raw-scan residual and enumeration refusals across the corpus."""

import json
import sys

path = sys.argv[1]
report = json.load(open(path))
units = report["units"]

print(f"units: {len(units)}")

total_absent = 0
refusal_counts = {}
nonzero = []
for unit in units:
    enumeration = unit.get("enumeration") or {}
    absent = enumeration.get("absent_from_raw_scan") or []
    total_absent += len(absent)
    if absent:
        nonzero.append((unit.get("unit") or unit.get("path"), len(absent), absent))
    for refusal in enumeration.get("refusals") or []:
        key = refusal if isinstance(refusal, str) else json.dumps(refusal, sort_keys=True)
        refusal_counts[key] = refusal_counts.get(key, 0) + 1

print(f"total absent_from_raw_scan rows: {total_absent}")
print(f"units with a nonzero residual: {len(nonzero)}")
for name, count, names in nonzero:
    print(f"  {name}: {count}")
    for entry in names:
        print(f"      {entry}")

print(f"enumeration refusal grounds across the corpus: {refusal_counts or '{} (none)'}")

verdicts = {}
definitions = 0
for unit in units:
    verdicts[unit.get("result")] = verdicts.get(unit.get("result"), 0) + 1
    enumeration = unit.get("enumeration") or {}
    definitions += len(enumeration.get("definitions") or [])
print(f"unit verdicts: {verdicts}")
print(f"enumerated definitions (denominator): {definitions}")
