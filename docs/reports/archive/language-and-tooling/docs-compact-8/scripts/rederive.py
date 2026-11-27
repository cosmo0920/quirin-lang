#!/usr/bin/env python3
"""Re-derive every headline figure from its own source, so a rebase re-derives
rather than rebasing the numbers. Prints one line per figure."""
import collections
import gzip
import json
import os
import pathlib
import subprocess
import tomllib

W = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a9936abd188804098"
)


if not (W / "contract/qcore").is_dir():
    raise SystemExit("Historical checkpoint only: set W to the report revision checkout "
                     "containing contract/qcore. Use docs/STATUS.md for current measurements.")

def out(label, value):
    print(f"{label:44s} {value}")


d = json.load(gzip.open(W / "docs/reports/w37-corpus-contracts-4/artifacts/diagnostics.json.gz"))
units = d["units"]
out("unit statuses", dict(collections.Counter(u["status"] for u in units)))
fc = collections.Counter()
for u in units:
    for f in u["functions"]:
        fc[f["status"]] += 1
out("functions", f"{sum(fc.values())} {dict(fc)}")
out("diagnostic_counts", json.dumps(d["diagnostic_counts"], sort_keys=True))
out("flb_input_chunk_create", next(
    f["status"] for u in units for f in u["functions"]
    if f.get("function") == "flb_input_chunk_create"))

m = json.load(open(W / "docs/reevaluation/ct_manifest.json"))
cases = m["cases"]
out("CT mapping", dict(collections.Counter(c["mapping_status"] for c in cases)))
out("CT unmapped", [c["id"] for c in cases if c["mapping_status"] != "mapped"])
for stage in ("native_sanitized", "proof"):
    ids = [c["id"] for c in cases
           if (c.get("stages") or {}).get(stage, {}).get("status") == "executed"]
    out(f"CT {stage} executed", f"{len(ids)} {ids}")

out("contract modules", sum(
    1 for _, _, fs in os.walk(W / "contract/qcore") for f in fs if f.endswith(".qcore")))
decl = subprocess.run(
    ["grep", "-rhc", "requires resource_contract_v1;", "--include=*.qcore", "contract/qcore"],
    cwd=W, capture_output=True, text=True)
out("resource-v1 declarations", sum(int(x) for x in decl.stdout.split()))
out("manifest-selected", len(json.load(open(W / "contract/cycle19-resource-contracts.json"))["modules"]))

out("crates/ dirs", len(os.listdir(W / "crates")))
out("workspace members", len(tomllib.load(open(W / "Cargo.toml", "rb"))["workspace"]["members"]))
out("providers members", len(tomllib.load(open(W / "providers/Cargo.toml", "rb"))["workspace"]["members"]))
for rel in ("crates/qcore-schemas", "compiler", "crates/qcore-verifier-host",
            "crates/qcore-verification"):
    t = tomllib.load(open(W / rel / "Cargo.toml", "rb"))
    out(f"version {rel}", t["package"]["version"])
out("ChunkIO package_version", json.load(
    open(W / "providers/qcore-provider-chunkio/provider-template.json"))["package_version"])

out("Audit*.lean on disk", len(list((W / "proof").glob("Audit*.lean"))))
out("lean-toolchain", (W / "proof/lean-toolchain").read_text().strip())
out("examples families", len([p for p in (W / "examples").iterdir() if p.is_dir()]))
out("reserved source words", len(
    (W / "measure/reserved-words.txt").read_text().split()))

mx = json.load(open(W / "docs/reviews/r0_3/acceptance_matrix.json"))
out("named_followups carried", len(mx["named_followups"]))
out("named_followups derived", mx["dimensions"]["named_followups"])
out("named_followups mine", [e["id"] for e in mx["named_followups"]
                             if e.get("recorded_by") == "DOCS-COMPACT-8"])

for f in ("AGENTS.md", "CLAUDE.md", "README.md"):
    out(f"bytes {f}", (W / f).stat().st_size)
