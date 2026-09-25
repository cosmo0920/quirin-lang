#!/usr/bin/env python3
"""Card W45-SUB-OBJECT-STORES: attribute every diagnostic and status delta.

Usage: attribute.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS AFTER_STORES_JSONL
                    AFTER_STATEMENTS_JSONL BEFORE_STATEMENTS_JSONL
                    [--json OUT] [--text OUT]

The AFTER streams are the sub-object store and statement census of the AFTER
tree; their run reproduces the AFTER diagnostics byte for byte.

Classes, per removed or added leaf (a (unit, function, line, category,
description) multiset element):

* `store-admitted` — a removed gate leaf at a line where the AFTER tree
  admitted a sub-object store (`SubObjectStore`) and no gate refusal remains;
* `store-obligation` — an added leaf at a line where a store was admitted: the
  admitted store's own obligation (a stored call's argument, a base's
  C02-P-ALIAS peer, a later name of an escaped identity) now reports;
* `callee-summary` — a removed `call to uncontracted function X` leaf whose
  callee X is a function of the same unit with an admitted store (it gained a
  validated W4 summary), and a removed leaf at a line whose BEFORE statement
  (statement census of the BEFORE tree) calls such a callee or a function
  whose status moved (its summary changed, so the call's actuals are now
  admitted);
* `escape-names-unknown` — an added `unknown_alias` of a name in a function
  with an admitted store: after an escape every name of the escaped identity
  is unknown, so a later use or release reports it;
* `downstream` — any other delta in a function with an admitted store;
* `unattributed` — everything else (the card requires none).
"""

import collections
import gzip
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import store_census as census  # noqa: E402


def leaves(document):
    out = collections.Counter()
    for unit in document["units"]:
        for function in unit["functions"]:
            for diagnostic in function["diagnostics"]:
                out[
                    (
                        unit["file"],
                        function["function"],
                        diagnostic["line"],
                        diagnostic["category"],
                        diagnostic["description"],
                    )
                ] += 1
    return out


def statuses(document):
    return {
        (unit["file"], function["function"]): function["status"]
        for unit in document["units"]
        for function in unit["functions"]
    }


def main():
    before_path, after_path, stores_path, statements_path, before_statements_path = sys.argv[1:6]
    options = sys.argv[6:]
    json_out = options[options.index("--json") + 1] if "--json" in options else None
    text_out = options[options.index("--text") + 1] if "--text" in options else None
    before = census.read_json(before_path)
    after = census.read_json(after_path)
    stores_raw = census.load(stores_path)
    statements_raw = census.load(statements_path)
    mapping = census.join_units(stores_raw + statements_raw, after)
    stores = census.dedupe(stores_raw, mapping)
    files = [unit["file"] for unit in after["units"]]

    admitted_lines = set()
    admitted_functions = set()
    for key, record in stores.items():
        if record["outcome"] == "SubObjectStore":
            unit, function, line = files[key[0]], key[1], key[2]
            admitted_lines.add((unit, function, line))
            admitted_functions.add((unit, function))

    sb_, sa_ = statuses(before), statuses(after)
    moved = collections.defaultdict(set)
    for key in sb_:
        if sb_[key] != sa_.get(key):
            moved[key[0]].add(key[1])
    before_statements_raw = census.load(before_statements_path)
    before_mapping = census.join_units(before_statements_raw, before)
    before_files = [unit["file"] for unit in before["units"]]
    calls_at = collections.defaultdict(set)
    for key, record in census.dedupe(before_statements_raw, before_mapping).items():
        texts = record["texts"]
        for index in range(len(texts) - 1):
            if texts[index + 1] == "(":
                calls_at[(before_files[key[0]], key[1], key[2])].add(texts[index])
    b, a = leaves(before), leaves(after)
    removed, added = b - a, a - b
    callees_with_store = collections.defaultdict(set)
    for unit, function in admitted_functions:
        callees_with_store[unit].add(function)
    after_lines = {
        (key[0], key[1], key[2]) for key in a if key[4] == census.GATE
    }

    rows = []
    for sign, delta in (("-", removed), ("+", added)):
        for key, count in delta.items():
            unit, function, line, category, description = key
            where = (unit, function, line)
            callee = re.match(r"call to uncontracted function `([^`]*)`", description)
            if sign == "-" and description == census.GATE and where in admitted_lines and where not in after_lines:
                cls = "store-admitted"
            elif sign == "+" and where in admitted_lines:
                cls = "store-obligation"
            elif sign == "-" and callee and callee.group(1) in callees_with_store[unit]:
                cls = "callee-summary"
            elif sign == "-" and calls_at[where] & (callees_with_store[unit] | moved[unit]):
                cls = "callee-summary"
            elif (
                sign == "+"
                and category == "unknown_alias"
                and (unit, function) in admitted_functions
            ):
                cls = "escape-names-unknown"
            elif (unit, function) in admitted_functions:
                cls = "downstream"
            else:
                cls = "unattributed"
            rows.append(
                {
                    "sign": sign,
                    "count": count,
                    "class": cls,
                    "unit": unit,
                    "function": function,
                    "line": line,
                    "category": category,
                    "description": description,
                }
            )

    sb, sa = statuses(before), statuses(after)
    moves = []
    for key in sorted(sb):
        if sb[key] != sa.get(key):
            reason = (
                "store-admitted"
                if key in admitted_functions
                else "callee-summary"
                if any(
                    row["unit"] == key[0] and row["function"] == key[1] and row["class"] == "callee-summary"
                    for row in rows
                )
                else "unattributed"
            )
            moves.append(
                {"unit": key[0], "function": key[1], "before": sb[key], "after": sa.get(key), "reason": reason}
            )

    summary = collections.Counter()
    for row in rows:
        summary[(row["sign"], row["class"], row["category"])] += row["count"]
    unattributed = sum(row["count"] for row in rows if row["class"] == "unattributed") + sum(
        1 for move in moves if move["reason"] == "unattributed"
    )
    payload = {
        "attribution_complete": unattributed == 0,
        "unattributed": unattributed,
        "diagnostic_counts": {"before": before["diagnostic_counts"], "after": after["diagnostic_counts"]},
        "classes": [
            {"sign": sign, "class": cls, "category": category, "leaves": count}
            for (sign, cls, category), count in sorted(summary.items())
        ],
        "moves": moves,
        "admitted_store_statements": sum(
            1 for record in stores.values() if record["outcome"] == "SubObjectStore"
        ),
        "functions_with_an_admitted_store": len(admitted_functions),
    }
    lines = [json.dumps({k: v for k, v in payload.items() if k != "moves"}, indent=1)]
    lines += [f"{m['unit']}::{m['function']} {m['before']} -> {m['after']} ({m['reason']})" for m in moves]
    text = "\n".join(lines) + "\n"
    if text_out:
        pathlib.Path(text_out).write_text(text)
    sys.stdout.write(text)
    if json_out:
        pathlib.Path(json_out).write_text(json.dumps(payload, indent=1) + "\n")
        full = json.dumps(rows, indent=1).encode()
        pathlib.Path(json_out.replace(".json", "-leaves.json.gz")).write_bytes(gzip.compress(full, mtime=0))
    return 0


if __name__ == "__main__":
    sys.exit(main())
