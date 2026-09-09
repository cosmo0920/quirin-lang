#!/usr/bin/env python3
"""Re-add the hand-maintained ct_conformance.newly_decidable_not_mapped_reasons map.

`build_matrix.py` owns `ct_conformance` wholly and drops this key on every
regeneration — recorded as follow-on D02-GEN-CTREASONS by card HYGIENE-16 — so
it is carried forward here from the committed matrix rather than lost. Both
derivations check the *acceptance record's* copy of the same map against the
manifest; this copy is commentary and is not checked, which is exactly why the
follow-on stays open.
"""
from __future__ import annotations

import json
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]  # docs/reports/wave16-corpus/ -> repository root
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
KEY = "newly_decidable_not_mapped_reasons"

committed = json.loads(
    subprocess.run(
        ["git", "-C", str(ROOT), "show", f"HEAD:docs/reviews/r0_3/acceptance_matrix.json"],
        capture_output=True, text=True, check=True,
    ).stdout
)
carried = committed["ct_conformance"][KEY]

matrix = json.loads(MATRIX.read_text())
ct = matrix["ct_conformance"]
assert KEY not in ct, "the generator no longer drops the map; retire D02-GEN-CTREASONS"

# The map must not describe a case the manifest reports as mapped — the exact
# staleness HYGIENE-16 found. Refuse rather than carry it silently.
unmapped = set(ct["newly_decidable_not_mapped"]) | {
    row["id"] for row in ct["ct_rows"] if row.get("mapping_status") != "mapped"
}
stale = sorted(k for k in carried if not k.startswith("_") and k not in unmapped)
if stale:
    raise SystemExit(f"carried reasons map describes mapped case(s) {stale}; drop them first")

ct[KEY] = carried
MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
print(f"carried {KEY}: {sorted(k for k in carried if not k.startswith('_'))}")
