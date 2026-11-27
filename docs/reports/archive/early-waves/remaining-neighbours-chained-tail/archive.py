#!/usr/bin/env python3
"""Archive one produced 185-unit report set for card NEIGHBOURS-CHAINED-TAIL.

Read-only over the produced reports; it writes the four gzipped artifacts and a
manifest carrying each one's uncompressed SHA-256, in exactly the layout the
`docs/reports/*/artifacts/manifest.json` convention already uses, so
`docs/reports/corpus-repin/diff_artifacts.py --archived` can read it back.

    python3 -B docs/reports/remaining-neighbours-chained-tail/archive.py \\
        --measured measure/after \\
        --output docs/reports/remaining-neighbours-chained-tail/artifacts
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import pathlib
import sys

# The archived name -> the file a produced report set writes.
NAMES = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--measured", required=True, type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    arguments = parser.parse_args()
    arguments.output.mkdir(parents=True, exist_ok=True)

    manifest = {}
    for name, produced in sorted(NAMES.items()):
        source = arguments.measured / produced
        payload = source.read_bytes()
        target = arguments.output / f"{name}.json.gz"
        with gzip.GzipFile(filename="", mode="wb", fileobj=target.open("wb"), mtime=0) as out:
            out.write(payload)
        manifest[name] = {
            "archived_path": str(target),
            "produced_file": produced,
            "uncompressed_bytes": len(payload),
            "uncompressed_sha256": hashlib.sha256(payload).hexdigest(),
        }
        print(f"{name}: {len(payload)} bytes -> {target}")

    (arguments.output / "manifest.json").write_text(
        json.dumps(manifest, indent=1, sort_keys=True) + "\n"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
