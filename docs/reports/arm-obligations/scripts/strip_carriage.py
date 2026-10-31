#!/usr/bin/env python3
"""ARM-OBLIGATIONS: undo exactly this card's row additions on a TIP corpus
side, checking each one, so the rest of the identity check is RULE36-PILOT's
`corpus_substitution.py` unchanged.

For every function row of TIP's `resource-diagnostics.json` and
`resource-diagnostics.components.json` (matched to BASE by library, file,
occurrence and function) that carries `rule_obligations`:

1. the field must be exactly rule 37's premise
   (`{"open": [], "premises": [{"name": "asserted_allocation", "rule": "37"}]}`):
   no open obligation, no other rule;
2. its `hypothesis_context.assumptions` must be exactly the card's fold of
   BASE's value (`sha256({"rule_premises": [...], "sources": <BASE>})`, the
   compact serde_json spelling with sorted keys), and is restored to BASE's;
3. the field is removed.

Each folded digest must come from one BASE digest; its occurrences in the
contract sidecar (the dependency section quotes the consulting rows'
contexts) are restored to it. Every other byte is left alone. Writes the stripped side under
OUT (the same layout: OUT/corpus/<files>, OUT/<corpus-null…>), prints the
census (rows, units, verdicts of the carrying rows) and exits 0 only when every
carried row passes 1 and 2.

Usage: strip_carriage.py BASE_SIDE_CORPUS TIP_SIDE_CORPUS OUT
(each `.../sides/<name>/corpus`, holding `corpus/` and the null files).
"""
import hashlib
import json
import shutil
import sys
from collections import Counter
from pathlib import Path

EXPECTED = {"open": [], "premises": [{"name": "asserted_allocation", "rule": "37"}]}
ROWS = ("resource-diagnostics.json", "resource-diagnostics.components.json")
CONTRACTS = "resource-diagnostics.contracts.json"


def fold(base, premises):
    text = json.dumps({"rule_premises": premises, "sources": base}, separators=(",", ":"),
                      sort_keys=True)
    return hashlib.sha256(text.encode()).hexdigest()


def function_rows(value, path=()):
    """Every dict with a `function` and a `hypothesis_context`, keyed by the
    nearest enclosing unit's (library, file) and the function name."""
    if isinstance(value, dict):
        if "function" in value and "hypothesis_context" in value:
            yield path, value
        unit = (value.get("library"), value.get("file")) if "file" in value else None
        for key, child in value.items():
            yield from function_rows(child, path + ((unit,) if unit else ()) + (key,))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from function_rows(child, path + (index,))


def main():
    base_dir, tip_dir, out = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    if out.exists():
        print(f"{out} exists; use a fresh path")
        return 2
    shutil.copytree(tip_dir, out)
    ok = True
    folded = {}
    for name in ROWS:
        base = json.loads((base_dir / "corpus" / name).read_text())
        tip = json.loads((tip_dir / "corpus" / name).read_text())
        base_rows = {path: row for path, row in function_rows(base)}
        carried, units, statuses, bad = 0, Counter(), Counter(), 0
        for path, row in function_rows(tip):
            field = row.pop("rule_obligations", None)
            if field is None:
                continue
            carried += 1
            unit = next((p for p in path if isinstance(p, tuple)), None)
            units[unit] += 1
            statuses[row.get("status")] += 1
            before = base_rows.get(path)
            if field != EXPECTED or before is None:
                bad += 1
                continue
            base_assumptions = before["hypothesis_context"]["assumptions"]
            if row["hypothesis_context"]["assumptions"] != fold(base_assumptions,
                                                                EXPECTED["premises"]):
                bad += 1
                continue
            row["hypothesis_context"]["assumptions"] = base_assumptions
            folded.setdefault(fold(base_assumptions, EXPECTED["premises"]), set()).add(
                base_assumptions)
        (out / "corpus" / name).write_text(json.dumps(tip, indent=2) + "\n")
        print(f"{name}: {carried} rows carried rule 37's premise in {len(units)} units "
              f"(statuses {dict(sorted(statuses.items()))}); failing rows: {bad}")
        if name == ROWS[0]:
            for unit, count in sorted(units.items(), key=str):
                print(f"  {unit[0]}/{unit[1]}: {count}")
        ok &= bad == 0
    # The contract sidecar's dependency section quotes each consulting row's
    # hypothesis context; its folded digest is restored the same way.
    one_to_one = all(len(bases) == 1 for bases in folded.values())
    path = out / "corpus" / CONTRACTS
    text = path.read_text()
    replaced = 0
    for after, bases in folded.items():
        replaced += text.count(after)
        text = text.replace(after, next(iter(bases)))
    path.write_text(text)
    print(f"{CONTRACTS}: {len(folded)} folded assumptions digest(s), one-to-one: {one_to_one}; "
          f"{replaced} occurrence(s) restored")
    ok &= one_to_one
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
