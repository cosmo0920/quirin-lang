#!/usr/bin/env python3
"""breakdown.py -- NONNULL-MEMBER-UNIFIED-QUALIFIER: the corpus rows resting
on a premised plain member (artifacts/compare.json), by member, by whether
they already listed the premise through another fact, by null verdict and by
file; the five members' fold records."""
import collections
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
data = json.loads((HERE.parent / "artifacts/compare.json").read_text())["corpus"]["null"]
rows = data["resting_rows"]
out = {
    "by_member": data["resting_by_member"],
    "gained_by_verdict": dict(collections.Counter(r["verdict"] for r in rows if not r["listed_before"])),
    "already_listed_by_verdict": dict(collections.Counter(r["verdict"] for r in rows if r["listed_before"])),
    "provisional_unsafe": sorted({(r["function"], r["file"]) for r in rows if r["verdict"] == "Unsafe"}),
    "by_file": collections.Counter(r["file"].split("/")[-1] for r in rows).most_common(),
    "files": len({r["file"] for r in rows}),
    "member_declaration_premises": data["member_declaration_premises_after"],
}
(HERE.parent / "artifacts/breakdown.json").write_text(json.dumps(out, indent=1) + "\n")
print(json.dumps({k: v for k, v in out.items() if k != "by_file"}, indent=1))
print("by_file (top 6):", out["by_file"][:6])
