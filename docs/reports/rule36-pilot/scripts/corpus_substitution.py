#!/usr/bin/env python3
"""RULE36-PILOT: the corpus route's identity up to a checked one-to-one
substitution (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY).

The pinned corpus route digests each unit's working directory into its build
identity, and the measure tool runs each side in its own `sides/<name>/
work-corpus`, so a published baseline and a measured side never agree byte
for byte. This checks, for BASE and SIDE corpus report directories:

1. per unit (library, file, occurrence) exactly one build identity on each
   side, the same unit set, and the map BASE identity -> SIDE identity a
   bijection (one-to-one, one per unit);
2. after spelling each side's directory prefix as `<SIDE>` and substituting
   SIDE's identities by their BASE images, each JSON report artifact is
   compared exactly; a file still differing is compared again with every
   remaining 64-hex digest blanked and every list read as a multiset, and the
   number of leaf paths that differed is printed;
3. every unit's status and every function's status, diagnostics and contract
   dependencies are equal (no row moved).

Usage: corpus_substitution.py BASE_CORPUS_DIR SIDE_CORPUS_DIR BASE_PREFIX SIDE_PREFIX
(the corpus directories holding resource-diagnostics.json; the prefixes are the
side directories each run spelled, e.g. .../sides/tip).
Exit 0 only when 1 and 3 hold and every file is equal at step 2, either
exactly or blanked with the same number of distinct digests on each side.
"""
import json
import re
import sys
from collections import Counter
from pathlib import Path

HEX = re.compile(r"\b[0-9a-f]{64}\b")
# Derived ids (e.g. `resource_contracts:<24 hex>`) truncate digests of the
# build identity, so the blanked comparison blanks every 16-64 hex run.
DERIVED = re.compile(r"\b[0-9a-f]{16,64}\b")
FILES = ("resource-diagnostics.json", "resource-diagnostics.components.json",
         "resource-diagnostics.contracts.json", "resource-diagnostics.source-effects.json",
         "resource-diagnostics.chunk-contracts.json", "monkey-source.json",
         "../corpus-null.jsonl", "../corpus-null.summary.json", "../corpus-outer.json")


def load(text, name):
    if name.endswith(".jsonl"):
        return [json.loads(line) for line in text.splitlines() if line.strip()]
    return json.loads(text)


def units(report):
    seen = Counter()
    out = {}
    for unit in report["units"]:
        key = (unit["library"], unit["file"])
        seen[key] += 1
        out[key + (seen[key],)] = unit
    return out


def identities(unit):
    return {f["hypothesis_context"]["build_identity"] for f in unit["functions"]}


def canon(value, blank):
    if isinstance(value, dict):
        if blank:  # keys may hold derived ids too; keep colliding keys apart
            return sorted(([DERIVED.sub("#", k), canon(v, blank)] for k, v in value.items()),
                          key=lambda v: json.dumps(v, sort_keys=True))
        return {k: canon(v, blank) for k, v in value.items()}
    if isinstance(value, list):
        items = [canon(v, blank) for v in value]
        return sorted(items, key=lambda v: json.dumps(v, sort_keys=True)) if blank else items
    if isinstance(value, str) and blank:
        return DERIVED.sub("#", value)
    return value


def leaves(value, prefix=""):
    if isinstance(value, dict):
        for k, v in value.items():
            yield from leaves(v, f"{prefix}.{k}")
    elif isinstance(value, list):
        for i, v in enumerate(value):
            yield from leaves(v, f"{prefix}[{i}]")
    else:
        yield prefix, value


def main():
    base_dir, side_dir, base_prefix, side_prefix = (Path(sys.argv[1]), Path(sys.argv[2]),
                                                    sys.argv[3], sys.argv[4])
    ok = True
    base = json.loads((base_dir / "resource-diagnostics.json").read_text())
    side = json.loads((side_dir / "resource-diagnostics.json").read_text())
    base_units, side_units = units(base), units(side)
    if set(base_units) != set(side_units):
        print(f"unit sets differ: {len(base_units)} vs {len(side_units)}")
        return 1
    mapping = {}
    for key in base_units:
        left, right = identities(base_units[key]), identities(side_units[key])
        if not base_units[key]["functions"] and not side_units[key]["functions"]:
            continue  # a unit with no analysed function has no build identity
        if len(left) != 1 or len(right) != 1:
            print(f"unit {key}: {len(left)} / {len(right)} build identities")
            ok = False
            continue
        mapping[next(iter(right))] = next(iter(left))
    with_functions = sum(1 for u in base_units.values() if u["functions"])
    one_to_one = len(set(mapping)) == len(set(mapping.values())) == with_functions
    print(f"units {len(base_units)}; functions {sum(len(u['functions']) for u in base_units.values())}; "
          f"build identities one-to-one: {one_to_one} ({len(mapping)} pairs, "
          f"{sum(a == b for a, b in mapping.items())} unchanged)")
    ok &= one_to_one
    moved = 0
    for key, left in base_units.items():
        right = side_units[key]
        if left["status"] != right["status"]:
            moved += 1
        lf = {f["function"]: f for f in left["functions"]}
        rf = {f["function"]: f for f in right["functions"]}
        if set(lf) != set(rf):
            moved += 1
            continue
        for name, f in lf.items():
            g = rf[name]
            if (f["status"], f["diagnostics"], f.get("contract_dependencies")) != \
                    (g["status"], g["diagnostics"], g.get("contract_dependencies")):
                moved += 1
    print(f"rows moved or changed (unit status, function status/diagnostics/dependencies): {moved}")
    ok &= moved == 0
    for name in FILES:
        left_path, right_path = base_dir / name, side_dir / name
        if not left_path.is_file() and not right_path.is_file():
            continue
        left_text = left_path.read_text().replace(base_prefix, "<SIDE>")
        right_text = right_path.read_text().replace(side_prefix, "<SIDE>")
        right_text = HEX.sub(lambda m: mapping.get(m.group(0), m.group(0)), right_text)
        if left_text == right_text:
            print(f"{name}: equal under the substitution")
            continue
        a, b = load(left_text, name), load(right_text, name)
        differing = sum(1 for (p, x), (q, y) in zip(leaves(a), leaves(b)) if p != q or x != y)
        blank_equal = canon(a, True) == canon(b, True)
        distinct = (len(set(DERIVED.findall(left_text))), len(set(DERIVED.findall(right_text))))
        print(f"{name}: differs after substitution ({differing} leaf positions); "
              f"equal with remaining digests blanked and lists as multisets: {blank_equal}; "
              f"distinct digests {distinct[0]} / {distinct[1]}")
        ok &= blank_equal and distinct[0] == distinct[1]
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
