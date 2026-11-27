#!/usr/bin/env python3
"""Card W43-INDIRECT-ACCESS-GENERAL: validate C-GATEWAY-INDIRECT-CENSUS-2's
three-way proxy against this card's measured admission.

The proxy (`docs/reports/c-gateway-indirect-census-2/scripts/analyse.py`)
predicts, per census shape S = (base kind, premise, steps, member class,
position), which functions admitting S would leave leaf-free (CLEAR),
exchanged for another refusal (EXCH) or unmoved. It has never met an actual
admission (`INDIRECT-EXCHANGE-PROXY-UNVALIDATED`).

This card admits a *typed* rule, not a census shape, so the comparison is made
per census shape over the functions the proxy would clear or exchange, and for
each such function the measured outcome is read from AFTER:

* `cleared` - the function holds no `indirect memory access` leaf in AFTER and
  gained no `unresolved resource identity` leaf at a coordinate where it held
  one before;
* `exchanged` - its indirect leaves are gone but an unknown-alias leaf appeared
  at one of their coordinates;
* `unmoved` - it still holds an indirect leaf.

Only shapes at least one of whose statements this card's forecast mirror
admits are compared: for any other shape the admission is not the proxy's
premise and no disagreement is meaningful.

Usage: proxy_validation.py CENSUS_RUN AFTER_RUN OUT_JSON OUT_TXT
"""

import collections
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import shape_census as census  # noqa: E402
import typed_scalar  # noqa: E402

STEP = {"Arrow": "->", "Dot": ".", "Subscript": "[]", "Deref": "*"}


def census2_shape(record):
    chains = record["chains"]
    if not chains:
        return ("no-chain", "-", "-", "-", record["position"])
    worst = max(chains, key=lambda c: (c["base_may_bind"], c["depth"]))
    return (
        worst["base_kind"],
        "may-bind" if worst["base_may_bind"] else "never-bound",
        "".join(STEP[s] for s in worst["steps"]) or "-",
        worst["member_class"],
        record["position"],
    )


def chain_shape(chain, record):
    return (
        chain["base_kind"],
        "may-bind" if chain["base_may_bind"] else "never-bound",
        "".join(STEP[s] for s in chain["steps"]) or "-",
        chain["member_class"],
        record["position"],
    )


def main():
    run = pathlib.Path(sys.argv[1])
    after = json.load(open(pathlib.Path(sys.argv[2]) / "resource-diagnostics.json"))
    out_json = pathlib.Path(sys.argv[3])
    out_txt = pathlib.Path(sys.argv[4])
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    assignment = census.match_units([indirect], unit_keys)

    after_leaves = {}
    for unit in after["units"]:
        ukey = (unit.get("library"), unit["file"])
        for function in unit["functions"]:
            after_leaves[ukey + (function["function"],)] = [(d["line"], d["description"]) for d in function["diagnostics"]]

    own = collections.defaultdict(list)
    touched = collections.defaultdict(set)
    admitted_shapes = set()
    for fkey, data in functions.items():
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            if description != census.INDIRECT or seq is None:
                continue
            for record in indirect.get(seq, {}).get((fkey[2], line, description), []):
                shape = census2_shape(record)
                own[fkey].append(record)
                touched[shape].add(fkey)
                if typed_scalar.classify(record):
                    admitted_shapes.add(shape)

    def blockers(fkey):
        return {census.normalize(d) for _, d in functions[fkey]["leaves"]}

    def exchanges(record, candidate):
        if record["contains_call"] and not record.get("call_contracted"):
            return True
        return any(chain_shape(c, record) != candidate for c in record["chains"])

    def measured(fkey):
        before_lines = {l for l, d in functions[fkey]["leaves"] if d == census.INDIRECT}
        leaves = after_leaves.get(fkey, [])
        if any(d == census.INDIRECT for _, d in leaves):
            # Separate the typed rule's partiality from the proxy's error: a
            # statement of the shape the rule does not admit keeps its leaf
            # whatever the proxy says.
            if any(not typed_scalar.classify(r) for r in own[fkey]):
                return "unmoved (a statement is outside the typed rule)"
            return "unmoved"
        if any(d.startswith("unresolved resource identity") and l in before_lines for l, d in leaves):
            return "exchanged"
        return "cleared"

    rows = []
    agreement = collections.Counter()
    for shape in sorted(admitted_shapes, key=lambda s: -len(touched[s])):
        predicted = {}
        for fkey in touched[shape]:
            if functions[fkey]["status"] != "Unknown" or blockers(fkey) != {census.INDIRECT}:
                predicted[fkey] = "unmoved"
            elif any(census2_shape(r) != shape for r in own[fkey]):
                predicted[fkey] = "unmoved"
            elif any(exchanges(r, shape) for r in own[fkey]):
                predicted[fkey] = "exchanged"
            else:
                predicted[fkey] = "cleared"
        table = collections.Counter()
        for fkey, p in predicted.items():
            if p == "unmoved":
                continue
            m = measured(fkey)
            table[(p, m)] += 1
            agreement[(p, m)] += 1
        rows.append({
            "shape": list(shape),
            "functions_touched": len(touched[shape]),
            "predicted": dict(collections.Counter(predicted.values())),
            "predicted_vs_measured": {f"{p} -> {m}": n for (p, m), n in sorted(table.items())},
        })
    result = {
        "compared_shapes": len(rows),
        "overall_predicted_vs_measured": {f"{p} -> {m}": n for (p, m), n in sorted(agreement.items())},
        "rows": rows,
    }
    out_json.write_text(json.dumps(result, indent=1) + "\n")
    lines = [f"shapes compared: {len(rows)}", "overall (predicted -> measured), over functions the proxy calls CLEAR or EXCH:"]
    for k, v in result["overall_predicted_vs_measured"].items():
        lines.append(f"  {v:5}  {k}")
    lines.append("")
    for row in rows:
        if not row["predicted_vs_measured"]:
            continue
        lines.append(f"{' | '.join(row['shape'])}: touched {row['functions_touched']}, {row['predicted_vs_measured']}")
    out_txt.write_text("\n".join(lines) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
