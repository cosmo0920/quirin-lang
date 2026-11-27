#!/usr/bin/env python3
"""Write a `baseline_identity.py`-shaped manifest for the W46 combined corpus.

Usage: baseline_manifest.py COMBINED_ARTIFACT_DIR OUT_DIR

The combined archive's manifest keys its files as `corpus/<name>`; the W43
identity checker expects the five artifact labels. The written manifest points
at the archived gzip files by absolute path and copies their raw SHA-256s, so
the checker still verifies each archive against its own recorded digest.
"""

import json
import pathlib
import sys

LABELS = {
    "diagnostics": "resource-diagnostics.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "source": "monkey-source.json",
}

source = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
manifest = json.loads((source / "manifest.json").read_text())
result = {}
for label, name in LABELS.items():
    entry = manifest[f"corpus/{name}"]
    result[label] = {"file": str(source / entry["file"]), "raw_sha256": entry["raw_sha256"]}
(out / "manifest.json").write_text(json.dumps(result, indent=1) + "\n")
