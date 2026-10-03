#!/usr/bin/env python3
"""CAST-FORMALIZATION: the cast census (task 1) over one route's run.

Usage: census.py DIAGNOSTICS PARITY.jsonl OUT.json

Reads the route's `resource-diagnostics.json` and its `QCORE_TYPED_PATH_PARITY`
stream. Every `cast-semantics-census` row is one explicit cast (a Clang
`CStyleCastExpr`) of a function body, with its Clang kind, conversion class
(`cast_semantics::classify`), position and the line of its enclosing
statement. Rows are de-duplicated by (unit source, function, line, statement
line, tokens): a body the route builds twice records twice.

A leaf (one diagnostic of a function) is a **cast leaf** when a cast of the
same function sits on the leaf's line or its statement starts there. The
census counts, per (class, position): the casts, the cast leaves and their
refusal families (backticked names erased), the functions and the Unknown
functions holding one; and the Unknown functions every one of whose leaves is
a cast leaf ("sole": the upper bound of what a cast reading alone could free).

The join is by function name and line: report rows carry the unit file,
parity rows the unit's source digest, so two units defining one name with a
cast leaf on one line would be merged (none is expected; the count of such
names is reported as `ambiguous_names`).
"""

import argparse
import collections
import json
import pathlib
import re

ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}
TICKS = re.compile(r"`[^`]*`")


def family(description):
    return TICKS.sub("`_`", description)


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("diagnostics")
    parser.add_argument("parity")
    parser.add_argument("out")
    args = parser.parse_args()
    data = load(args.diagnostics)

    seen = set()
    casts = collections.defaultdict(list)  # function -> [row]
    for line in open(args.parity, encoding="utf-8"):
        if '"cast-semantics-census"' not in line:
            continue
        row = json.loads(line)
        key = (row["source_sha256"], row["function"], row["line"], row["statement_line"],
               tuple(row.get("texts") or ()))
        if key in seen:
            continue
        seen.add(key)
        casts[row["function"]].append(row)

    functions = {}
    units_of = collections.defaultdict(set)
    for unit in data["units"]:
        for entry in unit["functions"]:
            key = (unit["file"], entry["function"])
            units_of[entry["function"]].add(unit["file"])
            record = functions.setdefault(key, {"status": entry["status"], "leaves": []})
            if ORDER.get(entry["status"], 2) > ORDER.get(record["status"], 2):
                record["status"] = entry["status"]
            record["leaves"] += [(d.get("line"), d.get("category", ""), d.get("description", ""))
                                 for d in entry.get("diagnostics") or []]

    by_cell = collections.defaultdict(lambda: {
        "casts": 0, "leaves": 0, "families": collections.Counter(), "functions": set(),
        "unknown_functions": set(), "kinds": collections.Counter()})
    for function, rows in casts.items():
        for row in rows:
            cell = by_cell[(row["class"], row["cast_position"])]
            cell["casts"] += 1
            cell["kinds"][row["cast_kind"]] += 1

    leaf_cells = collections.Counter()
    sole, any_cast = [], []
    families = collections.Counter()
    total_leaves = 0
    ambiguous = set()
    for (file, function), record in functions.items():
        rows = casts.get(function, [])
        if not rows:
            continue
        if len(units_of[function]) > 1:
            ambiguous.add(function)
        lines = collections.defaultdict(set)
        for row in rows:
            for at in (row["line"], row["statement_line"]):
                if at is not None:
                    lines[at].add((row["class"], row["cast_position"]))
        cast_leaves = 0
        for at, category, description in record["leaves"]:
            cells = lines.get(at)
            if not cells:
                continue
            cast_leaves += 1
            total_leaves += 1
            families[family(description)] += 1
            for cell_key in cells:
                cell = by_cell[cell_key]
                cell["leaves"] += 1
                cell["families"][family(description)] += 1
                cell["functions"].add("%s::%s" % (file, function))
                if record["status"] == "Unknown":
                    cell["unknown_functions"].add("%s::%s" % (file, function))
                leaf_cells[cell_key] += 1
        if cast_leaves:
            any_cast.append("%s::%s" % (file, function))
            if record["status"] == "Unknown" and cast_leaves == len(record["leaves"]):
                sole.append("%s::%s" % (file, function))

    cells = []
    for (cls, position), cell in sorted(by_cell.items(), key=lambda kv: -kv[1]["leaves"]):
        cells.append({
            "class": cls, "position": position, "casts": cell["casts"],
            "kinds": dict(cell["kinds"]), "leaves": cell["leaves"],
            "functions": len(cell["functions"]), "unknown_functions": len(cell["unknown_functions"]),
            "families": dict(cell["families"].most_common(12)),
        })
    summary = {
        "casts": sum(len(rows) for rows in casts.values()),
        "functions_with_casts": len(casts),
        "cast_leaves": total_leaves,
        "functions_with_cast_leaves": len(any_cast),
        "sole_unknown_functions": len(sole),
        "ambiguous_names": sorted(ambiguous),
        "families": dict(families.most_common(40)),
        "by_class": {cls: sum(c["casts"] for c in cells if c["class"] == cls)
                     for cls in sorted({c["class"] for c in cells})},
        "by_position": {pos: sum(c["casts"] for c in cells if c["position"] == pos)
                        for pos in sorted({c["position"] for c in cells})},
        "cells": cells,
        "sole": sorted(sole),
    }
    pathlib.Path(args.out).write_text(json.dumps(summary, indent=1, ensure_ascii=False) + "\n")
    text = pathlib.Path(args.out).with_suffix(".txt")
    lines = ["casts %d in %d functions; cast leaves %d in %d functions; sole-cast Unknown %d" % (
        summary["casts"], summary["functions_with_casts"], summary["cast_leaves"],
        summary["functions_with_cast_leaves"], summary["sole_unknown_functions"])]
    lines.append("")
    lines.append("%-20s %-24s %7s %7s %6s %6s" % ("class", "position", "casts", "leaves", "fns", "unk"))
    for c in cells:
        lines.append("%-20s %-24s %7d %7d %6d %6d" % (c["class"], c["position"], c["casts"],
                                                       c["leaves"], c["functions"], c["unknown_functions"]))
    lines.append("")
    lines.append("families of cast leaves:")
    for name, count in families.most_common(40):
        lines.append("%6d  %s" % (count, name))
    text.write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
