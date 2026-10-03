#!/usr/bin/env python3
"""callee_census.py CLASSIFY.json [--json OUT] [--top N] -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, task 2: the callees behind the
`uncontracted-callee` and `return-uncontracted-callee` classes of
classify.py's output (the functions blocked by the access refusal alone whose
refused statements call a callee with no contract or summary in force).

For each callee: the functions it frees alone (every leaf of the function is
in the two classes and names this callee only), the functions it touches, its
leaves, and the fixed point in rank order (functions freed when the callees
are lifted cumulatively, top first, counting only functions all of whose
leaves are in the two classes). The callee of a leaf is the first call of the
refused statement (`find_call`'s choice: the first identifier followed by `(`
that is not a keyword or `sizeof`), as rule W44's census row names it.

A function with a leaf in any other class is not freed by any callee (it is
counted under `mixed`).
"""
import argparse
import collections
import json
import pathlib
import re

TARGET = {"uncontracted-callee", "return-uncontracted-callee"}
NOT_CALLEES = {"sizeof", "if", "while", "for", "switch", "return", "_Alignof", "__alignof__",
               "__typeof__", "typeof", "__builtin_offsetof", "offsetof"}
NAME = re.compile(r"^[A-Za-z_]\w*$")


def first_callee(texts):
    tokens = texts.split()
    for index in range(len(tokens) - 1):
        if tokens[index + 1] == "(" and NAME.match(tokens[index]) \
                and tokens[index] not in NOT_CALLEES:
            return tokens[index]
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("classify")
    parser.add_argument("--json", type=pathlib.Path)
    parser.add_argument("--top", type=int, default=40)
    args = parser.parse_args()
    data = json.loads(pathlib.Path(args.classify).read_text())
    classes = {(f["unit"], f["function"]): f["classes"] for f in data["functions"]}
    callees = collections.defaultdict(list)
    leaves = collections.Counter()
    units = collections.defaultdict(set)
    for leaf in data["leaves"]:
        if leaf["class"] in TARGET:
            name = first_callee(leaf["texts"]) or "?"
            callees[(leaf["unit"], leaf["function"])].append(name)
            leaves[name] += 1
            units[name].add(leaf["unit"])
    alone, touched = collections.Counter(), collections.Counter()
    pure = {}
    mixed = 0
    for key, cls in classes.items():
        names = set(callees.get(key, []))
        for name in names:
            touched[name] += 1
        if set(cls) <= TARGET:
            pure[key] = names
            if len(names) == 1:
                alone[next(iter(names))] += 1
        elif names:
            mixed += 1
    ranking = sorted(touched, key=lambda name: (-alone[name], -touched[name], name))
    lifted, cumulative = set(), []
    for name in ranking:
        lifted.add(name)
        freed = sum(1 for names in pure.values() if names <= lifted)
        cumulative.append([name, freed])
    rows = [{"callee": name, "alone": alone[name], "touched": touched[name],
             "leaves": leaves[name], "units": sorted(units[name])} for name in ranking]
    record = {
        "functions_in_classes_alone": len(pure),
        "functions_touched_but_mixed": mixed,
        "callees": len(ranking),
        "ranking": rows,
        "fixed_point_in_rank_order": cumulative,
    }
    for row, (_, freed) in zip(rows[:args.top], cumulative):
        print("%-48s alone %3d touched %3d leaves %3d cumulative %3d  %s" % (
            row["callee"], row["alone"], row["touched"], row["leaves"], freed,
            ",".join(row["units"][:3])))
    print("pure %d, mixed %d, callees %d" % (len(pure), mixed, len(ranking)))
    if args.json:
        args.json.write_text(json.dumps(record, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
