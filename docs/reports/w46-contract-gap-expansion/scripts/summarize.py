#!/usr/bin/env python3
"""Summarize a diagnostic artifact without collapsing compile configurations.

The separate unique-function row joins each (source file, function) across
configurations, using Safe < Pending < Unknown < Unsafe. Exclusions stay
separate. This is a scope census, not a new verdict or certificate.
"""

import argparse
import collections
import gzip
import json
import pathlib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    opener = gzip.open if args.input.suffix == ".gz" else open
    with opener(args.input, "rt") as stream:
        data = json.load(stream)
    rank = {name: i for i, name in enumerate(["Safe", "Pending", "Unknown", "Unsafe"])}
    configurations = collections.Counter()
    unique = collections.defaultdict(set)
    exclusions = set()
    held = []
    unc_leaves = unc_unknown_leaves = unc_functions = unc_only_functions = 0
    for ordinal, unit in enumerate(data["units"]):
        for item in unit.get("not_projected", []):
            exclusions.add((unit["file"], json.dumps(item, sort_keys=True)))
        for function in unit["functions"]:
            status = function["status"]
            if status not in rank:
                raise SystemExit(f"unexpected function status {status!r}")
            configurations[status] += 1
            unique[(unit["file"], function["function"])].add(status)
            diagnostics = function.get("diagnostics") or []
            unc = [d for d in diagnostics if d["description"].startswith("call to uncontracted function `")]
            unc_leaves += len(unc)
            if status == "Unknown" and unc:
                unc_unknown_leaves += len(unc)
                unc_functions += 1
                unc_only_functions += len(unc) == len(diagnostics)
            for diagnostic in diagnostics:
                if "contract-refuted-by-body" in diagnostic["description"]:
                    held.append(dict(unit_index=ordinal, file=unit["file"],
                                     function=function["function"], diagnostic=diagnostic))
    result = {
        "input": str(args.input), "scope": data.get("scope"),
        "unit_rows": len(data["units"]),
        "unit_statuses": dict(collections.Counter(u["status"] for u in data["units"])),
        "function_configuration_statuses": dict(configurations),
        "unique_function_statuses_joined": dict(collections.Counter(max(s, key=rank.get) for s in unique.values())),
        "unique_functions": len(unique), "unique_not_projected_records": len(exclusions),
        "configuration_disagreements": [dict(file=k[0], function=k[1], statuses=sorted(v))
                                        for k, v in sorted(unique.items()) if len(v) > 1],
        "diagnostic_counts": data["diagnostic_counts"],
        "uncontracted_leaves_all_statuses": unc_leaves,
        "uncontracted_leaves_on_unknown_functions": unc_unknown_leaves,
        "unknown_functions_with_uncontracted_leaves": unc_functions,
        "unknown_functions_with_only_uncontracted_leaves": unc_only_functions,
        "held_refutation_diagnostics": held,
    }
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({k: v for k, v in result.items() if k not in ["held_refutation_diagnostics", "scope"]}, indent=2))


if __name__ == "__main__":
    main()
