#!/usr/bin/env python3
"""W48-BOTTOM-UP-CALLEE-BODIES: calls held inside position leaves.

Usage: held_calls.py DIAGNOSTICS PARITY.jsonl OUT_JSON

closure_r3.py reads callee names from `call to uncontracted function` and
`nested call` leaves only. A call inside a `return`, a controlling
expression or a `switch` is reported as one position leaf
(`unmodeled call, aggregate or indirect access in POSITION`) with no callee
name, and a call to a `static` header body W4 derives but cannot validate
leaves no call leaf at all (W48-CALLEE-CLOSURE-R3 section 3 and 6). This
script recovers the called names of every such leaf from the parity rows
written at the same (function, line) (their `texts`: identifiers followed by
`(`, keywords and builtins excluded) and measures, per serving rule:

* `calibration`: position leaves whose every held call is to a Safe body in
  the calling unit, yet still refused. The summary was in force, so the
  call is not the refusal; this is the model's false-positive rate, and
  the model counts these leaves as *not* served;
* the closure where a position leaf is served when it holds at least one
  call, every held name has a Safe body under the serving rule, and the
  same leaf is not in the calibration set; and the cross-unit route's yield
  (`cross` minus `same`).

Upper bound: a served position leaf may still refuse on its own shape once
the call is modelled (the calibration bounds that from the same-unit side).
"""
import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import closure_r3  # noqa: E402

POSITION = re.compile(r"unsupported: unmodeled call, aggregate or indirect access in "
                      r"(return expression|branch condition|loop condition|for-loop condition|"
                      r"do-while condition|switch expression)")
KEYWORDS = {"sizeof", "if", "while", "for", "switch", "return", "_Alignof", "__alignof__",
            "_Generic", "__builtin_offsetof", "__extension__", "typeof", "__typeof__",
            "_Static_assert", "defined"}
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*$")


def held(texts):
    names = []
    for here, after in zip(texts, texts[1:]):
        if after == "(" and IDENT.match(here) and here not in KEYWORDS \
                and not closure_r3.BUILTIN.match(here):
            names.append(here)
    return names


def main() -> int:
    diagnostics, parity, out = sys.argv[1:4]
    defined_in, functions = closure_r3.analyse(closure_r3.load(diagnostics))
    unknown = {k for k, v in functions.items() if v["status"] == "Unknown"}
    base_safe = {k for k, v in functions.items() if v["status"] == "Safe"}
    wanted = collections.defaultdict(set)
    for key in unknown:
        for leaf in functions[key]["others"]:
            if POSITION.fullmatch(closure_r3.family(leaf)):
                wanted[(key[1], leaf.get("line"))].add(key)
    texts = {}
    with open(parity) as handle:
        for line in handle:
            if '"texts": []' in line:
                continue
            row = json.loads(line)
            key = (row.get("function"), row.get("line"))
            if key in wanted and row.get("texts"):
                current = texts.get(key)
                if current is None or len(row["texts"]) > len(current):
                    texts[key] = row["texts"]
    calls_of = {key: held(value) for key, value in texts.items()}

    def home(file, name, cross):
        homes = defined_in.get(name, set())
        if file in homes:
            return (file, name)
        if cross and len(homes) == 1:
            return (next(iter(homes)), name)
        return None

    calibration = set()
    for (function, line), owners in wanted.items():
        names = calls_of.get((function, line)) or []
        for owner in owners:
            if names and all(home(owner[0], n, False) in base_safe for n in names):
                calibration.add((owner, line))

    def leaf_served(owner, leaf, safe, cross):
        if not POSITION.fullmatch(closure_r3.family(leaf)) or (owner, leaf.get("line")) in calibration:
            return False
        names = calls_of.get((owner[1], leaf.get("line"))) or []
        return bool(names) and all(home(owner[0], n, cross) in safe for n in names)

    def closure(cross):
        safe, freed, number = set(base_safe), {}, 0
        while True:
            added = []
            for key in unknown:
                if key in freed:
                    continue
                entry = functions[key]
                if all(home(key[0], n, cross) in safe for n in entry["calls"]) \
                        and all(leaf_served(key, l, safe, cross) for l in entry["others"]):
                    added.append(key)
            if not added:
                return freed
            for key in added:
                freed[key] = number
            safe |= set(added)
            number += 1

    same, cross = closure(False), closure(True)
    held_names = collections.Counter()
    leaf_count = collections.Counter()
    for (function, line), owners in wanted.items():
        names = calls_of.get((function, line))
        leaf_count["with texts" if names is not None else "no texts"] += len(owners)
        if names:
            leaf_count["holding a call"] += len(owners)
            for n in set(names):
                held_names[n] += len(owners)
    document = {
        "position_leaves": sum(len(v) for v in wanted.values()), "leaf_count": dict(leaf_count),
        "calibration_refused_with_safe_same_unit_calls": len(calibration),
        "same_unit": {"freed": len(same), "rounds": dict(collections.Counter(same.values())),
                      "functions": sorted(f"{k[0]}::{k[1]}@{r}" for k, r in same.items())},
        "cross_unit": {"freed": len(cross), "rounds": dict(collections.Counter(cross.values())),
                       "functions": sorted(f"{k[0]}::{k[1]}@{r}" for k, r in cross.items())},
        "cross_only": sorted(f"{k[0]}::{k[1]}@{r}" for k, r in cross.items() if k not in same),
        "held_names": dict(held_names.most_common(200)),
    }
    pathlib.Path(out).write_text(json.dumps(document, indent=1) + "\n")
    print(f"position leaves {document['position_leaves']}: {dict(leaf_count)}; "
          f"calibration (refused although every held call is a Safe same-unit body): {len(calibration)}")
    print(f"closure with held calls served: same-unit {len(same)} {dict(collections.Counter(same.values()))}; "
          f"cross-unit {len(cross)} {dict(collections.Counter(cross.values()))}; "
          f"cross-unit only {len(document['cross_only'])}")
    for name in document["cross_only"][:80]:
        print("   ", name)
    return 0


if __name__ == "__main__":
    sys.exit(main())
