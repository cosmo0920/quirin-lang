#!/usr/bin/env python3
"""Card FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS: row-by-row movement between two
runs of one route: custody status moves, rows whose leaves changed, and null
property moves, each with the leaves that differ.

A row is (unit file, its occurrence among same-file units, function), so the
static and shared configurations of a core file stay apart. Leaves compare as
multisets of (category, description, origin).

usage: compare.py BEFORE_REPORT AFTER_REPORT BEFORE_NULL AFTER_NULL OUT.json
"""
import collections
import json
import sys


def rows(report):
    seen = collections.Counter()
    out = {}
    for unit in json.load(open(report))["units"]:
        seen[unit["file"]] += 1
        for function in unit.get("functions", []):
            key = (unit["file"], seen[unit["file"]], function["function"])
            leaves = collections.Counter(
                (d.get("category"), d.get("description"), d.get("origin"))
                for d in function.get("diagnostics", []))
            out[key] = (function.get("status"), leaves)
    return out


def null_rows(stream):
    seen = collections.Counter()
    out = {}
    for raw in open(stream):
        row = json.loads(raw)
        key0 = (row.get("library"), row.get("file"), row.get("function"))
        seen[key0] += 1
        null = row.get("null") or {}
        verdict = null.get("verdict")
        if verdict == "Safe":
            verdict = "Safe (conditional)" if null.get("assumptions") else "Safe (unconditional)"
        reasons = sorted(null.get("reasons") or [])
        out[key0 + (seen[key0],)] = (verdict, reasons, len(null.get("checks") or []))
    return out


def leaf_list(counter):
    return [{"category": c, "description": d, "origin": o, "count": n}
            for (c, d, o), n in sorted(counter.items(), key=lambda x: (x[0][2] or "", x[0][1] or ""))]


def main():
    before_report, after_report, before_null, after_null, out = sys.argv[1:6]
    before, after = rows(before_report), rows(after_report)
    moves, trades = [], []
    for key in sorted(set(before) | set(after)):
        b = before.get(key, (None, collections.Counter()))
        a = after.get(key, (None, collections.Counter()))
        if b == a:
            continue
        entry = {
            "unit": key[0], "occurrence": key[1], "function": key[2],
            "before": b[0], "after": a[0],
            "removed": leaf_list(b[1] - a[1]), "added": leaf_list(a[1] - b[1]),
        }
        (moves if b[0] != a[0] else trades).append(entry)
    nb, na = null_rows(before_null), null_rows(after_null)
    null_moves = []
    for key in sorted(set(nb) | set(na), key=lambda k: tuple(str(x) for x in k)):
        b, a = nb.get(key), na.get(key)
        if b != a:
            null_moves.append({"library": key[0], "file": key[1], "function": key[2],
                               "occurrence": key[3], "before": b, "after": a})
    status_counter = collections.Counter((m["before"], m["after"]) for m in moves)
    null_counter = collections.Counter(
        ((m["before"] or [None])[0], (m["after"] or [None])[0]) for m in null_moves)
    result = {
        "card": "FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS",
        "before": before_report, "after": after_report,
        "status_moves": {f"{b} -> {a}": n for (b, a), n in sorted(status_counter.items(), key=str)},
        "rows_trading_leaves": len(trades),
        "null_moves": {f"{b} -> {a}": n for (b, a), n in sorted(null_counter.items(), key=str)},
        "null_rows_changed": len(null_moves),
        "moves": moves, "trades": trades, "null": null_moves,
    }
    with open(out, "w") as handle:
        json.dump(result, handle, indent=1, ensure_ascii=False)
        handle.write("\n")
    print(json.dumps({k: result[k] for k in (
        "status_moves", "rows_trading_leaves", "null_moves", "null_rows_changed")}))


if __name__ == "__main__":
    main()
