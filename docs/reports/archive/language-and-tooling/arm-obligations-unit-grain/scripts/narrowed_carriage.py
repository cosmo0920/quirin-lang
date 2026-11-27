#!/usr/bin/env python3
"""ARM-OBLIGATIONS-UNIT-GRAIN: check that BASE -> TIP on the corpus only
narrows rule 37's carriage, and undo the narrowing on BASE so the rest of the
identity check is RULE36-PILOT's `corpus_substitution.py` unchanged.

For every function row of `resource-diagnostics.json` and
`resource-diagnostics.components.json` (matched by library, file, occurrence
and function; the walk is ARM-OBLIGATIONS' `strip_carriage.function_rows`):

1. a TIP row may carry `rule_obligations` only where the BASE row does, with
   the same value and the same `hypothesis_context.assumptions` (kept);
2. a BASE row carrying `rule_obligations` must carry exactly rule 37's premise
   (`{"open": [], "premises": [{"name": "asserted_allocation", "rule": "37"}]}`);
3. where TIP dropped it, BASE's `assumptions` must be exactly the
   ARM-OBLIGATIONS fold of TIP's value
   (`sha256({"rule_premises": [...], "sources": <TIP>})`, compact sorted-key
   spelling); the BASE row is then rewritten to TIP's value and loses the
   field (dropped);
4. no row gains a field or changes its status.

Each folded digest must map to one TIP digest. BASE's contract sidecar
quotes every consulting row's hypothesis context (`"artifact": A,
"assumptions": D`); the quotation of each dropped row's artifact is rewritten
to TIP's digest, and an artifact that is dropped in one row and kept in
another fails the check. Writes the normalised BASE side
under OUT (the same layout as `.../sides/<name>/corpus`), prints the census
(kept and dropped rows by status and unit, TSV lists under OUT/../) and exits
0 only when every row passes 1-4.

Usage: narrowed_carriage.py BASE_SIDE_CORPUS TIP_SIDE_CORPUS OUT LISTS_DIR
"""
import hashlib
import json
import re
import shutil
import sys
from collections import Counter
from pathlib import Path

EXPECTED = {"open": [], "premises": [{"name": "asserted_allocation", "rule": "37"}]}
ROWS = ("resource-diagnostics.json", "resource-diagnostics.components.json")
CONTRACTS = "resource-diagnostics.contracts.json"


def fold(sources, premises):
    text = json.dumps({"rule_premises": premises, "sources": sources}, separators=(",", ":"),
                      sort_keys=True)
    return hashlib.sha256(text.encode()).hexdigest()


def function_rows(value, path=()):
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
    base_dir, tip_dir, out, lists = (Path(argument) for argument in sys.argv[1:5])
    if out.exists():
        print(f"{out} exists; use a fresh path")
        return 2
    shutil.copytree(base_dir, out)
    lists.mkdir(parents=True, exist_ok=True)
    ok = True
    folded = {}
    dropped_artifacts = {}
    kept_artifacts = set()
    for name in ROWS:
        base = json.loads((out / "corpus" / name).read_text())
        tip = json.loads((tip_dir / "corpus" / name).read_text())
        tip_rows = {path: row for path, row in function_rows(tip)}
        kept, dropped, bad = Counter(), Counter(), []
        kept_units, dropped_units = Counter(), Counter()
        listing = []
        seen = set()
        for path, row in function_rows(base):
            seen.add(path)
            after = tip_rows.get(path)
            unit = next((p for p in path if isinstance(p, tuple)), None)
            label = f"{unit[0]}/{unit[1]}::{row['function']}" if unit else row["function"]
            if after is None:
                bad.append((label, "no TIP row"))
                continue
            if after.get("status") != row.get("status"):
                bad.append((label, "status moved"))
            field = row.get("rule_obligations")
            tip_field = after.get("rule_obligations")
            if field is None:
                if tip_field is not None:
                    bad.append((label, "TIP gained a carriage"))
                continue
            if field != EXPECTED:
                bad.append((label, "BASE carriage is not rule 37's premise"))
                continue
            status = row.get("status")
            base_assumptions = row["hypothesis_context"]["assumptions"]
            tip_assumptions = after["hypothesis_context"]["assumptions"]
            if tip_field is not None:
                if tip_field != field or tip_assumptions != base_assumptions:
                    bad.append((label, "kept carriage differs"))
                    continue
                kept[status] += 1
                kept_artifacts.add(row["hypothesis_context"]["artifact"])
                kept_units[unit] += 1
                listing.append(("kept", status, label))
                continue
            if base_assumptions != fold(tip_assumptions, EXPECTED["premises"]):
                bad.append((label, "dropped row's BASE assumptions are not the fold of TIP's"))
                continue
            del row["rule_obligations"]
            row["hypothesis_context"]["assumptions"] = tip_assumptions
            folded.setdefault(base_assumptions, set()).add(tip_assumptions)
            dropped_artifacts[row["hypothesis_context"]["artifact"]] = (base_assumptions,
                                                                        tip_assumptions)
            dropped[status] += 1
            dropped_units[unit] += 1
            listing.append(("dropped", status, label))
        for path, row in tip_rows.items():
            if path not in seen:
                bad.append((row["function"], "no BASE row"))
        (out / "corpus" / name).write_text(json.dumps(base, indent=2) + "\n")
        print(f"{name}: kept {sum(kept.values())} {dict(sorted(kept.items()))} in "
              f"{len(kept_units)} units; dropped {sum(dropped.values())} "
              f"{dict(sorted(dropped.items()))} in {len(dropped_units)} units; "
              f"carrying units at BASE {len(set(kept_units) | set(dropped_units))}; "
              f"failing rows: {len(bad)}")
        for label, why in bad[:40]:
            print(f"  FAIL {label}: {why}")
        if name == ROWS[0]:
            with (lists / "rows.tsv").open("w") as handle:
                handle.write("carriage\tstatus\trow\n")
                for entry in sorted(listing):
                    handle.write("\t".join(entry) + "\n")
            with (lists / "units.tsv").open("w") as handle:
                handle.write("unit\tkept\tdropped\n")
                for unit in sorted(set(kept_units) | set(dropped_units), key=str):
                    handle.write(f"{unit[0]}/{unit[1]}\t{kept_units[unit]}\t"
                                 f"{dropped_units[unit]}\n")
        ok &= not bad
    one_to_one = all(len(tips) == 1 for tips in folded.values())
    ambiguous = sorted(set(dropped_artifacts) & kept_artifacts)
    path = out / "corpus" / CONTRACTS
    text = path.read_text()
    replaced = 0

    def rewrite(match):
        nonlocal replaced
        entry = dropped_artifacts.get(json.loads(match.group(2)))
        if entry is None or match.group(4) != entry[0]:
            return match.group(0)
        replaced += 1
        return match.group(1) + match.group(2) + match.group(3) + entry[1] + match.group(5)

    pattern = re.compile(r'("artifact":\s*)("(?:[^"\\]|\\.)*")(,\s*"assumptions":\s*")'
                         r'([0-9a-f]{64})(")')
    text = pattern.sub(rewrite, text)
    path.write_text(text)
    remaining = sum(text.count(before) for before in folded)
    print(f"{CONTRACTS}: {len(folded)} folded assumptions digest(s), one-to-one: {one_to_one}; "
          f"{replaced} dropped row quotation(s) rewritten; {remaining} kept quotation(s) left; "
          f"artifacts both kept and dropped: {len(ambiguous)}")
    ok &= one_to_one and not ambiguous
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
