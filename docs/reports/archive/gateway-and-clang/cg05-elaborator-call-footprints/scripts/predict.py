#!/usr/bin/env python3
"""CG05-ELABORATOR-CALL-FOOTPRINTS: the functions this card could move, from
one BEFORE run (its diagnostics and the census.py rows).

usage: predict.py RUN_DIR ROWS.jsonl [--json OUT]

A function is touched when one of its elaborator rows is refused as a call
`unsequenced with another event (CG05)` (both follow-up shapes), or one of its
nested-call statement rows as `footprints do not commute`. It is freed alone
(upper bound) when every non-Safe leaf it has lies on such a row's line;
`strict` also requires no `unknown_alias` leaf. Parity rows carry the function
name and the preprocessed line, which every diagnostic carries too.
"""

import argparse
import collections
import json
import pathlib


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("rows", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    parser.add_argument("--show", action="store_true", help="print the freed functions' rows")
    parser.add_argument("--residual", action="store_true", help="print the rows that are not uncontracted")
    args = parser.parse_args()
    lines = collections.defaultdict(set)
    family = collections.defaultdict(set)
    population = []
    for raw in args.rows.open():
        row = json.loads(raw)
        population.append(row)
        reason = row.get("new_refusal") or ""
        if "unsequenced with another event (CG05)" in reason:
            lines[row["function"]].add(row["line"])
            family[row["function"]].add("elaborator")
        elif "footprints do not commute" in reason:
            lines[row["function"]].add(row["line"])
            family[row["function"]].add("address")
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    touched = {}
    for unit in data["units"]:
        for function in unit.get("functions") or []:
            name = function.get("function")
            if name not in lines or function.get("status") == "Safe":
                continue
            leaves = [d for d in function.get("diagnostics", []) if d.get("category") != "note"]
            outside = [d for d in leaves if d.get("line") not in lines[name]]
            alias = [d for d in outside if d.get("category") == "unknown_alias"]
            blocking = [d for d in outside if d.get("category") != "unknown_alias"]
            key = f"{pathlib.Path(unit['file']).name}::{name}"
            touched[key] = {
                "status": function.get("status"),
                "family": sorted(family[name]),
                "leaves_on_population_lines": len(leaves) - len(outside),
                "population_leaves": collections.Counter(
                    d["description"] for d in leaves if d.get("line") in lines[name]
                ).most_common(6),
                "other_blocking": collections.Counter(d["description"] for d in blocking).most_common(4),
                "other_alias": len(alias),
                "freed_alone": not blocking,
                "freed_alone_strict": not blocking and not alias,
            }
    # Each population row's own leaves, by whether one names its callee
    # uncontracted (no footprint can be read: it stays refused).
    leaves_at = collections.defaultdict(list)
    for unit in data["units"]:
        for function in unit.get("functions") or []:
            name = function.get("function")
            if name in lines:
                for leaf in function.get("diagnostics", []):
                    if leaf.get("line") in lines[name]:
                        leaves_at[(name, leaf["line"])].append(leaf["description"])
    row_classes = collections.Counter()
    residual = []
    for row in population:
        reason = row.get("new_refusal") or ""
        if row["new_admitted"] or not ("unsequenced with another event (CG05)" in reason
                                       or "footprints do not commute" in reason):
            continue
        callee = reason.split("`")[1] if reason.count("`") >= 2 else ""
        found = leaves_at.get((row["function"], row["line"]), [])
        if any(f"uncontracted function `{callee}`" in leaf for leaf in found):
            row_classes["callee uncontracted (stays refused)"] += 1
            continue
        elif not found:
            row_classes["no leaf on the line"] += 1
        else:
            row_classes["other leaf on the line"] += 1
        residual.append((row, found))
    record = {
        "population_rows_by_leaf": dict(row_classes.most_common()),
        "touched_functions": len(touched),
        "touched_by_family": dict(collections.Counter("+".join(v["family"]) for v in touched.values())),
        "freed_alone": sorted(k for k, v in touched.items() if v["freed_alone"]),
        "freed_alone_strict": sorted(k for k, v in touched.items() if v["freed_alone_strict"]),
        "functions": touched,
    }
    text = json.dumps(record, indent=1, ensure_ascii=False)
    if args.json:
        args.json.write_text(text + "\n")
    print(json.dumps({k: record[k] for k in ("population_rows_by_leaf", "touched_functions", "touched_by_family", "freed_alone", "freed_alone_strict")}, indent=1))
    if args.residual:
        for row, found in residual:
            print(row["function"], " ".join(row["texts"])[:110], "--", found[:2])
    if args.show:
        freed = {key.split("::", 1)[1] for key in record["freed_alone"]}
        for row in population:
            if row["function"] in freed and not row["new_admitted"]:
                print(row["function"], row["line"], " ".join(row["texts"])[:120], "--", (row.get("new_refusal") or "")[:50])
        for key in record["freed_alone"]:
            print("LEAVES", key, record["functions"][key]["population_leaves"])


if __name__ == "__main__":
    main()
