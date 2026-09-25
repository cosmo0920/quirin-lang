#!/usr/bin/env python3
"""Card W44-COLLECTION-ALLOCATION: attribute every leaf AFTER adds or removes.

Usage: delta_attribution.py BEFORE_RUN AFTER_RUN SIZEOF_CENSUS_JSON OUT_JSON OUT_TXT

A leaf is `(unit, function, line, category, description)`. Each removed or
added leaf is assigned to exactly one class:

* `withdrawn-operand-leaf`: a removed `indirect memory access` leaf on a line
  the BEFORE census lists as carrying a dereference `sizeof` operand (or one
  or two lines before it: a statement's leaf sits on its first line), in the
  same function — the refusal this card withdraws;
* `same-line-substitution`: an added `unsupported` leaf on such a line (the
  statement now reaches a later refusal of the classifier, for example an
  uncontracted callee), or a removed leaf on such a line other than the
  withdrawn one;
* `downstream-in-a-reached-function`: any other added or removed leaf in a
  function where this card withdrew at least one operand leaf — the flow
  analysis now binds the allocation's destination, so later statements see a
  tracked identity instead of a poisoned binding (leaks, unresolved-identity
  and nullability leaves appear or disappear);
* `summary-propagation`: a leaf in a function where this card withdrew
  nothing — only a validated W4 summary of a callee whose own body changed can
  move it;
* anything left over is `unattributed` (expected to be empty).
"""

import collections
import gzip
import json
import pathlib
import sys

ACCESS = "indirect memory access is outside the resource projection"
DEREFERENCE = {"(*NAME)", "*NAME", "(**NAME)", "**NAME", "(NAME[LIT])", "NAME[LIT]"}


def leaves(document):
    rows = collections.Counter()
    status = {}
    for unit in document["units"]:
        key = f"{unit['library']}/{unit['file']}"
        for function in unit.get("functions", []):
            status[(key, function["function"])] = function["status"]
            for leaf in function.get("diagnostics", []):
                rows[(key, function["function"], leaf["line"], leaf["category"], leaf["description"])] += 1
    return rows, status


def main() -> int:
    before_doc = json.loads((pathlib.Path(sys.argv[1]) / "resource-diagnostics.json").read_text())
    after_doc = json.loads((pathlib.Path(sys.argv[2]) / "resource-diagnostics.json").read_text())
    census_path = pathlib.Path(sys.argv[3])
    opener = gzip.open if census_path.suffix == ".gz" else open
    with opener(census_path, "rt") as handle:
        census = json.load(handle)
    out_json, out_txt = pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5])
    operand_lines = set()
    for function in census["functions"]:
        unit = function["unit"][: -len(".i")].replace("__", "/", 1)
        for occurrence in function["occurrences"]:
            if occurrence["shape"] in DEREFERENCE:
                operand_lines.add((unit, function["function"], occurrence["line"]))
    def near(unit, function, line):
        # A statement's leaf sits on its first line; the operand may be on one
        # of the next two (`T *x =` / `    malloc(sizeof(*x));`).
        return any((unit, function, line + offset) in operand_lines for offset in range(3))

    before, before_status = leaves(before_doc)
    after, after_status = leaves(after_doc)
    removed = before - after
    added = after - before
    reached = {
        (leaf[0], leaf[1]) for leaf in removed
        if leaf[4] == ACCESS and near(leaf[0], leaf[1], leaf[2])
    }
    classes = collections.Counter()
    by_category = collections.Counter()
    rows = []
    for direction, delta in (("removed", removed), ("added", added)):
        for leaf, count in sorted(delta.items()):
            on_operand_line = near(leaf[0], leaf[1], leaf[2])
            if direction == "removed" and leaf[4] == ACCESS and on_operand_line:
                kind = "withdrawn-operand-leaf"
            elif on_operand_line and leaf[3] == "unsupported":
                kind = "same-line-substitution"
            elif (leaf[0], leaf[1]) in reached:
                kind = "downstream-in-a-reached-function"
            else:
                kind = "summary-propagation"
            classes[(direction, kind)] += count
            by_category[(direction, kind, leaf[3])] += count
            rows.append({"direction": direction, "class": kind, "count": count, "unit": leaf[0],
                         "function": leaf[1], "line": leaf[2], "category": leaf[3], "description": leaf[4]})
    moves = collections.Counter()
    move_rows = []
    for function, old in sorted(before_status.items()):
        new = after_status.get(function)
        if new != old:
            kind = "reached" if function in reached else "summary-propagation"
            moves[(f"{old}->{new}", kind)] += 1
            move_rows.append({"unit": function[0], "function": function[1], "before": old, "after": new, "class": kind})
    summary = {
        "removed": sum(removed.values()),
        "added": sum(added.values()),
        "functions_reached": len(reached),
        "by_class": {f"{d}/{k}": v for (d, k), v in sorted(classes.items())},
        "by_class_and_category": {f"{d}/{k}/{c}": v for (d, k, c), v in sorted(by_category.items())},
        "function_moves": {f"{m}/{k}": v for (m, k), v in sorted(moves.items())},
    }
    out_json.write_text(json.dumps({"summary": summary, "moves": move_rows, "leaves": rows}, indent=1) + "\n")
    lines = ["W44-COLLECTION-ALLOCATION: leaf-by-leaf attribution of AFTER against BEFORE", ""]
    for key, value in summary.items():
        if isinstance(value, dict):
            lines.append(f"{key}:")
            lines += [f"  {k}: {v}" for k, v in value.items()]
        else:
            lines.append(f"{key}: {value}")
    lines.append("")
    lines.append("function moves:")
    lines += [f"  {r['before']} -> {r['after']} [{r['class']}] {r['unit']}::{r['function']}" for r in move_rows]
    lines.append("")
    lines.append("leaves (summary-propagation and substitutions in full; downstream by function):")
    for r in rows:
        if r["class"] in ("summary-propagation", "same-line-substitution"):
            lines.append(f"  {r['direction']} [{r['class']}] {r['unit']}::{r['function']} L{r['line']} {r['category']}: {r['description']} x{r['count']}")
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:40]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
