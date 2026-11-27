#!/usr/bin/env python3
"""Card W45-SUB-OBJECT-STORES: the leaf and function forecast from the census.

Usage: forecast.py STORES_JSONL STATEMENTS_JSONL DIAGNOSTICS_JSON [--json OUT]

Reads the same streams as store_census.py and reuses its join, de-duplication
and forecast classes. A gate leaf (a `(unit, function, line)` carrying the
`indirect memory access` refusal) is forecast to leave when every
gate-refused statement the statement census recorded at that line is a store
record whose forecast is `clear` (lower bound) or `clear`/`flow` (upper
bound). A function is forecast to move to `Safe` when it is Unknown and every
one of its diagnostics is such a leaf.
"""

import collections
import gzip
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import store_census as census  # noqa: E402


def main():
    stores_path, statements_path, diagnostics_path = sys.argv[1:4]
    json_out = sys.argv[sys.argv.index("--json") + 1] if "--json" in sys.argv else None
    document = census.read_json(diagnostics_path)
    stores_raw = census.load(stores_path)
    statements_raw = census.load(statements_path)
    mapping = census.join_units(stores_raw + statements_raw, document)
    stores = census.dedupe(stores_raw, mapping)
    statements = census.dedupe(statements_raw, mapping)
    forecast = {}
    for key, record in stores.items():
        gate = record["outcome"] == "Unknown" and record["description"] == census.GATE
        forecast[key] = census.forecast_class(record) if gate and not record["comparison"] else "refused"
    gate_statements = collections.defaultdict(list)
    for key, record in statements.items():
        if record["outcome"] == census.GATE:
            gate_statements[key[:3]].append(key)

    def line_leaves(line_key, admitted):
        keys = gate_statements.get(line_key, [])
        return bool(keys) and all(forecast.get(key) in admitted for key in keys)

    out = {}
    for label, admitted in (("clear", {"clear"}), ("upper", {"clear", "flow"})):
        leaves = collections.Counter()
        movers = []
        for index, unit in enumerate(document["units"]):
            for function in unit["functions"]:
                gate_lines = {
                    d["line"] for d in function["diagnostics"] if d["description"] == census.GATE
                }
                gone = {line for line in gate_lines if line_leaves((index, function["function"], line), admitted)}
                leaves[function["status"]] += len(gone)
                others = [
                    d
                    for d in function["diagnostics"]
                    if not (d["description"] == census.GATE and d["line"] in gone)
                ]
                if function["status"] == "Unknown" and gone and not others:
                    movers.append(f"{unit['file']}::{function['function']}")
        out[label] = {"leaves_by_status": dict(leaves), "leaves": sum(leaves.values()), "movers": sorted(movers)}
    text = json.dumps(out, indent=1)
    print(text)
    if json_out:
        pathlib.Path(json_out).write_text(text + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
