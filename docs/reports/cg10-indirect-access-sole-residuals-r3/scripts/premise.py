#!/usr/bin/env python3
"""MEMBER-ACCESS-PROJECTION step 1, stage 2: one refusal premise per
indirect-access leaf, first that holds in a fixed order (README section 1).

usage: premise.py REPORT IND_LEAVES OUT_DIR [--streams RUN_DIR]
"""
import collections
import gzip
import json
import pathlib
import re
import sys

IND = "indirect memory access is outside the resource projection"
UNCONTRACTED = re.compile(r"^call to uncontracted function `([^`]+)`")
FORM = ["indirect-call", "union", "bitfield", "volatile-atomic", "statement-expression", "cast",
        "pointer-arithmetic", "pointer-field-step", "aggregate-value", "incomplete-record"]


def callee_table(report):
    contracted, uncontracted = set(), set()
    for unit in report["units"]:
        for function in unit["functions"]:
            deps = function.get("contract_dependencies") or {}
            contracted.update(deps.get("declarations", []))
            contracted.update(deps.get("summaries", []))
            for leaf in function["diagnostics"]:
                match = UNCONTRACTED.match(leaf["description"])
                if match:
                    uncontracted.add(match.group(1))
    return contracted, uncontracted - contracted


def path_premise(path):
    """The premise one path fails, in the order of the README."""
    causes = set(path["causes"])
    for cause in FORM:
        if cause in causes:
            return "form:" + cause
    for cause in sorted(causes):
        if cause.startswith("root:"):
            return "form:other-root"
    if "static-local" in causes:
        return "root:static-local"
    if path["root_kind"] == "global":
        return "root:file-scope"
    if path["root_kind"] == "call-result":
        return "root:call-result"
    if path["root_kind"] == "other":
        return "form:other-root"
    access, value = path["access"], path["value"]
    if access in ("store", "null-store", "increment"):
        if value in ("pointer", "function_pointer"):
            rhs = path.get("rhs", "?")
            if access == "null-store" or rhs in ("null", "null+cast"):
                return "store:pointer-null"
            if rhs.startswith("call:"):
                return "store:pointer-call-result"
            if rhs.startswith(("formal", "variable")):
                return "store:pointer-name"
            if rhs.startswith(("string", "function")):
                return "store:pointer-identity-free"
            return "store:pointer-other"
        return "store:custody-free"
    if value == "address":
        return "address:sub-object"
    if value in ("pointer", "function_pointer"):
        return "read:pointer-member"
    return "read:custody-free"


def position(path):
    if path.get("call_argument"):
        return "call-argument"
    if path["access"] in ("store", "null-store", "increment"):
        return "store"
    if path.get("declaration"):
        return "local-initializer"
    return "expression"


def main():
    report = json.load(open(sys.argv[1]))
    rows = [json.loads(line) for line in gzip.open(sys.argv[2], "rt")]
    out = pathlib.Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    contracted, uncontracted = callee_table(report)
    leaf_premise = []
    for row in rows:
        paths = row["paths"]
        callees = [c for c in row["calls"]]
        # Statement-level premises first.
        if not paths:
            premise = "no-evaluated-path"
        else:
            per_path = [path_premise(p) for p in paths]
            forms = [p for p in per_path if p.startswith(("form:", "root:"))]
            unc = [c for c in callees if c in uncontracted]
            if forms:
                premise = sorted(forms)[0]
            elif callees and callees[0] == "<indirect>":
                premise = "call:indirect-callee"
            elif unc:
                premise = "call:uncontracted-callee"
            else:
                # The most custody-bearing path decides the leaf.
                order = ["store:pointer-call-result", "store:pointer-name", "store:pointer-other",
                         "store:pointer-identity-free", "store:pointer-null", "read:pointer-member",
                         "address:sub-object", "store:custody-free", "read:custody-free"]
                premise = min(per_path, key=order.index)
        row["premise"] = premise
        row["path_premises"] = [path_premise(p) for p in paths]
        leaf_premise.append(row)
    table = collections.defaultdict(lambda: {"sites": 0, "functions": set(), "sole": set(),
                                             "macro": 0, "units": set()})
    sole_functions = set()
    for row in leaf_premise:
        key = (row["file"], row["function"])
        if set(row["families"]) <= {IND} and row["status"] == "Unknown":
            sole_functions.add(key)
    by_function = collections.defaultdict(set)
    for row in leaf_premise:
        by_function[(row["file"], row["function"])].add(row["premise"])
    for row in leaf_premise:
        entry = table[row["premise"]]
        entry["sites"] += 1
        entry["functions"].add((row["file"], row["function"]))
        entry["units"].add(row["file"])
        entry["macro"] += row["macro"]
        key = (row["file"], row["function"])
        if key in sole_functions and by_function[key] == {row["premise"]}:
            entry["sole"].add(key)
    lines = ["| premise | sites | macro-expanded | functions | sole-blocker functions | units |",
             "|---|---:|---:|---:|---:|---:|"]
    for premise, entry in sorted(table.items(), key=lambda item: -item[1]["sites"]):
        lines.append(f"| `{premise}` | {entry['sites']} | {entry['macro']} | {len(entry['functions'])} | "
                     f"{len(entry['sole'])} | {len(entry['units'])} |")
    total_sites = sum(e["sites"] for e in table.values())
    lines.append(f"| total | {total_sites} | {sum(e['macro'] for e in table.values())} | "
                 f"{len(by_function)} | {len(sole_functions)} (sole IND) | "
                 f"{len({r['file'] for r in leaf_premise})} |")
    (out / "premise-table.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    mixed = sum(1 for k in sole_functions if len(by_function[k]) > 1)
    print(f"sole-IND functions with more than one premise: {mixed}")
    with gzip.open(out / "premises.jsonl.gz", "wt") as handle:
        for row in leaf_premise:
            handle.write(json.dumps(row) + "\n")
    # Path-level table over every path of every leaf, by position.
    paths = collections.Counter((position(p), path_premise(p)) for r in leaf_premise for p in r["paths"])
    with open(out / "path-table.md", "w") as handle:
        handle.write("| position | path premise | paths |\n|---|---|---:|\n")
        for (pos, prem), count in sorted(paths.items(), key=lambda item: -item[1]):
            handle.write(f"| {pos} | `{prem}` | {count} |\n")


if __name__ == "__main__":
    main()
