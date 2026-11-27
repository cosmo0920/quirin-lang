#!/usr/bin/env python3
"""Size the refused typed positions by shape and rank shapes by functions.

Usage: rank_shapes.py RUN_DIR OUT_JSON [--system]

RUN_DIR holds `resource-diagnostics.json` and `parity.jsonl` of a run of the
parity-shape build (`typed_position_shapes.rs`), whose refused condition,
condition-assignment, return and call-actual rows each carry a
`residual-shape` row beside them.

A row's **shape** is `root-premise` when the same node is admitted with every
root premise granted, `no-unique-node` when no unique Clang node matched,
`unclassified` when the refused node shows no unread construct, and
otherwise the sorted set of unread constructs joined by `+`.

Join. A refused row belongs to the leaf of the same function with the
greatest internal line <= the row's line, within 6 lines (a multi-line
condition or call names its first line; P3's continuation rule). A COND,
RET or IND leaf is **covered by shape S** when it has at least one row and
every row it has is of shape S. An Unknown function is freed by S **alone**
when every leaf is covered by S (`strict`), or every leaf other than an
`unknown_alias` is (`with-alias`: the alias leaves may be poisoned by the
refusal, rule 10a, or may stay). Functions are keyed (unit file, function);
with `--system` they are joined over configurations (the worst status).
"""

import collections
import gzip
import json
import pathlib
import sys

POSITIONS = ("condition", "condition-assignment", "return", "call-argument")
CONTINUATION = 6
ORDER = {"Safe": 0, "Pending": 1, "Unknown": 2, "Unsafe": 3}


def family(leaf):
    category = leaf["category"]
    if category != "unsupported":
        return category
    description = leaf["description"]
    if description == "indirect memory access is outside the resource projection":
        return "IND"
    if description.startswith("call to uncontracted function"):
        return "UNC"
    if description == "unmodeled call, aggregate or indirect access in return expression":
        return "RET"
    if description.startswith("unmodeled call, aggregate or indirect access in"):
        return "COND"
    return "OTHER"


def shape(row):
    if row.get("admitted_with_every_root_premise"):
        return "root-premise"
    if row.get("node") != "unique":
        return "no-unique-node"
    return "+".join(row["features"]) or "unclassified"


def opener(path):
    return gzip.open(path, "rt") if str(path).endswith(".gz") else open(path)


def main():
    run, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    system = "--system" in sys.argv
    diagnostics = run / "resource-diagnostics.json"
    parity = run / "parity.jsonl"
    with opener(diagnostics) as handle:
        report = json.load(handle)

    # Refused position rows (the P3 refusal) and their residual-shape rows.
    refused = set()
    shapes = {}
    with opener(parity) as handle:
        for line in handle:
            row = json.loads(line)
            position = row.get("position")
            if position in POSITIONS and row.get("old_route") == "old_refusal" \
                    and not row.get("new_admitted"):
                refused.add((row["source_sha256"], row["function"], row["line"],
                             tuple(row["texts"]), position))
            elif position == "residual-shape":
                shapes[(row["source_sha256"], row["function"], row["line"],
                        tuple(row["texts"]), row["of_position"])] = shape(row)
    missing = refused - set(shapes)
    rows = collections.defaultdict(set)  # (function, line) -> {(position, shape, texts)}
    tally = collections.Counter()
    tally_functions = collections.defaultdict(set)
    for key in refused:
        _, function, line, texts, position = key
        label = shapes.get(key, "no-shape-row")
        rows[(function, line)].add((position, label, texts))
    seen = set()
    for (function, line), entries in rows.items():
        for position, label, texts in entries:
            if (function, line, texts, position) in seen:
                continue
            seen.add((function, line, texts, position))
            tally[(position, label)] += 1
            tally_functions[(position, label)].add(function)

    # Functions and their leaves.
    functions = {}
    defined = collections.Counter()
    for unit in report["units"]:
        for entry in unit.get("functions", []):
            key = (unit["file"], entry["function"])
            if key not in functions:
                defined[entry["function"]] += 1
            record = functions.setdefault(key, {"status": entry["status"], "leaves": []})
            if ORDER[entry["status"]] > ORDER[record["status"]]:
                record["status"] = entry["status"]
            record["leaves"].extend(entry.get("diagnostics", []))

    by_function = collections.defaultdict(list)
    for (name, line), entries in rows.items():
        by_function[name].append((line, entries))

    def leaf_shapes(function, leaves):
        """Map each access leaf index to the set of shapes of its rows."""
        lines = sorted({leaf["line"] for leaf in leaves
                        if family(leaf) in ("COND", "RET", "IND")})
        owned = collections.defaultdict(set)
        for line, entries in by_function.get(function, ()):
            candidates = [l for l in lines if l <= line <= l + CONTINUATION]
            if not candidates:
                continue
            owner = max(candidates)
            for _, label, _ in entries:
                owned[owner].add(label)
        return owned

    alone = {"strict": collections.defaultdict(list), "with-alias": collections.defaultdict(list)}
    unknown = 0
    for (file, function), record in sorted(functions.items()):
        if record["status"] != "Unknown":
            continue
        unknown += 1
        leaves = record["leaves"]
        owned = leaf_shapes(function, leaves)
        labels = set()
        blocked_elsewhere = False
        alias = False
        for leaf in leaves:
            name = family(leaf)
            if name == "unknown_alias":
                alias = True
                continue
            if name in ("COND", "RET", "IND") and owned.get(leaf["line"]):
                labels |= owned[leaf["line"]]
                continue
            blocked_elsewhere = True
        if blocked_elsewhere or len(labels) != 1:
            continue
        label = next(iter(labels))
        entry = f"{file}::{function}"
        alone["with-alias"][label].append(entry)
        if not alias:
            alone["strict"][label].append(entry)

    ranking = sorted(
        ({"shape": label,
          "alone_strict": len(alone["strict"].get(label, [])),
          "alone_with_alias": len(alone["with-alias"].get(label, [])),
          "rows": sum(v for (p, l), v in tally.items() if l == label),
          "rows_by_position": {p: v for (p, l), v in tally.items() if l == label}}
         for label in {l for (_, l) in tally}),
        key=lambda r: (-r["alone_with_alias"], -r["alone_strict"], -r["rows"], r["shape"]))
    result = {
        "run": str(run),
        "refused_rows": len(seen),
        "refused_rows_without_shape_row": len(missing),
        "unknown_functions": unknown,
        "function_names_defined_in_several_units": sum(1 for v in defined.values() if v > 1),
        "rows_by_position": dict(collections.Counter(p for (_, _, _, _, p) in refused)),
        "ranking": ranking,
        "tally": [{"position": p, "shape": l, "rows": v, "functions": len(tally_functions[(p, l)])}
                  for (p, l), v in sorted(tally.items(), key=lambda kv: -kv[1])],
        "alone": {variant: {label: sorted(names) for label, names in table.items()}
                  for variant, table in alone.items()},
    }
    out.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    print(f"refused rows {len(seen)} (without shape row {len(missing)}), Unknown functions {unknown}")
    for row in ranking[:40]:
        print(f"{row['alone_with_alias']:5d} {row['alone_strict']:5d} {row['rows']:6d}  {row['shape']}  {row['rows_by_position']}")


if __name__ == "__main__":
    main()
