#!/usr/bin/env python3
"""The syntactic reach of the element rules on the corpus, and what they did.

Usage: element_reach.py CENSUS_RUN AFTER_RUN BEFORE_RUN OUT_JSON OUT_TXT

CENSUS_RUN is a `--census` run of a binary built with the `indirect-census`
feature; its `statement-census.jsonl` carries one record per element operation
the classifier lowered (`element operation over ...; fallback: ...`) and per
element null test (`element null test over ...`). Records are de-duplicated on
(unit, function, line, outcome), since W4 derivation, validation and the final
analysis each classify a body. For every lowered operation the script checks,
in AFTER and BEFORE, that the leaf at that line carrying the fallback's
description is present in both: an operation whose array resolved would have
removed it.
"""

import collections
import json
import pathlib
import re
import sys


def main() -> int:
    census, after, before = (pathlib.Path(arg) for arg in sys.argv[1:4])
    out_json, out_txt = pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5])
    records = set()
    with open(census / "statement-census.jsonl") as handle:
        for line in handle:
            record = json.loads(line)
            if not record["outcome"].startswith("element "):
                continue
            records.add((record["unit_seq"], record["function"], record["line"], record["outcome"]))

    def leaves(run):
        rows = collections.Counter()
        for unit in json.loads((run / "resource-diagnostics.json").read_text())["units"]:
            for function in unit.get("functions", []):
                for leaf in function.get("diagnostics", []):
                    rows[(function["function"], leaf["line"], leaf["description"])] += 1
        return rows

    after_leaves, before_leaves = leaves(after), leaves(before)
    rows = []
    kept = 0
    for unit_seq, function, line, outcome in sorted(records, key=lambda r: (r[1], r[2], r[3])):
        fallback = re.sub(r"^.*; fallback: ", "", outcome)
        base = re.search(r"over `([^`]*)`", outcome).group(1)
        present_after = after_leaves[(function, line, fallback)] > 0
        present_before = before_leaves[(function, line, fallback)] > 0
        kept += present_after and present_before
        rows.append({
            "function": function, "line": line, "base": base,
            "kind": "null test" if outcome.startswith("element null test") else "store or release",
            "fallback": fallback, "fallback_leaf_before": present_before,
            "fallback_leaf_after": present_after,
        })
    functions = sorted({row["function"] for row in rows})
    summary = {
        "distinct_lowered_operations": len(rows),
        "null_tests": sum(1 for row in rows if row["kind"] == "null test"),
        "stores_or_releases": sum(1 for row in rows if row["kind"] != "null test"),
        "functions": len(functions),
        "fallback_leaf_kept_in_both_runs": kept,
    }
    out_json.write_text(json.dumps({"summary": summary, "operations": rows}, indent=1) + "\n")
    lines = ["W43-INDEXED-RESOURCE-COLLECTIONS: element operations lowered on the corpus", ""]
    lines += [f"{key}: {value}" for key, value in summary.items()]
    lines.append("")
    for row in rows:
        lines.append(
            f"  {row['function']} L{row['line']} {row['kind']} over `{row['base']}`: fallback "
            f"`{row['fallback']}` before={row['fallback_leaf_before']} after={row['fallback_leaf_after']}"
        )
    out_txt.write_text("\n".join(lines) + "\n")
    print("\n".join(lines[:8]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
