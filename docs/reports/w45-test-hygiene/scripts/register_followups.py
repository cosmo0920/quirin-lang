#!/usr/bin/env python3
"""Register W45-TEST-HYGIENE's follow-up in the committed acceptance matrix,
in the generator's byte format (indent=1, ensure_ascii=False, trailing newline),
keeping `dimensions.named_followups` equal to the section's length."""
import json
import sys
from pathlib import Path

ROOT = Path(sys.argv[1]).resolve()
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"

ENTRY = {
    "id": "W45-PRODUCTION-WALKS-READ-WORKING-TREE",
    "title": "Production and validation readers that tests exercise on the real tree read the working tree, not the tracked files",
    "why": (
        "W45-TEST-HYGIENE (validation constraint 29) moved every test-side repository walk to tracked "
        "files; the readers the tests call still read the directory. An untracked or ignored module in a "
        "contract root makes `qcore_contract::load_path_manifest::qcore_files` refuse the load path "
        "(QI014), failing 21 tests in five compiler test binaries after the card as before it "
        "(`docs/reports/w45-test-hygiene/README.md`, set B). Size: 9 readers: `load_path_manifest` "
        "`qcore_files`; `compiler/src/first_party.rs` profile discovery; `scripts/contract_corpus.py`'s "
        "default walk as `build_matrix.py` uses it; `build_matrix.py`'s `proof/Audit*.lean` and "
        "provider-template globs; `scripts/reconciliation_sweep.py`'s report, rule and increment scans; "
        "`scripts/build_docs_index.py`; `scripts/check_lean_audits.py`'s `Audit*.lean` glob; "
        "`providers/contracts.py`; `providers/package.py`'s template glob and contract copy. The two "
        "loaders read an installed tree by design and refuse fail-closed; the repository generators can "
        "publish an untracked file's contribution. DISCHARGE: per reader, either it reads tracked files "
        "when run on a repository checkout (the generators and package build), or a recorded decision "
        "that it reads its directory by design with its fail-closed behaviour stated (the loaders)."
    ),
    "source": "docs/reports/w45-test-hygiene/README.md",
    "recorded_by": "W45-TEST-HYGIENE",
}

matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
ids = {entry["id"] for entry in matrix["named_followups"]}
if ENTRY["id"] in ids:
    sys.exit(f"{ENTRY['id']} already registered")
matrix["named_followups"].append(ENTRY)
matrix["dimensions"]["named_followups"] = len(matrix["named_followups"])
MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print("named_followups", matrix["dimensions"]["named_followups"])
