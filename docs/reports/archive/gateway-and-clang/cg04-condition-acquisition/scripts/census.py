#!/usr/bin/env python3
"""CG04-CONDITION-ACQUISITION: the condition-desugar census from a parity stream.

Usage: census.py PARITY_JSONL [--json OUT]

Reads the `QCORE_TYPED_PATH_PARITY` stream of a corpus run and keeps the rows
this card's adapter writes (`position: "condition-desugar"`), one per
consultation of the desugaring. The analyzer parses a function several times
(derivation rounds, validation, the final analysis), so rows are keyed by
(analysed source, function, line, tokens) and the last row per key is kept,
the final analysis's. Prints the outcome classes: admitted (with the widening
of rule 10x each admission took), and refused by refusal variant, with the
number of distinct consultations and functions.
"""

import argparse
import collections
import json
import pathlib


def rows(path: pathlib.Path):
    kept = {}
    with path.open() as stream:
        for line in stream:
            if '"condition-desugar"' not in line:
                continue
            row = json.loads(line)
            if row.get("position") != "condition-desugar":
                continue
            key = (row.get("source_sha256"), row["function"], row["line"], tuple(row["texts"]))
            kept[key] = row
    return kept


def classify(row) -> str:
    if row.get("new_admitted"):
        widened = [name for name in ("split", "type_operator", "elaborated_leaf")
                   if row.get(f"widened_{name}")]
        return "admitted" + (":" + "+".join(widened) if widened else "")
    refusal = row.get("new_refusal") or "not-kept"
    variant = refusal.split("(")[0]
    if variant in ("LeafShape", "CallRefused"):
        if variant == "CallRefused":
            reason = refusal.split('", "', 1)[-1].rstrip('")')
            return f"CallRefused: {reason}"
        return refusal
    return variant


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("parity", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    kept = rows(args.parity)
    classes = collections.defaultdict(lambda: {"consultations": 0, "functions": set()})
    for (source, function, _, _), row in kept.items():
        entry = classes[classify(row)]
        entry["consultations"] += 1
        entry["functions"].add((source, function))
    record = {
        name: {"consultations": entry["consultations"], "functions": len(entry["functions"])}
        for name, entry in sorted(classes.items(), key=lambda item: -item[1]["consultations"])
    }
    for name, entry in record.items():
        print(f"{entry['consultations']:6} {entry['functions']:5}  {name}")
    if args.json:
        args.json.write_text(json.dumps(record, indent=1, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
