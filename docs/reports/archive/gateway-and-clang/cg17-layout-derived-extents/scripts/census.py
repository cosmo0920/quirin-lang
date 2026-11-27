#!/usr/bin/env python3
"""census.py PARITY.jsonl [BEFORE_RUN] -- card CG17-LAYOUT-DERIVED-EXTENTS:
the layout census of one parity stream (`position: layout-extents-*`).

Counts, de-duplicated by (source, function, line, form, fields):
* every census row by form and decision (VLA, flexible-array access,
  container_of shape, derived-address conversion, offsetof count);
* every consultation of the route-G hooks (VLA declaration, container_of
  statement and return) by answer;
and, given a BEFORE run directory, the BEFORE diagnostics that stand on a
line holding a census form (the leaves the card could touch), by form.
"""

import collections
import gzip
import json
import pathlib
import sys


def rows(path):
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            if '"layout-extents-' not in line:
                continue
            row = json.loads(line)
            if row.get("position", "").startswith("layout-extents-"):
                yield row


def key(row):
    fields = {k: v for k, v in row.items() if k not in ("dialect", "frontend_sha256")}
    return json.dumps(fields, sort_keys=True)


def load_report(run):
    path = pathlib.Path(run) / "resource-diagnostics.json"
    if not path.exists():
        path = pathlib.Path(str(path) + ".gz")
        return json.load(gzip.open(path, "rt", encoding="utf-8"))
    return json.loads(path.read_text(encoding="utf-8"))


def main():
    parity = sys.argv[1]
    seen = set()
    census = collections.Counter()
    hooks = collections.Counter()
    functions = collections.defaultdict(set)
    lines = collections.defaultdict(set)
    vla_rows = []
    for row in rows(parity):
        k = key(row)
        if k in seen:
            continue
        seen.add(k)
        position = row["position"]
        if position == "layout-extents-census":
            form = row["form"]
            if form == "offsetof":
                census[("offsetof", "expressions")] += row["count"]
                functions["offsetof"].add(row["function"])
                continue
            decision = row.get("decision") or row.get("extent", "")
            if form == "flexible-array":
                decision = row.get("extent", "")
            census[(form, decision)] += 1
            functions[form].add(row["function"])
            lines[form].add((row["function"], row["line"]))
            if form == "vla":
                vla_rows.append(row)
        else:
            answer = row.get("decision") or row.get("answer", "")
            hooks[(position, answer)] += 1
    out = {
        "census": {f"{form} / {decision}": n for (form, decision), n in sorted(census.items())},
        "functions": {form: len(names) for form, names in sorted(functions.items())},
        "hooks": {f"{position} / {answer}": n for (position, answer), n in sorted(hooks.items())},
        "vla": vla_rows,
    }
    if len(sys.argv) > 2:
        report = load_report(sys.argv[2])
        leaves = collections.Counter()
        # Parity rows name no unit: the join is by (function, line), so a
        # static function name shared by two units may join both.
        for unit in report["units"]:
            for function in unit["functions"]:
                name = function["function"]
                for diagnostic in function.get("diagnostics", []):
                    line = diagnostic.get("line")
                    label = diagnostic.get("category", "")
                    if diagnostic.get("description"):
                        label += ": " + diagnostic["description"]
                    for form, places in lines.items():
                        if (name, line) in places:
                            leaves[(form, label)] += 1
        out["before_leaves_on_census_lines"] = {
            f"{form} / {kind}": n for (form, kind), n in sorted(leaves.items())
        }
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
