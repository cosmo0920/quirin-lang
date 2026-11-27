#!/usr/bin/env python3
"""rebase_audit.py DIFF -- PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS:
the audit_card_lines.sh check in Python. DIFF is
`git diff --output=DIFF <old base> <pre-rebase tip> -- <record files>`; every
non-empty line the card added there must still be present in the rebased
file. Also checks the ledger: the card's entry is gone, its two
registrations are present once each, and the list length equals
dimensions.named_followups."""
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
diff = Path(sys.argv[1]).read_text().splitlines()
current = None
added = 0
missing = []
for line in diff:
    if line.startswith("+++ b/"):
        current = line[6:]
        continue
    if line.startswith("+") and not line.startswith("+++") and current and line[1:].strip():
        added += 1
        if line[1:] not in (REPO / current).read_text():
            missing.append((current, line[1:121]))
print(f"card-added record lines: {added}; missing after the rebase: {len(missing)}")
for item in missing:
    print("  missing:", item)
matrix = json.loads((REPO / "docs/reviews/r0_3/acceptance_matrix.json").read_text())
ids = [entry["id"] for entry in matrix["named_followups"]]
ok = (
    "PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS" not in ids
    and ids.count("PROVIDER-PACKAGE-PY-WRITES-EXECUTABLE-IN-PROCESS") == 1
    and ids.count("PREPARE-COPY-KEEPS-EXECUTABLE-MODE-IN-PROCESS") == 1
    and len(ids) == len(set(ids)) == matrix["dimensions"]["named_followups"]
)
print(f"ledger: {len(ids)} entries, dimensions.named_followups "
      f"{matrix['dimensions']['named_followups']}, card entries as expected: {ok}")
sys.exit(0 if ok and not missing else 1)
