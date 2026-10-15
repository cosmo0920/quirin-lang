#!/usr/bin/env python3
"""rows_table.py ROWS.json NAME OUT.json -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED:
from premise_rows.py's output, keep run NAME's per-route counts and its Safe /
Pending rows resting on the 34 (an artifact), and print those rows as
Markdown table lines."""
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text())
name = sys.argv[2]
out = {}
for route, entry in data.items():
    e = entry[name]
    out[route] = {k: e[k] for k in ("sha256", "rows_resting_on_the_34", "by_status_and_premise",
                                     "carrying_premise_in_sidecar", "body_standing", "counts",
                                     "safe_rollup")}
    out[route]["safe_or_pending_rows"] = [r for r in e["rows"] if r["status"] in ("Safe", "Pending")]
    if "comparison" in entry:
        out[route]["comparison"] = entry["comparison"]
pathlib.Path(sys.argv[3]).write_text(json.dumps(out, indent=1) + "\n")
for route, e in out.items():
    for r in e["safe_or_pending_rows"]:
        unit = r["unit"].split("lib/librdkafka-2.15.0/src/")[-1]
        decls = ", ".join(f"`{x}`" for x in r["declarations_of_the_34"])
        via = ", ".join(f"`{x}`" for x in r["declarations_carrying_the_premise"]) or "none"
        print(f"| {route} | `{unit}` | `{r['function']}` | {r['status']} | {decls} | {via} |")
