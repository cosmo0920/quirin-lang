#!/usr/bin/env python3
"""Write a `baseline_identity.py`-shaped manifest for a W47 corpus archive.

Usage: label_manifest.py ARCHIVE_DIR OUT_DIR

W47 archives (archive.py) key their manifest by file name; the W43 identity
checker expects the five artifact labels. The written manifest points at the
archived gzip files by absolute path with their raw SHA-256s, so the checker
still verifies each archive against its own recorded digest.
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
result = {label: {"file": str(source / manifest[name]["file"]), "raw_sha256": manifest[name]["raw_sha256"]}
          for label, name in LABELS.items()}
(out / "manifest.json").write_text(json.dumps(result, indent=1) + "\n")
