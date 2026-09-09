#!/usr/bin/env python3
"""Archive a produced 185-unit artifact set as a corpus baseline.

Wave-16 consolidation. Writes the five gzip artifacts plus `manifest.json`
carrying each artifact's uncompressed byte length and SHA-256, in the shape
every archive from CORPUS-REPIN onward uses. Read-only on its input.

    python3 -B measure/archive.py --measured measure/runs/wave16 \
        --output docs/reports/wave16-corpus/artifacts
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import pathlib

# archived name -> produced file name
ARTIFACTS = {
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "contracts": "resource-diagnostics.contracts.json",
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "source-effects": "resource-diagnostics.source-effects.json",
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--measured", required=True, type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    arguments = parser.parse_args()
    arguments.output.mkdir(parents=True, exist_ok=True)
    manifest = {}
    for archived, produced in sorted(ARTIFACTS.items()):
        raw = (arguments.measured / produced).read_bytes()
        target = arguments.output / f"{archived}.json.gz"
        # mtime=0 so the archive is reproducible from the same bytes.
        with gzip.GzipFile(filename="", mode="wb", fileobj=target.open("wb"), mtime=0) as handle:
            handle.write(raw)
        manifest[archived] = {
            "archived_path": f"docs/reports/{arguments.output.parent.name}/artifacts/{archived}.json.gz",
            "produced_file": produced,
            "uncompressed_bytes": len(raw),
            "uncompressed_sha256": hashlib.sha256(raw).hexdigest(),
        }
        print(f"{archived}: {len(raw)} bytes -> {target}")
    (arguments.output / "manifest.json").write_text(json.dumps(manifest, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
