#!/usr/bin/env python3
"""entries.py AFTER_DIAGNOSTICS PARITY_JSONL OUT_JSON -- card CG10-COMMON-TYPED-PLACE:
the fate of every leaf CG00 attributed to this card's entries (M01, I04).

CG00's leaf attribution (artifacts/attribution-leaves.jsonl.gz, on the
baseline ae17152b) names each unsupported leaf by (file, function, line,
description); its `file` is the origin file, not the unit, so leaves are
keyed by (function, internal line, description). A leaf is *gone* when the
AFTER resource-diagnostics.json has no unsupported leaf with that key. For each gone leaf, the AFTER leaves at the
same line are read: an `unknown_alias` there means the place is now read and
its flow obligation failed (the root, or `R->F`). The `typed-place` parity
rows then split every refusal the place layer made, by cause.
"""
import collections
import gzip
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
LEAVES = HERE.parents[1] / "cg00-clang-gap-baseline/artifacts/attribution-leaves.jsonl.gz"


def main() -> int:
    after = json.loads(pathlib.Path(sys.argv[1]).read_text())
    present = collections.Counter()
    by_line = collections.defaultdict(list)
    status = {}
    for unit in after["units"]:
        for function in unit["functions"]:
            status.setdefault(function["function"], set()).add(function["status"])
            for leaf in function["diagnostics"]:
                key = (function["function"], leaf.get("line"))
                by_line[key].append(leaf["category"])
                if leaf["category"] == "unsupported":
                    present[key + (leaf["description"],)] += 1
    fate = collections.Counter()
    functions = collections.defaultdict(set)
    with gzip.open(LEAVES, "rt") as stream:
        for line in stream:
            row = json.loads(line)
            entry = row.get("entry")
            if entry not in ("M01", "I04"):
                continue
            key = (row["function"], row["line"], row["description"])
            if present[key] > 0:
                present[key] -= 1
                fate[(entry, row["reason"], "kept")] += 1
                continue
            alias = "unknown_alias" in by_line[key[:2]]
            fate[(entry, row["reason"], "gone, unknown_alias at the line" if alias else "gone")] += 1
            functions[(entry, "/".join(sorted(status.get(key[0], {"absent"}))))].add(key[0])
    refusals = collections.Counter()
    admitted = collections.Counter()
    with open(sys.argv[2]) as stream:
        for line in stream:
            row = json.loads(line)
            if row.get("position") != "typed-place":
                continue
            for place in row["places"]:
                if place["admitted"]:
                    kinds = "/".join(step["kind"] for step in place["place"]["steps"])
                    admitted[kinds] += 1
                else:
                    refusals[place["refusal"]] += 1
    result = {
        "leaves": {" | ".join(key): count for key, count in sorted(fate.items())},
        "functions_with_a_gone_leaf": {
            f"{entry} -> {state}": len(keys) for (entry, state), keys in sorted(functions.items(), key=str)
        },
        "place_decisions_admitted_by_shape": dict(admitted.most_common(25)),
        "place_decisions_refused_by_cause": dict(refusals.most_common()),
    }
    pathlib.Path(sys.argv[3]).write_text(json.dumps(result, indent=1))
    json.dump(result, sys.stdout, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
