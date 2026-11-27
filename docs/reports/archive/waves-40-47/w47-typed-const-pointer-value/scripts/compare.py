#!/usr/bin/env python3
"""Verdict-and-leaf comparison of two resource-diagnostics.json files.

Usage: compare.py BEFORE AFTER OUT_JSON [--parity AFTER_PARITY] [--census CENSUS_STDERR]

Rows are keyed by (unit file, configuration index within the file, function).
For every row whose status or leaf multiset (category, description, line)
changes it records both, the removed and added leaves, and an attribution:

* ``direct``: every removed leaf is an unsupported COND/RET/IND leaf on a line
  where the AFTER parity stream has an admitted typed row of the same
  function that the BEFORE run did not admit (``--parity`` names the AFTER
  stream; the BEFORE stream is read from the sibling directory given by
  ``--before-parity``), and every added leaf is an ``unknown_alias`` leaf or
  sits on such a line;
* ``cascade``: no leaf of the function's own changed lines is at a newly
  admitted row (a summary, callee or unit effect), listed for reading;

and whether the function's census rows (``--census``) cover it.
"""
import argparse
import collections
import json


def family(d):
    desc = d.get("description") or ""
    if d.get("category") != "unsupported":
        return d.get("category")
    if desc == "indirect memory access is outside the resource projection":
        return "IND"
    if desc.startswith("unmodeled call, aggregate or indirect access in return"):
        return "RET"
    if desc.startswith("unmodeled call, aggregate or indirect access in"):
        return "COND"
    return "OTHER"


def load(path):
    data = json.load(open(path))
    rows, units = {}, {}
    seen = collections.Counter()
    for unit in data["units"]:
        seen[unit["file"]] += 1
        ukey = (unit["file"], seen[unit["file"]])
        units[ukey] = unit.get("status")
        for row in unit.get("functions", []):
            key = (unit["file"], seen[unit["file"]], row["function"])
            leaves = collections.Counter(
                (d.get("category"), d.get("description"), d.get("line"))
                for d in row.get("diagnostics", []))
            rows[key] = (row.get("status"), leaves)
    return data, rows, units


def admitted(path):
    out = collections.defaultdict(set)
    if not path:
        return out
    for line in open(path):
        row = json.loads(line)
        if row.get("position") == "root-declarations":
            continue
        if row.get("new_admitted") and not row.get("old_admitted"):
            out[row["function"]].add((row["line"], row.get("position", "statement"),
                                      " ".join(row["texts"])))
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("before")
    parser.add_argument("after")
    parser.add_argument("out")
    parser.add_argument("--parity")
    parser.add_argument("--before-parity")
    parser.add_argument("--census")
    args = parser.parse_args()
    bdata, before, bunits = load(args.before)
    adata, after, aunits = load(args.after)
    new_rows = admitted(args.parity)
    old_rows = admitted(args.before_parity)
    census = collections.defaultdict(set)
    if args.census:
        for line in open(args.census, errors="replace"):
            if line.startswith("W47CP\t"):
                _, function, row_line, position, bits, texts = line.rstrip("\n").split("\t", 5)
                census[function].add((int(row_line), int(bits)))
    changed = []
    moves = collections.Counter()
    for key in sorted(set(before) | set(after)):
        bs, bl = before.get(key, (None, collections.Counter()))
        as_, al = after.get(key, (None, collections.Counter()))
        if bs == as_ and bl == al:
            continue
        removed = bl - al
        added = al - bl
        function = key[2]
        lines = {line for (line, _, _) in new_rows.get(function, set()) - old_rows.get(function, set())}
        direct = bool(removed or added) and all(
            family({"category": c, "description": d}) in ("IND", "RET", "COND") and line in lines
            for (c, d, line) in removed) and all(
            c == "unknown_alias" or line in lines for (c, d, line) in added)
        entry = {
            "file": key[0], "configuration": key[1], "function": function,
            "status": [bs, as_],
            "removed": [list(k) + [n] for k, n in sorted(removed.items(), key=str)],
            "added": [list(k) + [n] for k, n in sorted(added.items(), key=str)],
            "newly_admitted_rows": sorted([list(r) for r in
                                           new_rows.get(function, set()) - old_rows.get(function, set())]),
            "census_rows": sorted([list(r) for r in census.get(function, set())]),
            "attribution": "direct" if direct else "cascade-or-other",
        }
        changed.append(entry)
        if bs != as_:
            moves[f"{bs} -> {as_}"] += 1
    unit_moves = {f"{k[0]}#{k[1]}": [bunits.get(k), aunits.get(k)]
                  for k in sorted(set(bunits) | set(aunits)) if bunits.get(k) != aunits.get(k)}
    tally = lambda rows: dict(collections.Counter(s for s, _ in rows.values()))
    summary = {
        "functions_before": tally(before), "functions_after": tally(after),
        "units_before": dict(collections.Counter(bunits.values())),
        "units_after": dict(collections.Counter(aunits.values())),
        "diagnostic_counts_before": bdata.get("diagnostic_counts"),
        "diagnostic_counts_after": adata.get("diagnostic_counts"),
        "status_moves": dict(moves), "unit_moves": unit_moves,
        "changed_rows": len(changed),
        "attribution": dict(collections.Counter(e["attribution"] for e in changed)),
        "leaves_removed_by_family": dict(collections.Counter(
            family({"category": r[0], "description": r[1]}) for e in changed for r in e["removed"] for _ in range(r[3]))),
        "leaves_added_by_family": dict(collections.Counter(
            family({"category": r[0], "description": r[1]}) for e in changed for r in e["added"] for _ in range(r[3]))),
    }
    json.dump({"summary": summary, "changed": changed}, open(args.out, "w"), indent=1)
    print(json.dumps(summary, indent=1))
    for e in changed:
        if e["status"][0] != e["status"][1]:
            print("MOVE", e["status"], e["file"].split("/")[-1], e["configuration"], e["function"], e["attribution"])


if __name__ == "__main__":
    main()
