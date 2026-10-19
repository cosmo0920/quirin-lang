#!/usr/bin/env python3
"""Restore and verify the exact main baseline from the checked-detach archive."""
from pathlib import Path
import gzip
import hashlib
import json
import sys

REPO = Path(__file__).resolve().parents[4]
ARCHIVE = REPO / "docs/reports/cjson-detach-same-item/artifacts"
NAMES = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}


def restore(source: Path, target: Path, expected: dict) -> None:
    data = gzip.decompress(source.read_bytes())
    assert len(data) == expected["bytes"], source
    assert hashlib.sha256(data).hexdigest() == expected["sha256"], source
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)


def main() -> None:
    destination = Path(sys.argv[1]).resolve()
    assert not destination.exists(), "baseline hydration must have a fresh destination"
    destination.mkdir(parents=True)
    for route in ("corpus", "system", "slice"):
        archived = ARCHIVE / route / "after"
        manifest = json.loads((archived / "manifest.json").read_text())
        target = destination / route
        diagnostics = target / "corpus" if route == "corpus" else target
        for stem, name in NAMES.items():
            entry = manifest[stem]
            if entry["present"]:
                restore(archived / f"{stem}.json.gz", diagnostics / name, entry)
        for name in (f"{route}-null.jsonl", f"{route}-outer.json"):
            restore(archived / f"{name}.gz", target / name, manifest[name])
        (target / "baseline-provenance.json").write_text(json.dumps({
            "source": str(archived), "manifest": manifest,
            "declared_main_base": "b5a00e7dc23628f9b9317ab5e8f2fb5d7ec365d7",
        }, indent=2) + "\n")
    print(destination)


if __name__ == "__main__":
    main()
