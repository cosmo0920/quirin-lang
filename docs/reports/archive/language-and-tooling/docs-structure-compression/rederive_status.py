#!/usr/bin/env python3
"""Re-derive the status snapshot at this card's checkpoint from primary inputs."""
from pathlib import Path
from collections import Counter
import gzip
import json
import sys

ROOT = Path(__file__).resolve().parents[3]
REPORT = Path(__file__).resolve().parent

if not (ROOT / "contract/qcore").is_dir():
    raise SystemExit("Historical checkpoint only: contract/qcore no longer exists. "
                     "Use docs/STATUS.md for current measurements; replay this report at its recorded revision.")


def read(path):
    return json.loads((ROOT / path).read_text())


cases = read("docs/reevaluation/ct_manifest.json")["cases"]
result = {"ct": {"mapping": dict(Counter(c["mapping_status"] for c in cases)),
                 "unmapped": [c["id"] for c in cases if c["mapping_status"] != "mapped"]}}
for stage in ("proof", "native_sanitized"):
    rows = [c for c in cases if c.get("stages", {}).get(stage, {}).get("status") == "executed"]
    result["ct"][stage] = {"ids": [c["id"] for c in rows],
                           "host_gates": {c["id"]: c["stages"][stage].get("host_gate") for c in rows}}
source = "docs/reports/w37-corpus-contracts-4/artifacts/diagnostics.json.gz"
with gzip.open(ROOT / source) as stream:
    data = json.load(stream)
result["corpus"] = {"source": source,
                    "unit_statuses": dict(Counter(u["status"] for u in data["units"])),
                    "function_statuses": dict(Counter(f["status"] for u in data["units"] for f in u["functions"])),
                    "diagnostic_counts": data["diagnostic_counts"]}
contracts = list((ROOT / "contract/qcore").rglob("*.qcore"))
result["contracts"] = {"modules": len(contracts),
                       "declarations": sum(p.read_text().count("requires resource_contract_v1;") for p in contracts),
                       "selected": len(read("contract/cycle19-resource-contracts.json")["modules"])}
matrix = read("docs/reviews/r0_3/acceptance_matrix.json")
result["followups"] = {"carried": len(matrix["named_followups"]),
                       "derived": matrix["dimensions"]["named_followups"]}
result["last_merged_gate"] = read("docs/reports/wave40-close/gate.json")
result["lean_audits"] = len(list((ROOT / "proof").glob("Audit*.lean")))
target = REPORT / "status-sources.json"
if "--check" in sys.argv:
    assert json.loads(target.read_text()) == result, "status sources moved since this checkpoint"
    print("Status snapshot matches primary inputs")
else:
    target.write_text(json.dumps(result, indent=2) + "\n")
    print("Wrote status-sources.json")
